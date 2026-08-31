use std::{
    collections::{HashMap, HashSet, VecDeque},
    env, fs,
    fs::OpenOptions,
    io,
    io::Write,
    path::{Component, Path, PathBuf},
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use crossterm::{
    clipboard::CopyToClipboard,
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use leo::{
    DerivedJob, LeoDocument, NodeId, OriginalExternalState, Outline, Position, PositionId,
    SourceLocation, WritableExternalFile, derived_filename, external_filename, external_format,
    load_derived_files, load_derived_jobs, path_directive, referenced_nodes, save_document,
    search_outline, track_external_rename,
};
use ratatui::{
    Terminal,
    backend::{CrosstermBackend, TestBackend},
    buffer::{Buffer, Cell},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use regex::{Regex, RegexBuilder};
use serde::Deserialize;

#[derive(Clone)]
struct Row {
    position: PositionId,
    node: NodeId,
    depth: usize,
    has_children: bool,
}

/// A mouse-drag text selection within the body pane, in logical (line,
/// char-column) coordinates over the node's full (unscrolled) text --
/// matching the coordinate space `body_scroll`/`body_horizontal_scroll`
/// offset into.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct BodySelection {
    anchor: (usize, usize),
    cursor: (usize, usize),
}

struct App {
    document: LeoDocument,
    path: PathBuf,
    expanded: HashSet<PositionId>,
    selected: usize,
    selection_anchor: Option<usize>,
    marked: HashSet<PositionId>,
    outline_scroll: usize,
    body_scroll: usize,
    body_page_size: usize,
    body_scroll_max: usize,
    body_horizontal_scroll: usize,
    body_horizontal_scroll_max: usize,
    body_selection: Option<BodySelection>,
    body_wrap: bool,
    body_full_width: bool,
    outline_full_width: bool,
    split_horizontal: bool,
    help: bool,
    status: String,
    flash: Option<(String, Instant)>,
    input: Option<HeadlineInput>,
    body_input: Option<BodyEdit>,
    find: Option<FindInput>,
    search: Option<FindInput>,
    palette: Option<ActionPalette>,
    action_output: Option<ActionOutput>,
    logs: VecDeque<String>,
    log_view: bool,
    log_scroll: usize,
    log_repl: Option<ReplInput>,
    /// Drag-to-select state for the log pane -- same shape as
    /// `body_selection`, kept separate since the two views are never open
    /// at once and index into different line sources.
    log_selection: Option<BodySelection>,
    dirty: bool,
    dirty_nodes: HashSet<NodeId>,
    updated_nodes: HashSet<NodeId>,
    quit_armed: bool,
    reload_armed: bool,
    clipboard: Option<ClipboardTree>,
    load_derived: bool,
    source_locations: HashMap<PositionId, SourceLocation>,
    source_nodes: HashMap<NodeId, SourceLocation>,
    derived_nodes: HashSet<NodeId>,
    writable_external: HashMap<NodeId, WritableExternalFile>,
    original_external: OriginalExternalState,
    /// Open handle for `--debug FILE`, written to by [`App::debug`] -- kept
    /// as a raw `File` rather than a buffered writer so every line lands on
    /// disk immediately, since the whole point is reading it back while the
    /// TUI might be stuck.
    debug_log: Option<fs::File>,
    #[cfg(feature = "syntax")]
    syntax: crate::syntax::SyntaxHighlighter,
    #[cfg(feature = "syntax")]
    syntax_enabled: bool,
    #[cfg(feature = "syntax")]
    highlight_cache: HashMap<PositionId, Text<'static>>,
    #[cfg(feature = "syntax")]
    preview_enabled: bool,
    #[cfg(feature = "syntax")]
    preview_cache: HashMap<PositionId, Text<'static>>,
    #[cfg(feature = "syntax")]
    wrap_before_preview: Option<bool>,
    #[cfg(feature = "syntax")]
    wrap_by_language: HashMap<String, bool>,
}

impl App {
    #[allow(clippy::too_many_arguments)]
    fn new(
        document: LeoDocument,
        path: PathBuf,
        status: String,
        source_locations: HashMap<PositionId, SourceLocation>,
        source_nodes: HashMap<NodeId, SourceLocation>,
        derived_nodes: HashSet<NodeId>,
        writable_external: HashMap<NodeId, WritableExternalFile>,
        original_external: OriginalExternalState,
        load_derived: bool,
    ) -> Self {
        let expanded = document
            .outline
            .roots
            .iter()
            .enumerate()
            .map(|(index, _)| PositionId(index.to_string()))
            .collect();
        Self {
            document,
            path,
            expanded,
            selected: 0,
            selection_anchor: None,
            marked: HashSet::new(),
            outline_scroll: 0,
            body_scroll: 0,
            body_page_size: 1,
            body_scroll_max: 0,
            body_horizontal_scroll: 0,
            body_horizontal_scroll_max: 0,
            body_selection: None,
            body_wrap: false,
            body_full_width: false,
            outline_full_width: false,
            split_horizontal: true,
            help: false,
            status,
            flash: None,
            input: None,
            body_input: None,
            find: None,
            search: None,
            palette: None,
            action_output: None,
            logs: VecDeque::new(),
            log_view: false,
            log_scroll: 0,
            log_repl: None,
            log_selection: None,
            dirty: false,
            dirty_nodes: HashSet::new(),
            updated_nodes: HashSet::new(),
            quit_armed: false,
            reload_armed: false,
            clipboard: None,
            load_derived,
            source_locations,
            source_nodes,
            derived_nodes,
            writable_external,
            original_external,
            debug_log: None,
            #[cfg(feature = "syntax")]
            syntax: crate::syntax::SyntaxHighlighter::new(),
            #[cfg(feature = "syntax")]
            syntax_enabled: true,
            #[cfg(feature = "syntax")]
            highlight_cache: HashMap::new(),
            #[cfg(feature = "syntax")]
            preview_enabled: false,
            #[cfg(feature = "syntax")]
            preview_cache: HashMap::new(),
            #[cfg(feature = "syntax")]
            wrap_before_preview: None,
            #[cfg(feature = "syntax")]
            wrap_by_language: HashMap::new(),
        }
    }

    /// Sets `--debug FILE`'s already-opened handle. A separate setter
    /// rather than another `App::new` parameter so the many test call
    /// sites (which never pass one) don't all need updating.
    fn with_debug_log(mut self, debug_log: Option<fs::File>) -> Self {
        self.debug_log = debug_log;
        self
    }

    /// Appends one timestamped line to `--debug FILE`, flushed immediately
    /// -- a no-op when `--debug` wasn't given. Kept deliberately terse
    /// (single `write!` call, no buffering) since it exists specifically to
    /// survive the process getting stuck right after it's called.
    fn debug(&mut self, msg: impl std::fmt::Display) {
        let Some(file) = self.debug_log.as_mut() else {
            return;
        };
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let _ = writeln!(file, "[{:.3}] {msg}", elapsed.as_secs_f64());
        let _ = file.flush();
    }

    fn rows(&self) -> Vec<Row> {
        fn visit(
            positions: &[Position],
            parent: &str,
            depth: usize,
            expanded: &HashSet<PositionId>,
            rows: &mut Vec<Row>,
        ) {
            for (index, position) in positions.iter().enumerate() {
                let path = if parent.is_empty() {
                    index.to_string()
                } else {
                    format!("{parent}/{index}")
                };
                let id = PositionId(path.clone());
                rows.push(Row {
                    position: id.clone(),
                    node: position.node.clone(),
                    depth,
                    has_children: !position.children.is_empty(),
                });
                if expanded.contains(&id) {
                    visit(&position.children, &path, depth + 1, expanded, rows);
                }
            }
        }
        let mut rows = Vec::new();
        visit(
            &self.document.outline.roots,
            "",
            0,
            &self.expanded,
            &mut rows,
        );
        rows
    }

    fn move_selection(&mut self, delta: isize) {
        self.selection_anchor = None;
        self.extend_selection(delta);
    }

    fn extend_selection(&mut self, delta: isize) {
        let len = self.rows().len();
        if len == 0 {
            self.selected = 0;
            return;
        }
        let selected = self.selected.saturating_add_signed(delta).min(len - 1);
        if selected != self.selected {
            self.selected = selected;
            self.reset_body_view();
        }
    }

    /// Resets everything that only makes sense for the previously-selected
    /// node's body: scroll position and any in-progress or finished mouse
    /// text selection.
    fn reset_body_view(&mut self) {
        self.body_scroll = 0;
        self.body_horizontal_scroll = 0;
        self.body_selection = None;
    }

    fn scroll_body(&mut self, pages: isize) {
        let amount = self.body_page_size.max(1);
        self.body_scroll = self
            .body_scroll
            .saturating_add_signed(pages.saturating_mul(amount as isize))
            .min(self.body_scroll_max);
    }

    fn scroll_body_lines(&mut self, lines: isize) {
        self.body_scroll = self
            .body_scroll
            .saturating_add_signed(lines)
            .min(self.body_scroll_max);
    }

    fn scroll_body_horizontal(&mut self, columns: isize) {
        if self.wrap_for(self.selected_position().as_ref()) {
            return;
        }
        self.body_horizontal_scroll = self
            .body_horizontal_scroll
            .saturating_add_signed(columns)
            .min(self.body_horizontal_scroll_max);
    }

    fn selected_position(&self) -> Option<PositionId> {
        self.rows()
            .get(self.selected)
            .map(|row| row.position.clone())
    }

    #[cfg(feature = "syntax")]
    fn language_at(&self, position: &PositionId) -> Option<String> {
        syntax_context(&self.document.outline, position).0
    }

    /// The word-wrap preference for `position`'s detected language (via
    /// `@language`/inherited context, not raw file extension), or the shared
    /// default bucket (`body_wrap`) when there's no detected language, no
    /// selection, or the `syntax` feature is disabled.
    fn wrap_for(&self, position: Option<&PositionId>) -> bool {
        #[cfg(feature = "syntax")]
        if let Some(position) = position
            && let Some(language) = self.language_at(position)
        {
            return self
                .wrap_by_language
                .get(&language)
                .copied()
                .unwrap_or(false);
        }
        self.body_wrap
    }

    fn set_wrap_for(&mut self, position: Option<&PositionId>, value: bool) {
        #[cfg(feature = "syntax")]
        if let Some(position) = position
            && let Some(language) = self.language_at(position)
        {
            self.wrap_by_language.insert(language, value);
            self.reset_body_view();
            return;
        }
        self.body_wrap = value;
        self.reset_body_view();
    }

    #[cfg(feature = "syntax")]
    fn toggle_preview(&mut self) {
        self.preview_enabled = !self.preview_enabled;
        let position = self.selected_position();
        if self.preview_enabled {
            self.wrap_before_preview = Some(self.wrap_for(position.as_ref()));
            self.set_wrap_for(position.as_ref(), true);
        } else if let Some(previous_wrap) = self.wrap_before_preview.take() {
            self.set_wrap_for(position.as_ref(), previous_wrap);
        }
        self.status = format!(
            "rendered preview {}",
            if self.preview_enabled { "on" } else { "off" }
        );
    }

    fn toggle_body_wrap(&mut self) {
        let position = self.selected_position();
        let wrap = !self.wrap_for(position.as_ref());
        self.set_wrap_for(position.as_ref(), wrap);
        self.status = format!("word wrap {}", if wrap { "enabled" } else { "disabled" });
    }

    /// Toggles the mark on every row in the current selection range (just
    /// the current row when there's no active Shift-range). If any are
    /// unmarked, marks all of them; if all are already marked, unmarks all
    /// of them -- so one keypress on a range always leaves it in a single,
    /// predictable state. `x`/`c` prefer the marked set over the row-range
    /// selection when it's non-empty (see `selected_tree_roots`).
    fn toggle_mark(&mut self) {
        let rows = self.selected_rows();
        if rows.is_empty() {
            return;
        }
        let all_marked = rows.iter().all(|row| self.marked.contains(&row.position));
        for row in &rows {
            if all_marked {
                self.marked.remove(&row.position);
            } else {
                self.marked.insert(row.position.clone());
            }
        }
        self.status = format!(
            "{} (● {})",
            if all_marked { "unmarked" } else { "marked" },
            self.marked.len()
        );
    }

    fn clear_marks(&mut self) {
        if self.marked.is_empty() {
            return;
        }
        self.marked.clear();
        self.status = "all marks cleared".into();
    }

    fn toggle(&mut self, expand: bool) {
        let rows = self.rows();
        let Some(row) = rows.get(self.selected) else {
            return;
        };
        if expand && row.has_children {
            self.expanded.insert(row.position.clone());
        }
        if !expand {
            self.expanded.remove(&row.position);
        }
    }

    fn selected_row(&self) -> Option<Row> {
        self.rows().get(self.selected).cloned()
    }

    fn selected_rows(&self) -> Vec<Row> {
        let rows = self.rows();
        let anchor = self.selection_anchor.unwrap_or(self.selected);
        let start = anchor.min(self.selected);
        let end = anchor.max(self.selected).min(rows.len().saturating_sub(1));
        rows.get(start..=end).unwrap_or_default().to_vec()
    }

    fn editable(&mut self, row: &Row) -> bool {
        if self.derived_nodes.contains(&row.node) && !self.writable_derived(&row.node) {
            self.status = "@auto descendants are read-only; press o to edit the source".into();
            false
        } else {
            true
        }
    }

    fn writable_derived(&self, node: &NodeId) -> bool {
        self.writable_external
            .values()
            .any(|file| file.original.nodes.contains_key(node))
    }

    fn readonly_derived(&self, node: &NodeId) -> bool {
        self.derived_nodes.contains(node) && !self.writable_derived(node)
    }

    /// Whether `node` may not take a newly nested child: a read-only
    /// derived node (an `@auto`/`@auto-dir`-produced descendant, or a thin
    /// file's own if it isn't writable), or the root of an `@auto`-family
    /// node itself. Both regenerate their entire child list from scratch on
    /// every load -- unlike a writable `@file`/`@thin`/`@file-thin`/`@f`
    /// root, which the TUI already permits structural edits under -- so
    /// anything demoted under one here would silently vanish on the next
    /// reload rather than round-trip.
    fn refuses_new_children(&self, node: &NodeId) -> bool {
        self.readonly_derived(node)
            || derived_filename(&self.document.outline.nodes[node].headline)
                .is_some_and(|(auto, _, _)| auto)
    }
}

struct HeadlineInput {
    node: NodeId,
    input: ratatui_textarea::TextArea<'static>,
    original: String,
    inserted_position: Option<PositionId>,
}

impl HeadlineInput {
    fn value(&self) -> &str {
        &self.input.lines()[0]
    }
}

/// Ties a [`ratatui_textarea::TextArea`]'s text-editing state to the node
/// it's editing, for the quick body entry (`b`).
struct BodyEdit {
    node: NodeId,
    input: ratatui_textarea::TextArea<'static>,
    /// Layout to restore on commit/cancel -- opening the editor forces the
    /// body pane full-width (there's more to work with than the narrow
    /// default split, and editing while `outline_full_width` is set would
    /// leave the field invisible), so the prior state has to be remembered.
    restore_body_full_width: bool,
    restore_outline_full_width: bool,
}

struct FindInput {
    query: String,
    matches: Vec<PositionId>,
    active: usize,
    original: Option<PositionId>,
}

struct ReplInput {
    input: ratatui_textarea::TextArea<'static>,
}

impl ReplInput {
    fn new(value: impl Into<String>) -> Self {
        let mut input = ratatui_textarea::TextArea::new(vec![value.into()]);
        input.move_cursor(ratatui_textarea::CursorMove::End);
        Self { input }
    }

    fn value(&self) -> &str {
        &self.input.lines()[0]
    }
}

struct ActionPalette {
    query: String,
    matches: Vec<PaletteEntry>,
    active: usize,
    /// Messages from `@import`ed scripts that failed to compile or threw
    /// while loading -- see `palette_entries`. Independent of `matches`:
    /// still populated (and shown) even when filtering leaves `matches`
    /// empty, since a broken script and "no entries match your query" are
    /// different problems.
    errors: Vec<String>,
}

/// One entry in the action palette (`a`): an `@action` node in the
/// outline, a zero-input command an `@import`ed script's `COMMANDS` map
/// names (see `discover_commands`), or a built-in editor command from the
/// static `COMMANDS` array. `label` is precomputed at [`palette_entries`]
/// time so filtering/drawing never need to re-walk the outline or re-read
/// a script per entry. `doc` is the description `COMMANDS` gave a script
/// command, shown for the active entry below the list; always `None` for
/// an `@action` or built-in entry, neither of which has such a
/// description.
#[derive(Clone, Debug, PartialEq)]
struct PaletteEntry {
    label: String,
    doc: Option<String>,
    kind: PaletteEntryKind,
}

#[derive(Clone, Debug, PartialEq)]
enum PaletteEntryKind {
    Action(PositionId),
    Command {
        script: PathBuf,
        name: String,
    },
    /// Index into the static `COMMANDS` array of built-in editor commands
    /// (e.g. "Import new files into @path") -- these need no script or
    /// outline node, just a check of whether they apply to the current
    /// selection ([`CommandSpec::available`]).
    Builtin(usize),
}

struct CommandSpec {
    name: &'static str,
    available: fn(&App) -> bool,
    run: fn(&mut App),
}

const COMMANDS: &[CommandSpec] = &[CommandSpec {
    name: "Import new files into @path",
    available: command_import_available,
    run: command_import_run,
}];

struct ActionOutput {
    node: NodeId,
    name: String,
    interpreter: &'static str,
    status: Option<i32>,
    text: String,
}

#[derive(Clone)]
struct ClipboardTree {
    roots: Vec<Position>,
    nodes: HashMap<NodeId, leo::Node>,
}

#[derive(Clone, Copy)]
enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
}

fn build_app(path: PathBuf, load_derived: bool, debug_log_path: Option<PathBuf>) -> Result<App> {
    let debug_log = debug_log_path
        .map(|path| {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .with_context(|| format!("open debug log {}", path.display()))
        })
        .transpose()?;
    let mut document = LeoDocument::open(&path)?;
    let (
        status,
        source_locations,
        source_nodes,
        derived_nodes,
        writable_external,
        original_external,
    ) = if load_derived {
        let report = load_derived_files(&mut document.outline, &path);
        let status = if report.errors.is_empty() {
            format!("loaded {} derived file(s)", report.loaded)
        } else {
            format!(
                "loaded {}; {} error(s): {}",
                report.loaded,
                report.errors.len(),
                report.errors.join(" | ")
            )
        };
        (
            status,
            report.locations,
            report.node_locations,
            report.derived_nodes,
            report.writable_external,
            OriginalExternalState {
                children: report.original_children,
                bodies: report.original_bodies,
                nodes: report.original_nodes,
            },
        )
    } else {
        (
            "derived files disabled".to_owned(),
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
            OriginalExternalState::default(),
        )
    };
    Ok(App::new(
        document,
        path,
        status,
        source_locations,
        source_nodes,
        derived_nodes,
        writable_external,
        original_external,
        load_derived,
    )
    .with_debug_log(debug_log))
}

/// Runs `body` against a real alternate-screen terminal, guaranteeing the
/// terminal is restored to normal afterwards even if `body` fails.
fn with_real_terminal<F>(body: F) -> Result<()>
where
    F: FnOnce(&mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()>,
{
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = body(&mut terminal);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        DisableBracketedPaste,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    result
}

pub fn run(path: PathBuf, load_derived: bool, debug_log_path: Option<PathBuf>) -> Result<()> {
    let mut app = build_app(path, load_derived, debug_log_path)?;
    app.debug("session started");
    with_real_terminal(|terminal| event_loop(terminal, &mut app))
}

enum KeyOutcome {
    Continue,
    Quit,
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let event = event::read()?;
        if let Event::Paste(text) = event {
            handle_paste(app, text);
            continue;
        }
        if let Event::Mouse(mouse) = event {
            if app.input.is_none()
                && app.find.is_none()
                && app.search.is_none()
                && app.palette.is_none()
                && !app.help
            {
                handle_mouse(app, terminal.size()?.into(), mouse);
            }
            continue;
        }
        let Event::Key(key) = event else { continue };
        if let KeyOutcome::Quit = handle_key(app, key, Some(&mut *terminal)) {
            return Ok(());
        }
    }
}

/// Routes a bracketed-paste `Event::Paste` (a terminal-native paste, e.g.
/// Cmd+V/Ctrl+Shift+V -- the terminal emulator delivers the clipboard text
/// directly, so no OS clipboard access is needed here). The quick body
/// entry (`b`) consumes a paste as one chunk via
/// [`insert_paste_into_body`], inserting it as a single atomic edit. Every
/// other text field expects one key at a time (as an un-bracketed paste
/// would have delivered before), so its paste is replayed character by
/// character through the same handler a keypress would use -- identical
/// behavior to before, just arriving as a single clean event instead of a
/// flood of raw bytes that risked being misparsed as escape sequences.
fn handle_paste(app: &mut App, text: String) {
    if app.body_input.is_some() {
        insert_paste_into_body(app, text);
        return;
    }
    let replay: fn(&mut App, KeyEvent) = if app.input.is_some() {
        handle_headline_input
    } else if app.find.is_some() {
        handle_find_input
    } else if app.search.is_some() {
        handle_search_input
    } else if app.palette.is_some() {
        handle_palette_input
    } else if app.log_repl.is_some() {
        handle_log_repl_key
    } else {
        return;
    };
    for character in text.chars() {
        let code = match character {
            '\n' | '\r' => KeyCode::Enter,
            '\t' => KeyCode::Tab,
            other => KeyCode::Char(other),
        };
        replay(app, KeyEvent::new(code, KeyModifiers::NONE));
    }
}

/// Dispatches a single key press. `terminal` is `None` for headless/scripted
/// runs with no real terminal to suspend into -- keys that would normally
/// open an external editor just report that they were skipped instead.
fn handle_key(
    app: &mut App,
    key: KeyEvent,
    terminal: Option<&mut Terminal<CrosstermBackend<io::Stdout>>>,
) -> KeyOutcome {
    if key.kind != KeyEventKind::Press {
        return KeyOutcome::Continue;
    }
    app.debug(format!("key: {:?} mods={:?}", key.code, key.modifiers));
    if app.input.is_some() {
        handle_headline_input(app, key);
        return KeyOutcome::Continue;
    }
    if app.body_input.is_some() {
        handle_body_input(app, key);
        return KeyOutcome::Continue;
    }
    if app.find.is_some() {
        handle_find_input(app, key);
        return KeyOutcome::Continue;
    }
    if app.search.is_some() {
        handle_search_input(app, key);
        return KeyOutcome::Continue;
    }
    if app.palette.is_some() {
        handle_palette_input(app, key);
        return KeyOutcome::Continue;
    }
    if app.log_repl.is_some() {
        handle_log_repl_key(app, key);
        return KeyOutcome::Continue;
    }
    if app.log_view {
        handle_log_view_key(app, key);
        return KeyOutcome::Continue;
    }
    if app.help {
        if matches!(
            key.code,
            KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Esc
        ) {
            app.help = false;
        }
        return KeyOutcome::Continue;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('p') => start_find(app),
            KeyCode::Char('r') => reload(app),
            KeyCode::Char('s') => save(app),
            KeyCode::Up => move_selected(app, MoveDirection::Up),
            KeyCode::Down => move_selected(app, MoveDirection::Down),
            KeyCode::Left => move_selected(app, MoveDirection::Left),
            KeyCode::Right => move_selected(app, MoveDirection::Right),
            _ => {}
        }
        return KeyOutcome::Continue;
    }
    match key.code {
        KeyCode::Char('c') if key.modifiers.is_empty() => copy_selected(app),
        KeyCode::Char('C') if key.modifiers == KeyModifiers::SHIFT => {
            copy_location_to_clipboard(app);
        }
        KeyCode::Char('x') if key.modifiers.is_empty() => cut_selected(app),
        KeyCode::Char('v') if key.modifiers.is_empty() => paste_tree(app, false),
        KeyCode::Char('V') if key.modifiers == KeyModifiers::SHIFT => paste_tree(app, true),
        KeyCode::Char('m') if key.modifiers.is_empty() => app.toggle_mark(),
        KeyCode::Char('M') if key.modifiers == KeyModifiers::SHIFT => app.clear_marks(),
        KeyCode::Char('n') if key.modifiers.is_empty() => cycle_clone(app, 1),
        KeyCode::Char('N') if key.modifiers == KeyModifiers::SHIFT => cycle_clone(app, -1),
        KeyCode::Up if key.modifiers == KeyModifiers::SHIFT => {
            app.selection_anchor.get_or_insert(app.selected);
            app.extend_selection(-1);
        }
        KeyCode::Down if key.modifiers == KeyModifiers::SHIFT => {
            app.selection_anchor.get_or_insert(app.selected);
            app.extend_selection(1);
        }
        KeyCode::Char('?') => app.help = true,
        KeyCode::Char('l') if key.modifiers.is_empty() => {
            app.log_view = true;
            app.log_selection = None;
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            if !app.dirty || app.quit_armed {
                return KeyOutcome::Quit;
            }
            app.quit_armed = true;
            app.status = "unsaved changes; press q again to discard, or Ctrl-S to save".into();
        }
        KeyCode::Char('o') => match terminal {
            Some(t) => open_selected(t, app),
            None => app.status = "open skipped (no terminal in this run)".into(),
        },
        KeyCode::Char('f') => {
            app.body_full_width = !app.body_full_width;
            app.outline_full_width = false;
            app.status = if app.body_full_width {
                "body pane expanded to full width"
            } else {
                "outline pane restored"
            }
            .into();
        }
        KeyCode::Char('F') => {
            app.outline_full_width = !app.outline_full_width;
            app.body_full_width = false;
            app.status = if app.outline_full_width {
                "outline pane expanded to full width"
            } else {
                "body pane restored"
            }
            .into();
        }
        KeyCode::Char('s') if key.modifiers.is_empty() => {
            app.split_horizontal = !app.split_horizontal;
            app.status = if app.split_horizontal {
                "split horizontally (outline above body)"
            } else {
                "split vertically (outline beside body)"
            }
            .into();
        }
        KeyCode::Char('W') => app.toggle_body_wrap(),
        KeyCode::Char('/') if key.modifiers.is_empty() => start_search(app),
        KeyCode::Char('a') if key.modifiers.is_empty() => start_palette(app),
        KeyCode::Char('i') if key.modifiers.is_empty() => insert_headline(app),
        KeyCode::Char('h') if key.modifiers.is_empty() => edit_headline(app),
        KeyCode::Char('b') if key.modifiers.is_empty() => quick_edit_body(app),
        #[cfg(feature = "syntax")]
        KeyCode::Char('y') => {
            app.syntax_enabled = !app.syntax_enabled;
            app.status = format!(
                "syntax highlighting {}",
                if app.syntax_enabled { "on" } else { "off" }
            );
        }
        #[cfg(feature = "syntax")]
        KeyCode::Char('p') => app.toggle_preview(),
        KeyCode::Down if app.body_full_width => app.scroll_body_lines(1),
        KeyCode::Up if app.body_full_width => app.scroll_body_lines(-1),
        KeyCode::Down => app.move_selection(1),
        KeyCode::Up => app.move_selection(-1),
        KeyCode::Right if app.body_full_width => app.scroll_body_horizontal(4),
        KeyCode::Left if app.body_full_width => app.scroll_body_horizontal(-4),
        KeyCode::Enter => match terminal {
            Some(t) => open_selected(t, app),
            None => app.status = "open skipped (no terminal in this run)".into(),
        },
        KeyCode::Right => {
            app.selection_anchor = None;
            app.toggle(true);
        }
        KeyCode::Left => {
            app.selection_anchor = None;
            app.toggle(false);
        }
        KeyCode::Home => {
            app.selection_anchor = None;
            app.selected = 0;
            app.reset_body_view();
        }
        KeyCode::End => {
            app.selection_anchor = None;
            app.selected = app.rows().len().saturating_sub(1);
            app.reset_body_view();
        }
        KeyCode::PageUp => app.scroll_body(-1),
        KeyCode::PageDown => app.scroll_body(1),
        _ => {}
    }
    KeyOutcome::Continue
}

fn handle_mouse(app: &mut App, area: Rect, mouse: MouseEvent) {
    let kind = mouse.kind;
    if matches!(kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown) {
        handle_mouse_scroll(app, area, kind, mouse);
        return;
    }
    if !matches!(
        kind,
        MouseEventKind::Down(MouseButton::Left)
            | MouseEventKind::Drag(MouseButton::Left)
            | MouseEventKind::Up(MouseButton::Left)
    ) {
        return;
    }
    // The log view is a full-screen overlay (see `draw_log`) with nothing
    // in common with the outline/body split below -- route to its own
    // handler instead of interpreting these coordinates against panes
    // that aren't even on screen right now.
    if app.log_view {
        handle_log_mouse(app, area, kind, mouse);
        return;
    }
    let content_height = area.height.saturating_sub(1);
    let content = Rect::new(area.x, area.y, area.width, content_height);
    let columns = content_columns(content, app);
    let outline = columns[0];
    let in_outline = !app.body_full_width
        && mouse.column >= outline.x
        && mouse.column < outline.right()
        && mouse.row >= outline.y.saturating_add(1)
        && mouse.row < outline.bottom().saturating_sub(1);
    if in_outline {
        handle_outline_mouse(app, outline, kind, mouse);
        return;
    }
    if app.outline_full_width {
        return;
    }
    handle_body_mouse(app, columns[1], kind, mouse);
}

/// Mouse wheel: moves the outline selection when the wheel is over the
/// tree, same as Up/Down, or scrolls the body content when it's over the
/// node pane, same as PgUp/PgDn -- there's no independent outline scroll
/// position to move (the render clamps it to follow `selected`), so
/// wheeling over the tree has to move the selection itself.
fn handle_mouse_scroll(app: &mut App, area: Rect, kind: MouseEventKind, mouse: MouseEvent) {
    const LINES_PER_NOTCH: isize = 3;
    let delta = if matches!(kind, MouseEventKind::ScrollUp) {
        -LINES_PER_NOTCH
    } else {
        LINES_PER_NOTCH
    };
    if app.log_view {
        app.log_scroll = app.log_scroll.saturating_add_signed(-delta);
        return;
    }
    let content_height = area.height.saturating_sub(1);
    let content = Rect::new(area.x, area.y, area.width, content_height);
    let columns = content_columns(content, app);
    let outline = columns[0];
    let in_outline = !app.body_full_width
        && mouse.column >= outline.x
        && mouse.column < outline.right()
        && mouse.row >= outline.y.saturating_add(1)
        && mouse.row < outline.bottom().saturating_sub(1);
    if in_outline {
        app.move_selection(delta);
        return;
    }
    if app.outline_full_width {
        return;
    }
    app.scroll_body_lines(delta);
}

/// Drag-to-select log text and copy the selection to the system clipboard
/// on release -- the log-pane counterpart of `handle_body_mouse`, using
/// `log_view_layout`/`log_view_range` to land on the exact same
/// coordinates `draw_log` rendered.
fn handle_log_mouse(app: &mut App, area: Rect, kind: MouseEventKind, mouse: MouseEvent) {
    let (log_area, input_area) = log_view_layout(area, app.log_repl.is_some());
    if let Some(input_area) = input_area
        && mouse.row >= input_area.y
    {
        return;
    }
    if log_area.width == 0 || log_area.height == 0 {
        return;
    }
    let starting = matches!(kind, MouseEventKind::Down(MouseButton::Left));
    if starting
        && (mouse.column < log_area.x
            || mouse.column >= log_area.right()
            || mouse.row < log_area.y
            || mouse.row >= log_area.bottom())
    {
        return;
    }
    if !starting && app.log_selection.is_none() {
        return;
    }
    let (start, end) = log_view_range(&app.logs, log_area, app.log_scroll);
    if start == end {
        return;
    }
    let clamped_row = mouse
        .row
        .clamp(log_area.y, log_area.bottom().saturating_sub(1));
    let clamped_column = mouse
        .column
        .clamp(log_area.x, log_area.right().saturating_sub(1));
    let line_index = (start + usize::from(clamped_row - log_area.y)).min(end.saturating_sub(1));
    let line_len = app
        .logs
        .get(line_index)
        .map_or(0, |line| line.chars().count());
    let column_index = usize::from(clamped_column - log_area.x).min(line_len);
    let position = (line_index, column_index);

    match kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.log_selection = Some(BodySelection {
                anchor: position,
                cursor: position,
            });
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some(selection) = app.log_selection.as_mut() {
                selection.cursor = position;
            }
        }
        MouseEventKind::Up(MouseButton::Left) => copy_log_selection_to_clipboard(app),
        _ => {}
    }
}

fn copy_log_selection_to_clipboard(app: &mut App) {
    let Some(selection) = app.log_selection else {
        return;
    };
    let lines: Vec<String> = app.logs.iter().cloned().collect();
    let Some(text) = selected_body_text(&lines, selection) else {
        return;
    };
    match execute!(
        io::stdout(),
        CopyToClipboard::to_clipboard_from(text.clone())
    ) {
        Ok(()) => {
            let chars = text.chars().count();
            app.status = format!(
                "copied {chars} selected character{} to clipboard",
                if chars == 1 { "" } else { "s" }
            );
        }
        Err(error) => app.status = format!("clipboard copy failed: {error}"),
    }
}

/// Click-to-select and click-the-expand-marker, plus drag-to-extend the
/// same multi-row tree selection that Shift-↑/↓ builds (`selection_anchor`
/// fixed at the press, `selected` following the pointer). Releasing after
/// an actual drag copies the selected headlines to the system clipboard,
/// same as releasing a body drag copies the selected body text.
fn handle_outline_mouse(app: &mut App, outline: Rect, kind: MouseEventKind, mouse: MouseEvent) {
    let row = app.outline_scroll + usize::from(mouse.row - outline.y - 1);
    let rows = app.rows();
    if let MouseEventKind::Up(MouseButton::Left) = kind {
        let Some(anchor) = app.selection_anchor else {
            return;
        };
        // Trust the release position over whatever a jittery mid-drag event
        // last left `selected` at: many terminals report a spurious one-row
        // Drag for what's really just a plain click, which would otherwise
        // both leave the wrong row selected and copy headlines nobody meant
        // to select.
        if rows.get(row).is_some() {
            if row != app.selected {
                app.reset_body_view();
            }
            app.selected = row;
        }
        if app.selected == anchor {
            app.selection_anchor = None;
        } else {
            copy_outline_selection_to_clipboard(app);
        }
        return;
    }
    let Some(clicked) = rows.get(row) else {
        return;
    };
    match kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let marker_start = outline.x + 1 + u16::try_from(clicked.depth * 2).unwrap_or(u16::MAX);
            let on_marker = clicked.has_children
                && mouse.column >= marker_start
                && mouse.column < marker_start + 2;
            let position = clicked.position.clone();
            app.selection_anchor = None;
            if row != app.selected {
                app.reset_body_view();
            }
            app.selected = row;
            if on_marker {
                app.toggle(!app.expanded.contains(&position));
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            app.selection_anchor.get_or_insert(app.selected);
            if row != app.selected {
                app.reset_body_view();
            }
            app.selected = row;
        }
        _ => {}
    }
}

/// Copies the headlines spanned by `selection_anchor`..=`selected` (one per
/// line, indented 2 spaces per depth level relative to the shallowest
/// selected row -- same look as the outline, just left-aligned) to the
/// system clipboard via OSC 52. A no-op for a drag that collapsed back onto
/// its own start row -- nothing was actually selected.
fn copy_outline_selection_to_clipboard(app: &mut App) {
    let anchor = app.selection_anchor.unwrap_or(app.selected);
    let start = anchor.min(app.selected);
    let end = anchor.max(app.selected);
    if start == end {
        return;
    }
    let rows = app.rows();
    let end = end.min(rows.len().saturating_sub(1));
    if start > end {
        return;
    }
    let text = outline_selection_text(app, &rows[start..=end]);
    match execute!(
        io::stdout(),
        CopyToClipboard::to_clipboard_from(text.clone())
    ) {
        Ok(()) => {
            let count = end - start + 1;
            app.status = format!(
                "copied {count} headline{} to clipboard",
                if count == 1 { "" } else { "s" }
            );
        }
        Err(error) => app.status = format!("clipboard copy failed: {error}"),
    }
}

/// One headline per line, each indented 2 spaces per depth level relative
/// to the shallowest row in `rows` -- the same nesting the outline shows,
/// just left-aligned to column 0.
fn outline_selection_text(app: &App, rows: &[Row]) -> String {
    let min_depth = rows.iter().map(|row| row.depth).min().unwrap_or(0);
    rows.iter()
        .map(|row| {
            let indent = "  ".repeat(row.depth.saturating_sub(min_depth));
            let headline = &app.document.outline.nodes[&row.node].headline;
            format!("{indent}{headline}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Drag-to-select body text and copy the selection to the system clipboard
/// (via OSC 52, same mechanism as `copy_location_to_clipboard`) on release.
///
/// Only available when the body isn't word-wrapped: `body_scroll` counts
/// *visual* rows once wrap is on (`Paragraph::line_count` under wrap), so a
/// screen row no longer maps onto a single logical line/column, and
/// ratatui's wrap doesn't expose the reverse mapping needed to fix that.
fn handle_body_mouse(app: &mut App, body_area: Rect, kind: MouseEventKind, mouse: MouseEvent) {
    let node_area = Block::default().borders(Borders::ALL).inner(body_area);
    if node_area.width == 0 || node_area.height == 0 {
        return;
    }
    let starting = matches!(kind, MouseEventKind::Down(MouseButton::Left));
    if starting
        && (mouse.column < node_area.x
            || mouse.column >= node_area.right()
            || mouse.row < node_area.y
            || mouse.row >= node_area.bottom())
    {
        return;
    }
    if !starting && app.body_selection.is_none() {
        return;
    }
    let rows = app.rows();
    let Some(row) = rows.get(app.selected).cloned() else {
        return;
    };
    if starting && app.wrap_for(Some(&row.position)) {
        app.status = "mouse text selection needs word-wrap off (press W)".into();
        return;
    }
    let lines = body_plain_lines(app, &row);
    if lines.is_empty() {
        return;
    }

    let clamped_row = mouse
        .row
        .clamp(node_area.y, node_area.bottom().saturating_sub(1));
    let clamped_column = mouse
        .column
        .clamp(node_area.x, node_area.right().saturating_sub(1));
    let line_index =
        (app.body_scroll + usize::from(clamped_row - node_area.y)).min(lines.len() - 1);
    let column_index = (app.body_horizontal_scroll + usize::from(clamped_column - node_area.x))
        .min(lines[line_index].chars().count());
    let position = (line_index, column_index);

    match kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.body_selection = Some(BodySelection {
                anchor: position,
                cursor: position,
            });
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some(selection) = app.body_selection.as_mut() {
                selection.cursor = position;
            }
        }
        MouseEventKind::Up(MouseButton::Left) => copy_body_selection_to_clipboard(app, &lines),
        _ => {}
    }
}

/// The node body's plain text (no syntax-highlight styling), split into
/// lines matching `body_text`'s line layout -- what mouse coordinates and
/// the rendered selection highlight both index into.
fn body_plain_lines(app: &mut App, row: &Row) -> Vec<String> {
    let text = if let Some(output) = app
        .action_output
        .as_ref()
        .filter(|out| out.node == row.node)
    {
        Text::from(output.text.clone())
    } else {
        body_text(app, row)
    };
    text.lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect()
}

fn copy_body_selection_to_clipboard(app: &mut App, lines: &[String]) {
    let Some(selection) = app.body_selection else {
        return;
    };
    let Some(text) = selected_body_text(lines, selection) else {
        return;
    };
    match execute!(
        io::stdout(),
        CopyToClipboard::to_clipboard_from(text.clone())
    ) {
        Ok(()) => {
            let chars = text.chars().count();
            app.status = format!(
                "copied {chars} selected character{} to clipboard",
                if chars == 1 { "" } else { "s" }
            );
        }
        Err(error) => app.status = format!("clipboard copy failed: {error}"),
    }
}

/// The text a `BodySelection` covers, or `None` for an empty (click, no
/// drag) selection.
fn selected_body_text(lines: &[String], selection: BodySelection) -> Option<String> {
    let (start, end) = if selection.anchor <= selection.cursor {
        (selection.anchor, selection.cursor)
    } else {
        (selection.cursor, selection.anchor)
    };
    if start == end {
        return None;
    }
    let (start_line, start_col) = start;
    let (end_line, end_col) = end;
    if start_line >= lines.len() {
        return None;
    }
    let end_line = end_line.min(lines.len() - 1);
    let mut text = String::new();
    for (line_index, line) in lines.iter().enumerate().take(end_line + 1).skip(start_line) {
        if line_index > start_line {
            text.push('\n');
        }
        let line_len = line.chars().count();
        let (from, to) = if line_index == start_line && line_index == end_line {
            (start_col.min(line_len), end_col.min(line_len))
        } else if line_index == start_line {
            (start_col.min(line_len), line_len)
        } else if line_index == end_line {
            (0, end_col.min(line_len))
        } else {
            (0, line_len)
        };
        text.extend(line.chars().skip(from).take(to.saturating_sub(from)));
    }
    Some(text)
}

/// Cap on `app.logs` so a long session doesn't grow the ring buffer without
/// bound; old lines fall off the front once this is exceeded.
const LOG_CAPACITY: usize = 5000;

/// Appends `text`'s lines to the log ring buffer, trimming the oldest lines
/// once it exceeds `LOG_CAPACITY`, and snaps the view back to the latest
/// output (`log_scroll = 0`) the way `tail -f` would.
fn push_log(app: &mut App, text: &str) {
    for line in text.split('\n') {
        if line.is_empty() {
            continue;
        }
        app.debug(format!("log: {line}"));
        app.logs.push_back(line.to_owned());
    }
    while app.logs.len() > LOG_CAPACITY {
        app.logs.pop_front();
    }
    app.log_scroll = 0;
}

/// Like `push_log`, but prefixes each non-empty line with `[{prefix}]` so
/// output from different `@action` runs stays distinguishable in the log.
fn push_log_lines(app: &mut App, prefix: &str, text: &str) {
    for line in text.split('\n') {
        if line.is_empty() {
            continue;
        }
        push_log(app, &format!("[{prefix}] {line}"));
    }
}

fn handle_log_view_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('l') | KeyCode::Char('q') | KeyCode::Esc => {
            app.log_view = false;
            app.log_repl = None;
            app.log_selection = None;
        }
        KeyCode::Enter => app.log_repl = Some(ReplInput::new("")),
        KeyCode::Up => app.log_scroll = app.log_scroll.saturating_add(1),
        KeyCode::Down => app.log_scroll = app.log_scroll.saturating_sub(1),
        KeyCode::PageUp => app.log_scroll = app.log_scroll.saturating_add(20),
        KeyCode::PageDown => app.log_scroll = app.log_scroll.saturating_sub(20),
        _ => {}
    }
}

fn handle_log_repl_key(app: &mut App, key: KeyEvent) {
    let input = app.log_repl.as_mut().expect("repl input exists");
    match key.code {
        KeyCode::Esc => app.log_repl = None,
        KeyCode::Enter => {
            let input = app.log_repl.take().expect("repl input exists");
            if !input.value().is_empty() {
                run_repl_snippet(app, input.value());
            }
        }
        _ => {
            input.input.input(key);
        }
    }
}

/// Runs `snippet` as a rhai script bound to the currently selected node (the
/// same `run_bound` path `@action` rhai scripts use, so `doc`/`target`/`p`
/// all behave identically), and appends the echoed snippet plus its output
/// to the log.
fn run_repl_snippet(app: &mut App, snippet: &str) {
    let Some(row) = app.selected_row() else {
        push_log(app, "no node selected");
        return;
    };
    let document = std::mem::replace(&mut app.document, LeoDocument::empty());
    let outcome = crate::rhai_run::run_bound(document, app.path.clone(), &row.position, snippet);
    app.document = outcome.document;
    if outcome.touched {
        mark_outline_touched(app);
    }
    push_log(app, &format!("> {snippet}"));
    push_log(app, &outcome.stdout);
    if !outcome.stderr.is_empty() {
        push_log(app, &outcome.stderr);
    }
}

fn start_find(app: &mut App) {
    let original = app.selected_row().map(|row| row.position);
    app.find = Some(FindInput {
        query: String::new(),
        matches: Vec::new(),
        active: 0,
        original,
    });
    app.status = "find headline: type to search, Enter accepts, Esc cancels".into();
}

fn handle_find_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            app.find = None;
            app.status = "headline selected".into();
        }
        KeyCode::Esc => {
            let original = app.find.take().and_then(|find| find.original);
            if let Some(position) = original {
                reveal_and_select(app, &position);
            }
            app.status = "headline find cancelled".into();
        }
        KeyCode::Backspace => {
            app.find.as_mut().expect("find input exists").query.pop();
            update_find_matches(app, 0);
        }
        KeyCode::Down => cycle_find_match(app, 1),
        KeyCode::Up => cycle_find_match(app, -1),
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            app.find
                .as_mut()
                .expect("find input exists")
                .query
                .push(character);
            update_find_matches(app, 0);
        }
        _ => {}
    }
}

fn update_find_matches(app: &mut App, active: usize) {
    let query = app
        .find
        .as_ref()
        .expect("find input exists")
        .query
        .to_lowercase();
    let matches = if query.is_empty() {
        Vec::new()
    } else {
        all_rows(&app.document.outline)
            .into_iter()
            .filter(|row| {
                app.document.outline.nodes[&row.node]
                    .headline
                    .to_lowercase()
                    .contains(&query)
            })
            .map(|row| row.position)
            .collect::<Vec<_>>()
    };
    let active = active.min(matches.len().saturating_sub(1));
    let selected = matches.get(active).cloned();
    let find = app.find.as_mut().expect("find input exists");
    find.matches = matches;
    find.active = active;
    if let Some(position) = selected {
        reveal_and_select(app, &position);
    }
}

fn cycle_find_match(app: &mut App, delta: isize) {
    let find = app.find.as_ref().expect("find input exists");
    if find.matches.is_empty() {
        return;
    }
    let len = find.matches.len() as isize;
    let active = (find.active as isize + delta).rem_euclid(len) as usize;
    let position = find.matches[active].clone();
    app.find.as_mut().expect("find input exists").active = active;
    reveal_and_select(app, &position);
}

fn start_search(app: &mut App) {
    let original = app.selected_row().map(|row| row.position);
    app.search = Some(FindInput {
        query: String::new(),
        matches: Vec::new(),
        active: 0,
        original,
    });
    app.status = "search headlines and body: type to search, Enter accepts, Esc cancels".into();
}

fn handle_search_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            app.search = None;
            app.status = "search match selected".into();
        }
        KeyCode::Esc => {
            let original = app.search.take().and_then(|search| search.original);
            if let Some(position) = original {
                reveal_and_select(app, &position);
            }
            app.status = "search cancelled".into();
        }
        KeyCode::Backspace => {
            app.search
                .as_mut()
                .expect("search input exists")
                .query
                .pop();
            update_search_matches(app, 0);
        }
        KeyCode::Down => cycle_search_match(app, 1),
        KeyCode::Up => cycle_search_match(app, -1),
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            app.search
                .as_mut()
                .expect("search input exists")
                .query
                .push(character);
            update_search_matches(app, 0);
        }
        _ => {}
    }
}

/// An `@action` node is a runnable node: its body is executed as a script
/// when chosen from the action palette (`a`). Any node, anywhere in the
/// outline, can be an action; the name shown in the palette is the headline
/// with the `@action` marker stripped.
fn is_action_headline(headline: &str) -> bool {
    let trimmed = headline.trim_start();
    trimmed == "@action" || trimmed.starts_with("@action ") || trimmed.starts_with("@action\t")
}

fn action_name(headline: &str) -> &str {
    headline
        .trim_start()
        .strip_prefix("@action")
        .unwrap_or(headline)
        .trim()
}

fn action_rows(outline: &Outline) -> Vec<Row> {
    all_rows(outline)
        .into_iter()
        .filter(|row| is_action_headline(&outline.nodes[&row.node].headline))
        .collect()
}

/// An `@import path.rhai` node registers every function that path's script
/// lists in its `COMMANDS` array as a runnable command in the action
/// palette. Unlike `@action`, an import applies outline-wide regardless of
/// where its node sits in the tree, and it has no body of its own to run
/// -- only the headline's path argument matters.
fn is_import_headline(headline: &str) -> bool {
    let trimmed = headline.trim_start();
    trimmed.starts_with("@import ") || trimmed.starts_with("@import\t")
}

fn import_path_arg(headline: &str) -> &str {
    headline
        .trim_start()
        .strip_prefix("@import")
        .unwrap_or("")
        .trim()
}

/// The resolved, deduplicated paths every `@import` node in the outline
/// names, in no particular order -- imports apply outline-wide, so this
/// scans every node once by gnx (`outline.nodes`) rather than every
/// position, unlike `action_rows`, where a clone occurrence matters.
/// Relative paths resolve against the open `.leo` file's own directory,
/// the same convention `doc.dir()` uses.
fn import_script_paths(app: &App) -> Vec<PathBuf> {
    let dir = app
        .path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut paths: Vec<PathBuf> = app
        .document
        .outline
        .nodes
        .values()
        .filter(|node| is_import_headline(&node.headline))
        .map(|node| import_path_arg(&node.headline))
        .filter(|path| !path.is_empty())
        .map(|path| dir.join(path))
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

/// Every entry the action palette (`a`) offers, plus any script errors hit
/// along the way: `@action` nodes from the outline, commands discovered
/// from every `@import`ed script's `COMMANDS` map (unioned with whatever
/// its `available_commands(doc, target)`, if it defines one, returns for
/// the current selection -- commands whose palette availability is
/// conditional on the selected node, keyed by name against the static set
/// so a name in both wins with `available_commands`'s, possibly different,
/// description), and finally the built-in `COMMANDS` array's own entries
/// (editor commands with no script or outline node behind them, like
/// "Import new files into @path") filtered by each one's own
/// `CommandSpec::available` check. No selected row (an empty outline) just
/// skips the dynamic script half and the built-in check both; the static
/// `@import` set still shows. A script that fails to compile or throws
/// while loading (see `discover_commands`/`discover_available_commands`)
/// contributes no entries and its error message instead -- `draw_palette_panel`
/// shows these in place of the usual per-entry description line, since a
/// script silently contributing nothing to the palette would otherwise look
/// identical to one that legitimately declares no commands. Recomputed on
/// open and on every keystroke (like `action_rows` before it) rather than
/// cached -- outlines and imported scripts are both small enough that
/// re-scanning is cheap, and this way the palette never shows a command a
/// script no longer declares.
///
/// Ordered so a command `available_commands` singled out for the current
/// selection sorts ahead of everything that's always listed regardless of
/// selection (`@action` nodes, a script's static `COMMANDS`, and built-ins)
/// -- it's the most likely thing you actually want to run right now, so it
/// shows up before typing a single filter character.
fn palette_entries(app: &App) -> (Vec<PaletteEntry>, Vec<String>) {
    let outline = &app.document.outline;
    // Split into two buckets so entries an `available_commands` singled out
    // for the current selection -- the ones most likely to be what you
    // actually want to run right now -- sort to the very front, ahead of
    // everything that's always listed regardless of selection. Concatenated
    // back into one list (`context_entries` first) just before returning.
    let mut context_entries: Vec<PaletteEntry> = Vec::new();
    let mut other_entries: Vec<PaletteEntry> = action_rows(outline)
        .into_iter()
        .map(|row| PaletteEntry {
            label: action_name(&outline.nodes[&row.node].headline).to_owned(),
            doc: None,
            kind: PaletteEntryKind::Action(row.position),
        })
        .collect();
    let mut errors = Vec::new();
    let target_position = app.selected_row().map(|row| row.position);
    for script in import_script_paths(app) {
        let stem = script
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("script");
        let mut commands = match crate::rhai_run::discover_commands(&script) {
            Ok(commands) => commands,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        let mut dynamic_names: HashSet<String> = HashSet::new();
        if let Some(target_position) = &target_position {
            match crate::rhai_run::discover_available_commands(
                &app.document,
                app.path.clone(),
                &script,
                target_position,
            ) {
                Ok(dynamic_commands) => {
                    for dynamic in dynamic_commands {
                        dynamic_names.insert(dynamic.name.clone());
                        match commands.iter_mut().find(|c| c.name == dynamic.name) {
                            Some(existing) => *existing = dynamic,
                            None => commands.push(dynamic),
                        }
                    }
                    // `COMMANDS` alone already lists alphabetically (it
                    // reads back as a `rhai::Map`, a `BTreeMap`) -- keep
                    // that property after folding in a name
                    // `available_commands` only just added, rather than
                    // leaving new names trailing in whatever order it
                    // returned them.
                    commands.sort_by(|a, b| a.name.cmp(&b.name));
                }
                Err(error) => errors.push(error),
            }
        }
        for command in commands {
            let entry = PaletteEntry {
                label: format!("{}  ({stem})", command.name),
                doc: command.doc,
                kind: PaletteEntryKind::Command {
                    script: script.clone(),
                    name: command.name.clone(),
                },
            };
            if dynamic_names.contains(&command.name) {
                context_entries.push(entry);
            } else {
                other_entries.push(entry);
            }
        }
    }
    for (index, command) in COMMANDS.iter().enumerate() {
        if (command.available)(app) {
            other_entries.push(PaletteEntry {
                label: command.name.to_owned(),
                doc: None,
                kind: PaletteEntryKind::Builtin(index),
            });
        }
    }
    context_entries.append(&mut other_entries);
    (context_entries, errors)
}

fn start_palette(app: &mut App) {
    app.debug("palette: computing entries...");
    let (matches, errors) = palette_entries(app);
    app.debug(format!(
        "palette: {} entries, {} error(s)",
        matches.len(),
        errors.len()
    ));
    // Nothing to run and nothing to report -- opening the palette anyway
    // would drop the user into a filter box that can never match
    // anything, with no visible sign they're still "inside" it (every
    // further keystroke just extends an unmatchable query instead of
    // doing what it looks like it should). Report it on the status line
    // instead and skip the dead end. A script error still opens the
    // palette, since that's worth seeing even with zero runnable entries.
    if matches.is_empty() && errors.is_empty() {
        app.palette = None;
        app.status = "no commands available".into();
        return;
    }
    app.palette = Some(ActionPalette {
        query: String::new(),
        matches,
        active: 0,
        errors,
    });
    app.status = "run action: type to filter, Enter runs, Esc cancels".into();
}

fn handle_palette_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let entry = app
                .palette
                .as_ref()
                .and_then(|palette| palette.matches.get(palette.active).cloned());
            app.palette = None;
            match entry {
                Some(PaletteEntry {
                    kind: PaletteEntryKind::Action(position),
                    ..
                }) => {
                    // Capture the selection as it stood *before* the action
                    // node takes it over below, so the env vars describe the
                    // node the user meant to act on, not the action node
                    // itself.
                    let target = app
                        .selected_row()
                        .map(|row| row.position)
                        .unwrap_or_else(|| position.clone());
                    reveal_and_select(app, &position);
                    run_action(app, &position, &target);
                }
                Some(PaletteEntry {
                    kind: PaletteEntryKind::Command { script, name },
                    ..
                }) => {
                    run_command(app, &script, &name);
                }
                Some(PaletteEntry {
                    kind: PaletteEntryKind::Builtin(index),
                    ..
                }) => {
                    (COMMANDS[index].run)(app);
                }
                None => {
                    app.status = "no matching action".into();
                }
            }
        }
        KeyCode::Esc => {
            app.palette = None;
            app.status = "action palette cancelled".into();
        }
        KeyCode::Backspace => {
            app.palette
                .as_mut()
                .expect("palette input exists")
                .query
                .pop();
            update_palette_matches(app);
        }
        KeyCode::Down => cycle_palette_match(app, 1),
        KeyCode::Up => cycle_palette_match(app, -1),
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            app.palette
                .as_mut()
                .expect("palette input exists")
                .query
                .push(character);
            update_palette_matches(app);
        }
        _ => {}
    }
}

fn update_palette_matches(app: &mut App) {
    let query = app
        .palette
        .as_ref()
        .expect("palette input exists")
        .query
        .to_lowercase();
    let (all_entries, errors) = palette_entries(app);
    let matches = all_entries
        .into_iter()
        .filter(|entry| entry.label.to_lowercase().contains(&query))
        .collect::<Vec<_>>();
    let palette = app.palette.as_mut().expect("palette input exists");
    palette.active = palette.active.min(matches.len().saturating_sub(1));
    palette.matches = matches;
    palette.errors = errors;
}

fn cycle_palette_match(app: &mut App, delta: isize) {
    let palette = app.palette.as_mut().expect("palette input exists");
    if palette.matches.is_empty() {
        return;
    }
    let len = palette.matches.len() as isize;
    palette.active = (palette.active as isize + delta).rem_euclid(len) as usize;
}

fn command_import_available(app: &App) -> bool {
    app.selected_row().is_some_and(|row| {
        path_directive(&app.document.outline.nodes[&row.node].headline).is_some()
    })
}

/// Scans the selected `@path` node's directory for files that don't already
/// have a matching `@auto`/`@file`/... child, and adds one `@auto <name>`
/// node per new file, immediately loading and expanding its content so it's
/// visible without a save+reload round trip. Subdirectories without a
/// matching `@path` child get an `@path <name>` node of their own, so a
/// directory holding only subdirectories still produces something to import
/// into next, rather than requiring each `@path` node to be created by hand
/// first.
fn command_import_run(app: &mut App) {
    let Some(row) = app.selected_row() else {
        app.status = "select a @path node to import into".into();
        return;
    };
    if path_directive(&app.document.outline.nodes[&row.node].headline).is_none() {
        app.status = "select a @path node to import into".into();
        return;
    }
    if !app.editable(&row) {
        return;
    }
    let Some(position) = app.document.outline.position(&row.position) else {
        return;
    };
    let existing_files: HashSet<String> = position
        .children
        .iter()
        .filter_map(|child| {
            derived_filename(&app.document.outline.nodes[&child.node].headline)
                .map(|(_, _, filename)| filename.to_owned())
        })
        .collect();
    let existing_dirs: HashSet<String> = position
        .children
        .iter()
        .filter_map(|child| path_directive(&app.document.outline.nodes[&child.node].headline))
        .collect();

    let directory = resolved_directory(app, &row);
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) => {
            app.status = format!("import failed to read {}: {error}", directory.display());
            return;
        }
    };
    let mut new_files = Vec::new();
    let mut new_dirs = Vec::new();
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if kind.is_file() && !existing_files.contains(&name) {
            new_files.push(name);
        } else if kind.is_dir() && !existing_dirs.contains(&name) {
            new_dirs.push(name);
        }
    }
    new_files.sort();
    new_dirs.sort();

    if new_files.is_empty() && new_dirs.is_empty() {
        app.status = format!(
            "no new files or subdirectories to import from {}",
            directory.display()
        );
        return;
    }

    let file_count = new_files.len();
    let dir_count = new_dirs.len();
    let mut child_index = position.children.len();
    let mut jobs = Vec::new();
    let mut new_positions = Vec::new();
    let headlines = new_files
        .into_iter()
        .map(|filename| (format!("@auto {filename}"), Some(filename)))
        .chain(
            new_dirs
                .into_iter()
                .map(|name| (format!("@path {name}"), None)),
        );
    for (headline, filename) in headlines {
        let mut id = fresh_node_id();
        while app.document.outline.nodes.contains_key(&id) {
            id = fresh_node_id();
        }
        app.document.outline.nodes.insert(
            id.clone(),
            leo::Node {
                id: id.clone(),
                headline,
                body: String::new(),
                vnode_attributes: HashMap::new(),
                tnode_attributes: HashMap::new(),
            },
        );
        let Some(children) = children_mut(&mut app.document.outline, Some(&row.position)) else {
            continue;
        };
        children.push(Position {
            node: id.clone(),
            children: Vec::new(),
        });
        let position_id = if row.position.0.is_empty() {
            child_index.to_string()
        } else {
            format!("{}/{}", row.position.0, child_index)
        };
        child_index += 1;
        if let Some(filename) = filename {
            jobs.push(DerivedJob {
                position: PositionId(position_id.clone()),
                path: directory.join(filename),
                auto: true,
                directive: "@auto".to_owned(),
                root: id,
            });
        }
        new_positions.push(PositionId(position_id));
    }

    let report = load_derived_jobs(&mut app.document.outline, jobs);
    app.source_locations.extend(report.locations);
    app.source_nodes.extend(report.node_locations);
    app.derived_nodes.extend(report.derived_nodes);
    app.writable_external.extend(report.writable_external);
    app.original_external
        .children
        .extend(report.original_children);
    app.original_external.bodies.extend(report.original_bodies);
    app.original_external.nodes.extend(report.original_nodes);
    for position in new_positions {
        app.expanded.insert(position);
    }

    app.dirty = true;
    app.quit_armed = false;
    let load_note = if report.errors.is_empty() {
        String::new()
    } else {
        format!(
            "; {} error(s) loading content: {}",
            report.errors.len(),
            report.errors.join(" | ")
        )
    };
    app.status = format!(
        "imported {file_count} new file(s) and {dir_count} new subdirectory(ies){load_note} \
         (Ctrl-S to save)"
    );
}

/// Resolves a node's on-disk directory the same way derived `@auto`/`@file`
/// bodies are located: the outline's own directory plus every ancestor
/// (and the node's own) `@path` directive, in order.
fn resolved_directory(app: &App, row: &Row) -> PathBuf {
    let mut path = app
        .path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let mut prefix = String::new();
    for component in row.position.0.split('/') {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(component);
        let Some(position) = app.document.outline.position(&PositionId(prefix.clone())) else {
            break;
        };
        let node = &app.document.outline.nodes[&position.node];
        if let Some(directory) =
            path_directive(&node.headline).or_else(|| path_directive(&node.body))
        {
            path.push(directory);
        }
    }
    path
}

/// Builds the slash-separated headline path (see `Outline::resolve_headline_path`)
/// from the outline roots down to `row`, escaping any literal `/` or `\` in
/// a headline so the result round-trips back through that resolver.
fn headline_path(app: &App, row: &Row) -> String {
    let mut parts = Vec::new();
    let mut prefix = String::new();
    for component in row.position.0.split('/') {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(component);
        let Some(position) = app.document.outline.position(&PositionId(prefix.clone())) else {
            break;
        };
        let headline = &app.document.outline.nodes[&position.node].headline;
        parts.push(escape_headline_path_component(headline));
    }
    parts.join("/")
}

fn escape_headline_path_component(headline: &str) -> String {
    headline.replace('\\', "\\\\").replace('/', "\\/")
}

/// Removes `@language xxx` directive lines from a body before it is run as
/// a rhai script: legacy bodies may still carry a directive (from before
/// every `@action` implicitly ran as rhai), but it isn't itself valid rhai.
fn strip_language_directive(body: &str) -> String {
    body.lines()
        .filter(|line| {
            let is_directive = line
                .trim_start()
                .strip_prefix("@language")
                .is_some_and(|rest| rest.split_whitespace().next().is_some());
            !is_directive
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Marks the outline dirty and drops caches keyed on the old outline
/// contents/layout after a `doc`-mutating rhai action changed it in place.
fn mark_outline_touched(app: &mut App) {
    app.dirty = true;
    app.quit_armed = false;
    #[cfg(feature = "syntax")]
    app.highlight_cache.clear();
    #[cfg(feature = "syntax")]
    app.preview_cache.clear();
    app.source_locations.clear();
    app.selected = app.selected.min(app.rows().len().saturating_sub(1));
}

/// Runs the body of the `@action` node at `position` as a rhai script and
/// puts the result in `app.action_output`, which the body pane shows in
/// place of the node's body until the selection moves to a different node.
///
/// `doc` (bound to the outline already open in this session) and `target`
/// (the gnx of the node the user had selected when they invoked the action
/// -- not the `@action` node itself, which may live anywhere in the tree)
/// are predefined in the script's scope. Its `print`/`debug` output becomes
/// the action's displayed output.
fn run_action(app: &mut App, position: &PositionId, target: &PositionId) {
    let Some(row) = all_rows(&app.document.outline)
        .into_iter()
        .find(|row| &row.position == position)
    else {
        return;
    };
    let node = &app.document.outline.nodes[&row.node];
    let name = action_name(&node.headline).to_owned();
    // Falls back to the action's own row when `target` no longer resolves
    // (e.g. it was removed by an earlier action in the same session).
    let target_row = all_rows(&app.document.outline)
        .into_iter()
        .find(|row| &row.position == target)
        .unwrap_or_else(|| row.clone());
    // The `@language` directive isn't itself valid rhai, so it must be
    // stripped before the body reaches the engine.
    let body = strip_language_directive(&node.body);

    app.status = format!("running '{name}' with rhai...");
    app.debug(format!("action: running '{name}'..."));
    let document = std::mem::replace(&mut app.document, LeoDocument::empty());
    let outcome =
        crate::rhai_run::run_bound(document, app.path.clone(), &target_row.position, &body);
    app.debug(format!(
        "action: '{name}' returned status={:?}",
        outcome.status
    ));
    app.document = outcome.document;
    if outcome.touched {
        mark_outline_touched(app);
    }
    push_log_lines(app, &name, &outcome.stdout);
    push_log_lines(app, &name, &outcome.stderr);
    let (interpreter, status, stdout, stderr) =
        ("rhai", outcome.status, outcome.stdout, outcome.stderr);

    let mut text = stdout;
    if !stderr.is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&stderr);
    }

    app.status = match status {
        Some(0) => format!("'{name}' finished"),
        Some(code) => format!("'{name}' exited with status {code}"),
        None => format!("'{name}' did not complete"),
    };
    app.action_output = Some(ActionOutput {
        node: row.node,
        name,
        interpreter,
        status,
        text,
    });
}

/// Runs the palette command named `fn_name` from the `@import`ed script at
/// `script` and puts the result in `app.action_output`, shown in the
/// currently selected node's body pane until the selection moves -- unlike
/// `run_action`, a command isn't itself an outline node, so there's no
/// action node to reveal/select first; whatever was already selected keeps
/// its place, is passed to the command as `target`, and shows the output.
fn run_command(app: &mut App, script: &Path, fn_name: &str) {
    let Some(row) = app.selected_row() else {
        app.status = format!("select a node first to run '{fn_name}'");
        return;
    };
    let node = row.node.clone();

    app.status = format!("running '{fn_name}' with rhai...");
    app.debug(format!(
        "command: running '{fn_name}' from {}...",
        script.display()
    ));
    let document = std::mem::replace(&mut app.document, LeoDocument::empty());
    let outcome =
        crate::rhai_run::run_command(document, app.path.clone(), script, &row.position, fn_name);
    app.debug(format!(
        "command: '{fn_name}' returned status={:?}",
        outcome.status
    ));
    app.document = outcome.document;
    if outcome.touched {
        mark_outline_touched(app);
    }
    push_log_lines(app, fn_name, &outcome.stdout);
    push_log_lines(app, fn_name, &outcome.stderr);
    let (status, stdout, stderr) = (outcome.status, outcome.stdout, outcome.stderr);

    let mut text = stdout;
    if !stderr.is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&stderr);
    }

    app.status = match status {
        Some(0) => format!("'{fn_name}' finished"),
        Some(code) => format!("'{fn_name}' exited with status {code}"),
        None => format!("'{fn_name}' did not complete"),
    };
    app.action_output = Some(ActionOutput {
        node,
        name: fn_name.to_owned(),
        interpreter: "rhai",
        status,
        text,
    });
}

fn update_search_matches(app: &mut App, active: usize) {
    let query = app
        .search
        .as_ref()
        .expect("search input exists")
        .query
        .clone();
    let matches = if query.is_empty() {
        Vec::new()
    } else if let Ok(pattern) = RegexBuilder::new(&regex::escape(&query))
        .case_insensitive(true)
        .build()
    {
        search_outline(&app.document.outline, &[pattern])
            .into_iter()
            .map(|result| result.position)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let active = active.min(matches.len().saturating_sub(1));
    let selected = matches.get(active).cloned();
    let search = app.search.as_mut().expect("search input exists");
    search.matches = matches;
    search.active = active;
    if let Some(position) = selected {
        reveal_and_select(app, &position);
        scroll_body_to_first_match(app, &query);
    }
}

fn cycle_search_match(app: &mut App, delta: isize) {
    let search = app.search.as_ref().expect("search input exists");
    if search.matches.is_empty() {
        return;
    }
    let len = search.matches.len() as isize;
    let active = (search.active as isize + delta).rem_euclid(len) as usize;
    let position = search.matches[active].clone();
    let query = search.query.clone();
    app.search.as_mut().expect("search input exists").active = active;
    reveal_and_select(app, &position);
    scroll_body_to_first_match(app, &query);
}

/// Scrolls the body pane to the first line matching `query`, so a
/// body-content hit is visible without extra manual scrolling.
fn scroll_body_to_first_match(app: &mut App, query: &str) {
    if query.is_empty() {
        return;
    }
    let Some(row) = app.selected_row() else {
        return;
    };
    let needle = query.to_lowercase();
    let line = app.document.outline.nodes[&row.node]
        .body
        .lines()
        .position(|line| line.to_lowercase().contains(&needle))
        .unwrap_or(0);
    app.body_scroll = line;
}

fn all_rows(outline: &Outline) -> Vec<Row> {
    fn collect(position: &Position, path: String, depth: usize, rows: &mut Vec<Row>) {
        rows.push(Row {
            position: PositionId(path.clone()),
            node: position.node.clone(),
            depth,
            has_children: !position.children.is_empty(),
        });
        for (index, child) in position.children.iter().enumerate() {
            collect(child, format!("{path}/{index}"), depth + 1, rows);
        }
    }
    let mut rows = Vec::new();
    for (index, root) in outline.roots.iter().enumerate() {
        collect(root, index.to_string(), 0, &mut rows);
    }
    rows
}

/// Captures which *nodes* (not positions) are currently expanded, so their
/// expand state can survive a structural edit that shifts `PositionId`
/// index paths out from under them (paste, cut, move/indent/outdent).
fn snapshot_expanded_nodes(app: &App) -> HashSet<NodeId> {
    app.expanded
        .iter()
        .filter_map(|position| {
            app.document
                .outline
                .position(position)
                .map(|entry| entry.node.clone())
        })
        .collect()
}

/// Re-locates the nodes captured by `snapshot_expanded_nodes` in the
/// (now mutated) outline and rebuilds `app.expanded` from their current
/// positions, dropping any that were removed by the edit.
fn restore_expanded_nodes(app: &mut App, nodes: HashSet<NodeId>) {
    if nodes.is_empty() {
        return;
    }
    app.expanded = all_rows(&app.document.outline)
        .into_iter()
        .filter(|row| nodes.contains(&row.node))
        .map(|row| row.position)
        .collect();
}

/// Captures which *nodes* (not positions) are currently marked, so mark
/// state can survive a structural edit that shifts `PositionId` index paths
/// out from under them -- same rationale as `snapshot_expanded_nodes`. A
/// node cut along with its mark simply won't be found by
/// `restore_marked_nodes` afterwards, which is what clears it.
fn snapshot_marked_nodes(app: &App) -> HashSet<NodeId> {
    app.marked
        .iter()
        .filter_map(|position| {
            app.document
                .outline
                .position(position)
                .map(|entry| entry.node.clone())
        })
        .collect()
}

fn restore_marked_nodes(app: &mut App, nodes: HashSet<NodeId>) {
    if nodes.is_empty() {
        return;
    }
    app.marked = all_rows(&app.document.outline)
        .into_iter()
        .filter(|row| nodes.contains(&row.node))
        .map(|row| row.position)
        .collect();
}

fn reveal_and_select(app: &mut App, position: &PositionId) {
    let components = position.0.split('/').collect::<Vec<_>>();
    for end in 1..components.len() {
        app.expanded.insert(PositionId(components[..end].join("/")));
    }
    select_position(app, position);
}

/// `n`/`N`: jumps to the next/previous occurrence of the selected node,
/// wrapping around, expanding ancestors as needed so a collapsed occurrence
/// is still reachable.
fn cycle_clone(app: &mut App, direction: isize) {
    let Some(row) = app.selected_row() else {
        return;
    };
    let positions = positions_of(&app.document.outline, &row.node);
    if positions.len() <= 1 {
        app.status = "node has no other occurrences".into();
        return;
    }
    let Some(current) = positions.iter().position(|p| p == &row.position) else {
        return;
    };
    let len = positions.len() as isize;
    let next = (current as isize + direction).rem_euclid(len) as usize;
    reveal_and_select(app, &positions[next]);
    app.status = format!("clone {}/{}", next + 1, positions.len());
}

fn cancel_headline_edit(app: &mut App) {
    let Some(state) = app.input.take() else {
        return;
    };
    if let Some(position) = state.inserted_position {
        remove_position(&mut app.document.outline, &position);
        app.document.outline.nodes.remove(&state.node);
    } else if let Some(node) = app.document.outline.nodes.get_mut(&state.node) {
        node.headline = state.original;
    }
}

/// Core of [`commit_headline_edit`]: validates and writes the typed
/// headline. Returns `false` (leaving the edit open) if the headline is
/// empty. Doesn't chain into another insert on its own -- see
/// [`commit_headline_edit`] and [`commit_or_cancel_headline_edit`], which
/// each decide that differently.
fn commit_headline_edit_without_chaining(app: &mut App) -> bool {
    let Some(state) = app.input.as_ref() else {
        return false;
    };
    let headline = state.value().trim().to_owned();
    let node_id = state.node.clone();
    if headline.is_empty() {
        app.status = "headline may not be empty".into();
        return false;
    }
    let Some(node) = app.document.outline.nodes.get_mut(&node_id) else {
        app.status = "edited node no longer exists; edit discarded".into();
        app.input = None;
        return false;
    };
    node.headline = headline.clone();
    if let Some(row) = app.rows().iter().find(|row| row.node == node_id).cloned()
        && external_filename(&headline).is_some()
        && let Some(path) = dynamic_source_location(app, &row).map(|location| location.path)
    {
        track_external_rename(
            &mut app.writable_external,
            node_id.clone(),
            path,
            external_format(&headline),
        );
    }
    app.dirty_nodes.insert(node_id);
    app.input = None;
    app.dirty = true;
    app.quit_armed = false;
    #[cfg(feature = "syntax")]
    app.highlight_cache.clear();
    #[cfg(feature = "syntax")]
    app.preview_cache.clear();
    true
}

/// Accepts the in-progress headline edit, same as pressing Enter: commits
/// it and, if it was a freshly-inserted node, immediately starts editing
/// the next sibling. Returns `false` (leaving the edit open) if the
/// headline is empty.
fn commit_headline_edit(app: &mut App) -> bool {
    let chain = app
        .input
        .as_ref()
        .is_some_and(|state| state.inserted_position.is_some());
    if !commit_headline_edit_without_chaining(app) {
        return false;
    }
    if chain {
        insert_headline(app);
    } else {
        app.status = "headline changed (Ctrl-S to save)".into();
    }
    true
}

/// Up/Down while editing a headline: commit the in-progress text and move,
/// same as [`commit_headline_edit`] does for Enter, but without chaining
/// into another insert -- the arrow means "move on", not "add another
/// sibling". An empty headline can't be committed, so that case falls back
/// to [`cancel_headline_edit`] instead: discarding a still-empty
/// freshly-inserted node, or reverting a rename to its original text,
/// rather than leaving the edit open and blocking navigation.
fn commit_or_cancel_headline_edit(app: &mut App) {
    if commit_headline_edit_without_chaining(app) {
        app.status = "headline changed (Ctrl-S to save)".into();
    } else {
        cancel_headline_edit(app);
        app.status = "headline edit cancelled".into();
    }
}

fn handle_headline_input(app: &mut App, key: KeyEvent) {
    let Some(state) = app.input.as_mut() else {
        return;
    };
    match key.code {
        KeyCode::Enter => {
            commit_headline_edit(app);
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if commit_headline_edit(app) {
                save(app);
            }
        }
        KeyCode::Esc => {
            cancel_headline_edit(app);
            app.status = "headline edit cancelled".into();
        }
        // Ctrl-arrow reorders/(de|pro)motes the node being edited, same as
        // outside edit mode, and stays in edit mode -- it moves the tree,
        // not the text cursor or the selection.
        KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
            move_selected(app, MoveDirection::Up);
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
            move_selected(app, MoveDirection::Down);
        }
        KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
            move_selected(app, MoveDirection::Left);
        }
        KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
            move_selected(app, MoveDirection::Right);
        }
        KeyCode::Up => {
            commit_or_cancel_headline_edit(app);
            app.move_selection(-1);
        }
        KeyCode::Down => {
            commit_or_cancel_headline_edit(app);
            app.move_selection(1);
        }
        // The initial full-value selection (`select_all` in `edit_headline`)
        // is replaced automatically: `TextArea::insert_char`/`insert_str`
        // delete an active selection before inserting.
        //
        // Left/Right are a special case: `TextArea`'s default keymap moves
        // them one character from wherever the cursor logically sits (the
        // end, since `select_all` leaves it there), but exiting a selection
        // via an arrow key should collapse to the edge it points at instead.
        // Home/End don't need this -- they're already absolute moves.
        KeyCode::Left if state.input.is_selecting() => {
            state.input.cancel_selection();
            state.input.move_cursor(ratatui_textarea::CursorMove::Head);
        }
        KeyCode::Right if state.input.is_selecting() => {
            state.input.cancel_selection();
            state.input.move_cursor(ratatui_textarea::CursorMove::End);
        }
        _ => {
            state.input.input(key);
        }
    }
}

fn edit_headline(app: &mut App) {
    let Some(row) = app.selected_row() else {
        return;
    };
    if !app.editable(&row) {
        return;
    }
    let original = app.document.outline.nodes[&row.node].headline.clone();
    let mut input = ratatui_textarea::TextArea::new(vec![original.clone()]);
    input.select_all();
    app.input = Some(HeadlineInput {
        node: row.node,
        input,
        original,
        inserted_position: None,
    });
    app.status = "editing headline: Enter accepts, Esc cancels".into();
}

/// Opens the quick body entry (`b`): an inline field, prefilled with the
/// current body and fully selected so the first keystroke or paste
/// replaces it, for fast one-off entry or pasting without leaving the TUI
/// for `$EDITOR`. See [`handle_body_input`] and [`insert_paste_into_body`].
fn quick_edit_body(app: &mut App) {
    let Some(row) = app.selected_row() else {
        return;
    };
    if !app.editable(&row) {
        return;
    }
    let original = app.document.outline.nodes[&row.node].body.clone();
    let restore_body_full_width = app.body_full_width;
    let restore_outline_full_width = app.outline_full_width;
    app.body_full_width = true;
    app.outline_full_width = false;
    let is_empty = original.is_empty();
    let mut input =
        ratatui_textarea::TextArea::new(original.split('\n').map(str::to_owned).collect());
    if !is_empty {
        input.select_all();
    }
    app.body_input = Some(BodyEdit {
        node: row.node,
        input,
        restore_body_full_width,
        restore_outline_full_width,
    });
    app.status =
        "quick entry: type or paste, Ctrl-D accepts, Ctrl-S accepts+saves, Esc cancels".into();
}

fn cancel_body_edit(app: &mut App) {
    if let Some(edit) = app.body_input.take() {
        app.body_full_width = edit.restore_body_full_width;
        app.outline_full_width = edit.restore_outline_full_width;
    }
    app.status = "body edit cancelled".into();
}

/// Accepts the in-progress body edit. Unlike [`commit_headline_edit`], an
/// empty body is valid (it just clears the node), so there's nothing to
/// reject here -- this always succeeds once `body_input` is set.
fn commit_body_edit(app: &mut App) -> bool {
    let Some(edit) = app.body_input.as_ref() else {
        return false;
    };
    let body = edit.input.lines().join("\n");
    let node_id = edit.node.clone();
    let restore_body_full_width = edit.restore_body_full_width;
    let restore_outline_full_width = edit.restore_outline_full_width;
    app.document
        .outline
        .nodes
        .get_mut(&node_id)
        .expect("edited node exists")
        .body = body;
    app.dirty_nodes.insert(node_id);
    app.body_input = None;
    app.body_full_width = restore_body_full_width;
    app.outline_full_width = restore_outline_full_width;
    app.dirty = true;
    app.quit_armed = false;
    #[cfg(feature = "syntax")]
    app.highlight_cache.clear();
    #[cfg(feature = "syntax")]
    app.preview_cache.clear();
    app.body_scroll = 0;
    app.body_horizontal_scroll = 0;
    app.status = "body changed (Ctrl-S to save)".into();
    true
}

/// Inserts clipboard text pasted (via bracketed paste) while the quick body
/// entry is open, as one atomic edit (so a single Ctrl-Z / Backspace at the
/// following char position doesn't shred it back apart character by
/// character).
fn insert_paste_into_body(app: &mut App, text: String) {
    let Some(edit) = app.body_input.as_mut() else {
        return;
    };
    edit.input.insert_str(text);
}

/// `Ctrl-D`/`Ctrl-S`/`Esc` are intercepted here rather than forwarded to
/// [`ratatui_textarea::TextArea::input`]: its own default keymap binds
/// `Ctrl-D` to deleting the character under the cursor (an Emacs-style
/// binding), which would collide with this app's "commit" shortcut.
fn handle_body_input(app: &mut App, key: KeyEvent) {
    let Some(edit) = app.body_input.as_mut() else {
        return;
    };
    match key.code {
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            commit_body_edit(app);
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if commit_body_edit(app) {
                save(app);
            }
        }
        KeyCode::Esc => cancel_body_edit(app),
        _ => {
            edit.input.input(key);
        }
    }
}

fn insert_headline(app: &mut App) {
    let Some(row) = app.selected_row() else {
        return;
    };
    if !app.editable(&row) {
        return;
    }
    let Some((parent, index)) = split_position(&row.position) else {
        return;
    };
    let id = fresh_node_id();
    let position = Position {
        node: id.clone(),
        children: Vec::new(),
    };
    let Some(siblings) = children_mut(&mut app.document.outline, parent.as_ref()) else {
        return;
    };
    siblings.insert(index + 1, position);
    let inserted = join_position(parent.as_ref(), index + 1);
    app.document.outline.nodes.insert(
        id.clone(),
        leo::Node {
            id: id.clone(),
            headline: "New Headline".into(),
            body: String::new(),
            vnode_attributes: HashMap::new(),
            tnode_attributes: HashMap::new(),
        },
    );
    select_position(app, &inserted);
    app.input = Some(HeadlineInput {
        node: id,
        input: ratatui_textarea::TextArea::default(),
        original: String::new(),
        inserted_position: Some(inserted),
    });
    app.status = "new headline: type a name, Enter accepts and adds another, Esc cancels".into();
}

fn copy_selected(app: &mut App) {
    let rows = selected_tree_roots(app);
    if rows.is_empty() {
        return;
    }
    let roots: Vec<_> = rows
        .iter()
        .filter_map(|row| app.document.outline.position(&row.position).cloned())
        .collect();
    let ids = referenced_nodes(&roots);
    let nodes = ids
        .into_iter()
        .filter_map(|id| {
            app.document
                .outline
                .nodes
                .get(&id)
                .cloned()
                .map(|node| (id, node))
        })
        .collect();
    let count = roots.len();
    app.clipboard = Some(ClipboardTree { roots, nodes });
    app.status = format!(
        "{count} tree{} copied; v pastes a copy, Shift-V pastes clones",
        if count == 1 { "" } else { "s" }
    );
}

fn cut_selected(app: &mut App) {
    let mut rows = selected_tree_roots(app);
    if rows.is_empty() {
        return;
    }
    if rows
        .iter()
        .any(|row| cut_would_orphan_derived_content(app, &row.position))
    {
        app.status = "@auto subtrees cannot be cut".into();
        return;
    }
    copy_selected(app);
    rows.sort_by_key(|row| path_indices(&row.position));
    let expanded_nodes = snapshot_expanded_nodes(app);
    let marked_nodes = snapshot_marked_nodes(app);
    for row in rows.iter().rev() {
        remove_position(&mut app.document.outline, &row.position);
    }
    let referenced = referenced_nodes(&app.document.outline.roots);
    app.document
        .outline
        .nodes
        .retain(|id, _| referenced.contains(id));
    app.dirty = true;
    app.quit_armed = false;
    #[cfg(feature = "syntax")]
    app.highlight_cache.clear();
    #[cfg(feature = "syntax")]
    app.preview_cache.clear();
    app.source_locations.clear();
    restore_expanded_nodes(app, expanded_nodes);
    restore_marked_nodes(app, marked_nodes);
    app.selected = app.selected.min(app.rows().len().saturating_sub(1));
    app.selection_anchor = None;
    app.status = format!(
        "{} tree{} cut; v pastes copies, Shift-V retains identities (Ctrl-S to save)",
        rows.len(),
        if rows.len() == 1 { "" } else { "s" }
    );
}

fn paste_tree(app: &mut App, as_clone: bool) {
    let Some(clipboard) = app.clipboard.clone() else {
        app.status = "tree clipboard is empty".into();
        return;
    };
    if as_clone
        && clipboard
            .roots
            .iter()
            .any(|root| app.readonly_derived(&root.node))
    {
        app.status = "cannot clone @auto derived content; only its root can be cloned".into();
        return;
    }
    let (parent, insert_at) = if let Some(row) = app.selected_row() {
        if !app.editable(&row) {
            app.status = "cannot paste beside an @auto subtree".into();
            return;
        }
        let Some((parent, index)) = split_position(&row.position) else {
            return;
        };
        (parent, index + 1)
    } else {
        (None, 0)
    };
    if as_clone
        && parent.as_ref().is_some_and(|parent| {
            app.document
                .outline
                .position(parent)
                .is_some_and(|position| clipboard.nodes.contains_key(&position.node))
        })
    {
        app.status = "cannot paste a clone inside its own tree".into();
        return;
    }
    if as_clone {
        for (id, node) in &clipboard.nodes {
            app.document
                .outline
                .nodes
                .entry(id.clone())
                .or_insert_with(|| node.clone());
        }
        let expanded_nodes = snapshot_expanded_nodes(app);
        let marked_nodes = snapshot_marked_nodes(app);
        let Some(siblings) = children_mut(&mut app.document.outline, parent.as_ref()) else {
            return;
        };
        let count = clipboard.roots.len();
        siblings.splice(insert_at..insert_at, clipboard.roots);
        let target = join_position(parent.as_ref(), insert_at);
        app.dirty = true;
        app.quit_armed = false;
        #[cfg(feature = "syntax")]
        app.highlight_cache.clear();
        #[cfg(feature = "syntax")]
        app.preview_cache.clear();
        app.source_locations.clear();
        restore_expanded_nodes(app, expanded_nodes);
        restore_marked_nodes(app, marked_nodes);
        select_position(app, &target);
        app.status = format!("{count} tree(s) pasted as clones (Ctrl-S to save)");
        app.flash = Some((format!("PASTED {count} TREE(S) AS CLONES"), Instant::now()));
        return;
    }
    let mut ids = HashMap::new();
    for old in clipboard.nodes.keys() {
        let mut id = fresh_node_id();
        while app.document.outline.nodes.contains_key(&id) || ids.values().any(|v| v == &id) {
            id = fresh_node_id();
        }
        ids.insert(old.clone(), id);
    }
    let remap = |position: &Position, remap: &HashMap<NodeId, NodeId>| -> Position {
        fn visit(position: &Position, remap: &HashMap<NodeId, NodeId>) -> Position {
            Position {
                node: remap[&position.node].clone(),
                children: position
                    .children
                    .iter()
                    .map(|child| visit(child, remap))
                    .collect(),
            }
        }
        visit(position, remap)
    };
    let pasted: Vec<_> = clipboard
        .roots
        .iter()
        .map(|root| remap(root, &ids))
        .collect();
    for (old, mut node) in clipboard.nodes {
        let id = ids[&old].clone();
        node.id.clone_from(&id);
        app.document.outline.nodes.insert(id, node);
    }
    let expanded_nodes = snapshot_expanded_nodes(app);
    let marked_nodes = snapshot_marked_nodes(app);
    let Some(siblings) = children_mut(&mut app.document.outline, parent.as_ref()) else {
        return;
    };
    let count = pasted.len();
    siblings.splice(insert_at..insert_at, pasted);
    let target = join_position(parent.as_ref(), insert_at);
    app.dirty = true;
    app.quit_armed = false;
    #[cfg(feature = "syntax")]
    app.highlight_cache.clear();
    #[cfg(feature = "syntax")]
    app.preview_cache.clear();
    app.source_locations.clear();
    restore_expanded_nodes(app, expanded_nodes);
    restore_marked_nodes(app, marked_nodes);
    select_position(app, &target);
    app.status = format!("{count} tree(s) pasted (Ctrl-S to save)");
    app.flash = Some((
        format!("PASTED {count} INDEPENDENT TREE(S)"),
        Instant::now(),
    ));
}

fn selected_tree_roots(app: &App) -> Vec<Row> {
    let rows = if app.marked.is_empty() {
        app.selected_rows()
    } else {
        app.rows()
            .into_iter()
            .filter(|row| app.marked.contains(&row.position))
            .collect()
    };
    let selected: HashSet<_> = rows.iter().map(|row| row.position.0.clone()).collect();
    rows.into_iter()
        .filter(|row| {
            let mut path = row.position.0.as_str();
            while let Some((parent, _)) = path.rsplit_once('/') {
                if selected.contains(parent) {
                    return false;
                }
                path = parent;
            }
            true
        })
        .collect()
}

fn path_indices(position: &PositionId) -> Vec<usize> {
    position
        .0
        .split('/')
        .filter_map(|part| part.parse().ok())
        .collect()
}

fn move_selected(app: &mut App, direction: MoveDirection) {
    let selected = selected_tree_roots(app);
    if selected.len() > 1 {
        move_selected_block(app, direction, selected);
        return;
    }
    let Some(row) = app.selected_row() else {
        return;
    };
    if !app.editable(&row) {
        app.status = "@auto subtrees cannot be moved".into();
        return;
    }
    let Some((parent, index)) = split_position(&row.position) else {
        return;
    };
    let mut expanded_nodes = snapshot_expanded_nodes(app);
    let marked_nodes = snapshot_marked_nodes(app);
    let target = match direction {
        MoveDirection::Up | MoveDirection::Down => {
            let Some(siblings) = children_mut(&mut app.document.outline, parent.as_ref()) else {
                return;
            };
            let other = if matches!(direction, MoveDirection::Up) {
                index.checked_sub(1)
            } else if index + 1 < siblings.len() {
                Some(index + 1)
            } else {
                None
            };
            let Some(other) = other else {
                app.status = "node is already at the edge".into();
                return;
            };
            siblings.swap(index, other);
            join_position(parent.as_ref(), other)
        }
        MoveDirection::Right => {
            let Some(previous) = index.checked_sub(1) else {
                app.status = "no previous sibling to become parent".into();
                return;
            };
            let previous_node = sibling_node_at(&app.document.outline, parent.as_ref(), previous);
            if previous_node.is_some_and(|node| app.refuses_new_children(&node)) {
                app.status = "@auto/@auto-dir nodes cannot take new children".into();
                return;
            }
            let Some(siblings) = children_mut(&mut app.document.outline, parent.as_ref()) else {
                return;
            };
            let position = siblings.remove(index);
            let child_index = siblings[previous].children.len();
            expanded_nodes.insert(siblings[previous].node.clone());
            siblings[previous].children.push(position);
            let previous_path = join_position(parent.as_ref(), previous);
            join_position(Some(&previous_path), child_index)
        }
        MoveDirection::Left => {
            let Some(parent_id) = parent else {
                app.status = "top-level nodes cannot be promoted".into();
                return;
            };
            let Some((grandparent, parent_index)) = split_position(&parent_id) else {
                return;
            };
            let position = {
                let Some(siblings) = children_mut(&mut app.document.outline, Some(&parent_id))
                else {
                    return;
                };
                siblings.remove(index)
            };
            let Some(grand_siblings) =
                children_mut(&mut app.document.outline, grandparent.as_ref())
            else {
                return;
            };
            grand_siblings.insert(parent_index + 1, position);
            join_position(grandparent.as_ref(), parent_index + 1)
        }
    };
    app.dirty = true;
    app.dirty_nodes.insert(row.node);
    app.quit_armed = false;
    #[cfg(feature = "syntax")]
    app.highlight_cache.clear();
    #[cfg(feature = "syntax")]
    app.preview_cache.clear();
    app.source_locations.clear();
    restore_expanded_nodes(app, expanded_nodes);
    restore_marked_nodes(app, marked_nodes);
    select_position(app, &target);
    app.status = "node moved (Ctrl-S to save)".into();
}

fn move_selected_block(app: &mut App, direction: MoveDirection, rows: Vec<Row>) {
    if rows.iter().any(|row| app.readonly_derived(&row.node)) {
        app.status = "@auto subtrees cannot be moved".into();
        return;
    }
    let locations: Vec<_> = rows
        .iter()
        .filter_map(|row| split_position(&row.position))
        .collect();
    if locations.len() != rows.len()
        || locations
            .iter()
            .any(|(parent, _)| parent != &locations[0].0)
        || locations.windows(2).any(|pair| pair[1].1 != pair[0].1 + 1)
    {
        app.status = "multi-node moves require consecutive siblings".into();
        return;
    }
    let parent = locations[0].0.clone();
    let start = locations[0].1;
    let count = rows.len();
    let mut expanded_nodes = snapshot_expanded_nodes(app);
    let marked_nodes = snapshot_marked_nodes(app);
    let (first, last) = match direction {
        MoveDirection::Up => {
            let Some(insert_at) = start.checked_sub(1) else {
                app.status = "selection is already at the edge".into();
                return;
            };
            let siblings = children_mut(&mut app.document.outline, parent.as_ref()).unwrap();
            let block: Vec<_> = siblings.drain(start..start + count).collect();
            siblings.splice(insert_at..insert_at, block);
            (
                join_position(parent.as_ref(), insert_at),
                join_position(parent.as_ref(), insert_at + count - 1),
            )
        }
        MoveDirection::Down => {
            let siblings = children_mut(&mut app.document.outline, parent.as_ref()).unwrap();
            if start + count >= siblings.len() {
                app.status = "selection is already at the edge".into();
                return;
            }
            let block: Vec<_> = siblings.drain(start..start + count).collect();
            let insert_at = start + 1;
            siblings.splice(insert_at..insert_at, block);
            (
                join_position(parent.as_ref(), insert_at),
                join_position(parent.as_ref(), insert_at + count - 1),
            )
        }
        MoveDirection::Right => {
            let Some(previous) = start.checked_sub(1) else {
                app.status = "no previous sibling to become parent".into();
                return;
            };
            let previous_node = sibling_node_at(&app.document.outline, parent.as_ref(), previous);
            if previous_node.is_some_and(|node| app.refuses_new_children(&node)) {
                app.status = "@auto/@auto-dir nodes cannot take new children".into();
                return;
            }
            let siblings = children_mut(&mut app.document.outline, parent.as_ref()).unwrap();
            let block: Vec<_> = siblings.drain(start..start + count).collect();
            let child_index = siblings[previous].children.len();
            expanded_nodes.insert(siblings[previous].node.clone());
            siblings[previous].children.extend(block);
            let parent_path = join_position(parent.as_ref(), previous);
            (
                join_position(Some(&parent_path), child_index),
                join_position(Some(&parent_path), child_index + count - 1),
            )
        }
        MoveDirection::Left => {
            let Some(parent_id) = parent else {
                app.status = "top-level nodes cannot be promoted".into();
                return;
            };
            let Some((grandparent, parent_index)) = split_position(&parent_id) else {
                return;
            };
            let block = {
                let siblings = children_mut(&mut app.document.outline, Some(&parent_id)).unwrap();
                siblings.drain(start..start + count).collect::<Vec<_>>()
            };
            let insert_at = parent_index + 1;
            let siblings = children_mut(&mut app.document.outline, grandparent.as_ref()).unwrap();
            siblings.splice(insert_at..insert_at, block);
            (
                join_position(grandparent.as_ref(), insert_at),
                join_position(grandparent.as_ref(), insert_at + count - 1),
            )
        }
    };
    app.dirty = true;
    app.dirty_nodes.extend(rows.into_iter().map(|row| row.node));
    app.quit_armed = false;
    #[cfg(feature = "syntax")]
    app.highlight_cache.clear();
    #[cfg(feature = "syntax")]
    app.preview_cache.clear();
    app.source_locations.clear();
    restore_expanded_nodes(app, expanded_nodes);
    restore_marked_nodes(app, marked_nodes);
    select_position(app, &first);
    let first_index = app.selected;
    select_position(app, &last);
    app.selection_anchor = Some(first_index);
    app.status = format!("{count} nodes moved (Ctrl-S to save)");
}

fn save(app: &mut App) {
    match save_document(
        &app.document,
        &app.path,
        &mut app.writable_external,
        &app.original_external,
    ) {
        Ok(()) => {
            app.dirty = false;
            app.dirty_nodes.clear();
            app.quit_armed = false;
            app.reload_armed = false;
            app.status = format!("saved {}", app.path.display());
        }
        Err(error) => app.status = format!("save failed: {error}"),
    }
}

fn reload(app: &mut App) {
    if app.dirty && !app.reload_armed {
        app.reload_armed = true;
        app.status =
            "unsaved changes; press Ctrl-R again to discard and reload, or Ctrl-S to save".into();
        return;
    }

    let selected_node = app.selected_row().map(|row| row.node);
    let previous_bodies: HashMap<NodeId, String> = app
        .document
        .outline
        .nodes
        .iter()
        .map(|(id, node)| (id.clone(), node.body.clone()))
        .collect();
    let mut document = match LeoDocument::open(&app.path) {
        Ok(document) => document,
        Err(error) => {
            app.status = format!("reload failed: {error}");
            return;
        }
    };
    let (
        source_locations,
        source_nodes,
        derived_nodes,
        writable_external,
        original_external,
        derived_status,
    ) = if app.load_derived {
        let report = load_derived_files(&mut document.outline, &app.path);
        let status = if report.errors.is_empty() {
            format!("loaded {} derived file(s)", report.loaded)
        } else {
            format!(
                "loaded {}; {} error(s): {}",
                report.loaded,
                report.errors.len(),
                report.errors.join(" | ")
            )
        };
        (
            report.locations,
            report.node_locations,
            report.derived_nodes,
            report.writable_external,
            OriginalExternalState {
                children: report.original_children,
                bodies: report.original_bodies,
                nodes: report.original_nodes,
            },
            status,
        )
    } else {
        (
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
            OriginalExternalState::default(),
            "derived files disabled".to_owned(),
        )
    };

    let changed_bodies: HashSet<NodeId> = document
        .outline
        .nodes
        .iter()
        .filter(|(id, node)| {
            previous_bodies
                .get(id)
                .is_some_and(|old_body| old_body != &node.body)
        })
        .map(|(id, _)| id.clone())
        .collect();
    let mut updated_nodes = HashSet::new();
    for root in &document.outline.roots {
        mark_updated_ancestors(root, &changed_bodies, &mut updated_nodes);
    }
    app.updated_nodes = updated_nodes;
    app.document = document;
    app.source_locations = source_locations;
    app.source_nodes = source_nodes;
    app.derived_nodes = derived_nodes;
    app.writable_external = writable_external;
    app.original_external = original_external;
    app.dirty = false;
    app.dirty_nodes.clear();
    app.quit_armed = false;
    app.reload_armed = false;
    app.body_scroll = 0;
    app.body_horizontal_scroll = 0;
    #[cfg(feature = "syntax")]
    app.highlight_cache.clear();
    #[cfg(feature = "syntax")]
    app.preview_cache.clear();
    if let Some(node) = selected_node {
        if let Some(index) = app.rows().iter().position(|row| row.node == node) {
            app.selected = index;
        } else {
            app.selected = app.selected.min(app.rows().len().saturating_sub(1));
        }
    }
    app.status = format!("reloaded {} ({derived_status})", app.path.display());
}

/// Marks `position`'s node as updated if its body changed directly, or if
/// any descendant did -- so a collapsed `@file`/`@auto` root (or any
/// collapsed ancestor) still shows that something changed underneath it.
fn mark_updated_ancestors(
    position: &Position,
    changed: &HashSet<NodeId>,
    updated: &mut HashSet<NodeId>,
) -> bool {
    let mut is_updated = changed.contains(&position.node);
    for child in &position.children {
        if mark_updated_ancestors(child, changed, updated) {
            is_updated = true;
        }
    }
    if is_updated {
        updated.insert(position.node.clone());
    }
    is_updated
}

/// Whether cutting `position` would remove the only remaining occurrence of
/// a read-only derived node directly. Cutting an @auto root itself is always
/// safe -- its whole subtree, root and derived children alike, leaves
/// together, so nothing is left half-synced -- only cutting a read-only
/// descendant on its own, while its root and siblings stay behind, orphans
/// it. A duplicate occurrence (its node id also appears elsewhere, e.g.
/// because its @auto root was cloned) is safe to cut either way: the content
/// survives via the other occurrence.
fn cut_would_orphan_derived_content(app: &App, position: &PositionId) -> bool {
    app.document
        .outline
        .position(position)
        .is_some_and(|position| {
            app.readonly_derived(&position.node)
                && clone_count(&app.document.outline, &position.node) <= 1
        })
}

fn split_position(id: &PositionId) -> Option<(Option<PositionId>, usize)> {
    let (parent, index) =
        id.0.rsplit_once('/')
            .map_or((None, id.0.as_str()), |(p, i)| {
                (Some(PositionId(p.to_owned())), i)
            });
    Some((parent, index.parse().ok()?))
}

fn join_position(parent: Option<&PositionId>, index: usize) -> PositionId {
    PositionId(parent.map_or_else(|| index.to_string(), |p| format!("{}/{index}", p.0)))
}

/// The node at `index` among `parent`'s children (or the roots, if
/// `parent` is `None`), without requiring a mutable borrow of `outline` --
/// so a caller can check it before deciding whether a subsequent
/// `children_mut` mutation is even allowed.
fn sibling_node_at(outline: &Outline, parent: Option<&PositionId>, index: usize) -> Option<NodeId> {
    let Some(parent) = parent else {
        return outline
            .roots
            .get(index)
            .map(|position| position.node.clone());
    };
    let position = outline.position(parent)?;
    position
        .children
        .get(index)
        .map(|position| position.node.clone())
}

fn children_mut<'a>(
    outline: &'a mut Outline,
    parent: Option<&PositionId>,
) -> Option<&'a mut Vec<Position>> {
    let Some(parent) = parent else {
        return Some(&mut outline.roots);
    };
    let mut indices = parent.0.split('/').map(str::parse::<usize>);
    let mut position = outline.roots.get_mut(indices.next()?.ok()?)?;
    for index in indices {
        position = position.children.get_mut(index.ok()?)?;
    }
    Some(&mut position.children)
}

fn remove_position(outline: &mut Outline, id: &PositionId) -> Option<Position> {
    let (parent, index) = split_position(id)?;
    let siblings = children_mut(outline, parent.as_ref())?;
    (index < siblings.len()).then(|| siblings.remove(index))
}

fn select_position(app: &mut App, id: &PositionId) {
    app.selection_anchor = None;
    if let Some(index) = app.rows().iter().position(|row| &row.position == id) {
        if index != app.selected {
            app.body_scroll = 0;
            app.body_horizontal_scroll = 0;
        }
        app.selected = index;
    }
}

fn fresh_node_id() -> NodeId {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    NodeId(format!(
        "cub.{}.{}",
        duration.as_secs(),
        duration.subsec_nanos()
    ))
}

fn content_columns(area: Rect, app: &App) -> Vec<Rect> {
    let direction = if app.split_horizontal {
        Direction::Vertical
    } else {
        Direction::Horizontal
    };
    if app.body_full_width {
        Layout::default()
            .direction(direction)
            .constraints([Constraint::Length(0), Constraint::Percentage(100)])
            .split(area)
            .to_vec()
    } else if app.outline_full_width {
        Layout::default()
            .direction(direction)
            .constraints([Constraint::Percentage(100), Constraint::Length(0)])
            .split(area)
            .to_vec()
    } else {
        Layout::default()
            .direction(direction)
            .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(area)
            .to_vec()
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    // While find/search/the action palette is active, the bottom bar
    // becomes a docked minibuffer: a candidate list with the query/status
    // line at the very bottom, sized into the layout rather than floated
    // over the panes, so it can never cover their content. Find/search cap
    // the list at 5 rows (a quick jump aid); the action palette shows as
    // many actions as fit, reserving a few rows so the outline/body panes
    // stay visible, since it's meant to be a full command list.
    let finder_matches = app
        .find
        .as_ref()
        .or(app.search.as_ref())
        .map(|state| state.matches.len().min(5));
    let bottom_height = if let Some(shown) = finder_matches {
        shown as u16 + 1
    } else if let Some(palette) = app.palette.as_ref() {
        let max_visible = frame.area().height.saturating_sub(6).max(1);
        palette.matches.len().min(usize::from(max_visible)) as u16 + 1
    } else {
        1
    };
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(bottom_height)])
        .split(frame.area());
    let columns = content_columns(areas[0], app);
    let rows = app.rows();
    let selection_anchor = app.selection_anchor.unwrap_or(app.selected);
    let selection_start = selection_anchor.min(app.selected);
    let selection_end = selection_anchor.max(app.selected);
    let items: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let node = &app.document.outline.nodes[&row.node];
            let marker = if row.has_children {
                if app.expanded.contains(&row.position) {
                    "▾ "
                } else {
                    "▸ "
                }
            } else {
                "  "
            };
            let clone_count = clone_count(&app.document.outline, &row.node);
            let clone = if is_clone_root(&app.document.outline, &row.position, &row.node) {
                format!(" ⧉×{clone_count}")
            } else {
                String::new()
            };
            let input = app.input.as_ref().filter(|input| input.node == row.node);
            let mut spans = vec![Span::raw("  ".repeat(row.depth)), Span::raw(marker)];
            spans.push(dirty_marker(app.dirty_nodes.contains(&row.node)));
            spans.push(body_marker(
                !node.body.trim().is_empty(),
                app.updated_nodes.contains(&row.node),
                app.marked.contains(&row.position),
            ));
            // While this row is being edited, its headline and clone-count
            // text are left off the row entirely -- the `TextArea` overlay
            // rendered after the list (see `draw`) covers the same cells
            // with its own real per-cell cursor, so nothing needs to be
            // drawn underneath it.
            if input.is_none() {
                spans.extend(headline_spans(&node.headline));
                spans.push(Span::styled(
                    clone,
                    Style::default()
                        .fg(Color::LightMagenta)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            let item = ListItem::new(Line::from(spans));
            if (selection_start..=selection_end).contains(&index) && index != app.selected {
                item.style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                item
            }
        })
        .collect();
    let outline_page_size = usize::from(columns[0].height.saturating_sub(2)).max(1);
    let outline_context = 2.min(outline_page_size.saturating_sub(1) / 2);
    if app.selected.saturating_sub(outline_context) < app.outline_scroll {
        app.outline_scroll = app.selected.saturating_sub(outline_context);
    } else if app.selected + outline_context >= app.outline_scroll + outline_page_size {
        app.outline_scroll = app.selected + outline_context + 1 - outline_page_size;
    }
    app.outline_scroll = app
        .outline_scroll
        .min(rows.len().saturating_sub(outline_page_size));
    let mut state = ListState::default()
        .with_selected((!rows.is_empty()).then_some(app.selected))
        .with_offset(app.outline_scroll);
    if !app.body_full_width {
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::default().title(" Outline ").borders(Borders::ALL))
                .scroll_padding(outline_context)
                .highlight_style(
                    Style::default()
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
            columns[0],
            &mut state,
        );
    }
    app.outline_scroll = state.offset();

    // The headline being edited (if any) is drawn as a floating `TextArea`
    // positioned over its outline row rather than as spans inside the
    // `List`: a `List` only ever produces spans, and a real per-cell cursor
    // needs an actual widget render.
    if !app.body_full_width
        && let Some(input) = app.input.as_ref()
        && let Some(row_index) = rows.iter().position(|row| row.node == input.node)
    {
        let outline_area = columns[0];
        let inner_height = outline_area.height.saturating_sub(2);
        let visible_row = row_index as isize - app.outline_scroll as isize;
        if visible_row >= 0 && visible_row < inner_height as isize {
            let prefix_width = 2 * rows[row_index].depth as u16 + 6;
            let inner_x = outline_area.x + 1 + prefix_width;
            let inner_right = outline_area.x + outline_area.width.saturating_sub(1);
            if inner_x < inner_right {
                frame.render_widget(
                    &input.input,
                    Rect {
                        x: inner_x,
                        y: outline_area.y + 1 + visible_row as u16,
                        width: inner_right - inner_x,
                        height: 1,
                    },
                );
            }
        }
    }

    if !app.outline_full_width {
        if let Some(row) = rows.get(app.selected)
            && app
                .action_output
                .as_ref()
                .is_some_and(|out| out.node != row.node)
        {
            app.action_output = None;
        }
        // Cloned out (rather than kept as a borrow) so `body_text` below can
        // still take `app` mutably: it owns the syntax-highlight cache.
        let output_info = app.action_output.as_ref().and_then(|out| {
            rows.get(app.selected)
                .filter(|row| row.node == out.node)
                .map(|_| {
                    (
                        out.name.clone(),
                        out.interpreter,
                        out.status,
                        out.text.clone(),
                    )
                })
        });
        let node_block = Block::default()
            .title(match &output_info {
                Some((name, interpreter, status, _)) => Line::from(vec![
                    Span::raw(format!(" Output: {name} ({interpreter}) ")),
                    match status {
                        Some(0) => Span::styled("exit 0", Style::default().fg(Color::LightGreen)),
                        Some(code) => Span::styled(
                            format!("exit {code}"),
                            Style::default().fg(Color::LightRed),
                        ),
                        None => {
                            Span::styled("did not complete", Style::default().fg(Color::LightRed))
                        }
                    },
                ]),
                None => Line::from(node_title(
                    app.wrap_for(rows.get(app.selected).map(|row| &row.position)),
                )),
            })
            .borders(Borders::ALL);
        let node_area = node_block.inner(columns[1]);
        frame.render_widget(node_block, columns[1]);
        if let Some(row) = rows.get(app.selected) {
            let wrap = app.wrap_for(Some(&row.position));
            let editing_body = app
                .body_input
                .as_ref()
                .is_some_and(|edit| edit.node == row.node);
            if editing_body {
                let edit = app.body_input.as_mut().expect("editing_body is true");
                edit.input.set_wrap_mode(if wrap {
                    ratatui_textarea::WrapMode::WordOrGlyph
                } else {
                    ratatui_textarea::WrapMode::None
                });
                frame.render_widget(&edit.input, node_area);
            } else {
                let mut body = if let Some((_, _, _, text)) = output_info {
                    Text::from(text)
                } else {
                    body_text(app, row)
                };
                if let Some(search) = &app.search
                    && !search.query.is_empty()
                    && let Ok(pattern) = RegexBuilder::new(&regex::escape(&search.query))
                        .case_insensitive(true)
                        .build()
                {
                    body = highlight_query_in_text(body, &pattern);
                }
                if let Some(selection) = app.body_selection {
                    body = highlight_selection_in_text(body, selection);
                }
                let body_width = body.width();
                let mut paragraph = Paragraph::new(body);
                if wrap {
                    paragraph = paragraph.wrap(Wrap { trim: false });
                }
                app.body_page_size = usize::from(node_area.height).max(1);
                let body_height = paragraph.line_count(node_area.width);
                app.body_scroll_max = body_height.saturating_sub(app.body_page_size);
                app.body_horizontal_scroll_max = if wrap {
                    0
                } else {
                    body_width.saturating_sub(usize::from(node_area.width))
                };
                app.body_scroll = app.body_scroll.min(app.body_scroll_max);
                app.body_horizontal_scroll = app
                    .body_horizontal_scroll
                    .min(app.body_horizontal_scroll_max);
                frame.render_widget(
                    paragraph.scroll((
                        app.body_scroll.min(u16::MAX as usize) as u16,
                        app.body_horizontal_scroll.min(u16::MAX as usize) as u16,
                    )),
                    node_area,
                );
            }
        }
    }
    let flash = app
        .flash
        .as_ref()
        .filter(|(_, shown)| shown.elapsed() < Duration::from_secs(2))
        .map(|(message, _)| message.clone());
    if flash.is_none() {
        app.flash = None;
    }
    if let Some(find) = &app.find {
        draw_finder_panel(
            frame,
            areas[1],
            "> ",
            "Find headline",
            find,
            &app.document.outline,
        );
    } else if let Some(search) = &app.search {
        draw_finder_panel(
            frame,
            areas[1],
            "/ ",
            "Search (headline + body)",
            search,
            &app.document.outline,
        );
    } else if let Some(palette) = &app.palette {
        draw_palette_panel(frame, areas[1], palette);
    } else {
        let mut status = vec![Span::styled("[", Style::default().fg(Color::DarkGray))];
        status.push(Span::styled(
            flash.as_deref().unwrap_or(&app.status),
            if flash.is_some() {
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ));
        status.push(Span::styled(
            format!(
                "]   {}",
                controls(app.body_full_width, app.outline_full_width)
            ),
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(Paragraph::new(Line::from(status)), areas[1]);
    }
    if app.help {
        draw_help(frame, app.body_full_width, app.outline_full_width);
    }
    if app.log_view {
        draw_log(frame, app);
    }
}

/// The log pane's own bordered content area, plus its REPL input row when
/// active -- shared by `draw_log` (to render) and `handle_log_mouse` (to
/// map a click back to the same coordinates), so the two can never drift
/// apart. Title text doesn't affect a `Block`'s `.inner()` geometry, so
/// this builds an untitled throwaway block rather than duplicating
/// `draw_log`'s title strings.
fn log_view_layout(area: Rect, repl_active: bool) -> (Rect, Option<Rect>) {
    let inner = Block::default().borders(Borders::ALL).inner(area);
    if repl_active && inner.height > 0 {
        (
            Rect::new(
                inner.x,
                inner.y,
                inner.width,
                inner.height.saturating_sub(1),
            ),
            Some(Rect::new(
                inner.x,
                inner.y + inner.height - 1,
                inner.width,
                1,
            )),
        )
    } else {
        (inner, None)
    }
}

/// The `[start, end)` range of `logs` currently windowed into `log_area`,
/// given `log_scroll` (0 = pinned to the latest line) -- shared by
/// `draw_log` and `handle_log_mouse` for the same reason as
/// `log_view_layout`.
fn log_view_range(logs: &VecDeque<String>, log_area: Rect, log_scroll: usize) -> (usize, usize) {
    let visible = log_area.height as usize;
    let total = logs.len();
    let end = total.saturating_sub(log_scroll.min(total));
    let start = end.saturating_sub(visible);
    (start, end)
}

/// Renders the full-screen log/REPL overlay: the scrollback buffer of
/// `@action`/REPL rhai output, windowed by `app.log_scroll` (0 = pinned to
/// the latest line), plus a bottom input line when the REPL is active.
fn draw_log(frame: &mut ratatui::Frame<'_>, app: &App) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let title = if app.log_repl.is_some() {
        " Log — Esc: back to browse  Enter: run "
    } else {
        " Log — l/q/Esc: close  Enter: run rhai "
    };
    let block = Block::default().title(title).borders(Borders::ALL);
    frame.render_widget(block, area);

    let (log_area, input_area) = log_view_layout(area, app.log_repl.is_some());
    let (start, end) = log_view_range(&app.logs, log_area, app.log_scroll);
    // Owned lines (not borrowed `&str`s) so the selection highlight below
    // can reuse `highlight_selection_in_text`, which needs `Text<'static>`.
    let mut lines: Vec<Line<'static>> = app
        .logs
        .iter()
        .skip(start)
        .take(end - start)
        .map(|line| Line::from(line.clone()))
        .collect();
    if let Some(selection) = app.log_selection {
        lines = highlight_log_selection(lines, start, selection);
    }
    frame.render_widget(Paragraph::new(lines), log_area);

    if let (Some(input_area), Some(input)) = (input_area, app.log_repl.as_ref()) {
        let areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(input_area);
        frame.render_widget(Paragraph::new("> "), areas[0]);
        frame.render_widget(&input.input, areas[1]);
    }
}

/// Renders the docked find/search minibuffer: up to 5 candidate rows on
/// top of a single query/status line, all sized into `area` rather than
/// floated over the outline/body panes.
fn draw_finder_panel(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    prefix: &str,
    label: &str,
    state: &FindInput,
    outline: &Outline,
) {
    let rows = all_rows(outline);
    let shown = state.matches.len().min(5);
    let first = state.active.saturating_sub(4);
    let mut lines: Vec<Line> = state
        .matches
        .iter()
        .enumerate()
        .skip(first)
        .take(shown)
        .map(|(index, position)| {
            let row = rows
                .iter()
                .find(|row| &row.position == position)
                .expect("matched position exists");
            let marker = if index == state.active { "› " } else { "  " };
            let mut spans = vec![Span::raw(marker)];
            spans.extend(headline_spans(&outline.nodes[&row.node].headline));
            Line::from(spans)
        })
        .collect();
    let count = if state.query.is_empty() {
        String::new()
    } else if state.matches.is_empty() {
        "no matches".into()
    } else {
        format!("{} of {}", state.active + 1, state.matches.len())
    };
    lines.push(Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{prefix}{}▏", state.query)),
        Span::styled(
            format!("   {count}   ↑↓ cycle · Enter accept · Esc cancel"),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(Paragraph::new(lines), area);
}

/// Renders the docked action palette (`a`), listing `@action` node names,
/// imported script commands, and built-in editor commands (labels
/// precomputed by `palette_entries`) rather than full headlines. Laid out
/// the same as `draw_finder_panel`,
/// plus one reserved line below the list -- always present, even when
/// blank, so the list's height doesn't jump as the active entry changes --
/// showing the active entry's `doc` (an imported command's `COMMANDS`
/// description; `@action` entries have none).
fn draw_palette_panel(frame: &mut ratatui::Frame<'_>, area: Rect, state: &ActionPalette) {
    let shown = state
        .matches
        .len()
        .min(usize::from(area.height.saturating_sub(2)));
    let first = state.active.saturating_sub(shown.saturating_sub(1));
    let mut lines: Vec<Line> = state
        .matches
        .iter()
        .enumerate()
        .skip(first)
        .take(shown)
        .map(|(index, entry)| {
            let marker = if index == state.active { "› " } else { "  " };
            Line::from(format!("{marker}{}", entry.label))
        })
        .collect();
    let count = if state.matches.is_empty() && state.query.is_empty() {
        "no @action nodes or imported commands in this outline".to_owned()
    } else if state.matches.is_empty() {
        "no matches".into()
    } else {
        format!("{} of {}", state.active + 1, state.matches.len())
    };
    // A broken `@import`ed script would otherwise look identical to one
    // that legitimately declares no commands -- surface it here, taking
    // priority over the active entry's own description, since it's the
    // more actionable thing to see.
    if state.errors.is_empty() {
        let doc = state
            .matches
            .get(state.active)
            .and_then(|entry| entry.doc.as_deref())
            .unwrap_or("");
        lines.push(Line::styled(doc, Style::default().fg(Color::DarkGray)));
    } else {
        lines.push(Line::styled(
            state.errors.join(" | "),
            Style::default().fg(Color::LightRed),
        ));
    }
    lines.push(Line::from(vec![
        Span::styled("Run action: ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{}▏", state.query)),
        Span::styled(
            format!("   {count}   ↑↓ cycle · Enter run · Esc cancel"),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(Paragraph::new(lines), area);
}

fn dirty_marker(dirty: bool) -> Span<'static> {
    if dirty {
        Span::styled("* ", Style::default().fg(Color::LightRed))
    } else {
        Span::raw("  ")
    }
}

fn body_marker(has_body: bool, updated: bool, marked: bool) -> Span<'static> {
    if marked {
        Span::styled("● ", Style::default().fg(Color::LightYellow))
    } else if updated {
        Span::styled("↑ ", Style::default().fg(Color::LightGreen))
    } else if has_body {
        Span::styled("· ", Style::default().fg(Color::DarkGray))
    } else {
        Span::raw("  ")
    }
}

fn node_title(body_wrap: bool) -> &'static str {
    if body_wrap { " Node [wrap] " } else { " Node " }
}

fn headline_spans(headline: &str) -> Vec<Span<'_>> {
    let leading = headline.len() - headline.trim_start().len();
    let trimmed = &headline[leading..];
    if let Some(contents) = trimmed
        .strip_prefix("<<")
        .and_then(|section| section.strip_suffix(">>"))
    {
        let mut spans = Vec::with_capacity(4);
        if leading > 0 {
            spans.push(Span::raw(&headline[..leading]));
        }
        let marker_style = Style::default().fg(Color::Cyan);
        spans.push(Span::styled("<<", marker_style));
        spans.push(Span::raw(contents));
        spans.push(Span::styled(">>", marker_style));
        return spans;
    }

    let directive_len = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let directive = &trimmed[..directive_len];
    if !directive.starts_with('@') || directive.len() == 1 {
        return vec![Span::raw(headline)];
    }

    let mut spans = Vec::with_capacity(3);
    if leading > 0 {
        spans.push(Span::raw(&headline[..leading]));
    }
    spans.push(Span::styled(directive, Style::default().fg(Color::Cyan)));
    let remainder = &trimmed[directive_len..];
    if !remainder.is_empty() {
        let filename = matches!(
            directive,
            "@asis"
                | "@auto"
                | "@auto-dir"
                | "@auto-md"
                | "@auto-markdown"
                | "@clean"
                | "@edit"
                | "@f"
                | "@file"
                | "@file-thin"
                | "@nosent"
                | "@path"
                | "@thin"
        );
        if filename {
            let whitespace = remainder.len() - remainder.trim_start().len();
            if whitespace > 0 {
                spans.push(Span::raw(&remainder[..whitespace]));
            }
            if whitespace < remainder.len() {
                spans.push(Span::styled(
                    &remainder[whitespace..],
                    Style::default().fg(Color::Yellow),
                ));
            }
        } else {
            spans.push(Span::raw(remainder));
        }
    }
    spans
}

fn controls(body_full_width: bool, outline_full_width: bool) -> &'static str {
    if body_full_width {
        #[cfg(feature = "syntax")]
        return "? help  arrows scroll  W wrap  c/x/v/V tree  m/M mark  n/N clone  f split  F outline  s split dir  o open/edit  Ctrl-P find  / search  a commands/actions  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  y syntax  p preview  l log  q quit";
        #[cfg(not(feature = "syntax"))]
        return "? help  arrows scroll  W wrap  c/x/v/V tree  m/M mark  n/N clone  f split  F outline  s split dir  o open/edit  Ctrl-P find  / search  a commands/actions  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  l log  q quit";
    }
    if outline_full_width {
        return "? help  arrows navigate  W wrap  c/x/v/V tree  m/M mark  n/N clone  F split view  s split dir  o open/edit  Ctrl-P find  / search  a commands/actions  i new  h rename  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  l log  q quit";
    }
    #[cfg(feature = "syntax")]
    return "? help  arrows navigate  PgUp/PgDn body  W wrap  c/x/v/V tree  m/M mark  n/N clone  f body  F outline  s split dir  o open/edit  Ctrl-P find  / search  a commands/actions  i new  h rename  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  y syntax  p preview  l log  q quit";
    #[cfg(not(feature = "syntax"))]
    "? help  arrows navigate  PgUp/PgDn body  W wrap  c/x/v/V tree  m/M mark  n/N clone  f body  F outline  s split dir  o open/edit  Ctrl-P find  / search  a commands/actions  i new  h rename  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  l log  q quit"
}

fn draw_help(frame: &mut ratatui::Frame<'_>, body_full_width: bool, outline_full_width: bool) {
    let width = frame.area().width.saturating_sub(4).min(72);
    let height = frame.area().height.saturating_sub(2).min(30);
    let area = Rect::new(
        frame.area().x + (frame.area().width.saturating_sub(width)) / 2,
        frame.area().y + (frame.area().height.saturating_sub(height)) / 2,
        width,
        height,
    );
    let mut lines = if body_full_width {
        vec![
            Line::from("↑/↓              Scroll body vertically"),
            Line::from("Shift-↑/↓        Extend tree selection"),
            Line::from("←/→              Scroll body horizontally"),
            Line::from("Shift-W          Toggle body word wrap"),
            Line::from("PageUp/PageDown  Scroll body by one page"),
            Line::from("f                Restore split view"),
            Line::from("Shift-F          Show full-width outline"),
            Line::from("s                Toggle split direction"),
            Line::from("c/x              Copy/cut selected trees"),
            Line::from("Shift-C          Copy path:line (dir for @path) to clipboard"),
            Line::from("v / Shift-V      Paste copy / paste clone"),
            Line::from("m / Shift-M      Mark selected / clear all marks"),
            Line::from("n / Shift-N      Next/previous occurrence of this node"),
            Line::from("i                Insert a sibling"),
            Line::from("h                Rename the headline"),
            Line::from("Ctrl-↑↓←→        Move selected tree(s)"),
            Line::from("Ctrl-P           Find a headline"),
            Line::from("a                Command/action palette"),
            Line::from("/                Search headlines and body text"),
            Line::from("Ctrl-R           Reload from disk"),
            Line::from("Ctrl-S           Save"),
            Line::from("o                Edit body, or open derived source"),
            Line::from("b                Quick body entry (type or paste)"),
        ]
    } else if outline_full_width {
        vec![
            Line::from("↑/↓              Select previous/next node"),
            Line::from("Shift-↑/↓        Extend tree selection"),
            Line::from("←/→              Collapse/expand node"),
            Line::from("Enter            Open body editor"),
            Line::from("Home/End         Select first/last visible node"),
            Line::from("Shift-F          Restore split view"),
            Line::from("f                Show full-width body"),
            Line::from("s                Toggle split direction"),
            Line::from("Shift-W          Toggle body word wrap"),
            Line::from("c/x/v/V          Copy/cut/paste/clone"),
            Line::from("Shift-C          Copy path:line (dir for @path) to clipboard"),
            Line::from("m / Shift-M      Mark selected / clear all marks"),
            Line::from("n / Shift-N      Next/previous occurrence of this node"),
            Line::from("Ctrl-P           Find a headline"),
            Line::from("a                Command/action palette"),
            Line::from("/                Search headlines and body text"),
            Line::from("i                Insert a sibling"),
            Line::from("h                Rename the headline"),
            Line::from("Ctrl-↑↓←→        Move selected tree(s)"),
            Line::from("Ctrl-R           Reload from disk"),
            Line::from("Ctrl-S           Save"),
            Line::from("o                Edit body, or open derived source"),
            Line::from("b                Quick body entry (type or paste)"),
        ]
    } else {
        vec![
            Line::from("↑/↓              Select previous/next node"),
            Line::from("Shift-↑/↓        Extend tree selection"),
            Line::from("←/→              Collapse/expand node"),
            Line::from("Enter            Open body editor"),
            Line::from("Home/End         Select first/last visible node"),
            Line::from("f                Show full-width body"),
            Line::from("Shift-F          Show full-width outline"),
            Line::from("s                Toggle split direction"),
            Line::from("PageUp/PageDown  Scroll the body pane"),
            Line::from("Shift-W          Toggle body word wrap"),
            Line::from("Ctrl-P           Find a headline"),
            Line::from("a                Command/action palette"),
            Line::from("/                Search headlines and body text"),
            Line::from("i                Insert a sibling"),
            Line::from("h                Rename the headline"),
            Line::from("c                Copy selected tree"),
            Line::from("Shift-C          Copy path:line (dir for @path) to clipboard"),
            Line::from("x                Cut selected tree"),
            Line::from("v / Shift-V      Paste copy / paste clone"),
            Line::from("m / Shift-M      Mark selected / clear all marks"),
            Line::from("n / Shift-N      Next/previous occurrence of this node"),
            Line::from("Ctrl-↑↓←→        Move selected tree(s)"),
            Line::from("Ctrl-R           Reload from disk"),
            Line::from("Ctrl-S           Save"),
            Line::from("o                Edit body, or open derived source"),
            Line::from("b                Quick body entry (type or paste)"),
        ]
    };
    #[cfg(feature = "syntax")]
    lines.push(Line::from("y                Toggle syntax highlighting"));
    #[cfg(feature = "syntax")]
    lines.push(Line::from(
        "p                Toggle rendered preview (Markdown for now)",
    ));
    lines.extend([
        Line::from("l                Toggle full-screen log / rhai REPL"),
        Line::from("q or Esc         Quit"),
        Line::from(""),
        Line::styled(
            "Press ?, q, or Esc to close",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(" Command help ")
                .borders(Borders::ALL),
        ),
        area,
    );
}

/// Overlays a background highlight on substrings matching `pattern`,
/// splitting existing spans at the match boundaries while keeping each
/// fragment's original style (e.g. syntax-highlight foreground colors)
/// intact — only the background/bold modifier is added.
fn highlight_query_in_text(text: Text<'static>, pattern: &Regex) -> Text<'static> {
    Text::from(
        text.lines
            .into_iter()
            .map(|line| highlight_matches_in_line(line, pattern))
            .collect::<Vec<_>>(),
    )
}

fn highlight_matches_in_line(line: Line<'static>, pattern: &Regex) -> Line<'static> {
    let content: String = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect();
    let ranges: Vec<(usize, usize)> = pattern
        .find_iter(&content)
        .map(|found| (found.start(), found.end()))
        .collect();
    if ranges.is_empty() {
        return line;
    }
    let mut spans = Vec::new();
    let mut offset = 0usize;
    for span in line.spans {
        let text = span.content.into_owned();
        let span_start = offset;
        let span_end = span_start + text.len();
        offset = span_end;
        let mut cursor = 0usize;
        for &(match_start, match_end) in &ranges {
            let overlap_start = match_start.max(span_start);
            let overlap_end = match_end.min(span_end);
            if overlap_start >= overlap_end {
                continue;
            }
            let local_start = overlap_start - span_start;
            let local_end = overlap_end - span_start;
            if local_start > cursor {
                spans.push(Span::styled(
                    text[cursor..local_start].to_owned(),
                    span.style,
                ));
            }
            spans.push(Span::styled(
                text[local_start..local_end].to_owned(),
                span.style.bg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));
            cursor = local_end;
        }
        if cursor < text.len() {
            spans.push(Span::styled(text[cursor..].to_owned(), span.style));
        }
    }
    Line::from(spans)
}

/// Overlays the mouse-drag selection highlight, same span-splitting
/// technique as `highlight_query_in_text` but keyed by a per-line
/// char-column range instead of a regex match.
fn highlight_selection_in_text(text: Text<'static>, selection: BodySelection) -> Text<'static> {
    let (start, end) = if selection.anchor <= selection.cursor {
        (selection.anchor, selection.cursor)
    } else {
        (selection.cursor, selection.anchor)
    };
    if start == end {
        return text;
    }
    let (start_line, start_col) = start;
    let (end_line, end_col) = end;
    Text::from(
        text.lines
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                if index < start_line || index > end_line {
                    return line;
                }
                let (from, to) = if index == start_line && index == end_line {
                    (start_col, end_col)
                } else if index == start_line {
                    (start_col, usize::MAX)
                } else if index == end_line {
                    (0, end_col)
                } else {
                    (0, usize::MAX)
                };
                highlight_char_range_in_line(line, from, to)
            })
            .collect::<Vec<_>>(),
    )
}

/// Applies `log_selection`'s highlight to `lines`, the already-windowed
/// slice `draw_log` is about to render -- `selection`'s (line, column)
/// pairs are absolute indices into the full `logs` scrollback, so they're
/// shifted down by `start` (the window's first absolute line) before
/// reusing `highlight_selection_in_text`, which expects indices local to
/// the `Text` it's given.
fn highlight_log_selection(
    lines: Vec<Line<'static>>,
    start: usize,
    selection: BodySelection,
) -> Vec<Line<'static>> {
    let (sel_start, sel_end) = if selection.anchor <= selection.cursor {
        (selection.anchor, selection.cursor)
    } else {
        (selection.cursor, selection.anchor)
    };
    if sel_start == sel_end || lines.is_empty() || sel_end.0 < start {
        return lines;
    }
    let local_start_line = sel_start.0.saturating_sub(start);
    if local_start_line >= lines.len() {
        return lines;
    }
    let local_end_line = sel_end.0.saturating_sub(start).min(lines.len() - 1);
    let local_selection = BodySelection {
        anchor: (local_start_line, sel_start.1),
        cursor: (local_end_line, sel_end.1),
    };
    highlight_selection_in_text(Text::from(lines), local_selection).lines
}

fn highlight_char_range_in_line(line: Line<'static>, from: usize, to: usize) -> Line<'static> {
    if from >= to {
        return line;
    }
    let mut spans = Vec::new();
    let mut offset = 0usize;
    for span in line.spans {
        let chars: Vec<char> = span.content.chars().collect();
        let span_start = offset;
        let span_end = span_start + chars.len();
        offset = span_end;
        let overlap_start = from.max(span_start);
        let overlap_end = to.min(span_end);
        if overlap_start >= overlap_end {
            spans.push(Span::styled(
                chars.into_iter().collect::<String>(),
                span.style,
            ));
            continue;
        }
        let local_start = overlap_start - span_start;
        let local_end = overlap_end - span_start;
        if local_start > 0 {
            spans.push(Span::styled(
                chars[..local_start].iter().collect::<String>(),
                span.style,
            ));
        }
        spans.push(Span::styled(
            chars[local_start..local_end].iter().collect::<String>(),
            span.style.bg(Color::Blue).add_modifier(Modifier::BOLD),
        ));
        if local_end < chars.len() {
            spans.push(Span::styled(
                chars[local_end..].iter().collect::<String>(),
                span.style,
            ));
        }
    }
    Line::from(spans)
}

fn body_text(app: &mut App, row: &Row) -> Text<'static> {
    let body = app.document.outline.nodes[&row.node].body.clone();
    #[cfg(feature = "syntax")]
    {
        let (inherited_language, external_path) =
            syntax_context(&app.document.outline, &row.position);
        let source_path = app
            .source_locations
            .get(&row.position)
            .map(|location| location.path.as_path())
            .or(external_path.as_deref());

        if app.preview_enabled {
            if let Some(cached) = app.preview_cache.get(&row.position) {
                return cached.clone();
            }
            if let Some(rendered) =
                app.syntax
                    .render_preview(&body, source_path, inherited_language.as_deref())
            {
                app.preview_cache
                    .insert(row.position.clone(), rendered.clone());
                return rendered;
            }
        }

        if app.syntax_enabled {
            if let Some(cached) = app.highlight_cache.get(&row.position) {
                return cached.clone();
            }
            let highlighted = app.syntax.highlight_with_language(
                &body,
                source_path,
                inherited_language.as_deref(),
            );
            app.highlight_cache
                .insert(row.position.clone(), highlighted.clone());
            return highlighted;
        }
    }
    Text::from(body)
}

#[cfg(feature = "syntax")]
fn syntax_context(outline: &Outline, position: &PositionId) -> (Option<String>, Option<PathBuf>) {
    let mut language = None;
    let mut source_path = None;
    let mut prefix = String::new();

    for component in position.0.split('/') {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(component);
        let Some(position) = outline.position(&PositionId(prefix.clone())) else {
            break;
        };
        let node = &outline.nodes[&position.node];
        if is_rst_headline(&node.headline) {
            language = Some("rst".to_owned());
        }
        if let Some(value) = crate::syntax::language_directive(&node.body) {
            language = Some(value.to_owned());
        }
        if let Some(filename) = external_filename(&node.headline) {
            source_path = Some(PathBuf::from(filename));
        }
    }

    (language, source_path)
}

#[cfg(feature = "syntax")]
fn is_rst_headline(headline: &str) -> bool {
    headline
        .strip_prefix("@rst")
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
}

fn open_selected(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) {
    let Some(row) = app.rows().get(app.selected).cloned() else {
        return;
    };
    if let Some(location) = dynamic_source_location(app, &row)
        .or_else(|| app.source_locations.get(&row.position).cloned())
        .or_else(|| app.source_nodes.get(&row.node).cloned())
    {
        let original_source = fs::read(&location.path).ok();
        if let Err(error) = suspend_and_open(terminal, &location) {
            app.status = format!("open failed: {error}");
        } else {
            if original_source != fs::read(&location.path).ok() {
                reload(app);
            } else {
                app.status = format!("opened {}:{}", location.path.display(), location.line);
            }
        }
        return;
    }

    if !app.editable(&row) {
        return;
    }
    #[cfg(feature = "syntax")]
    let language = app.language_at(&row.position);
    #[cfg(not(feature = "syntax"))]
    let language: Option<String> = None;
    match edit_body_in_temp_file(
        terminal,
        &app.document.outline.nodes[&row.node].body,
        language.as_deref(),
    ) {
        Ok(Some(body)) => {
            app.document
                .outline
                .nodes
                .get_mut(&row.node)
                .expect("edited node exists")
                .body = body;
            app.dirty_nodes.insert(row.node);
            #[cfg(feature = "syntax")]
            app.highlight_cache.clear();
            #[cfg(feature = "syntax")]
            app.preview_cache.clear();
            app.body_scroll = 0;
            app.body_horizontal_scroll = 0;
            app.dirty = true;
            app.quit_armed = false;
            app.status = "body changed (Ctrl-S to save)".into();
        }
        Ok(None) => app.status = "body unchanged".into(),
        Err(error) => app.status = format!("body edit failed: {error}"),
    }
}

fn copy_location_to_clipboard(app: &mut App) {
    let Some(row) = app.rows().get(app.selected).cloned() else {
        return;
    };
    let headline = app.document.outline.nodes[&row.node].headline.clone();
    let text = if path_directive(&headline).is_some() {
        display_path(&resolved_directory(app, &row))
    } else {
        match dynamic_source_location(app, &row)
            .or_else(|| app.source_locations.get(&row.position).cloned())
            .or_else(|| app.source_nodes.get(&row.node).cloned())
        {
            Some(location) => format!(
                "{}:{}: [{headline}]",
                display_path(&location.path),
                location.line
            ),
            None => format!("{}: {}", display_path(&app.path), headline_path(app, &row)),
        }
    };
    match execute!(
        io::stdout(),
        CopyToClipboard::to_clipboard_from(text.clone())
    ) {
        Ok(()) => app.status = format!("copied to clipboard: {text}"),
        Err(error) => app.status = format!("clipboard copy failed: {error}"),
    }
}

/// Renders `path` as an absolute, `~`-abbreviated string for pasting
/// elsewhere, since a path relative to the app's launch directory (the
/// common case for derived-file locations) is meaningless out of context.
fn display_path(path: &Path) -> String {
    let cwd = env::current_dir().unwrap_or_default();
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    tildify(&absolutize(path, &cwd), home.as_deref())
}

fn absolutize(path: &Path, cwd: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    normalize_path(&joined)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

fn tildify(path: &Path, home: Option<&Path>) -> String {
    if let Some(home) = home
        && let Ok(relative) = path.strip_prefix(home)
    {
        return if relative.as_os_str().is_empty() {
            "~".to_owned()
        } else {
            format!("~/{}", relative.display())
        };
    }
    path.display().to_string()
}

fn edit_body_in_temp_file(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    original: &str,
    language: Option<&str>,
) -> Result<Option<String>> {
    let path = unique_body_temp_path(language);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(original.as_bytes())?;
    drop(file);

    let location = SourceLocation {
        path: path.clone(),
        line: 1,
    };
    let editor_result = suspend_and_open(terminal, &location);
    let result = editor_result.and_then(|()| {
        let edited = fs::read_to_string(&path)?;
        Ok((edited != original).then_some(edited))
    });
    let _ = fs::remove_file(path);
    result
}

fn unique_body_temp_path(language: Option<&str>) -> PathBuf {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    env::temp_dir().join(format!(
        "leo-cub-body-{}-{}-{}.{}",
        std::process::id(),
        duration.as_secs(),
        duration.subsec_nanos(),
        extension_for_language(language)
    ))
}

/// Maps an `@language` directive to the file extension used when a body is
/// opened in an external editor via a temp file, so editors that infer
/// syntax highlighting from the filename (nearly all of them) get it right
/// instead of falling back to plain text. Unrecognized or missing languages
/// fall back to `txt`; most languages whose directive name doesn't already
/// match a common extension are listed explicitly, and anything else is
/// assumed to match (e.g. `lua`, `json`, `toml`, `sql`).
fn extension_for_language(language: Option<&str>) -> &str {
    match language.unwrap_or("") {
        "python" | "python3" => "py",
        "javascript" | "node" => "js",
        "typescript" => "ts",
        "tsx" => "tsx",
        "rust" => "rs",
        "ruby" => "rb",
        "nu" | "nushell" => "nu",
        "bash" => "sh",
        "csharp" | "cs" => "cs",
        "golang" => "go",
        "objective-c" | "objc" | "objectivec" => "m",
        "markdown" => "md",
        "yml" => "yaml",
        "" => "txt",
        other => other,
    }
}

fn suspend_and_open(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    location: &SourceLocation,
) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    let editor_result = run_editor(location);
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    terminal.clear()?;
    editor_result
}

fn run_editor(location: &SourceLocation) -> Result<()> {
    let editor = env::var("VISUAL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| env::var("EDITOR").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| "vi".to_owned());
    let mut parts = editor.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty editor command"))?;
    let mut command = Command::new(program);
    command.args(parts);
    let name = Path::new(program)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(program);
    let path = location.path.as_os_str();
    match name {
        "vim" | "nvim" | "vi" | "view" | "nano" => {
            command.arg(format!("+{}", location.line)).arg(path);
        }
        "emacs" | "emacsclient" => {
            command.arg(format!("+{}:1", location.line)).arg(path);
        }
        "code" | "code-insiders" | "codium" => {
            command
                .arg("--goto")
                .arg(format!("{}:{}:1", location.path.display(), location.line));
        }
        "edit" | "msedit" | "hx" | "helix" | "kak" => {
            command.arg(format!("{}:{}:1", location.path.display(), location.line));
        }
        _ => {
            command.arg(path);
        }
    }
    let status = command.status()?;
    if !status.success() {
        anyhow::bail!("editor exited with {status}");
    }
    Ok(())
}

#[cfg(test)]
fn thin_filename(headline: &str) -> Option<&str> {
    derived_filename(headline).and_then(|(auto, _, filename)| (!auto).then_some(filename))
}

fn dynamic_source_location(app: &App, row: &Row) -> Option<SourceLocation> {
    let headline = &app.document.outline.nodes[&row.node].headline;
    // `@auto-dir`'s own argument is a directory/glob pattern, not an
    // openable file -- unlike every other directive `external_filename`
    // recognizes, treating it as a literal path here would build a
    // nonexistent one (e.g. ".../src/*.rs"). Pressing `o` on the
    // pattern-bearing root itself is a no-op, same as any other node with
    // no recorded source.
    if derived_filename(headline).is_some_and(|(_, directive, _)| directive == "@auto-dir") {
        return None;
    }
    let filename = external_filename(headline)?;
    let mut path = app
        .path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let mut prefix = String::new();
    for component in row.position.0.split('/') {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(component);
        let position = app.document.outline.position(&PositionId(prefix.clone()))?;
        let node = &app.document.outline.nodes[&position.node];
        // An `@auto-dir` ancestor's own resolved directory isn't itself a
        // `@path` node -- only the synthetic `@path` dirs *below* it
        // (mirroring the subdirectories its matches came from) are. So
        // this node's real path can't be reconstructed from `@path`
        // ancestors alone; bail and let the caller fall through to
        // `App::source_nodes`, which already has the exact path from
        // `AutoFile::file_paths` rather than a guess missing that segment.
        if derived_filename(&node.headline)
            .is_some_and(|(_, directive, _)| directive == "@auto-dir")
        {
            return None;
        }
        if let Some(directory) =
            path_directive(&node.headline).or_else(|| path_directive(&node.body))
        {
            path.push(directory);
        }
    }
    path.push(filename);
    Some(SourceLocation { path, line: 1 })
}

fn clone_count(outline: &Outline, id: &NodeId) -> usize {
    fn count(items: &[Position], id: &NodeId) -> usize {
        items
            .iter()
            .map(|p| usize::from(&p.node == id) + count(&p.children, id))
            .sum()
    }
    count(&outline.roots, id)
}

/// Every position where `id` occurs, in document (pre-order) order --
/// regardless of which ancestors are currently expanded. Powers cycling
/// between a node's clone occurrences with `n`/`N`.
fn positions_of(outline: &Outline, id: &NodeId) -> Vec<PositionId> {
    fn walk(positions: &[Position], parent: &str, id: &NodeId, out: &mut Vec<PositionId>) {
        for (index, position) in positions.iter().enumerate() {
            let path = if parent.is_empty() {
                index.to_string()
            } else {
                format!("{parent}/{index}")
            };
            if &position.node == id {
                out.push(PositionId(path.clone()));
            }
            walk(&position.children, &path, id, out);
        }
    }
    let mut out = Vec::new();
    walk(&outline.roots, "", id, &mut out);
    out
}

/// Whether `position` is where a clone actually originates, rather than a
/// descendant that only repeats because an ancestor of it was cloned. A
/// node's occurrences that are fully explained by its immediate parent's own
/// occurrence count (i.e. it shows up once per parent occurrence) inherit
/// the clone relationship instead of being an independent clone point.
fn is_clone_root(outline: &Outline, position: &PositionId, id: &NodeId) -> bool {
    let count = clone_count(outline, id);
    if count <= 1 {
        return false;
    }
    let parent_count = match split_position(position) {
        Some((Some(parent), _)) => outline
            .position(&parent)
            .map_or(1, |p| clone_count(outline, &p.node)),
        _ => 1,
    };
    count > parent_count
}

/// One step of a scripted TUI interaction, used as `Vec<Step>` literals in
/// regression tests to drive `App` headlessly.
// Read by apply_step_headless, only reachable from #[cfg(test)] regression
// tests; a non-test build sees these fields/functions as unused.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Step {
    /// Press a single key: a literal char ("j", "?"), a bare capital for
    /// shift ("C"), a named key ("Enter"/"Return", "Esc"/"Escape", "Tab",
    /// "Backspace", "Delete", "Space", "Up"/"Down"/"Left"/"Right", "Home",
    /// "End", "PageUp", "PageDown", "F1".."F12"), optionally prefixed with
    /// modifiers ("C-r", "S-Down", "C-S-Left"; "C"/"Ctrl", "S"/"Shift",
    /// "A"/"Alt").
    Key { key: String },
    /// Type each character of `text` as an individual unmodified keypress
    /// -- for filling in headline/find/search/action-palette inputs.
    Type { text: String },
    /// Sleep for `ms` milliseconds of wall-clock time. Rarely needed: flash
    /// messages and similar UI expire against real `Instant`s, not a
    /// virtual clock.
    Wait { ms: u64 },
    /// Resize the headless backend, e.g. to reproduce a narrow-terminal
    /// bug.
    Resize { cols: u16, rows: u16 },
    /// Render the current frame and write it to `path` as plain text.
    Screenshot { path: PathBuf },
    /// Fail unless the rendered screen contains `text`.
    AssertContains { text: String },
    /// Fail if the rendered screen contains `text`.
    AssertNotContains { text: String },
    /// Fail unless the status line contains `text`.
    AssertStatus { text: String },
    /// No-op; documents intent in a script.
    Comment {
        #[allow(dead_code)]
        text: String,
    },
}

/// Parses key notation like `"j"`, `"Enter"`, `"C-r"`, `"S-Down"` into a
/// [`KeyEvent`]. See [`Step::Key`] for the supported names.
fn parse_key(notation: &str) -> Result<KeyEvent> {
    let mut modifiers = KeyModifiers::empty();
    let mut rest = notation;
    while let Some((head, tail)) = rest.split_once('-') {
        match head {
            "C" | "Ctrl" => modifiers |= KeyModifiers::CONTROL,
            "S" | "Shift" => modifiers |= KeyModifiers::SHIFT,
            "A" | "Alt" => modifiers |= KeyModifiers::ALT,
            _ => break,
        }
        rest = tail;
    }
    let code = match rest {
        "Enter" | "Return" => KeyCode::Enter,
        "Esc" | "Escape" => KeyCode::Esc,
        "Tab" => KeyCode::Tab,
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Space" => KeyCode::Char(' '),
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        _ if rest.len() >= 2
            && rest.starts_with('F')
            && rest[1..].bytes().all(|b| b.is_ascii_digit()) =>
        {
            KeyCode::F(rest[1..].parse().context("invalid function key")?)
        }
        _ if rest.chars().count() == 1 => {
            let mut ch = rest.chars().next().expect("checked len == 1");
            if modifiers.contains(KeyModifiers::SHIFT) {
                ch = ch.to_ascii_uppercase();
            } else if ch.is_ascii_uppercase() {
                modifiers |= KeyModifiers::SHIFT;
            }
            KeyCode::Char(ch)
        }
        other => bail!("unrecognized key notation {notation:?} (key part {other:?})"),
    };
    Ok(KeyEvent::new(code, modifiers))
}

/// Renders `buffer` as plain text, one line per row, trailing blanks on
/// each row trimmed.
#[allow(dead_code)]
fn buffer_text(buffer: &Buffer) -> String {
    let width = buffer.area.width.max(1) as usize;
    buffer
        .content()
        .chunks(width)
        .map(|row| {
            row.iter()
                .map(Cell::symbol)
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Runs `steps` against a fresh headless (in-memory) terminal and `app`,
/// for regression tests. No key ever opens an external editor -- 'o' and
/// Enter on an out-of-tree node just set a status message instead.
#[allow(dead_code)]
fn run_script(app: &mut App, steps: &[Step]) -> Result<Terminal<TestBackend>> {
    let mut terminal = Terminal::new(TestBackend::new(100, 40))?;
    terminal.draw(|frame| draw(frame, app))?;
    for (i, step) in steps.iter().enumerate() {
        apply_step_headless(app, &mut terminal, step)
            .with_context(|| format!("script step {} failed: {step:?}", i + 1))?;
    }
    Ok(terminal)
}

#[allow(dead_code)]
fn apply_step_headless(
    app: &mut App,
    terminal: &mut Terminal<TestBackend>,
    step: &Step,
) -> Result<()> {
    match step {
        Step::Key { key } => {
            let event = parse_key(key)?;
            handle_key(app, event, None);
        }
        Step::Type { text } => {
            for ch in text.chars() {
                handle_key(
                    app,
                    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::empty()),
                    None,
                );
            }
        }
        Step::Wait { ms } => std::thread::sleep(Duration::from_millis(*ms)),
        Step::Resize { cols, rows } => terminal.backend_mut().resize(*cols, *rows),
        Step::Comment { .. } => {}
        Step::Screenshot { path } => {
            let text = buffer_text(terminal.backend().buffer());
            fs::write(path, text)
                .with_context(|| format!("write screenshot {}", path.display()))?;
        }
        Step::AssertContains { text } => {
            let screen = buffer_text(terminal.backend().buffer());
            if !screen.contains(text.as_str()) {
                bail!("expected screen to contain {text:?}; got:\n{screen}");
            }
        }
        Step::AssertNotContains { text } => {
            let screen = buffer_text(terminal.backend().buffer());
            if screen.contains(text.as_str()) {
                bail!("expected screen not to contain {text:?}; got:\n{screen}");
            }
        }
        Step::AssertStatus { text } => {
            if !app.status.contains(text.as_str()) {
                bail!("expected status to contain {text:?}; got {:?}", app.status);
            }
        }
    }
    terminal.draw(|frame| draw(frame, app))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use leo::{DerivedFile, ExternalFormat, RelativeFile, external_snapshot};

    #[test]
    fn extension_for_language_maps_directive_names_to_editor_extensions() {
        // A body-only node (no on-disk source file) opened for editing gets
        // a temp file named from its `@language` directive, so an external
        // editor picks the right filetype instead of falling back to plain
        // text -- e.g. a `@language python` action node must open as `.py`,
        // not `.txt`.
        assert_eq!(extension_for_language(Some("python")), "py");
        assert_eq!(extension_for_language(Some("rust")), "rs");
        assert_eq!(extension_for_language(Some("nu")), "nu");
        assert_eq!(extension_for_language(Some("lua")), "lua");
        assert_eq!(
            extension_for_language(Some("unknown-language")),
            "unknown-language"
        );
        assert_eq!(extension_for_language(None), "txt");
    }

    fn editing_app() -> App {
        let document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="a"><vh>A</vh><v t="b"><vh>B</vh></v><v t="c"><vh>C</vh></v></v></vnodes><tnodes><t tx="a"></t><t tx="b"></t><t tx="c"></t></tnodes></leo_file>"#,
        )
        .unwrap();
        App::new(
            document,
            PathBuf::from("test.leo"),
            String::new(),
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
            OriginalExternalState::default(),
            false,
        )
    }

    #[test]
    fn dynamic_source_location_skips_auto_dir_but_not_plain_auto() {
        // `@auto-dir`'s own argument is a glob/directory, not an openable
        // file -- `o` on that root node must not try to open a literal
        // "*.rs" path. A plain `@auto <path>` root is unaffected: it still
        // resolves straight to that one real file, as it always has.
        let mut app = editing_app();
        let node_b = NodeId::from("b");
        app.document
            .outline
            .nodes
            .get_mut(&node_b)
            .unwrap()
            .headline = "@auto-dir *.rs".into();
        let row = app.rows()[1].clone();
        assert_eq!(row.node, node_b);
        assert!(dynamic_source_location(&app, &row).is_none());

        app.document
            .outline
            .nodes
            .get_mut(&node_b)
            .unwrap()
            .headline = "@auto real.rs".into();
        let row = app.rows()[1].clone();
        let location = dynamic_source_location(&app, &row).unwrap();
        assert_eq!(location.path, Path::new("real.rs"));
    }

    #[test]
    fn dynamic_source_location_also_skips_auto_dir_descendants() {
        // A matched file nested under an `@auto-dir`'s synthetic `@path`
        // dirs has a bare-filename headline (see auto_dir.rs) -- ancestor
        // `@path` accumulation alone can't reconstruct its real path, since
        // the `@auto-dir` node's own resolved directory contributes no
        // `@path` node of its own. `o` on it must defer to
        // `App::source_nodes` (populated from `AutoFile::file_paths`)
        // rather than silently building a wrong path.
        let document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="dir"><vh>@auto-dir specs/**</vh><v t="leaf"><vh>@auto leaf.rs</vh></v></v></vnodes><tnodes><t tx="dir"></t><t tx="leaf"></t></tnodes></leo_file>"#,
        )
        .unwrap();
        let app = App::new(
            document,
            PathBuf::from("test.leo"),
            String::new(),
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
            OriginalExternalState::default(),
            false,
        );
        let row = Row {
            position: PositionId("0/0".into()),
            node: NodeId::from("leaf"),
            depth: 1,
            has_children: false,
        };
        assert!(dynamic_source_location(&app, &row).is_none());
    }

    #[test]
    fn status_precedes_controls_on_narrow_terminals() {
        let mut app = editing_app();
        app.status = "save failed: /missing/file: No such file or directory".into();
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let bottom_line: String = (0..50)
            .map(|x| buffer.cell((x, 9)).unwrap().symbol())
            .collect();
        assert!(bottom_line.starts_with("[save failed:"), "{bottom_line:?}");
    }

    #[test]
    fn search_popup_does_not_cover_the_highlighted_body_line() {
        let mut app = editing_app();
        app.split_horizontal = false;
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("b"))
            .unwrap()
            .body = "line one\nline two contains a needle here\nline three".into();
        app.selected = 1;
        start_search(&mut app);
        for character in "needle".chars() {
            handle_search_input(
                &mut app,
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
            );
        }
        assert_eq!(app.body_scroll, 1);

        let backend = TestBackend::new(200, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let matched_row = (0..30)
            .find(|&y| {
                let line: String = (85..200)
                    .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
                    .collect();
                line.trim_start()
                    .starts_with("line two contains a needle here")
            })
            .expect("matched body line should be visible, not hidden behind the search popup");
        let highlighted_cell = buffer
            .cell((85 + "line two contains a ".len() as u16, matched_row))
            .unwrap();
        assert_eq!(highlighted_cell.symbol(), "n");
        assert_eq!(highlighted_cell.style().bg, Some(Color::Yellow));
    }

    #[test]
    fn find_panel_docks_a_candidate_list_above_the_query_line() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .headline = "Alpha".into();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("b"))
            .unwrap()
            .headline = "Beta".into();
        start_find(&mut app);
        handle_find_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::SHIFT),
        );

        let backend = TestBackend::new(50, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let rendered: Vec<String> = (0..20)
            .map(|y| {
                (0..50)
                    .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
                    .collect::<String>()
            })
            .collect();
        // 2 matches: a 2-row candidate list docked directly above the
        // 1-row query line at the very bottom of the frame.
        assert!(rendered[17].starts_with("› Alpha"), "{:?}", rendered[17]);
        assert!(rendered[18].starts_with("  Beta"), "{:?}", rendered[18]);
        assert!(
            rendered[19].contains("Find headline") && rendered[19].contains("> a▏"),
            "{:?}",
            rendered[19]
        );
        // The outline/body panes still occupy every row above the panel.
        assert!(rendered[16].contains("Outline") || rendered[0].contains("Outline"));
    }

    #[test]
    fn detects_action_headlines_and_strips_the_marker() {
        assert!(is_action_headline("@action Build"));
        assert!(is_action_headline("@action"));
        assert!(!is_action_headline("@actionable"));
        assert!(!is_action_headline("Build"));
        assert_eq!(action_name("@action Build"), "Build");
        assert_eq!(action_name("@action   Say Hi  "), "Say Hi");
    }

    #[test]
    fn palette_skips_opening_with_nothing_to_run() {
        // No `@action` nodes and no `@import`ed script: there is nothing
        // the palette could ever offer, so opening it would drop the user
        // into a filter box that can never match anything -- instead it
        // should stay closed and just say so on the status line.
        let mut app = editing_app();

        start_palette(&mut app);

        assert!(app.palette.is_none());
        assert_eq!(app.status, "no commands available");
    }

    #[test]
    fn palette_lists_only_action_nodes_and_filters_by_name() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .headline = "@action Build".into();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("b"))
            .unwrap()
            .headline = "@action Test".into();
        // "c" is left as a plain headline and must not appear.
        start_palette(&mut app);
        assert_eq!(app.palette.as_ref().unwrap().matches.len(), 2);

        handle_palette_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
        );
        let palette = app.palette.as_ref().unwrap();
        assert_eq!(
            palette.matches,
            vec![PaletteEntry {
                label: "Build".into(),
                doc: None,
                kind: PaletteEntryKind::Action(PositionId("0".into())),
            }]
        );
    }

    #[test]
    fn palette_lists_and_runs_commands_from_an_imported_script() {
        let directory = env::temp_dir().join(format!(
            "leo-cub-tui-import-rhai-{}-{}",
            std::process::id(),
            fresh_node_id().0
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("lib.rhai"),
            r#"
const COMMANDS = #{ greet: "Say hello to the selected node." };

fn greet(doc, target) {
    target.h = target.h + " done";
    print("greeted " + target.h);
}

fn private_helper(doc, target) {
    doc.count()
}
"#,
        )
        .unwrap();

        let mut app = editing_app();
        app.path = directory.join("outline.leo");
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("c"))
            .unwrap()
            .headline = "@import lib.rhai".into();

        start_palette(&mut app);
        // "greet" is listed (it's in COMMANDS, with its description); the
        // description came through, and "private_helper" isn't listed at
        // all, even though it's a function in the same script.
        assert_eq!(
            app.palette.as_ref().unwrap().matches,
            vec![PaletteEntry {
                label: "greet  (lib)".into(),
                doc: Some("Say hello to the selected node.".into()),
                kind: PaletteEntryKind::Command {
                    script: directory.join("lib.rhai"),
                    name: "greet".into(),
                },
            }]
        );

        handle_palette_input(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(
            app.document.outline.nodes[&NodeId::from("a")].headline,
            "A done"
        );
        assert!(app.status.starts_with("'greet' finished"));
        let output = app.action_output.as_ref().expect("command produced output");
        assert_eq!(output.text.trim(), "greeted A done");

        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn debug_log_records_keys_and_palette_lookups() {
        let path = env::temp_dir().join(format!(
            "leo-cub-tui-debug-log-{}-{}.log",
            std::process::id(),
            fresh_node_id().0
        ));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .unwrap();
        let mut app = editing_app().with_debug_log(Some(file));

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            None,
        );
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
            None,
        );

        let contents = fs::read_to_string(&path).unwrap();
        assert!(
            contents.contains("key: Char('j')"),
            "should log the 'j' keypress: {contents}"
        );
        assert!(
            contents.contains("palette: computing entries"),
            "should log palette lookups starting: {contents}"
        );
        assert!(
            contents.contains("palette: ") && contents.contains("entries, "),
            "should log the resolved entry/error count: {contents}"
        );

        fs::remove_file(&path).ok();
    }

    #[test]
    fn palette_unions_available_commands_with_commands_and_lets_it_override_by_name() {
        let directory = env::temp_dir().join(format!(
            "leo-cub-tui-available-commands-{}-{}",
            std::process::id(),
            fresh_node_id().0
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("lib.rhai"),
            r#"
const COMMANDS = #{ always: "Always available.", both: "static doc" };

fn available_commands(doc, target) {
    let extra = #{};
    if target.b == "trigger" {
        extra.conditional = "Only when the body says trigger.";
    }
    extra.both = "dynamic doc";
    extra
}

fn always(doc, target) {}
fn conditional(doc, target) {}
fn both(doc, target) {}
"#,
        )
        .unwrap();

        let mut app = editing_app();
        app.path = directory.join("outline.leo");
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("c"))
            .unwrap()
            .headline = "@import lib.rhai".into();

        // Selection ("a", the default) has an empty body -- `conditional`
        // stays hidden, but `both`'s dynamic description already wins
        // over its static one from COMMANDS, and that dynamic origin also
        // puts it ahead of `always` (unconditional, so ranked after
        // whatever `available_commands` singled out for this selection).
        start_palette(&mut app);
        let labels: Vec<&str> = app
            .palette
            .as_ref()
            .unwrap()
            .matches
            .iter()
            .map(|entry| entry.label.as_str())
            .collect();
        assert_eq!(labels, vec!["both  (lib)", "always  (lib)"]);
        let both_doc = app
            .palette
            .as_ref()
            .unwrap()
            .matches
            .iter()
            .find(|entry| entry.label == "both  (lib)")
            .unwrap()
            .doc
            .clone();
        assert_eq!(both_doc, Some("dynamic doc".into()));

        // Give "a" a body that satisfies available_commands' condition --
        // "conditional" should now join the union, alongside "both", both
        // ranked ahead of "always" as dynamic-origin entries.
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .body = "trigger".into();
        start_palette(&mut app);
        let labels: Vec<&str> = app
            .palette
            .as_ref()
            .unwrap()
            .matches
            .iter()
            .map(|entry| entry.label.as_str())
            .collect();
        assert_eq!(
            labels,
            vec!["both  (lib)", "conditional  (lib)", "always  (lib)"]
        );

        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn palette_surfaces_an_imported_scripts_compile_error_instead_of_going_silent() {
        let directory = env::temp_dir().join(format!(
            "leo-cub-tui-broken-import-{}-{}",
            std::process::id(),
            fresh_node_id().0
        ));
        fs::create_dir_all(&directory).unwrap();
        // `shared` is a Rhai reserved word -- this fails to compile at
        // all, so both COMMANDS and any fn defs are unreachable. Without
        // surfacing the error, the palette would look exactly like an
        // import that legitimately declares no commands.
        fs::write(
            directory.join("broken.rhai"),
            "const COMMANDS = #{};\nfn shared(doc, target) {}\n",
        )
        .unwrap();

        let mut app = editing_app();
        app.path = directory.join("outline.leo");
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("c"))
            .unwrap()
            .headline = "@import broken.rhai".into();

        start_palette(&mut app);
        let palette = app.palette.as_ref().unwrap();
        assert!(palette.matches.is_empty());
        assert_eq!(palette.errors.len(), 1);
        assert!(
            palette.errors[0].contains("reserved keyword"),
            "{:?}",
            palette.errors[0]
        );
        assert!(
            palette.errors[0].contains("broken.rhai"),
            "{:?}",
            palette.errors[0]
        );

        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn palette_only_offers_the_builtin_import_command_on_a_path_node() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .headline = "@path src".into();
        app.selected = 0;
        start_palette(&mut app);
        assert_eq!(
            app.palette.as_ref().unwrap().matches,
            vec![PaletteEntry {
                label: "Import new files into @path".into(),
                doc: None,
                kind: PaletteEntryKind::Builtin(0),
            }]
        );

        let child_index = app
            .rows()
            .iter()
            .position(|row| row.node == NodeId::from("b"))
            .unwrap();
        app.selected = child_index;
        // Nothing available for this selection: the palette shouldn't open
        // into a dead-end filter box (see `palette_skips_opening_with_nothing_to_run`).
        start_palette(&mut app);
        assert!(app.palette.is_none());
        assert_eq!(app.status, "no commands available");
    }

    #[test]
    fn command_import_adds_only_new_files_under_the_selected_path_node() {
        let directory = env::temp_dir().join(format!(
            "leo-cub-tui-import-{}-{}",
            std::process::id(),
            fresh_node_id().0
        ));
        fs::create_dir_all(directory.join("src")).unwrap();
        fs::write(directory.join("src/a.txt"), "old").unwrap();
        fs::write(directory.join("src/b.txt"), "new").unwrap();

        let mut app = editing_app();
        app.path = directory.join("outline.leo");
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .headline = "@path src".into();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("b"))
            .unwrap()
            .headline = "@auto a.txt".into();
        app.document.outline.roots[0].children.truncate(1);
        app.document.outline.nodes.remove(&NodeId::from("c"));
        app.selected = 0;

        command_import_run(&mut app);

        let path_position = &app.document.outline.roots[0];
        assert_eq!(path_position.children.len(), 2);
        let added_position = PositionId("0/1".into());
        let added = &app.document.outline.nodes[&path_position.children[1].node];
        assert_eq!(added.headline, "@auto b.txt");
        assert_eq!(
            added.body, "new",
            "new derived nodes should have their content loaded immediately, without a save+reload"
        );
        assert!(
            app.expanded.contains(&added_position),
            "a freshly imported derived node should start expanded so its loaded content is visible"
        );
        assert!(app.dirty);
        assert!(app.status.starts_with("imported 1 new file(s)"));
        assert!(!app.status.contains("Ctrl-R"));

        command_import_run(&mut app);
        assert!(
            app.status
                .starts_with("no new files or subdirectories to import")
        );

        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn command_import_adds_path_nodes_for_subdirectories_with_no_direct_files() {
        let directory = env::temp_dir().join(format!(
            "leo-cub-tui-import-dirs-{}-{}",
            std::process::id(),
            fresh_node_id().0
        ));
        fs::create_dir_all(directory.join("src/nested")).unwrap();
        fs::write(directory.join("src/nested/lib.rs"), "pub fn f() {}").unwrap();

        let mut app = editing_app();
        app.path = directory.join("outline.leo");
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .headline = "@path src".into();
        app.document.outline.roots[0].children.clear();
        app.document.outline.nodes.remove(&NodeId::from("b"));
        app.document.outline.nodes.remove(&NodeId::from("c"));
        app.selected = 0;

        command_import_run(&mut app);

        let path_position = &app.document.outline.roots[0];
        assert_eq!(path_position.children.len(), 1);
        let added = &app.document.outline.nodes[&path_position.children[0].node];
        assert_eq!(added.headline, "@path nested");
        assert!(
            app.status
                .starts_with("imported 0 new file(s) and 1 new subdirectory(ies)")
        );

        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn running_an_action_shows_output_until_selection_moves_away() {
        let mut app = editing_app();
        {
            let node = app
                .document
                .outline
                .nodes
                .get_mut(&NodeId::from("b"))
                .unwrap();
            node.headline = "@action Greet".into();
            node.body = "print(\"hello-from-action\");".into();
        }
        app.selected = 1; // row "0/0" -> node "b"

        run_action(
            &mut app,
            &PositionId("0/0".into()),
            &PositionId("0/0".into()),
        );

        let output = app.action_output.as_ref().expect("action produced output");
        assert_eq!(output.node, NodeId::from("b"));
        assert_eq!(output.status, Some(0));
        assert!(
            output.text.contains("hello-from-action"),
            "{:?}",
            output.text
        );

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert!(
            app.action_output.is_some(),
            "output should still show while its node is selected"
        );

        app.selected = 0; // node "a"
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert!(
            app.action_output.is_none(),
            "output should clear once selection moves to a different node"
        );
    }

    #[test]
    fn a_rhai_action_populates_the_persistent_log() {
        let mut app = editing_app();
        {
            let node = app
                .document
                .outline
                .nodes
                .get_mut(&NodeId::from("b"))
                .unwrap();
            node.headline = "@action Greet in Rhai".into();
            node.body = "@language rhai\nprint(\"hello from rhai\");".into();
        }
        app.selected = 1;

        run_action(
            &mut app,
            &PositionId("0/0".into()),
            &PositionId("0/0".into()),
        );

        assert!(
            app.logs.iter().any(|line| line.contains("hello from rhai")),
            "{:?}",
            app.logs
        );
    }

    #[test]
    fn log_view_renders_buffered_lines_and_the_repl_input_line() {
        let mut app = editing_app();
        app.log_view = true;
        push_log(&mut app, "hello from rhai");
        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let screen = terminal.backend().buffer().content[..]
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(screen.contains("hello from rhai"), "{screen:?}");
        assert!(
            !screen.contains("> "),
            "no REPL line until Enter is pressed"
        );

        app.log_repl = Some(ReplInput::new("print(1)"));
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let screen = terminal.backend().buffer().content[..]
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(screen.contains("> print(1)"), "{screen:?}");
    }

    #[test]
    fn l_toggles_the_log_view_open_and_closed() {
        let mut app = editing_app();
        assert!(!app.log_view);

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
            None,
        );
        assert!(app.log_view);

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
            None,
        );
        assert!(!app.log_view);
    }

    #[test]
    fn enter_opens_the_repl_and_typing_l_inserts_a_literal_l_instead_of_closing() {
        let mut app = editing_app();
        app.log_view = true;

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            None,
        );
        assert!(app.log_repl.is_some());

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
            None,
        );
        assert!(
            app.log_view,
            "log view must stay open while typing in the REPL"
        );
        assert_eq!(app.log_repl.as_ref().map(|input| input.value()), Some("l"));
    }

    #[test]
    fn left_and_right_move_the_repl_cursor_for_mid_string_editing() {
        let mut app = editing_app();
        app.log_view = true;
        app.log_repl = Some(ReplInput::new("ac"));
        app.log_repl
            .as_mut()
            .unwrap()
            .input
            .move_cursor(ratatui_textarea::CursorMove::Jump(0, 1));

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
            None,
        );
        assert_eq!(
            app.log_repl.as_ref().map(|input| input.value()),
            Some("abc")
        );
        assert_eq!(app.log_repl.as_ref().unwrap().input.cursor(), (0, 2));

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
            None,
        );
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
            None,
        );
        assert_eq!(app.log_repl.as_ref().unwrap().input.cursor(), (0, 0));

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
            None,
        );
        assert_eq!(app.log_repl.as_ref().unwrap().input.cursor(), (0, 1));

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::End, KeyModifiers::NONE),
            None,
        );
        assert_eq!(app.log_repl.as_ref().unwrap().input.cursor(), (0, 3));

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Home, KeyModifiers::NONE),
            None,
        );
        assert_eq!(app.log_repl.as_ref().unwrap().input.cursor(), (0, 0));

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE),
            None,
        );
        assert_eq!(app.log_repl.as_ref().map(|input| input.value()), Some("bc"));
    }

    #[test]
    fn esc_leaves_the_repl_but_keeps_the_log_view_open_then_closes_it() {
        let mut app = editing_app();
        app.log_view = true;
        app.log_repl = Some(ReplInput::new("print(1)"));

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            None,
        );
        assert!(app.log_repl.is_none());
        assert!(
            app.log_view,
            "Esc from the REPL should return to browse mode"
        );

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            None,
        );
        assert!(
            !app.log_view,
            "Esc from browse mode should close the log view"
        );
    }

    #[test]
    fn repl_snippet_reads_the_selected_node_via_p_and_can_mutate_the_outline() {
        let mut app = editing_app();
        app.selected = 2; // row "0/1" -> node "c"
        app.log_view = true;
        app.log_repl = Some(ReplInput::new(""));

        for character in "print(p.h);".chars() {
            handle_key(
                &mut app,
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
                None,
            );
        }
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            None,
        );

        assert!(
            app.logs.iter().any(|line| line.contains("> print(p.h);")),
            "{:?}",
            app.logs
        );
        assert!(
            app.logs.iter().any(|line| line.contains('C')),
            "expected p.h's evaluation (\"C\") to print via the rhai REPL: {:?}",
            app.logs
        );

        app.log_repl = Some(ReplInput::new(""));
        for character in "doc.set_headline(target, \"C renamed\");".chars() {
            handle_key(
                &mut app,
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
                None,
            );
        }
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            None,
        );

        assert!(app.dirty);
        assert_eq!(
            app.document.outline.nodes[&NodeId::from("c")].headline,
            "C renamed"
        );
    }

    #[test]
    fn repl_snippet_via_p_sees_the_selected_clone_occurrence_not_the_first_one() {
        // A -> [Shared, C -> [Shared (clone)]] -- "shared" occurs twice.
        let mut app = App::new(
            LeoDocument::parse(
                r#"<leo_file><vnodes><v t="a"><vh>A</vh><v t="shared"><vh>Shared</vh></v><v t="c"><vh>C</vh><v t="shared"></v></v></v></vnodes><tnodes><t tx="a"></t><t tx="shared"></t><t tx="c"></t></tnodes></leo_file>"#,
            )
            .unwrap(),
            PathBuf::from("test.leo"),
            String::new(),
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
            OriginalExternalState::default(),
            false,
        );
        // Reveal the clone under C (collapsed by default) and select it --
        // row "0/1/0", the *second* occurrence of "shared".
        app.expanded.insert(PositionId("0/1".into()));
        app.selected = 3;
        assert_eq!(
            app.selected_row().map(|row| row.position),
            Some(PositionId("0/1/0".into()))
        );

        app.log_view = true;
        app.log_repl = Some(ReplInput::new(""));
        for character in "print(p.path());".chars() {
            handle_key(
                &mut app,
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
                None,
            );
        }
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            None,
        );

        // Anchored to the occurrence actually selected -- not the first
        // occurrence of "shared" ("A/Shared"), which a gnx-only lookup
        // would have returned.
        assert!(
            app.logs.iter().any(|line| line.contains("A/C/Shared")),
            "expected p.path() to name the selected clone occurrence: {:?}",
            app.logs
        );
        assert!(
            !app.logs.iter().any(|line| line.trim_end() == "A/Shared"),
            "should not have fallen back to the first occurrence: {:?}",
            app.logs
        );
    }

    #[test]
    fn a_rhai_action_reports_a_script_error_without_touching_the_outline() {
        let mut app = editing_app();
        let node_count_before = app.document.outline.nodes.len();
        {
            let node = app
                .document
                .outline
                .nodes
                .get_mut(&NodeId::from("b"))
                .unwrap();
            node.headline = "@action Broken Rhai".into();
            node.body = "@language rhai\nlet x = 1 / 0;".into();
        }
        app.selected = 1;

        run_action(
            &mut app,
            &PositionId("0/0".into()),
            &PositionId("0/0".into()),
        );

        assert!(!app.dirty);
        assert_eq!(app.document.outline.nodes.len(), node_count_before);
        let output = app.action_output.as_ref().expect("action produced output");
        assert_eq!(output.interpreter, "rhai");
        assert_ne!(output.status, Some(0));
    }

    #[test]
    fn a_rhai_action_gets_the_same_doc_api_as_cub_run_and_mutates_the_live_outline() {
        // Node "b" ("@action Rename target") is the action; node "c" ("C")
        // is the target it was invoked against -- `target` should be "c"'s
        // gnx, and `doc` should be bound to the outline already open in the
        // session, not a fresh one read from disk.
        let mut app = editing_app();
        {
            let node = app
                .document
                .outline
                .nodes
                .get_mut(&NodeId::from("b"))
                .unwrap();
            node.headline = "@action Rename target".into();
            node.body = "@language rhai\ndoc.set_headline(target, doc.headline(target) + \" (renamed)\");\nprint(doc.count());".into();
        }

        run_action(
            &mut app,
            &PositionId("0/0".into()),
            &PositionId("0/1".into()),
        );

        assert_eq!(
            app.document.outline.nodes[&NodeId::from("c")].headline,
            "C (renamed)"
        );
        assert!(app.dirty);
        let output = app.action_output.as_ref().expect("action produced output");
        assert_eq!(output.status, Some(0));
        assert_eq!(output.text.trim(), "3");
    }

    #[test]
    fn a_read_only_rhai_action_does_not_mark_the_outline_dirty() {
        let mut app = editing_app();
        {
            let node = app
                .document
                .outline
                .nodes
                .get_mut(&NodeId::from("b"))
                .unwrap();
            node.headline = "@action Inspect target".into();
            node.body = "@language rhai\nprint(doc.headline(target));".into();
        }

        run_action(
            &mut app,
            &PositionId("0/0".into()),
            &PositionId("0/1".into()),
        );

        assert!(!app.dirty);
        let output = app.action_output.as_ref().expect("action produced output");
        assert_eq!(output.text.trim(), "C");
    }

    #[test]
    fn enter_in_the_palette_runs_the_selected_action_and_selects_its_node() {
        let mut app = editing_app();
        {
            let node = app
                .document
                .outline
                .nodes
                .get_mut(&NodeId::from("b"))
                .unwrap();
            node.headline = "@action Greet".into();
            node.body = "print(\"hi\");".into();
        }

        start_palette(&mut app);
        for character in "greet".chars() {
            handle_palette_input(
                &mut app,
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
            );
        }
        handle_palette_input(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(app.palette.is_none());
        assert_eq!(app.selected_row().unwrap().node, NodeId::from("b"));
        assert_eq!(app.action_output.as_ref().unwrap().status, Some(0));
    }

    #[test]
    fn enter_in_the_palette_runs_the_action_against_the_node_selected_beforehand() {
        // "c" is selected when the palette opens; "b" is the action node
        // picked from it. `target` must describe "c" -- reproduces a bug
        // where it described "b" (the action itself) instead.
        let mut app = editing_app();
        {
            let node = app
                .document
                .outline
                .nodes
                .get_mut(&NodeId::from("b"))
                .unwrap();
            node.headline = "@action Greet".into();
            node.body = "print(target);".into();
        }
        app.selected = 2; // row "0/1" -> node "c"

        start_palette(&mut app);
        for character in "greet".chars() {
            handle_palette_input(
                &mut app,
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
            );
        }
        handle_palette_input(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        let output = app.action_output.as_ref().expect("action produced output");
        assert_eq!(output.text.trim(), "c");
    }

    #[test]
    fn recognizes_only_sentinelled_file_headlines() {
        assert_eq!(thin_filename("@file src/main.rs"), Some("src/main.rs"));
        assert_eq!(thin_filename("@file \"src/main.rs\""), Some("src/main.rs"));
        assert_eq!(thin_filename("@clean src/main.rs"), None);
        assert_eq!(thin_filename("ordinary"), None);
    }

    #[test]
    fn highlights_headline_directives_and_filenames() {
        let spans = headline_spans("@auto src/main.rs");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "@auto");
        assert_eq!(spans[0].style.fg, Some(Color::Cyan));
        assert_eq!(spans[1].content, " ");
        assert_eq!(spans[1].style.fg, None);
        assert_eq!(spans[2].content, "src/main.rs");
        assert_eq!(spans[2].style.fg, Some(Color::Yellow));
    }

    #[test]
    fn shows_a_colored_asterisk_only_for_dirty_nodes() {
        let dirty = dirty_marker(true);
        assert_eq!(dirty.content, "* ");
        assert_eq!(dirty.style.fg, Some(Color::LightRed));

        let clean = dirty_marker(false);
        assert_eq!(clean.content, "  ");
        assert_eq!(clean.style.fg, None);
    }

    #[test]
    fn shows_an_arrow_for_nodes_updated_by_reload_even_with_a_body() {
        let updated = body_marker(true, true, false);
        assert_eq!(updated.content, "↑ ");
        assert_eq!(updated.style.fg, Some(Color::LightGreen));

        let updated_without_body = body_marker(false, true, false);
        assert_eq!(updated_without_body.content, "↑ ");
        assert_eq!(updated_without_body.style.fg, Some(Color::LightGreen));
    }

    #[test]
    fn shows_a_subtle_dot_only_for_nodes_with_body_content() {
        let populated = body_marker(true, false, false);
        assert_eq!(populated.content, "· ");
        assert_eq!(populated.style.fg, Some(Color::DarkGray));

        let empty = body_marker(false, false, false);
        assert_eq!(empty.content, "  ");
        assert_eq!(empty.style.fg, None);
    }

    #[test]
    fn shows_a_dot_for_marked_nodes_taking_priority_over_updated_and_body() {
        let marked = body_marker(true, true, true);
        assert_eq!(marked.content, "● ");
        assert_eq!(marked.style.fg, Some(Color::LightYellow));

        let marked_without_body = body_marker(false, false, true);
        assert_eq!(marked_without_body.content, "● ");
        assert_eq!(marked_without_body.style.fg, Some(Color::LightYellow));
    }

    #[test]
    fn editing_a_headline_initially_selects_and_replaces_it() {
        let mut app = editing_app();

        edit_headline(&mut app);
        assert!(app.input.as_ref().unwrap().input.is_selecting());
        handle_headline_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::SHIFT),
        );

        let input = app.input.as_ref().unwrap();
        assert_eq!(input.value(), "Z");
        assert_eq!(input.input.cursor(), (0, 1));
        assert!(!input.input.is_selecting());
    }

    #[test]
    fn ctrl_arrow_while_editing_a_headline_moves_the_node_and_stays_in_edit_mode() {
        let mut app = editing_app();
        app.selected = 1; // node "b", first child of "a"

        edit_headline(&mut app);
        assert!(app.input.is_some());
        let editing_node = app.input.as_ref().unwrap().node.clone();
        assert_eq!(editing_node, NodeId::from("b"));

        handle_headline_input(
            &mut app,
            KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL),
        );

        assert!(
            app.input.is_some(),
            "Ctrl-Down should keep the headline editor open"
        );
        assert_eq!(
            app.input.as_ref().unwrap().node,
            editing_node,
            "still editing the same node, now moved"
        );
        let root = &app.document.outline.roots[0];
        assert_eq!(root.node, NodeId::from("a"));
        let siblings: Vec<_> = root
            .children
            .iter()
            .map(|child| child.node.clone())
            .collect();
        assert_eq!(
            siblings,
            vec![NodeId::from("c"), NodeId::from("b")],
            "Ctrl-Down should have swapped b and c"
        );
        assert!(app.dirty);
    }

    #[test]
    fn ctrl_s_while_editing_a_headline_commits_the_edit_and_saves() {
        let directory = env::temp_dir().join(format!(
            "leo-cub-tui-ctrl-s-headline-{}-{}",
            std::process::id(),
            fresh_node_id().0
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("test.leo");

        let mut app = editing_app();
        app.path = path.clone();

        edit_headline(&mut app);
        assert!(app.input.is_some());
        app.input.as_mut().unwrap().input = ratatui_textarea::TextArea::new(vec!["Renamed".into()]);
        handle_headline_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        );

        assert!(app.input.is_none(), "Ctrl-S should exit headline editing");
        assert_eq!(
            app.document.outline.nodes[&NodeId::from("a")].headline,
            "Renamed"
        );
        assert!(!app.dirty, "Ctrl-S should have saved the document");
        assert!(app.status.starts_with("saved"), "{}", app.status);
        assert!(path.exists());

        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn quick_edit_body_prefills_selects_and_expands_the_body_pane() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .body = "existing body".into();
        app.outline_full_width = true;

        quick_edit_body(&mut app);

        let edit = app.body_input.as_ref().unwrap();
        assert_eq!(edit.input.lines().join("\n"), "existing body");
        assert!(edit.input.is_selecting());
        assert!(
            app.body_full_width,
            "should expand to full width while editing"
        );
        assert!(
            !app.outline_full_width,
            "outline full-width would hide the body pane being edited"
        );
    }

    #[test]
    fn typing_over_a_selected_body_and_committing_writes_it_to_the_node() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .body = "old".into();

        quick_edit_body(&mut app);
        handle_body_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::NONE),
        );
        handle_body_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
        );

        assert!(app.body_input.is_none());
        assert_eq!(app.document.outline.nodes[&NodeId::from("a")].body, "Z");
        assert!(app.dirty);
        assert!(app.dirty_nodes.contains(&NodeId::from("a")));
    }

    #[test]
    fn ctrl_s_while_editing_body_commits_the_edit_and_saves() {
        let directory = env::temp_dir().join(format!(
            "leo-cub-tui-ctrl-s-body-{}-{}",
            std::process::id(),
            fresh_node_id().0
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("test.leo");

        let mut app = editing_app();
        app.path = path.clone();

        quick_edit_body(&mut app);
        handle_body_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::NONE),
        );
        handle_body_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        );

        assert!(app.body_input.is_none());
        assert_eq!(app.document.outline.nodes[&NodeId::from("a")].body, "Q");
        assert!(!app.dirty, "Ctrl-S should have saved the document");
        assert!(path.exists());

        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn esc_cancels_body_edit_without_touching_the_node_and_restores_layout() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .body = "untouched".into();
        app.body_full_width = false;
        app.outline_full_width = true;

        quick_edit_body(&mut app);
        assert!(app.body_full_width, "should force full width while editing");
        assert!(!app.outline_full_width);
        handle_body_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE),
        );
        handle_body_input(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert!(app.body_input.is_none());
        assert_eq!(
            app.document.outline.nodes[&NodeId::from("a")].body,
            "untouched"
        );
        assert!(
            !app.body_full_width && app.outline_full_width,
            "should restore the pre-edit layout, not just clear it"
        );
    }

    #[test]
    fn pasting_into_the_quick_body_entry_inserts_it_and_commits_it_verbatim() {
        let mut app = editing_app();

        quick_edit_body(&mut app);
        let pasted = "line one\nline two\nline three".to_owned();
        insert_paste_into_body(&mut app, pasted.clone());
        handle_body_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
        );

        assert_eq!(app.document.outline.nodes[&NodeId::from("a")].body, pasted);
    }

    #[test]
    fn renaming_an_external_headline_refreshes_its_path_and_delimiters() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .headline = "@file test.py".into();
        app.writable_external.insert(
            NodeId::from("a"),
            WritableExternalFile {
                path: PathBuf::from("test.py"),
                start_delimiter: "#".into(),
                end_delimiter: String::new(),
                original: Outline::default(),
                format: ExternalFormat::Thin,
            },
        );

        edit_headline(&mut app);
        app.input.as_mut().unwrap().input =
            ratatui_textarea::TextArea::new(vec!["@file test.md".into()]);
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        let file = &app.writable_external[&NodeId::from("a")];
        assert_eq!(file.path, PathBuf::from("test.md"));
        assert_eq!(file.start_delimiter, "#");
        assert_eq!(file.end_delimiter, "");
    }

    #[test]
    fn committing_a_new_auto_dir_headline_does_not_panic() {
        // Reproduces a real crash: `i` (insert_headline) starts a fresh node,
        // typing `@auto-dir *.md` as its headline and pressing Enter used to
        // panic in commit_headline_edit_without_chaining. `external_filename`
        // recognizes `@auto-dir` as an external directive, but
        // `dynamic_source_location` deliberately returns `None` for it (its
        // argument is a glob/directory, not an openable file) -- the commit
        // path unconditionally `.expect()`ed a location anyway.
        let mut app = editing_app();
        insert_headline(&mut app);
        app.input.as_mut().unwrap().input =
            ratatui_textarea::TextArea::new(vec!["@auto-dir *.md".into()]);
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        let node = app
            .document
            .outline
            .nodes
            .values()
            .find(|node| node.headline == "@auto-dir *.md")
            .expect("new node committed with its typed headline");
        assert!(!app.writable_external.contains_key(&node.id));
    }

    #[test]
    fn renaming_a_loaded_auto_root_to_at_f_and_saving_promotes_it_to_real_sentinel_segments() {
        // The workflow: open an outline with an `@auto script.rhai` node (its
        // functions get parsed in as real, live outline nodes), rename just
        // the root headline to `@f script.rhai`, and save. No dedicated
        // "convert" command exists -- renaming picks up `app.writable_external`
        // tracking (see `handle_headline_input`) with `original:
        // Outline::default()`, so `save`'s `prepare_external_updates` sees the
        // already-materialized `@auto`-parsed children as new/diverged content
        // and writes them out as sentinels, promoting the plain script in
        // place into a real, individually-editable `@f` file.
        let directory = env::temp_dir().join(format!(
            "leo-cub-tui-auto-to-f-{}-{}",
            std::process::id(),
            fresh_node_id().0
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("script.rhai"),
            "fn greet(name) {\n    \"hi \" + name\n}\n",
        )
        .unwrap();
        fs::write(
            directory.join("outline.leo"),
            r#"<leo_file><vnodes><v t="r"><vh>@auto script.rhai</vh></v></vnodes><tnodes><t tx="r"></t></tnodes></leo_file>"#,
        )
        .unwrap();

        let mut app = build_app(directory.join("outline.leo"), true, None).unwrap();
        app.selected = 0;
        edit_headline(&mut app);
        app.input.as_mut().unwrap().input =
            ratatui_textarea::TextArea::new(vec!["@f script.rhai".into()]);
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        save(&mut app);

        assert!(app.status.starts_with("saved"), "{}", app.status);
        let rewritten = fs::read_to_string(directory.join("script.rhai")).unwrap();
        assert!(
            rewritten.starts_with("//@+leo-ver=cub-1-thin\n"),
            "{rewritten}"
        );
        assert!(
            rewritten.contains("fn greet(name) {"),
            "the auto-parsed function body should survive the promotion: {rewritten}"
        );

        let reparsed = RelativeFile::parse(&rewritten).unwrap();
        assert_eq!(reparsed.outline.roots[0].children.len(), 1);
        assert_eq!(
            reparsed.outline.nodes[&reparsed.outline.roots[0].children[0].node].headline,
            "fn greet"
        );

        // The promotion round-trips: reopening the outline re-parses
        // script.rhai as `@f` (not `@auto`) and reconstructs the same node.
        let reopened = build_app(directory.join("outline.leo"), true, None).unwrap();
        assert_eq!(
            reopened.document.outline.nodes[&NodeId::from("r")].headline,
            "@f script.rhai"
        );
        assert_eq!(reopened.document.outline.roots[0].children.len(), 1);

        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn inserts_beside_an_auto_root_but_not_beside_its_derived_descendants() {
        let mut app = editing_app();
        app.derived_nodes.insert(NodeId::from("b"));

        insert_headline(&mut app);
        assert_eq!(app.document.outline.roots.len(), 2);
        assert!(app.input.is_some());

        app.input = None;
        select_position(&mut app, &PositionId("0/0".into()));
        insert_headline(&mut app);
        assert_eq!(app.document.outline.roots[0].children.len(), 2);
        assert!(app.input.is_none());
        assert_eq!(
            app.status,
            "@auto descendants are read-only; press o to edit the source"
        );
    }

    #[test]
    fn arrow_key_cancels_a_chained_insert_and_moves_out() {
        let mut app = editing_app();
        app.selected = 1; // node "b"

        insert_headline(&mut app);
        assert!(app.input.is_some());

        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

        assert!(app.input.is_none());
        assert_eq!(app.document.outline.nodes.len(), 3);
        let row = app.selected_row().unwrap();
        assert_eq!(app.document.outline.nodes[&row.node].headline, "C");

        app.selected = 1; // node "b" again
        edit_headline(&mut app);
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

        assert!(app.input.is_none());
        let row = app.selected_row().unwrap();
        assert_eq!(app.document.outline.nodes[&row.node].headline, "A");
    }

    #[test]
    fn arrow_key_commits_typed_headline_text_and_moves_without_chaining() {
        let mut app = editing_app();
        app.selected = 1; // node "b"
        let node = app.selected_row().unwrap().node;

        edit_headline(&mut app);
        handle_headline_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::NONE),
        );
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

        assert!(app.input.is_none());
        assert_eq!(app.document.outline.nodes[&node].headline, "Z");
        assert!(app.dirty_nodes.contains(&node));
        let row = app.selected_row().unwrap();
        assert_eq!(app.document.outline.nodes[&row.node].headline, "C");

        // Typed text in a freshly-inserted node is committed too, but
        // doesn't chain into starting yet another insert.
        insert_headline(&mut app);
        handle_headline_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE),
        );
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

        assert!(app.input.is_none());
        assert_eq!(app.document.outline.nodes.len(), 4);
        assert!(
            app.document
                .outline
                .nodes
                .values()
                .any(|node| node.headline == "D")
        );
    }

    #[test]
    fn accepting_a_new_headline_immediately_starts_the_next_sibling() {
        let mut app = editing_app();

        insert_headline(&mut app);
        for character in "First".chars() {
            handle_headline_input(
                &mut app,
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
            );
        }
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(app.document.outline.roots.len(), 3);
        assert_eq!(
            app.document.outline.nodes[&app.document.outline.roots[1].node].headline,
            "First"
        );
        let input = app.input.as_ref().expect("chained insert starts editing");
        assert!(input.inserted_position.is_some());
        assert_eq!(input.value(), "");

        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert!(app.input.is_none());
        assert_eq!(app.document.outline.roots.len(), 2);
    }

    #[test]
    fn edits_and_saves_a_thin_file_tree() {
        let directory = env::temp_dir().join(format!(
            "leo-cub-tui-file-edit-{}-{}",
            std::process::id(),
            fresh_node_id().0
        ));
        fs::create_dir_all(&directory).unwrap();
        let external_path = directory.join("test.py");
        let outline_path = directory.join("test.leo");
        fs::write(
            &external_path,
            "#@+leo-ver=5-thin\n#@+node:a: * @file test.py\n#@+others\n#@+node:b: ** B\n#@-others\n#@-leo\n",
        )
        .unwrap();

        let mut app = editing_app();
        app.path = outline_path.clone();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .headline = "@file test.py".into();
        app.document.outline.roots[0].children.truncate(1);
        app.document.outline.nodes.remove(&NodeId::from("c"));
        app.derived_nodes.insert(NodeId::from("b"));
        let original = external_snapshot(&app.document.outline, &NodeId::from("a"))
            .unwrap()
            .1;
        app.writable_external.insert(
            NodeId::from("a"),
            WritableExternalFile {
                path: external_path.clone(),
                start_delimiter: "#".into(),
                end_delimiter: String::new(),
                original,
                format: ExternalFormat::Thin,
            },
        );
        app.original_external
            .children
            .insert(NodeId::from("a"), Vec::new());
        app.original_external
            .bodies
            .insert(NodeId::from("a"), String::new());
        select_position(&mut app, &PositionId("0/0".into()));

        insert_headline(&mut app);
        handle_headline_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT),
        );
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        save(&mut app);

        assert!(!app.dirty, "{}", app.status);
        let written = DerivedFile::parse(&fs::read_to_string(&external_path).unwrap()).unwrap();
        assert_eq!(written.outline.roots[0].children.len(), 2);
        assert_eq!(
            written.outline.nodes[&written.outline.roots[0].children[1].node].headline,
            "C"
        );
        let persisted = LeoDocument::open(&outline_path).unwrap();
        assert!(persisted.outline.roots[0].children.is_empty());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn edits_and_saves_an_f_file_tree() {
        let directory = env::temp_dir().join(format!(
            "leo-cub-tui-f-edit-{}-{}",
            std::process::id(),
            fresh_node_id().0
        ));
        fs::create_dir_all(&directory).unwrap();
        let external_path = directory.join("test.py");
        let outline_path = directory.join("test.leo");
        fs::write(
            &external_path,
            "#@+leo-ver=cub-1-thin\n#@0 [a] @f test.py\n#@+others\n#@> B\n#@-others\n#@-leo\n",
        )
        .unwrap();

        let mut app = editing_app();
        app.path = outline_path.clone();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .headline = "@f test.py".into();
        app.document.outline.roots[0].children.truncate(1);
        app.document.outline.nodes.remove(&NodeId::from("c"));
        app.derived_nodes.insert(NodeId::from("b"));
        let original = external_snapshot(&app.document.outline, &NodeId::from("a"))
            .unwrap()
            .1;
        app.writable_external.insert(
            NodeId::from("a"),
            WritableExternalFile {
                path: external_path.clone(),
                start_delimiter: "#".into(),
                end_delimiter: String::new(),
                original,
                format: ExternalFormat::Relative,
            },
        );
        app.original_external
            .children
            .insert(NodeId::from("a"), Vec::new());
        app.original_external
            .bodies
            .insert(NodeId::from("a"), String::new());
        select_position(&mut app, &PositionId("0/0".into()));

        insert_headline(&mut app);
        handle_headline_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT),
        );
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        save(&mut app);

        assert!(!app.dirty, "{}", app.status);
        let written_text = fs::read_to_string(&external_path).unwrap();
        assert!(written_text.contains("@+leo-ver=cub-1-thin"));
        assert!(written_text.contains("@0 [a] @f test.py"));
        // Ordinary (non-clone, no-UA) nodes still carry no bracketed gnx.
        assert!(!written_text.contains("[b]"));
        assert!(!written_text.contains("[c]"));
        let written = RelativeFile::parse(&written_text).unwrap();
        assert_eq!(written.outline.roots[0].children.len(), 2);
        assert_eq!(
            written.outline.nodes[&written.outline.roots[0].children[1].node].headline,
            "C"
        );
        let persisted = LeoDocument::open(&outline_path).unwrap();
        assert!(persisted.outline.roots[0].children.is_empty());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn an_arrow_keeps_the_selected_headline_and_enables_cursor_editing() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId("a".into()))
            .unwrap()
            .headline = "Aβ".into();

        edit_headline(&mut app);
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        handle_headline_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE),
        );
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

        let input = app.input.as_ref().unwrap();
        assert_eq!(input.value(), "!A");
        assert_eq!(input.input.cursor(), (0, 2));
        assert!(!input.input.is_selecting());
    }

    #[test]
    fn reload_requires_confirmation_before_discarding_changes() {
        let mut app = editing_app();
        app.dirty = true;
        app.dirty_nodes.insert(NodeId("a".into()));

        reload(&mut app);

        assert!(app.dirty);
        assert!(app.reload_armed);
        assert!(app.dirty_nodes.contains(&NodeId("a".into())));
        assert!(app.status.contains("press Ctrl-R again"));
    }

    #[test]
    fn reload_replaces_the_document_and_clears_dirty_state() {
        let path = env::temp_dir().join(format!(
            "leo-cub-reload-{}-{}.leo",
            std::process::id(),
            fresh_node_id().0
        ));
        let disk_document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="a"><vh>From disk</vh></v></vnodes><tnodes><t tx="a"></t></tnodes></leo_file>"#,
        )
        .unwrap();
        disk_document.save(&path).unwrap();

        let mut app = editing_app();
        app.path.clone_from(&path);
        app.dirty = true;
        app.dirty_nodes.insert(NodeId("a".into()));
        app.reload_armed = true;
        reload(&mut app);

        assert_eq!(
            app.document.outline.nodes[&NodeId("a".into())].headline,
            "From disk"
        );
        assert!(!app.dirty);
        assert!(!app.reload_armed);
        assert!(app.dirty_nodes.is_empty());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn reload_marks_nodes_whose_body_changed_on_disk() {
        let path = env::temp_dir().join(format!(
            "leo-cub-reload-updated-{}-{}.leo",
            std::process::id(),
            fresh_node_id().0
        ));
        let disk_document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="a"><vh>A</vh></v><v t="b"><vh>B</vh></v></vnodes><tnodes><t tx="a">from disk</t><t tx="b"></t></tnodes></leo_file>"#,
        )
        .unwrap();
        disk_document.save(&path).unwrap();

        let mut app = editing_app();
        app.path.clone_from(&path);
        app.updated_nodes.insert(NodeId("stale".into()));
        reload(&mut app);

        assert!(app.updated_nodes.contains(&NodeId("a".into())));
        assert!(!app.updated_nodes.contains(&NodeId("b".into())));
        assert!(!app.updated_nodes.contains(&NodeId("stale".into())));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn reload_marks_every_ancestor_of_a_changed_descendant() {
        let path = env::temp_dir().join(format!(
            "leo-cub-reload-ancestors-{}-{}.leo",
            std::process::id(),
            fresh_node_id().0
        ));
        // Mirrors editing_app's tree (a > b, c) but with b's body changed on
        // disk, so a and b should both pick up the marker while c does not.
        let disk_document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="a"><vh>A</vh><v t="b"><vh>B</vh></v><v t="c"><vh>C</vh></v></v></vnodes><tnodes><t tx="a"></t><t tx="b">from disk</t><t tx="c"></t></tnodes></leo_file>"#,
        )
        .unwrap();
        disk_document.save(&path).unwrap();

        let mut app = editing_app();
        app.path.clone_from(&path);
        reload(&mut app);

        assert!(app.updated_nodes.contains(&NodeId("a".into())));
        assert!(app.updated_nodes.contains(&NodeId("b".into())));
        assert!(!app.updated_nodes.contains(&NodeId("c".into())));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn highlights_other_directives_without_treating_arguments_as_files() {
        let spans = headline_spans("@language rust");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].style.fg, Some(Color::Cyan));
        assert_eq!(spans[1].content, " rust");
        assert_eq!(spans[1].style.fg, None);
        assert_eq!(headline_spans("ordinary headline").len(), 1);
    }

    #[test]
    fn highlights_section_reference_markers() {
        let spans = headline_spans("  <<head contents>>");
        assert_eq!(spans.len(), 4);
        assert_eq!(spans[0].content, "  ");
        assert_eq!(spans[1].content, "<<");
        assert_eq!(spans[1].style.fg, Some(Color::Cyan));
        assert_eq!(spans[2].content, "head contents");
        assert_eq!(spans[2].style.fg, None);
        assert_eq!(spans[3].content, ">>");
        assert_eq!(spans[3].style.fg, Some(Color::Cyan));
        assert_eq!(headline_spans("<<unfinished").len(), 1);
    }

    #[test]
    fn recognizes_clean_files_for_syntax_context() {
        assert_eq!(external_filename("@clean src/main.rs"), Some("src/main.rs"));
        assert_eq!(
            external_filename("@clean \"src/main.rs\""),
            Some("src/main.rs")
        );
    }

    #[test]
    fn inherits_syntax_context_from_clean_file_ancestor() {
        let document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="a"><vh>@clean src/main.rs</vh><v t="b"><vh>child</vh></v></v></vnodes><tnodes><t tx="a">@language rust</t><t tx="b">fn child() {}</t></tnodes></leo_file>"#,
        )
        .unwrap();
        let (language, path) = syntax_context(&document.outline, &PositionId("0/0".into()));
        assert_eq!(language.as_deref(), Some("rust"));
        assert_eq!(path.as_deref(), Some(Path::new("src/main.rs")));
    }

    #[test]
    fn inherits_restructured_text_from_rst_ancestor() {
        let document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="a"><vh>@rst foo.html</vh><v t="b"><vh>child</vh></v></v></vnodes><tnodes><t tx="a"></t><t tx="b">**strong**</t></tnodes></leo_file>"#,
        )
        .unwrap();
        let (language, path) = syntax_context(&document.outline, &PositionId("0/0".into()));
        assert_eq!(language.as_deref(), Some("rst"));
        assert_eq!(path, None);

        assert!(is_rst_headline("@rst foo.html"));
        assert!(is_rst_headline("@rst"));
        assert!(!is_rst_headline("@rst3 foo.html"));
    }

    #[test]
    fn extracts_leo_path_directives() {
        assert_eq!(
            path_directive("@language rust\n@path <src/core>\n"),
            Some("src/core".into())
        );
    }

    #[test]
    fn moves_nodes_with_leo_control_arrow_semantics() {
        let mut app = editing_app();
        select_position(&mut app, &PositionId("0/1".into()));
        move_selected(&mut app, MoveDirection::Up);
        assert_eq!(
            app.document.outline.roots[0].children[0].node,
            NodeId::from("c")
        );

        move_selected(&mut app, MoveDirection::Down);
        move_selected(&mut app, MoveDirection::Right);
        assert_eq!(
            app.document.outline.roots[0].children[0].children[0].node,
            NodeId::from("c")
        );

        move_selected(&mut app, MoveDirection::Left);
        assert_eq!(
            app.document.outline.roots[0].children[1].node,
            NodeId::from("c")
        );
    }

    #[test]
    fn promoting_a_node_right_still_auto_expands_its_new_parent() {
        // The new-parent auto-expand used to be a direct app.expanded.insert
        // performed mid-move; restoring the pre-move expanded snapshot
        // afterward (to survive index shifts elsewhere) silently discarded
        // it. The auto-expand must be tracked through the same node-identity
        // snapshot/restore path so it isn't clobbered.
        let mut app = editing_app();
        select_position(&mut app, &PositionId("0/1".into())); // "c"
        move_selected(&mut app, MoveDirection::Right);

        assert_eq!(
            app.document.outline.roots[0].children[0].node,
            NodeId::from("b")
        );
        assert!(
            app.expanded.contains(&PositionId("0/0".into())),
            "b should be auto-expanded to reveal its newly promoted child c"
        );
        assert_eq!(app.rows().len(), 3, "c should be visible under b");
    }

    #[test]
    fn moves_an_auto_container_but_not_its_derived_descendants() {
        let document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="a"><vh>A</vh><v t="b"><vh>B</vh></v><v t="c"><vh>C</vh></v></v><v t="d"><vh>D</vh></v></vnodes><tnodes><t tx="a"></t><t tx="b"></t><t tx="c"></t><t tx="d"></t></tnodes></leo_file>"#,
        )
        .unwrap();
        let mut app = App::new(
            document,
            PathBuf::from("test.leo"),
            String::new(),
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
            OriginalExternalState::default(),
            false,
        );
        app.derived_nodes.insert(NodeId::from("b"));

        // "a" is an authored container whose child "b" is a read-only
        // derived node; the container itself should still be reorderable
        // as a whole subtree, carrying "b" along with it.
        select_position(&mut app, &PositionId("0".into()));
        move_selected(&mut app, MoveDirection::Down);
        assert_eq!(app.document.outline.roots[0].node, NodeId::from("d"));
        assert_eq!(app.document.outline.roots[1].node, NodeId::from("a"));
        assert_eq!(
            app.document.outline.roots[1].children[0].node,
            NodeId::from("b")
        );

        // The derived descendant itself still cannot be moved directly.
        select_position(&mut app, &PositionId("1/0".into()));
        move_selected(&mut app, MoveDirection::Down);
        assert_eq!(
            app.document.outline.roots[1].children[0].node,
            NodeId::from("b")
        );
        assert_eq!(app.status, "@auto subtrees cannot be moved");
    }

    #[test]
    fn demote_refuses_to_nest_a_normal_node_under_a_readonly_derived_sibling() {
        // Demoting only checks that the *selected* node is editable, not
        // that the previous sibling it's about to become a child of is
        // safe to nest under -- a read-only derived node (like an
        // @auto-dir's synthetic @path descendants) regenerates its entire
        // child list from scratch on every load, so anything nested under
        // it here would silently vanish on the next reload.
        let document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="p"><vh>P</vh><v t="b"><vh>B</vh></v><v t="c"><vh>C</vh></v></v></vnodes><tnodes><t tx="p"></t><t tx="b"></t><t tx="c"></t></tnodes></leo_file>"#,
        )
        .unwrap();
        let mut app = App::new(
            document,
            PathBuf::from("test.leo"),
            String::new(),
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
            OriginalExternalState::default(),
            false,
        );
        app.derived_nodes.insert(NodeId::from("b"));

        select_position(&mut app, &PositionId("0/1".into()));
        move_selected(&mut app, MoveDirection::Right);

        assert_eq!(
            app.document.outline.roots[0].children[1].node,
            NodeId::from("c")
        );
        assert!(
            app.document.outline.roots[0].children[1]
                .children
                .is_empty()
        );
        assert_eq!(app.status, "@auto/@auto-dir nodes cannot take new children");
    }

    #[test]
    fn demote_refuses_to_nest_a_normal_node_under_an_auto_dir_root() {
        // The @auto-dir root itself is deliberately excluded from
        // derived_nodes (so it can still be cut as a whole subtree), but
        // that must not make it look like a safe place to demote a normal
        // node into -- its children are regenerated from scratch too.
        let document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="p"><vh>P</vh><v t="dir"><vh>@auto-dir *.rs</vh></v><v t="e"><vh>E</vh></v></v></vnodes><tnodes><t tx="p"></t><t tx="dir"></t><t tx="e"></t></tnodes></leo_file>"#,
        )
        .unwrap();
        let mut app = App::new(
            document,
            PathBuf::from("test.leo"),
            String::new(),
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
            OriginalExternalState::default(),
            false,
        );

        select_position(&mut app, &PositionId("0/1".into()));
        move_selected(&mut app, MoveDirection::Right);

        assert_eq!(
            app.document.outline.roots[0].children[1].node,
            NodeId::from("e")
        );
        assert!(
            app.document.outline.roots[0].children[1]
                .children
                .is_empty()
        );
        assert_eq!(app.status, "@auto/@auto-dir nodes cannot take new children");
    }

    #[test]
    fn block_demote_also_refuses_a_readonly_derived_new_parent() {
        let document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="p"><vh>P</vh><v t="b"><vh>B</vh></v><v t="c"><vh>C</vh></v><v t="d"><vh>D</vh></v></v></vnodes><tnodes><t tx="p"></t><t tx="b"></t><t tx="c"></t><t tx="d"></t></tnodes></leo_file>"#,
        )
        .unwrap();
        let mut app = App::new(
            document,
            PathBuf::from("test.leo"),
            String::new(),
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
            OriginalExternalState::default(),
            false,
        );
        app.derived_nodes.insert(NodeId::from("b"));

        select_position(&mut app, &PositionId("0/1".into()));
        let anchor = app.selected;
        select_position(&mut app, &PositionId("0/2".into()));
        app.selection_anchor = Some(anchor);
        move_selected(&mut app, MoveDirection::Right);

        assert_eq!(
            app.document.outline.roots[0].children[1].node,
            NodeId::from("c")
        );
        assert_eq!(
            app.document.outline.roots[0].children[2].node,
            NodeId::from("d")
        );
        assert!(
            app.document.outline.roots[0].children[1]
                .children
                .is_empty()
        );
        assert_eq!(app.status, "@auto/@auto-dir nodes cannot take new children");
    }

    #[test]
    fn clicking_the_expand_marker_toggles_without_touching_other_columns() {
        let mut app = editing_app();
        assert_eq!(app.rows().len(), 3, "root starts expanded with 2 children");
        let area = Rect::new(0, 0, 80, 24);
        let click_on_marker = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };

        handle_mouse(&mut app, area, click_on_marker);
        assert_eq!(app.selected, 0);
        assert!(!app.expanded.contains(&PositionId("0".into())));
        assert_eq!(app.rows().len(), 1, "collapsing hides the children");

        handle_mouse(&mut app, area, click_on_marker);
        assert!(app.expanded.contains(&PositionId("0".into())));
        assert_eq!(app.rows().len(), 3, "clicking again re-expands");

        // Clicking elsewhere on the same row only selects it.
        app.selected = 0;
        let click_on_headline = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 2,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse(&mut app, area, click_on_headline);
        assert_eq!(app.selected, 1);
        assert!(app.expanded.contains(&PositionId("0".into())));
        assert_eq!(app.rows().len(), 3);
    }

    #[test]
    fn dragging_in_the_outline_extends_a_multi_row_selection() {
        let mut app = editing_app();
        assert_eq!(app.rows().len(), 3, "root starts expanded with 2 children");
        let area = Rect::new(0, 0, 80, 24);
        let press_first_row = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse(&mut app, area, press_first_row);
        assert_eq!(app.selected, 0);
        assert_eq!(app.selection_anchor, None);

        let drag_to_third_row = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 10,
            row: 3,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse(&mut app, area, drag_to_third_row);
        assert_eq!(
            app.selection_anchor,
            Some(0),
            "anchor stays pinned at the press row"
        );
        assert_eq!(app.selected, 2, "selection follows the drag");

        let release = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 10,
            row: 3,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse(&mut app, area, release);
        assert_eq!(app.selection_anchor, Some(0), "release doesn't collapse it");
        assert_eq!(app.selected, 2);
        assert_eq!(app.status, "copied 3 headlines to clipboard");
    }

    #[test]
    fn a_plain_click_in_the_outline_does_not_copy_anything() {
        let mut app = editing_app();
        app.status = "untouched".into();
        let area = Rect::new(0, 0, 80, 24);
        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 2,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse(&mut app, area, click);
        let release = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            ..click
        };
        handle_mouse(&mut app, area, release);
        assert_eq!(
            app.status, "untouched",
            "a plain click (no drag) shouldn't touch the clipboard"
        );
    }

    #[test]
    fn a_click_with_a_spurious_jittery_drag_still_just_selects_the_clicked_row() {
        let mut app = editing_app();
        assert_eq!(app.rows().len(), 3, "root starts expanded with 2 children");
        app.status = "untouched".into();
        let area = Rect::new(0, 0, 80, 24);
        let press = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse(&mut app, area, press);
        assert_eq!(app.selected, 0);

        // Some terminals report a one-row Drag for what's really just a
        // stationary click.
        let jittered_drag = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 10,
            row: 2,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse(&mut app, area, jittered_drag);
        assert_eq!(app.selection_anchor, Some(0));
        assert_eq!(app.selected, 1, "the spurious drag moves it mid-gesture");

        // But the release lands back on the row that was actually clicked.
        let release = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            ..press
        };
        handle_mouse(&mut app, area, release);
        assert_eq!(
            app.selected, 0,
            "release should correct the selection back to where the mouse actually let go"
        );
        assert_eq!(app.selection_anchor, None);
        assert_eq!(
            app.status, "untouched",
            "a click that jitters back to its start row shouldn't copy anything"
        );
    }

    #[test]
    fn mouse_wheel_over_the_outline_moves_the_selection() {
        let mut app = editing_app();
        assert_eq!(app.rows().len(), 3, "root starts expanded with 2 children");
        let area = Rect::new(0, 0, 80, 24);
        let scroll_down = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 10,
            row: 2,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse(&mut app, area, scroll_down);
        assert_eq!(app.selected, 2, "clamped at the last row");

        let scroll_up = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            ..scroll_down
        };
        handle_mouse(&mut app, area, scroll_up);
        assert_eq!(app.selected, 0, "clamped at the first row");
    }

    #[test]
    fn mouse_wheel_over_the_body_scrolls_it() {
        let mut app = editing_app();
        app.split_horizontal = false;
        app.body_scroll_max = 10;
        let area = Rect::new(0, 0, 80, 24);
        let scroll_down = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 50,
            row: 2,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse(&mut app, area, scroll_down);
        assert_eq!(app.body_scroll, 3);
        assert_eq!(
            app.selected, 0,
            "scrolling the body doesn't move the outline selection"
        );

        let scroll_up = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            ..scroll_down
        };
        handle_mouse(&mut app, area, scroll_up);
        assert_eq!(app.body_scroll, 0);
    }

    #[test]
    fn copied_headlines_keep_indentation_relative_to_the_shallowest_row() {
        let document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="a"><vh>A</vh><v t="b"><vh>B</vh><v t="d"><vh>D</vh></v></v><v t="c"><vh>C</vh></v></v></vnodes><tnodes><t tx="a"></t><t tx="b"></t><t tx="c"></t><t tx="d"></t></tnodes></leo_file>"#,
        )
        .unwrap();
        let mut app = App::new(
            document,
            PathBuf::from("test.leo"),
            String::new(),
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
            OriginalExternalState::default(),
            false,
        );
        app.expanded.insert(PositionId("0/0".into())); // expand B to reveal D
        let rows = app.rows();
        assert_eq!(rows.len(), 4, "A, B, D, C");

        // Select B..=C (depths 1, 2, 1): D should stay indented one level
        // deeper than its siblings B and C, not absolute-zero-based.
        let text = outline_selection_text(&app, &rows[1..=3]);
        assert_eq!(text, "B\n  D\nC");
    }

    #[test]
    fn dragging_in_the_body_selects_text_and_copies_it_on_release() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .body = "hello world\nsecond line".into();
        app.selected = 0;
        // Standalone body_area, independent of layout percentages: a
        // bordered block whose content (post-inset) starts at (41, 1).
        let body_area = Rect::new(40, 0, 40, 20);

        handle_body_mouse(
            &mut app,
            body_area,
            MouseEventKind::Down(MouseButton::Left),
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 47, // node_area.x(41) + col 6, just after "hello "
                row: 1,     // node_area.y(1) + line 0
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(
            app.body_selection,
            Some(BodySelection {
                anchor: (0, 6),
                cursor: (0, 6),
            })
        );

        handle_body_mouse(
            &mut app,
            body_area,
            MouseEventKind::Drag(MouseButton::Left),
            MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Left),
                column: 47, // col 6 again, just after "second"
                row: 2,     // line 1
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(
            app.body_selection,
            Some(BodySelection {
                anchor: (0, 6),
                cursor: (1, 6),
            })
        );

        handle_body_mouse(
            &mut app,
            body_area,
            MouseEventKind::Up(MouseButton::Left),
            MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: 47,
                row: 2,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(app.status, "copied 12 selected characters to clipboard");
        // The highlight stays visible after copying, like the search match
        // highlight does, until something else changes the selected row.
        assert!(app.body_selection.is_some());
    }

    #[test]
    fn dragging_in_the_log_view_selects_text_and_copies_it_on_release() {
        // Regression test: mouse events used to be routed through the
        // outline/body hit-testing even while the log view's full-screen
        // overlay was open, so a drag there never selected or copied
        // anything -- see `handle_log_mouse`.
        let mut app = editing_app();
        app.log_view = true;
        app.logs.push_back("hello world".into());
        app.logs.push_back("second line".into());
        // A full-terminal area, matching what `event_loop` passes in --
        // `log_view_layout` insets it by the log panel's 1-cell border, so
        // content starts at (1, 1).
        let area = Rect::new(0, 0, 80, 24);

        handle_log_mouse(
            &mut app,
            area,
            MouseEventKind::Down(MouseButton::Left),
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 7, // log_area.x(1) + col 6, just after "hello "
                row: 1,    // log_area.y(1) + line 0
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(
            app.log_selection,
            Some(BodySelection {
                anchor: (0, 6),
                cursor: (0, 6),
            })
        );

        handle_log_mouse(
            &mut app,
            area,
            MouseEventKind::Drag(MouseButton::Left),
            MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Left),
                column: 7, // col 6 again, just after "second"
                row: 2,    // line 1
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(
            app.log_selection,
            Some(BodySelection {
                anchor: (0, 6),
                cursor: (1, 6),
            })
        );

        handle_log_mouse(
            &mut app,
            area,
            MouseEventKind::Up(MouseButton::Left),
            MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: 7,
                row: 2,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(app.status, "copied 12 selected characters to clipboard");
    }

    #[test]
    fn a_plain_click_in_the_body_does_not_copy_anything() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .body = "hello world".into();
        app.selected = 0;
        app.status = "untouched".into();
        let body_area = Rect::new(40, 0, 40, 20);
        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 45,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        handle_body_mouse(&mut app, body_area, click.kind, click);
        let release = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            ..click
        };
        handle_body_mouse(&mut app, body_area, release.kind, release);
        assert_eq!(
            app.status, "untouched",
            "a zero-width selection copies nothing"
        );
    }

    #[test]
    fn mouse_selection_is_disabled_while_the_body_is_word_wrapped() {
        // body_scroll counts *visual* rows once wrap is on, so a screen
        // row no longer maps onto a single logical line/column -- rather
        // than select the wrong text, pressing should refuse and say why.
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .body = "hello world\nsecond line".into();
        app.selected = 0;
        app.body_wrap = true;
        let body_area = Rect::new(40, 0, 40, 20);

        let down = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 47,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        handle_body_mouse(&mut app, body_area, down.kind, down);
        assert_eq!(app.body_selection, None);
        assert_eq!(
            app.status,
            "mouse text selection needs word-wrap off (press W)"
        );
    }

    #[test]
    fn finds_headlines_incrementally_and_reveals_collapsed_matches() {
        let mut app = editing_app();
        app.expanded.clear();
        start_find(&mut app);
        handle_find_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
        );

        assert_eq!(
            app.selected_row().unwrap().position,
            PositionId("0/1".into())
        );
        assert!(app.expanded.contains(&PositionId("0".into())));
        assert_eq!(app.find.as_ref().unwrap().matches.len(), 1);
    }

    #[test]
    fn scrolls_body_by_a_page_and_stays_in_bounds() {
        let mut app = editing_app();
        app.body_page_size = 10;
        app.body_scroll_max = 24;

        app.scroll_body(1);
        assert_eq!(app.body_scroll, 10);
        app.scroll_body(2);
        assert_eq!(app.body_scroll, 24);
        app.scroll_body(-1);
        assert_eq!(app.body_scroll, 14);
        app.scroll_body(-2);
        assert_eq!(app.body_scroll, 0);
    }

    #[test]
    fn scrolls_body_by_lines_and_stays_in_bounds() {
        let mut app = editing_app();
        app.body_scroll_max = 3;

        app.scroll_body_lines(1);
        assert_eq!(app.body_scroll, 1);
        app.scroll_body_lines(10);
        assert_eq!(app.body_scroll, 3);
        app.scroll_body_lines(-1);
        assert_eq!(app.body_scroll, 2);
        app.scroll_body_lines(-10);
        assert_eq!(app.body_scroll, 0);
    }

    #[test]
    fn scrolls_body_horizontally_and_stays_in_bounds() {
        let mut app = editing_app();
        app.body_horizontal_scroll_max = 10;

        app.scroll_body_horizontal(4);
        assert_eq!(app.body_horizontal_scroll, 4);
        app.scroll_body_horizontal(20);
        assert_eq!(app.body_horizontal_scroll, 10);
        app.scroll_body_horizontal(-4);
        assert_eq!(app.body_horizontal_scroll, 6);
        app.scroll_body_horizontal(-20);
        assert_eq!(app.body_horizontal_scroll, 0);
    }

    #[test]
    fn toggling_word_wrap_resets_scrolling_and_updates_status() {
        let mut app = editing_app();
        app.body_scroll = 3;
        app.body_horizontal_scroll = 7;

        app.toggle_body_wrap();

        assert!(app.body_wrap);
        assert_eq!(app.body_scroll, 0);
        assert_eq!(app.body_horizontal_scroll, 0);
        assert_eq!(app.status, "word wrap enabled");
        assert_eq!(node_title(app.body_wrap), " Node [wrap] ");

        app.body_horizontal_scroll_max = 10;
        app.scroll_body_horizontal(4);
        assert_eq!(app.body_horizontal_scroll, 0);

        app.toggle_body_wrap();
        assert!(!app.body_wrap);
        assert_eq!(app.status, "word wrap disabled");
        assert_eq!(node_title(app.body_wrap), " Node ");
    }

    #[cfg(feature = "syntax")]
    #[test]
    fn toggling_preview_forces_wrap_on_and_restores_it_afterward() {
        let mut app = editing_app();
        assert!(!app.body_wrap);
        app.body_scroll = 3;
        app.body_horizontal_scroll = 7;

        app.toggle_preview();

        assert!(app.preview_enabled);
        assert!(app.body_wrap);
        assert_eq!(app.body_scroll, 0);
        assert_eq!(app.body_horizontal_scroll, 0);
        assert_eq!(app.status, "rendered preview on");

        app.toggle_preview();

        assert!(!app.preview_enabled);
        assert!(!app.body_wrap);
        assert_eq!(app.status, "rendered preview off");
    }

    #[cfg(feature = "syntax")]
    #[test]
    fn toggling_preview_does_not_clobber_an_explicit_wrap_preference() {
        let mut app = editing_app();
        app.toggle_body_wrap();
        assert!(app.body_wrap);

        app.toggle_preview();
        assert!(app.body_wrap);
        app.toggle_preview();

        assert!(app.body_wrap, "wrap was on before preview, should stay on");
    }

    #[cfg(feature = "syntax")]
    #[test]
    fn wrap_preference_is_remembered_per_language_with_a_separate_default_bucket() {
        let document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="a"><vh>rust body</vh></v><v t="b"><vh>plain body</vh></v></vnodes><tnodes><t tx="a">@language rust
fn main() {}</t><t tx="b">just notes</t></tnodes></leo_file>"#,
        )
        .unwrap();
        let mut app = App::new(
            document,
            PathBuf::from("test.leo"),
            String::new(),
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
            OriginalExternalState::default(),
            false,
        );

        app.selected = 0;
        assert!(!app.wrap_for(app.selected_position().as_ref()));
        app.toggle_body_wrap();
        assert!(app.wrap_for(app.selected_position().as_ref()));

        app.selected = 1;
        assert!(
            !app.wrap_for(app.selected_position().as_ref()),
            "the default bucket must not pick up the rust node's wrap state"
        );
        app.toggle_body_wrap();
        assert!(app.wrap_for(app.selected_position().as_ref()));

        app.selected = 0;
        assert!(
            app.wrap_for(app.selected_position().as_ref()),
            "the rust node's wrap state must not be clobbered by the default bucket"
        );
    }

    #[test]
    fn changing_selection_resets_body_scroll() {
        let mut app = editing_app();
        app.body_scroll = 12;

        app.move_selection(1);

        assert_eq!(app.selected, 1);
        assert_eq!(app.body_scroll, 0);
    }

    #[test]
    fn find_cycles_matches_and_escape_restores_selection() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .headline = "Alpha".into();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("b"))
            .unwrap()
            .headline = "Beta".into();
        start_find(&mut app);
        handle_find_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::SHIFT),
        );
        assert_eq!(app.find.as_ref().unwrap().matches.len(), 2);
        handle_find_input(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(
            app.selected_row().unwrap().position,
            PositionId("0/0".into())
        );

        handle_find_input(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.find.is_none());
        assert_eq!(app.selected_row().unwrap().position, PositionId("0".into()));
    }

    #[test]
    fn search_matches_body_content_and_escape_restores_selection() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("b"))
            .unwrap()
            .body = "contains needle text".into();
        start_search(&mut app);
        for character in "needle".chars() {
            handle_search_input(
                &mut app,
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
            );
        }

        assert_eq!(app.search.as_ref().unwrap().matches.len(), 1);
        assert_eq!(
            app.selected_row().unwrap().position,
            PositionId("0/0".into())
        );

        handle_search_input(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.search.is_none());
        assert_eq!(app.selected_row().unwrap().position, PositionId("0".into()));
    }

    #[test]
    fn search_scrolls_the_body_pane_to_the_matching_line() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("b"))
            .unwrap()
            .body = "one\ntwo\nneedle here\nfour".into();
        app.body_scroll = 5;

        start_search(&mut app);
        for character in "needle".chars() {
            handle_search_input(
                &mut app,
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
            );
        }

        assert_eq!(app.body_scroll, 2);
    }

    #[test]
    fn highlight_preserves_span_style_and_adds_a_background() {
        let pattern = RegexBuilder::new(&regex::escape("needle"))
            .case_insensitive(true)
            .build()
            .unwrap();
        let original_style = Style::default().fg(Color::Rgb(1, 2, 3));
        let line = Line::from(vec![Span::styled(
            "a Needle in text".to_owned(),
            original_style,
        )]);

        let highlighted = highlight_matches_in_line(line, &pattern);

        let rendered: String = highlighted
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(rendered, "a Needle in text");
        let matched = highlighted
            .spans
            .iter()
            .find(|span| span.content.as_ref() == "Needle")
            .expect("matched span present");
        assert_eq!(matched.style.fg, original_style.fg);
        assert_eq!(matched.style.bg, Some(Color::Yellow));
        let unmatched = highlighted
            .spans
            .iter()
            .find(|span| span.content.as_ref() == "a ")
            .expect("unmatched span present");
        assert_eq!(unmatched.style, original_style);
    }

    #[test]
    fn normal_paste_creates_an_independent_tree() {
        let mut app = editing_app();
        copy_selected(&mut app);
        paste_tree(&mut app, false);

        assert_eq!(app.document.outline.roots.len(), 2);
        assert_ne!(
            app.document.outline.roots[0].node,
            app.document.outline.roots[1].node
        );
        assert_ne!(
            app.document.outline.roots[0].children[0].node,
            app.document.outline.roots[1].children[0].node
        );
        assert_eq!(app.document.outline.nodes.len(), 6);
        assert!(app.dirty);
        assert_eq!(
            app.flash.as_ref().map(|(message, _)| message.as_str()),
            Some("PASTED 1 INDEPENDENT TREE(S)")
        );
    }

    #[test]
    #[cfg(feature = "syntax")]
    fn pasting_clears_stale_position_keyed_render_caches() {
        // highlight_cache/preview_cache are keyed by PositionId, a structural
        // index path, not a stable node identity. Inserting a tree shifts
        // sibling indices, so a position that previously held one node's
        // rendered content can be reassigned to a different node; without
        // invalidating the caches the new node inherits the old rendering
        // (e.g. an empty body) until the next reload.
        let mut app = editing_app();
        select_position(&mut app, &PositionId("0".into()));
        copy_selected(&mut app);
        app.highlight_cache
            .insert(PositionId("1".into()), Text::from("stale"));
        app.preview_cache
            .insert(PositionId("1".into()), Text::from("stale"));

        paste_tree(&mut app, false);

        assert!(app.highlight_cache.is_empty());
        assert!(app.preview_cache.is_empty());
    }

    #[test]
    fn pasting_before_a_sibling_preserves_its_expanded_state_and_source_location() {
        let document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="x"><vh>X</vh><v t="b"><vh>B</vh></v></v><v t="y"><vh>Y</vh></v></vnodes><tnodes><t tx="x"></t><t tx="b"></t><t tx="y"></t></tnodes></leo_file>"#,
        )
        .unwrap();
        let mut app = App::new(
            document,
            PathBuf::from("test.leo"),
            String::new(),
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
            OriginalExternalState::default(),
            false,
        );
        // App::new expands every root, so "y" (index 1) starts expanded.
        assert!(app.expanded.contains(&PositionId("1".into())));
        app.source_locations.insert(
            PositionId("1".into()),
            SourceLocation {
                path: PathBuf::from("y.txt"),
                line: 1,
            },
        );

        select_position(&mut app, &PositionId("0".into())); // "x"
        copy_selected(&mut app);
        paste_tree(&mut app, false); // inserts after "x", shifting "y" from index 1 to 2

        assert_eq!(app.document.outline.roots[2].node, NodeId::from("y"));
        assert!(
            app.expanded.contains(&PositionId("2".into())),
            "y's expanded state should follow it to its new position"
        );
        assert!(
            !app.expanded.contains(&PositionId("1".into())),
            "the freshly pasted node should not inherit y's old expanded state"
        );
        assert!(
            app.source_locations.is_empty(),
            "stale position-keyed source locations must not survive a structural edit"
        );
    }

    #[test]
    fn paste_as_clone_retains_node_identities() {
        let mut app = editing_app();
        copy_selected(&mut app);
        paste_tree(&mut app, true);

        assert_eq!(app.document.outline.roots.len(), 2);
        assert_eq!(app.document.outline.roots[0], app.document.outline.roots[1]);
        assert_eq!(clone_count(&app.document.outline, &NodeId::from("a")), 2);
        assert_eq!(app.document.outline.nodes.len(), 3);

        let outline = &app.document.outline;
        assert!(is_clone_root(
            outline,
            &PositionId("0".into()),
            &NodeId::from("a")
        ));
        assert!(is_clone_root(
            outline,
            &PositionId("1".into()),
            &NodeId::from("a")
        ));
        assert!(!is_clone_root(
            outline,
            &PositionId("0/0".into()),
            &NodeId::from("b")
        ));
        assert!(!is_clone_root(
            outline,
            &PositionId("1/0".into()),
            &NodeId::from("b")
        ));
    }

    #[test]
    fn cloning_an_auto_root_in_place_is_not_blocked_by_its_derived_children() {
        let mut app = editing_app();
        app.derived_nodes.insert(NodeId::from("b"));
        app.derived_nodes.insert(NodeId::from("c"));

        // Copying and immediately pasting-as-clone beside the same selected
        // row is the natural way to clone a node, including an @auto root
        // whose children are read-only derived content.
        copy_selected(&mut app);
        paste_tree(&mut app, true);

        assert_eq!(app.document.outline.roots.len(), 2);
        assert_eq!(app.document.outline.roots[0], app.document.outline.roots[1]);
        assert_eq!(app.status, "1 tree(s) pasted as clones (Ctrl-S to save)");
    }

    #[test]
    fn cloning_a_derived_descendant_directly_is_blocked() {
        let mut app = editing_app();
        app.derived_nodes.insert(NodeId::from("b"));
        app.derived_nodes.insert(NodeId::from("c"));

        app.selected = 1; // row "b", a derived descendant, not the @auto root
        copy_selected(&mut app);
        paste_tree(&mut app, true);

        assert_eq!(app.document.outline.roots.len(), 1);
        assert_eq!(
            app.status,
            "cannot clone @auto derived content; only its root can be cloned"
        );
    }

    #[test]
    fn cutting_a_derived_descendant_directly_is_blocked() {
        let mut app = editing_app();
        app.derived_nodes.insert(NodeId::from("b"));
        app.derived_nodes.insert(NodeId::from("c"));

        app.selected = 1; // row "b", a derived descendant, not the @auto root
        cut_selected(&mut app);

        assert_eq!(app.document.outline.roots.len(), 1);
        assert_eq!(app.status, "@auto subtrees cannot be cut");
    }

    #[test]
    fn cutting_the_auto_root_is_allowed_even_with_sole_derived_children() {
        let mut app = editing_app();
        app.derived_nodes.insert(NodeId::from("b"));
        app.derived_nodes.insert(NodeId::from("c"));

        // Row 0 ("a") is the @auto root; cutting it takes its read-only
        // children with it in the same action, so nothing is orphaned.
        cut_selected(&mut app);

        assert_eq!(app.document.outline.roots.len(), 0);
        assert_ne!(app.status, "@auto subtrees cannot be cut");
    }

    #[test]
    fn cutting_a_duplicate_auto_occurrence_is_allowed() {
        let mut app = editing_app();
        app.derived_nodes.insert(NodeId::from("b"));
        app.derived_nodes.insert(NodeId::from("c"));
        copy_selected(&mut app);
        paste_tree(&mut app, true);
        assert_eq!(app.document.outline.roots.len(), 2);

        app.selected = 0;
        app.selection_anchor = None;
        cut_selected(&mut app);

        assert_eq!(app.document.outline.roots.len(), 1);
        assert_ne!(app.status, "@auto subtrees cannot be cut");
        assert_eq!(app.document.outline.roots[0].node, NodeId::from("a"));
    }

    #[test]
    fn n_cycles_through_clone_occurrences_and_wraps() {
        let mut app = editing_app();
        // Clone the whole tree: "a" (and its children "b"/"c") now each
        // occur at both root 0 and root 1.
        copy_selected(&mut app);
        paste_tree(&mut app, true);
        assert_eq!(app.document.outline.roots.len(), 2);

        app.selected = 0;
        app.selection_anchor = None;
        assert_eq!(app.selected_row().unwrap().position, PositionId("0".into()));

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
            None,
        );
        assert_eq!(app.selected_row().unwrap().position, PositionId("1".into()));
        assert_eq!(app.status, "clone 2/2");

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
            None,
        );
        assert_eq!(
            app.selected_row().unwrap().position,
            PositionId("0".into()),
            "n wraps back around to the first occurrence"
        );
        assert_eq!(app.status, "clone 1/2");

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT),
            None,
        );
        assert_eq!(
            app.selected_row().unwrap().position,
            PositionId("1".into()),
            "Shift-N wraps backward to the last occurrence"
        );
        assert_eq!(app.status, "clone 2/2");
    }

    #[test]
    fn n_reports_no_other_occurrences_for_a_unique_node() {
        let mut app = editing_app();
        app.selected = 0;

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
            None,
        );

        assert_eq!(app.selected, 0);
        assert_eq!(app.status, "node has no other occurrences");
    }

    #[test]
    fn multi_selection_copies_and_cuts_sibling_trees() {
        let mut app = editing_app();
        app.selected = 1;
        app.selection_anchor = Some(1);
        app.extend_selection(1);

        let selected = selected_tree_roots(&app);
        assert_eq!(
            selected
                .iter()
                .map(|row| row.node.0.as_str())
                .collect::<Vec<_>>(),
            vec!["b", "c"]
        );
        copy_selected(&mut app);
        assert_eq!(app.clipboard.as_ref().unwrap().roots.len(), 2);

        cut_selected(&mut app);
        assert!(app.document.outline.roots[0].children.is_empty());
        assert_eq!(app.document.outline.nodes.len(), 1);
        assert!(app.selection_anchor.is_none());
    }

    #[test]
    fn copying_location_uses_the_known_source_line_and_headline() {
        let mut app = editing_app();
        app.selected = 1;
        app.source_nodes.insert(
            NodeId::from("b"),
            SourceLocation {
                path: PathBuf::from("/workspace/src/example.py"),
                line: 42,
            },
        );

        copy_location_to_clipboard(&mut app);

        assert_eq!(
            app.status,
            "copied to clipboard: /workspace/src/example.py:42: [B]"
        );
    }

    #[test]
    fn copying_location_falls_back_to_the_leo_file_and_headline_path_when_no_source_is_known() {
        let mut app = editing_app();
        app.path = PathBuf::from("/workspace/test.leo");

        copy_location_to_clipboard(&mut app);

        assert_eq!(app.status, "copied to clipboard: /workspace/test.leo: A");
    }

    #[test]
    fn copying_location_headline_path_includes_ancestors_and_escapes_slashes() {
        let mut app = editing_app();
        app.path = PathBuf::from("/workspace/test.leo");
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("b"))
            .unwrap()
            .headline = "fix a/b bug".into();
        app.selected = 1;

        copy_location_to_clipboard(&mut app);

        assert_eq!(
            app.status,
            r"copied to clipboard: /workspace/test.leo: A/fix a\/b bug"
        );
    }

    #[test]
    fn copying_location_on_a_path_node_copies_its_resolved_directory() {
        let mut app = editing_app();
        app.path = PathBuf::from("/workspace/test.leo");
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("b"))
            .unwrap()
            .headline = "@path src/core".into();
        app.selected = 1;

        copy_location_to_clipboard(&mut app);

        assert_eq!(app.status, "copied to clipboard: /workspace/src/core");
    }

    #[test]
    fn tildify_replaces_a_home_prefix_with_tilde() {
        assert_eq!(
            tildify(
                Path::new("/home/v/r/leo-rs/src/tui.rs"),
                Some(Path::new("/home/v"))
            ),
            "~/r/leo-rs/src/tui.rs"
        );
        assert_eq!(
            tildify(Path::new("/home/v"), Some(Path::new("/home/v"))),
            "~"
        );
        assert_eq!(
            tildify(Path::new("/etc/hosts"), Some(Path::new("/home/v"))),
            "/etc/hosts"
        );
        assert_eq!(tildify(Path::new("/etc/hosts"), None), "/etc/hosts");
    }

    #[test]
    fn absolutize_joins_relative_paths_onto_cwd_and_collapses_dot_segments() {
        assert_eq!(
            absolutize(Path::new("src/tui.rs"), Path::new("/home/v/r/leo-rs")),
            PathBuf::from("/home/v/r/leo-rs/src/tui.rs")
        );
        assert_eq!(
            absolutize(
                Path::new("/already/absolute/./foo/../bar.rs"),
                Path::new("/ignored")
            ),
            PathBuf::from("/already/absolute/bar.rs")
        );
    }

    #[test]
    fn control_arrows_move_a_multi_selection_as_a_block() {
        let mut app = editing_app();
        app.selected = 1;
        app.selection_anchor = Some(1);
        app.extend_selection(1);

        move_selected(&mut app, MoveDirection::Left);
        assert_eq!(
            app.document
                .outline
                .roots
                .iter()
                .map(|position| position.node.0.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
        assert_eq!(selected_tree_roots(&app).len(), 2);

        move_selected(&mut app, MoveDirection::Right);
        assert_eq!(app.document.outline.roots.len(), 1);
        assert_eq!(
            app.document.outline.roots[0]
                .children
                .iter()
                .map(|position| position.node.0.as_str())
                .collect::<Vec<_>>(),
            vec!["b", "c"]
        );
        assert_eq!(selected_tree_roots(&app).len(), 2);
    }

    #[test]
    fn cut_removes_the_tree_and_keeps_it_available_to_paste() {
        let mut app = editing_app();
        cut_selected(&mut app);
        assert!(app.document.outline.roots.is_empty());
        assert!(app.document.outline.nodes.is_empty());

        paste_tree(&mut app, false);
        assert_eq!(app.document.outline.roots.len(), 1);
        assert_eq!(app.document.outline.nodes.len(), 3);
    }

    #[test]
    fn toggle_mark_marks_then_unmarks_the_current_row() {
        let mut app = editing_app();
        app.selected = 1;
        let position = app.rows()[1].position.clone();

        app.toggle_mark();
        assert!(app.marked.contains(&position));

        app.toggle_mark();
        assert!(app.marked.is_empty());
    }

    #[test]
    fn clear_marks_empties_the_marked_set() {
        let mut app = editing_app();
        app.selected = 1;
        app.toggle_mark();
        assert_eq!(app.marked.len(), 1);

        app.clear_marks();
        assert!(app.marked.is_empty());
    }

    #[test]
    fn cut_prefers_the_marked_set_over_the_row_under_the_cursor() {
        let mut app = editing_app();
        let c_position = app.rows()[2].position.clone();
        app.marked.insert(c_position);
        // The cursor sits on "b", which is unmarked -- without mark
        // priority this would cut "b" instead of the marked "c".
        app.selected = 1;
        app.selection_anchor = None;

        cut_selected(&mut app);

        let remaining: Vec<_> = app
            .document
            .outline
            .nodes
            .values()
            .map(|node| node.id.0.clone())
            .collect();
        assert!(remaining.contains(&"a".into()));
        assert!(remaining.contains(&"b".into()));
        assert!(!remaining.contains(&"c".into()));
        assert!(
            app.marked.is_empty(),
            "the mark should clear once its node is cut"
        );
    }

    #[test]
    fn parse_key_covers_named_keys_modifiers_and_bare_letters() {
        assert_eq!(
            parse_key("j").unwrap(),
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty())
        );
        assert_eq!(
            parse_key("C").unwrap(),
            KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT)
        );
        assert_eq!(
            parse_key("S-c").unwrap(),
            KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT)
        );
        assert_eq!(
            parse_key("C-r").unwrap(),
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)
        );
        assert_eq!(
            parse_key("C-S-Left").unwrap(),
            KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL | KeyModifiers::SHIFT)
        );
        assert_eq!(
            parse_key("Enter").unwrap(),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
        );
        assert_eq!(
            parse_key("F5").unwrap(),
            KeyEvent::new(KeyCode::F(5), KeyModifiers::empty())
        );
        assert!(parse_key("NotAKey").is_err());
    }

    #[test]
    fn script_steps_can_type_and_assert_not_just_press_keys() {
        // A regression test written as a Vec<Step> literal: insert a node
        // via 'i', type its headline, confirm with Enter, and assert on
        // both the rendered screen and the status line -- the same flow a
        // human would drive by hand, but checked automatically.
        let mut app = editing_app();
        let steps = vec![
            Step::Key {
                key: "i".to_owned(),
            },
            Step::AssertStatus {
                text: "new headline".to_owned(),
            },
            Step::Type {
                text: "New Node Title".to_owned(),
            },
            Step::Key {
                key: "Enter".to_owned(),
            },
            Step::AssertContains {
                text: "New Node Title".to_owned(),
            },
            Step::Key {
                key: "Esc".to_owned(),
            },
        ];

        run_script(&mut app, &steps).unwrap();

        assert!(
            app.document
                .outline
                .nodes
                .values()
                .any(|node| node.headline == "New Node Title")
        );
        assert!(app.dirty);
    }

    #[test]
    fn assert_contains_step_fails_the_script_when_the_text_is_absent() {
        let mut app = editing_app();
        let steps = vec![Step::AssertContains {
            text: "text that is definitely not on screen".to_owned(),
        }];
        let error = run_script(&mut app, &steps).unwrap_err();
        assert!(format!("{error:#}").contains("expected screen to contain"));
    }
}
