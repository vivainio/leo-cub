use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    env, fs,
    fs::OpenOptions,
    io,
    io::Write,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use crossterm::{
    clipboard::CopyToClipboard,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use leo::{
    AutoFile, DerivedFile, ExternalFormat, LeoDocument, Node, NodeId, OperationBatch,
    OriginalExternalState, Outline, Position, PositionId, RelativeFile, WritableExternalFile,
    comment_delimiters, external_snapshot, format_for_directive, prepare_external_updates,
    referenced_nodes, restore_external_state, search_outline, write_external_updates,
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
    find: Option<FindInput>,
    search: Option<FindInput>,
    palette: Option<ActionPalette>,
    command_palette: Option<CommandPalette>,
    action_output: Option<ActionOutput>,
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
            find: None,
            search: None,
            palette: None,
            command_palette: None,
            action_output: None,
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
}

struct HeadlineInput {
    node: NodeId,
    value: String,
    original: String,
    cursor: usize,
    selected: bool,
    inserted_position: Option<PositionId>,
}

struct FindInput {
    query: String,
    matches: Vec<PositionId>,
    active: usize,
    original: Option<PositionId>,
}

struct ActionPalette {
    query: String,
    matches: Vec<PositionId>,
    active: usize,
}

struct CommandPalette {
    query: String,
    matches: Vec<usize>,
    active: usize,
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

fn build_app(path: PathBuf, load_derived: bool) -> Result<App> {
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
    ))
}

/// Runs `body` against a real alternate-screen terminal, guaranteeing the
/// terminal is restored to normal afterwards even if `body` fails.
fn with_real_terminal<F>(body: F) -> Result<()>
where
    F: FnOnce(&mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()>,
{
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = body(&mut terminal);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    result
}

pub fn run(path: PathBuf, load_derived: bool) -> Result<()> {
    let mut app = build_app(path, load_derived)?;
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
        if let Event::Mouse(mouse) = event {
            if app.input.is_none()
                && app.find.is_none()
                && app.search.is_none()
                && app.palette.is_none()
                && app.command_palette.is_none()
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
    if app.input.is_some() {
        handle_headline_input(app, key);
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
    if app.command_palette.is_some() {
        handle_command_palette_input(app, key);
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
        KeyCode::Up if key.modifiers == KeyModifiers::SHIFT => {
            app.selection_anchor.get_or_insert(app.selected);
            app.extend_selection(-1);
        }
        KeyCode::Down if key.modifiers == KeyModifiers::SHIFT => {
            app.selection_anchor.get_or_insert(app.selected);
            app.extend_selection(1);
        }
        KeyCode::Char('?') => app.help = true,
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
        KeyCode::Char('a') if key.modifiers.is_empty() => start_command_palette(app),
        KeyCode::Char('A') if key.modifiers == KeyModifiers::SHIFT => start_palette(app),
        KeyCode::Char('i') if key.modifiers.is_empty() => insert_headline(app),
        KeyCode::Char('h') if key.modifiers.is_empty() => edit_headline(app),
        #[cfg(feature = "syntax")]
        KeyCode::Char('y') => {
            app.syntax_enabled = !app.syntax_enabled;
            app.status = format!(
                "syntax highlighting {}",
                if app.syntax_enabled { "on" } else { "off" }
            );
        }
        #[cfg(feature = "syntax")]
        KeyCode::Char('m') => app.toggle_preview(),
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
    if !matches!(
        kind,
        MouseEventKind::Down(MouseButton::Left)
            | MouseEventKind::Drag(MouseButton::Left)
            | MouseEventKind::Up(MouseButton::Left)
    ) {
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
/// when chosen from the action palette (`Shift-A`). Any node, anywhere in the
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

fn start_palette(app: &mut App) {
    app.palette = Some(ActionPalette {
        query: String::new(),
        matches: action_rows(&app.document.outline)
            .into_iter()
            .map(|row| row.position)
            .collect(),
        active: 0,
    });
    app.status = "run action: type to filter, Enter runs, Esc cancels".into();
}

fn handle_palette_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let position = app
                .palette
                .as_ref()
                .and_then(|palette| palette.matches.get(palette.active).cloned());
            app.palette = None;
            if let Some(position) = position {
                // Capture the selection as it stood *before* the action node
                // takes it over below, so the env vars describe the node the
                // user meant to act on, not the action node itself.
                let target = app
                    .selected_row()
                    .map(|row| row.position)
                    .unwrap_or_else(|| position.clone());
                reveal_and_select(app, &position);
                run_action(app, &position, &target);
            } else {
                app.status = "no matching action".into();
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
    let matches = action_rows(&app.document.outline)
        .into_iter()
        .filter(|row| {
            action_name(&app.document.outline.nodes[&row.node].headline)
                .to_lowercase()
                .contains(&query)
        })
        .map(|row| row.position)
        .collect::<Vec<_>>();
    let palette = app.palette.as_mut().expect("palette input exists");
    palette.active = palette.active.min(matches.len().saturating_sub(1));
    palette.matches = matches;
}

fn cycle_palette_match(app: &mut App, delta: isize) {
    let palette = app.palette.as_mut().expect("palette input exists");
    if palette.matches.is_empty() {
        return;
    }
    let len = palette.matches.len() as isize;
    palette.active = (palette.active as isize + delta).rem_euclid(len) as usize;
}

/// General-purpose editor commands, run by name from `a`. Unlike the
/// `@action` palette (`Shift-A`), these aren't outline nodes; they're built-in
/// operations such as importing new files, chosen with `available` so the
/// list only shows commands that make sense for the current selection.
fn start_command_palette(app: &mut App) {
    app.command_palette = Some(CommandPalette {
        query: String::new(),
        matches: available_commands(app),
        active: 0,
    });
    app.status = "run command: type to filter, Enter runs, Esc cancels".into();
}

fn available_commands(app: &App) -> Vec<usize> {
    COMMANDS
        .iter()
        .enumerate()
        .filter(|(_, command)| (command.available)(app))
        .map(|(index, _)| index)
        .collect()
}

fn handle_command_palette_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let index = app
                .command_palette
                .as_ref()
                .and_then(|palette| palette.matches.get(palette.active).copied());
            app.command_palette = None;
            if let Some(index) = index {
                (COMMANDS[index].run)(app);
            } else {
                app.status = "no matching command".into();
            }
        }
        KeyCode::Esc => {
            app.command_palette = None;
            app.status = "command palette cancelled".into();
        }
        KeyCode::Backspace => {
            app.command_palette
                .as_mut()
                .expect("command palette input exists")
                .query
                .pop();
            update_command_palette_matches(app);
        }
        KeyCode::Down => cycle_command_palette_match(app, 1),
        KeyCode::Up => cycle_command_palette_match(app, -1),
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            app.command_palette
                .as_mut()
                .expect("command palette input exists")
                .query
                .push(character);
            update_command_palette_matches(app);
        }
        _ => {}
    }
}

fn update_command_palette_matches(app: &mut App) {
    let query = app
        .command_palette
        .as_ref()
        .expect("command palette input exists")
        .query
        .to_lowercase();
    let matches: Vec<usize> = available_commands(app)
        .into_iter()
        .filter(|&index| COMMANDS[index].name.to_lowercase().contains(&query))
        .collect();
    let palette = app
        .command_palette
        .as_mut()
        .expect("command palette input exists");
    palette.active = palette.active.min(matches.len().saturating_sub(1));
    palette.matches = matches;
}

fn cycle_command_palette_match(app: &mut App, delta: isize) {
    let palette = app
        .command_palette
        .as_mut()
        .expect("command palette input exists");
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

/// Maps an `@language` directive (see `syntax::language_directive`) to the
/// interpreter used to run an action's body, plus any fixed arguments
/// needed to make that interpreter read a full script from stdin (most
/// interpreters do this with no arguments at all; `nu` needs to be told to,
/// since a bare `nu` with piped stdin tries to start an interactive REPL).
/// Unrecognized or missing languages fall back to the shell, so a plain
/// script needs no directive.
fn interpreter_for(
    #[cfg_attr(not(feature = "syntax"), allow(unused_variables))] language: Option<&str>,
) -> (&'static str, &'static [&'static str]) {
    #[cfg(feature = "syntax")]
    match language.unwrap_or("sh") {
        "python" | "python3" => return ("python3", &[]),
        "javascript" | "js" | "node" => return ("node", &[]),
        "ruby" => return ("ruby", &[]),
        "bash" => return ("bash", &[]),
        "nu" | "nushell" => return ("nu", &["--stdin", "-c", "source /dev/stdin"]),
        _ => {}
    }
    ("sh", &[])
}

/// Removes `@language xxx` directive lines from a body before it is run as
/// a script: the directive picks the interpreter (see `interpreter_for`)
/// but isn't itself valid code in that language.
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

/// An `@action` body containing a bare `@apply` directive line asks for its
/// own stdout, once the script finishes, to be parsed as a `cub apply`-style
/// JSON operation batch and applied straight to the outline in memory,
/// instead of being shown as plain output text.
fn wants_apply(body: &str) -> bool {
    body.lines().any(|line| line.trim() == "@apply")
}

/// Removes bare `@apply` directive lines before the body is run as a
/// script, the same way `strip_language_directive` does for `@language`.
fn strip_apply_directive(body: &str) -> String {
    body.lines()
        .filter(|line| line.trim() != "@apply")
        .collect::<Vec<_>>()
        .join("\n")
}

/// Runs `body` as a Rhai script and returns a `(status, stdout, stderr)`
/// triple shaped like a subprocess's, so callers don't need a separate code
/// path: `print`/`debug` calls accumulate as "stdout", a script error (or
/// `Engine::eval`'s parse/runtime failure) becomes "stderr" with status `1`,
/// and a clean run is status `0`. No outline access yet -- that's a
/// follow-up; this only proves scripts can run in-process at all.
///
/// Deliberately doesn't receive the `CUB_GNX`/`CUB_HEADLINE`/etc. env vars
/// `run_action`'s subprocess path sets: those exist because a spawned
/// process has no other way to learn its target. A Rhai script runs
/// in-process, so once it gets outline access, its target's identity
/// belongs on that live object (e.g. a `target` position handle), not
/// duplicated as string env vars.
fn run_rhai_script(body: &str) -> (Option<i32>, String, String) {
    let output = Rc::new(RefCell::new(String::new()));
    let print_output = output.clone();
    let debug_output = output.clone();

    let mut engine = rhai::Engine::new();
    engine.on_print(move |s| {
        let mut output = print_output.borrow_mut();
        output.push_str(s);
        output.push('\n');
    });
    engine.on_debug(move |s, source, pos| {
        let mut output = debug_output.borrow_mut();
        match source {
            Some(source) => output.push_str(&format!("{source} @ {pos:?} | {s}\n")),
            None => output.push_str(&format!("{pos:?} | {s}\n")),
        }
    });

    match engine.eval::<rhai::Dynamic>(body) {
        Ok(_) => (Some(0), output.borrow().clone(), String::new()),
        Err(error) => (Some(1), output.borrow().clone(), error.to_string()),
    }
}

/// The GNX of the node one position above `position`, i.e. `position`'s
/// parent, or `None` when `position` is a root. Used to populate
/// `CUB_PARENT_GNX`: a script can't derive this itself since it only sees
/// what we hand it via env vars, not the outline.
fn parent_gnx(outline: &Outline, position: &PositionId) -> Option<NodeId> {
    let (parent, _) = position.0.rsplit_once('/')?;
    outline
        .position(&PositionId(parent.to_owned()))
        .map(|position| position.node.clone())
}

/// Runs the body of the `@action` node at `position` as a script and puts
/// the result in `app.action_output`, which the body pane shows in place of
/// the node's body until the selection moves to a different node.
///
/// The script's environment carries the node the user had selected when
/// they invoked the action (`target`) -- not the `@action` node itself,
/// which may live anywhere in the tree -- since a spawned process otherwise
/// has no way to know what it's meant to act on: `CUB_GNX` (the target's
/// gnx, e.g. for `insert-tree`'s `parent`), `CUB_PARENT_GNX` (unset for a
/// root), `CUB_HEADLINE`, `CUB_POSITION`, `CUB_PATH` (slash-separated
/// headline path), and `CUB_DOC` (the open `.leo` file's absolute path).
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
    let gnx = target_row.node.0.clone();
    let headline = app.document.outline.nodes[&target_row.node]
        .headline
        .clone();
    let headline_path = headline_path(app, &target_row);
    let parent_gnx = parent_gnx(&app.document.outline, &target_row.position).map(|id| id.0);
    let target_position = target_row.position.0.clone();
    let doc_path = absolutize(&app.path, &env::current_dir().unwrap_or_default())
        .to_string_lossy()
        .into_owned();
    #[cfg(feature = "syntax")]
    let language = crate::syntax::language_directive(&node.body).map(str::to_owned);
    #[cfg(not(feature = "syntax"))]
    let language: Option<String> = None;
    let apply_requested = wants_apply(&node.body);
    // The `@language`/`@apply` directives pick the interpreter and the
    // output handling but aren't themselves valid code in any language, so
    // they must not be sent to the interpreter.
    let body = strip_apply_directive(&strip_language_directive(&node.body));

    // `@language rhai` runs in-process instead of spawning a subprocess: no
    // interpreter to find on PATH, no stdin/stdout plumbing. Its `print`/
    // `debug` output stands in for stdout/stderr so the rest of this
    // function (status line, `@apply` handling, `ActionOutput`) doesn't need
    // to know the difference.
    let (interpreter, status, stdout, stderr) = if language.as_deref() == Some("rhai") {
        app.status = format!("running '{name}' with rhai...");
        let (status, stdout, stderr) = run_rhai_script(&body);
        ("rhai", status, stdout, stderr)
    } else {
        let (interpreter, interpreter_args) = interpreter_for(language.as_deref());
        app.status = format!("running '{name}' with {interpreter}...");
        // `parent()` of a bare filename like "foo.leo" is `Some("")`, and
        // `current_dir("")` fails at `chdir`, so only set a cwd when non-empty.
        let cwd = app
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf);
        let mut command = Command::new(interpreter);
        command.args(interpreter_args);
        if let Some(cwd) = &cwd {
            command.current_dir(cwd);
        }
        command
            .env("CUB_GNX", &gnx)
            .env("CUB_HEADLINE", &headline)
            .env("CUB_POSITION", &target_position)
            .env("CUB_PATH", &headline_path)
            .env("CUB_DOC", &doc_path);
        if let Some(parent_gnx) = &parent_gnx {
            command.env("CUB_PARENT_GNX", parent_gnx);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let outcome = command.spawn().and_then(move |mut child| {
            let mut stdin = child.stdin.take().expect("stdin was piped");
            let writer = std::thread::spawn(move || {
                let _ = stdin.write_all(body.as_bytes());
            });
            let output = child.wait_with_output();
            let _ = writer.join();
            output
        });

        let (status, stdout, stderr) = match outcome {
            Ok(output) => (
                output.status.code(),
                String::from_utf8_lossy(&output.stdout).into_owned(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ),
            Err(error) => (
                None,
                String::new(),
                format!("failed to run {interpreter}: {error}"),
            ),
        };
        (interpreter, status, stdout, stderr)
    };

    // `@apply` treats a successful run's stdout as a `cub apply`-style JSON
    // operation batch to apply to the outline in memory, rather than as
    // output text to display.
    let apply_summary =
        (apply_requested && status == Some(0)).then(|| {
            match serde_json::from_str::<OperationBatch>(&stdout) {
                Ok(batch) => match app.document.outline.apply(&batch) {
                    Ok(report) => {
                        app.dirty = true;
                        app.quit_armed = false;
                        #[cfg(feature = "syntax")]
                        app.highlight_cache.clear();
                        #[cfg(feature = "syntax")]
                        app.preview_cache.clear();
                        app.source_locations.clear();
                        app.selected = app.selected.min(app.rows().len().saturating_sub(1));
                        format!("applied {} operation(s) to the outline", report.applied)
                    }
                    Err(error) => format!("output parsed but could not be applied: {error}"),
                },
                Err(error) => format!("output was not a valid operation batch: {error}"),
            }
        });

    let mut text = stdout;
    if !stderr.is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&stderr);
    }

    app.status = match (status, &apply_summary) {
        (Some(0), Some(summary)) => format!("'{name}' finished; {summary}"),
        (Some(0), None) => format!("'{name}' finished"),
        (Some(code), _) => format!("'{name}' exited with status {code}"),
        (None, _) => format!("'{name}' did not complete"),
    };
    app.action_output = Some(ActionOutput {
        node: row.node,
        name,
        interpreter,
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

fn reveal_and_select(app: &mut App, position: &PositionId) {
    let components = position.0.split('/').collect::<Vec<_>>();
    for end in 1..components.len() {
        app.expanded.insert(PositionId(components[..end].join("/")));
    }
    select_position(app, position);
}

fn cancel_headline_edit(app: &mut App) {
    let input = app.input.take().expect("input exists");
    if let Some(position) = input.inserted_position {
        remove_position(&mut app.document.outline, &position);
        app.document.outline.nodes.remove(&input.node);
    } else {
        app.document
            .outline
            .nodes
            .get_mut(&input.node)
            .expect("node exists")
            .headline = input.original;
    }
}

fn handle_headline_input(app: &mut App, key: KeyEvent) {
    let Some(input) = app.input.as_mut() else {
        return;
    };
    match key.code {
        KeyCode::Enter => {
            let headline = input.value.trim().to_owned();
            let node_id = input.node.clone();
            let inserted_position = input.inserted_position.clone();
            if headline.is_empty() {
                app.status = "headline may not be empty".into();
                return;
            }
            app.document
                .outline
                .nodes
                .get_mut(&node_id)
                .expect("edited node exists")
                .headline = headline.clone();
            if let Some(row) = app.rows().iter().find(|row| row.node == node_id).cloned()
                && let Some(filename) = external_filename(&headline)
            {
                let (start_delimiter, end_delimiter) = comment_delimiters(Path::new(filename));
                let path = dynamic_source_location(app, &row)
                    .map(|location| location.path)
                    .expect("edited external node has a source path");
                app.writable_external
                    .entry(node_id.clone())
                    .and_modify(|file| {
                        file.path = path.clone();
                        file.start_delimiter = start_delimiter.to_owned();
                        file.end_delimiter = end_delimiter.to_owned();
                    })
                    .or_insert(WritableExternalFile {
                        path,
                        start_delimiter: start_delimiter.to_owned(),
                        end_delimiter: end_delimiter.to_owned(),
                        original: Outline::default(),
                        format: external_format(&headline),
                    });
            }
            app.dirty_nodes.insert(node_id);
            app.input = None;
            app.dirty = true;
            app.quit_armed = false;
            #[cfg(feature = "syntax")]
            app.highlight_cache.clear();
            #[cfg(feature = "syntax")]
            app.preview_cache.clear();
            if inserted_position.is_some() {
                insert_headline(app);
            } else {
                app.status = "headline changed (Ctrl-S to save)".into();
            }
        }
        KeyCode::Esc => {
            cancel_headline_edit(app);
            app.status = "headline edit cancelled".into();
        }
        KeyCode::Up => {
            cancel_headline_edit(app);
            app.status = "headline edit cancelled".into();
            app.move_selection(-1);
        }
        KeyCode::Down => {
            cancel_headline_edit(app);
            app.status = "headline edit cancelled".into();
            app.move_selection(1);
        }
        KeyCode::Backspace => {
            if input.selected {
                input.value.clear();
                input.cursor = 0;
                input.selected = false;
            } else if input.cursor > 0 {
                let previous = input.value[..input.cursor]
                    .char_indices()
                    .next_back()
                    .map_or(0, |(index, _)| index);
                input.value.drain(previous..input.cursor);
                input.cursor = previous;
            }
        }
        KeyCode::Delete => {
            if input.selected {
                input.value.clear();
                input.cursor = 0;
                input.selected = false;
            } else if input.cursor < input.value.len() {
                let next = input.cursor
                    + input.value[input.cursor..]
                        .chars()
                        .next()
                        .expect("cursor precedes a character")
                        .len_utf8();
                input.value.drain(input.cursor..next);
            }
        }
        KeyCode::Left => {
            if input.selected {
                input.cursor = 0;
                input.selected = false;
            } else if input.cursor > 0 {
                input.cursor = input.value[..input.cursor]
                    .char_indices()
                    .next_back()
                    .map_or(0, |(index, _)| index);
            }
        }
        KeyCode::Right => {
            if input.selected {
                input.cursor = input.value.len();
                input.selected = false;
            } else if input.cursor < input.value.len() {
                input.cursor += input.value[input.cursor..]
                    .chars()
                    .next()
                    .expect("cursor precedes a character")
                    .len_utf8();
            }
        }
        KeyCode::Home => {
            input.cursor = 0;
            input.selected = false;
        }
        KeyCode::End => {
            input.cursor = input.value.len();
            input.selected = false;
        }
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            if input.selected {
                input.value.clear();
                input.cursor = 0;
                input.selected = false;
            }
            input.value.insert(input.cursor, character);
            input.cursor += character.len_utf8();
        }
        _ => {}
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
    app.input = Some(HeadlineInput {
        node: row.node,
        value: original.clone(),
        cursor: original.len(),
        selected: true,
        original,
        inserted_position: None,
    });
    app.status = "editing headline: Enter accepts, Esc cancels".into();
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
        value: String::new(),
        original: String::new(),
        cursor: 0,
        selected: false,
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
    select_position(app, &target);
    app.status = format!("{count} tree(s) pasted (Ctrl-S to save)");
    app.flash = Some((
        format!("PASTED {count} INDEPENDENT TREE(S)"),
        Instant::now(),
    ));
}

fn selected_tree_roots(app: &App) -> Vec<Row> {
    let rows = app.selected_rows();
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
    select_position(app, &first);
    let first_index = app.selected;
    select_position(app, &last);
    app.selection_anchor = Some(first_index);
    app.status = format!("{count} nodes moved (Ctrl-S to save)");
}

fn save(app: &mut App) {
    let external_updates =
        match prepare_external_updates(&app.document.outline, &app.writable_external) {
            Ok(updates) => updates,
            Err(error) => {
                app.status = format!("save failed: {error}");
                return;
            }
        };
    let mut persisted = app.document.clone();
    restore_external_state(
        &mut persisted.outline,
        &app.original_external.children,
        &app.original_external.bodies,
        &app.original_external.nodes,
    );
    let referenced = referenced_nodes(&persisted.outline.roots);
    persisted
        .outline
        .nodes
        .retain(|id, _| referenced.contains(id));
    if let Err(error) = write_external_updates(&external_updates) {
        app.status = format!("save failed: {error}");
        return;
    }
    match persisted.save(&app.path) {
        Ok(()) => {
            for update in external_updates {
                if let Some(file) = app.writable_external.get_mut(&update.root) {
                    file.original = update.snapshot;
                }
            }
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
/// some read-only derived node beneath it (including `position` itself).
/// A duplicate occurrence (its node id also appears elsewhere, e.g. because
/// its @auto root was cloned) is safe to cut: the content survives via the
/// other occurrence.
fn cut_would_orphan_derived_content(app: &App, position: &PositionId) -> bool {
    fn walk(app: &App, position: &Position) -> bool {
        (app.readonly_derived(&position.node)
            && clone_count(&app.document.outline, &position.node) <= 1)
            || position.children.iter().any(|child| walk(app, child))
    }
    app.document
        .outline
        .position(position)
        .is_some_and(|position| walk(app, position))
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
    } else if let Some(palette) = app.command_palette.as_ref() {
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
            ));
            if let Some(input) = input {
                if input.selected {
                    spans.push(Span::styled(
                        input.value.as_str(),
                        Style::default().add_modifier(Modifier::REVERSED),
                    ));
                } else {
                    spans.extend(headline_spans(&input.value[..input.cursor]));
                    spans.push(Span::raw("▏"));
                    spans.extend(headline_spans(&input.value[input.cursor..]));
                }
            } else {
                spans.extend(headline_spans(&node.headline));
            }
            spans.push(Span::styled(
                clone,
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ));
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
        draw_palette_panel(frame, areas[1], palette, &app.document.outline);
    } else if let Some(palette) = &app.command_palette {
        draw_command_palette_panel(frame, areas[1], palette);
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

/// Renders the docked action palette (`Shift-A`), listing `@action` node names
/// rather than full headlines. Laid out the same as `draw_finder_panel`.
fn draw_palette_panel(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    state: &ActionPalette,
    outline: &Outline,
) {
    let rows = action_rows(outline);
    let shown = state
        .matches
        .len()
        .min(usize::from(area.height.saturating_sub(1)));
    let first = state.active.saturating_sub(shown.saturating_sub(1));
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
            let name = action_name(&outline.nodes[&row.node].headline);
            Line::from(format!("{marker}{name}"))
        })
        .collect();
    let count = if rows.is_empty() {
        "no @action nodes in this outline".to_owned()
    } else if state.matches.is_empty() {
        "no matches".into()
    } else {
        format!("{} of {}", state.active + 1, state.matches.len())
    };
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

/// Renders the docked command palette (`a`), listing built-in editor
/// commands (see `COMMANDS`). Laid out the same as `draw_palette_panel`.
fn draw_command_palette_panel(frame: &mut ratatui::Frame<'_>, area: Rect, state: &CommandPalette) {
    let shown = state
        .matches
        .len()
        .min(usize::from(area.height.saturating_sub(1)));
    let first = state.active.saturating_sub(shown.saturating_sub(1));
    let mut lines: Vec<Line> = state
        .matches
        .iter()
        .enumerate()
        .skip(first)
        .take(shown)
        .map(|(index, &command_index)| {
            let marker = if index == state.active { "› " } else { "  " };
            Line::from(format!("{marker}{}", COMMANDS[command_index].name))
        })
        .collect();
    let count = if state.matches.is_empty() {
        "no matching commands".to_owned()
    } else {
        format!("{} of {}", state.active + 1, state.matches.len())
    };
    lines.push(Line::from(vec![
        Span::styled("Run command: ", Style::default().fg(Color::DarkGray)),
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

fn body_marker(has_body: bool, updated: bool) -> Span<'static> {
    if updated {
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
        return "? help  arrows scroll  W wrap  c/x/v/V tree  f split  F outline  s split dir  o open/edit  Ctrl-P find  / search  a commands  Shift-A actions  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  y syntax  q quit";
        #[cfg(not(feature = "syntax"))]
        return "? help  arrows scroll  W wrap  c/x/v/V tree  f split  F outline  s split dir  o open/edit  Ctrl-P find  / search  a commands  Shift-A actions  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  q quit";
    }
    if outline_full_width {
        return "? help  arrows navigate  W wrap  c/x/v/V tree  F split view  s split dir  o open/edit  Ctrl-P find  / search  a commands  Shift-A actions  i new  h rename  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  q quit";
    }
    #[cfg(feature = "syntax")]
    return "? help  arrows navigate  PgUp/PgDn body  W wrap  c/x/v/V tree  f body  F outline  s split dir  o open/edit  Ctrl-P find  / search  a commands  Shift-A actions  i new  h rename  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  y syntax  q quit";
    #[cfg(not(feature = "syntax"))]
    "? help  arrows navigate  PgUp/PgDn body  W wrap  c/x/v/V tree  f body  F outline  s split dir  o open/edit  Ctrl-P find  / search  a commands  Shift-A actions  i new  h rename  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  q quit"
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
            Line::from("i                Insert a sibling"),
            Line::from("h                Rename the headline"),
            Line::from("Ctrl-↑↓←→        Move selected tree(s)"),
            Line::from("Ctrl-P           Find a headline"),
            Line::from("a                Command palette"),
            Line::from("Shift-A          Run an @action node"),
            Line::from("/                Search headlines and body text"),
            Line::from("Ctrl-R           Reload from disk"),
            Line::from("Ctrl-S           Save"),
            Line::from("o                Edit body, or open derived source"),
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
            Line::from("Ctrl-P           Find a headline"),
            Line::from("a                Command palette"),
            Line::from("Shift-A          Run an @action node"),
            Line::from("/                Search headlines and body text"),
            Line::from("i                Insert a sibling"),
            Line::from("h                Rename the headline"),
            Line::from("Ctrl-↑↓←→        Move selected tree(s)"),
            Line::from("Ctrl-R           Reload from disk"),
            Line::from("Ctrl-S           Save"),
            Line::from("o                Edit body, or open derived source"),
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
            Line::from("a                Command palette"),
            Line::from("Shift-A          Run an @action node"),
            Line::from("/                Search headlines and body text"),
            Line::from("i                Insert a sibling"),
            Line::from("h                Rename the headline"),
            Line::from("c                Copy selected tree"),
            Line::from("Shift-C          Copy path:line (dir for @path) to clipboard"),
            Line::from("x                Cut selected tree"),
            Line::from("v / Shift-V      Paste copy / paste clone"),
            Line::from("Ctrl-↑↓←→        Move selected tree(s)"),
            Line::from("Ctrl-R           Reload from disk"),
            Line::from("Ctrl-S           Save"),
            Line::from("o                Edit body, or open derived source"),
        ]
    };
    #[cfg(feature = "syntax")]
    lines.push(Line::from("y                Toggle syntax highlighting"));
    #[cfg(feature = "syntax")]
    lines.push(Line::from(
        "m                Toggle rendered preview (Markdown for now)",
    ));
    lines.extend([
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

#[derive(Default)]
struct LoadReport {
    loaded: usize,
    errors: Vec<String>,
    locations: HashMap<PositionId, SourceLocation>,
    node_locations: HashMap<NodeId, SourceLocation>,
    derived_nodes: HashSet<NodeId>,
    writable_external: HashMap<NodeId, WritableExternalFile>,
    original_children: HashMap<NodeId, Vec<Position>>,
    original_bodies: HashMap<NodeId, String>,
    original_nodes: HashMap<NodeId, Node>,
}

struct DerivedJob {
    position: PositionId,
    path: PathBuf,
    auto: bool,
    directive: String,
    root: NodeId,
}

#[derive(Clone)]
struct SourceLocation {
    path: PathBuf,
    line: usize,
}

fn load_derived_files(outline: &mut Outline, outline_path: &Path) -> LoadReport {
    let jobs = derived_jobs(outline, outline_path);
    load_derived_jobs(outline, jobs)
}

/// Runs a specific set of derived-file jobs against `outline`, rather than
/// every derived node in it. Used to fetch content for a handful of
/// freshly-created nodes (e.g. from `command_import_run`) without re-merging
/// -- and so silently discarding unsaved edits to -- every other derived node
/// in the document, which a full `load_derived_files` pass would do.
fn load_derived_jobs(outline: &mut Outline, jobs: Vec<DerivedJob>) -> LoadReport {
    let mut report = LoadReport::default();
    for job in jobs {
        let label = job.path.display().to_string();
        if !job.auto && !job.path.exists() {
            report.writable_external.insert(
                job.root.clone(),
                WritableExternalFile {
                    path: job.path.clone(),
                    start_delimiter: comment_delimiters(&job.path).0.to_owned(),
                    end_delimiter: comment_delimiters(&job.path).1.to_owned(),
                    original: Outline::default(),
                    format: format_for_directive(&job.directive),
                },
            );
            report.loaded += 1;
            continue;
        }
        let result = fs::read_to_string(&job.path)
            .map_err(|error| error.to_string())
            .and_then(|source| {
                let root_node = outline
                    .position(&job.position)
                    .map(|position| position.node.clone())
                    .ok_or_else(|| "derived root position disappeared".to_owned())?;
                let original_children = outline
                    .position(&job.position)
                    .map(|position| position.children.clone())
                    .unwrap_or_default();
                let original_body = outline.nodes[&root_node].body.clone();
                // Captured before merge_into prunes outline.nodes down to
                // what the freshly generated tree references: these ids
                // otherwise vanish from outline.nodes even though
                // original_children (restored just before serializing)
                // still points at them.
                let original_nodes: HashMap<NodeId, Node> = referenced_nodes(&original_children)
                    .into_iter()
                    .filter_map(|id| outline.nodes.get(&id).cloned().map(|node| (id, node)))
                    .collect();
                if job.auto {
                    let auto = AutoFile::parse_with_directive(
                        &job.path,
                        job.root.clone(),
                        &source,
                        Some(&job.directive),
                    )
                    .map_err(|error| error.to_string())?;
                    if !auto.merge_into(outline, &job.position) {
                        return Err("auto root position disappeared".to_owned());
                    }
                    report
                        .node_locations
                        .entry(auto.root.clone())
                        .or_insert(SourceLocation {
                            path: job.path.clone(),
                            line: 1,
                        });
                    for (id, line) in &auto.locations {
                        report
                            .node_locations
                            .entry(id.clone())
                            .or_insert(SourceLocation {
                                path: job.path.clone(),
                                line: *line,
                            });
                    }
                    report.derived_nodes.extend(
                        auto.outline
                            .nodes
                            .keys()
                            .filter(|id| **id != auto.root)
                            .cloned(),
                    );
                } else if job.directive == "@f" {
                    let derived =
                        RelativeFile::parse(&source).map_err(|error| error.to_string())?;
                    derived
                        .merge_into(outline, &job.position)
                        .map_err(|error| error.to_string())?;
                    let original = external_snapshot(outline, &derived.root)
                        .map(|(_, snapshot)| snapshot)
                        .ok_or_else(|| "merged external root disappeared".to_owned())?;
                    report.writable_external.insert(
                        derived.root.clone(),
                        WritableExternalFile {
                            path: job.path.clone(),
                            start_delimiter: derived.start_delimiter.clone(),
                            end_delimiter: derived.end_delimiter.clone(),
                            original,
                            format: ExternalFormat::Relative,
                        },
                    );
                    for (derived_position, line) in &derived.locations {
                        let suffix = derived_position
                            .0
                            .strip_prefix("0")
                            .unwrap_or(&derived_position.0);
                        let position = PositionId(format!("{}{}", job.position.0, suffix));
                        report.locations.insert(
                            position,
                            SourceLocation {
                                path: job.path.clone(),
                                line: *line,
                            },
                        );
                        if let Some(position) = derived.outline.position(derived_position) {
                            report
                                .node_locations
                                .entry(position.node.clone())
                                .or_insert(SourceLocation {
                                    path: job.path.clone(),
                                    line: *line,
                                });
                        }
                    }
                    report.derived_nodes.extend(
                        derived
                            .outline
                            .nodes
                            .keys()
                            .filter(|id| **id != derived.root)
                            .cloned(),
                    );
                } else {
                    let derived = DerivedFile::parse(&source).map_err(|error| error.to_string())?;
                    derived
                        .merge_into(outline, &job.position)
                        .map_err(|error| error.to_string())?;
                    let original = external_snapshot(outline, &derived.root)
                        .map(|(_, snapshot)| snapshot)
                        .ok_or_else(|| "merged external root disappeared".to_owned())?;
                    report.writable_external.insert(
                        derived.root.clone(),
                        WritableExternalFile {
                            path: job.path.clone(),
                            start_delimiter: derived.start_delimiter.clone(),
                            end_delimiter: derived.end_delimiter.clone(),
                            original,
                            format: ExternalFormat::Thin,
                        },
                    );
                    for (derived_position, line) in &derived.locations {
                        let suffix = derived_position
                            .0
                            .strip_prefix("0")
                            .unwrap_or(&derived_position.0);
                        let position = PositionId(format!("{}{}", job.position.0, suffix));
                        report.locations.insert(
                            position,
                            SourceLocation {
                                path: job.path.clone(),
                                line: *line,
                            },
                        );
                        if let Some(position) = derived.outline.position(derived_position) {
                            report
                                .node_locations
                                .entry(position.node.clone())
                                .or_insert(SourceLocation {
                                    path: job.path.clone(),
                                    line: *line,
                                });
                        }
                    }
                    report.derived_nodes.extend(
                        derived
                            .outline
                            .nodes
                            .keys()
                            .filter(|id| **id != derived.root)
                            .cloned(),
                    );
                }
                report
                    .original_children
                    .entry(root_node)
                    .or_insert(original_children);
                report
                    .original_bodies
                    .entry(job.root.clone())
                    .or_insert(original_body);
                for (id, node) in original_nodes {
                    report.original_nodes.entry(id).or_insert(node);
                }
                Ok(())
            });
        match result {
            Ok(()) => report.loaded += 1,
            Err(error) => report.errors.push(format!("{label}: {error}")),
        }
    }
    report
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

fn derived_jobs(outline: &Outline, outline_path: &Path) -> Vec<DerivedJob> {
    fn visit(
        outline: &Outline,
        positions: &[Position],
        parent_id: &str,
        base: &Path,
        inherited_paths: &[String],
        jobs: &mut Vec<DerivedJob>,
    ) {
        for (index, position) in positions.iter().enumerate() {
            let position_id = if parent_id.is_empty() {
                index.to_string()
            } else {
                format!("{parent_id}/{index}")
            };
            let node = &outline.nodes[&position.node];
            let mut paths = inherited_paths.to_vec();
            if let Some(path) =
                path_directive(&node.headline).or_else(|| path_directive(&node.body))
            {
                paths.push(path);
            }
            if let Some((auto, directive, filename)) = derived_filename(&node.headline) {
                let mut path = base.to_path_buf();
                for component in inherited_paths {
                    path.push(component);
                }
                path.push(filename);
                jobs.push(DerivedJob {
                    position: PositionId(position_id.clone()),
                    path,
                    auto,
                    directive: directive.to_owned(),
                    root: position.node.clone(),
                });
            }
            visit(
                outline,
                &position.children,
                &position_id,
                base,
                &paths,
                jobs,
            );
        }
    }
    let base = outline_path.parent().unwrap_or_else(|| Path::new("."));
    let mut jobs = Vec::new();
    visit(outline, &outline.roots, "", base, &[], &mut jobs);
    jobs
}

fn derived_filename(headline: &str) -> Option<(bool, &str, &str)> {
    let (directive, filename) = headline.trim().split_once(char::is_whitespace)?;
    matches!(
        directive,
        "@file" | "@thin" | "@file-thin" | "@f" | "@auto" | "@auto-md" | "@auto-markdown"
    )
    .then(|| {
        (
            directive.starts_with("@auto"),
            directive,
            strip_path_cruft(filename),
        )
    })
    .filter(|(_, _, filename)| !filename.is_empty())
}

/// Which sentinel writer/parser a directive's derived file uses. `@f` is the
/// only directive using the cub-1-thin relative-depth, optional-gnx grammar
/// (a leo-cub extension inspired by leo-editor issue #4928, not an official
/// Leo version tag); every other thin/file directive still uses the 5-thin
/// grammar in `derived.rs`.
fn external_format(headline: &str) -> ExternalFormat {
    match headline.trim().split_once(char::is_whitespace) {
        Some((directive, _)) => format_for_directive(directive),
        None => ExternalFormat::Thin,
    }
}

#[cfg(test)]
fn thin_filename(headline: &str) -> Option<&str> {
    derived_filename(headline).and_then(|(auto, _, filename)| (!auto).then_some(filename))
}

fn external_filename(headline: &str) -> Option<&str> {
    let (directive, filename) = headline.trim().split_once(char::is_whitespace)?;
    matches!(
        directive,
        "@file"
            | "@thin"
            | "@file-thin"
            | "@f"
            | "@clean"
            | "@auto"
            | "@auto-md"
            | "@auto-markdown"
    )
    .then(|| strip_path_cruft(filename))
    .filter(|filename| !filename.is_empty())
}

fn dynamic_source_location(app: &App, row: &Row) -> Option<SourceLocation> {
    let filename = external_filename(&app.document.outline.nodes[&row.node].headline)?;
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
        if let Some(directory) =
            path_directive(&node.headline).or_else(|| path_directive(&node.body))
        {
            path.push(directory);
        }
    }
    path.push(filename);
    Some(SourceLocation { path, line: 1 })
}

fn path_directive(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        line.strip_prefix("@path")
            .and_then(|rest| rest.starts_with(char::is_whitespace).then_some(rest))
            .map(strip_path_cruft)
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
    })
}

fn strip_path_cruft(path: &str) -> &str {
    let path = path.trim();
    if path.len() > 2 {
        let pair = (path.as_bytes()[0], path.as_bytes()[path.len() - 1]);
        if matches!(pair, (b'<', b'>') | (b'"', b'"') | (b'\'', b'\'')) {
            return path[1..path.len() - 1].trim();
        }
    }
    path
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

    #[cfg(feature = "syntax")]
    #[test]
    fn maps_language_directives_to_interpreters() {
        assert_eq!(interpreter_for(None), ("sh", [].as_slice()));
        assert_eq!(interpreter_for(Some("bash")), ("bash", [].as_slice()));
        assert_eq!(interpreter_for(Some("python")), ("python3", [].as_slice()));
        assert_eq!(
            interpreter_for(Some("nu")),
            ("nu", ["--stdin", "-c", "source /dev/stdin"].as_slice())
        );
        assert_eq!(
            interpreter_for(Some("nushell")),
            ("nu", ["--stdin", "-c", "source /dev/stdin"].as_slice())
        );
        assert_eq!(interpreter_for(Some("cobol")), ("sh", [].as_slice()));
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
        assert_eq!(palette.matches, vec![PositionId("0".into())]);
    }

    #[test]
    fn command_palette_only_offers_import_on_a_path_node() {
        let mut app = editing_app();
        app.document
            .outline
            .nodes
            .get_mut(&NodeId::from("a"))
            .unwrap()
            .headline = "@path src".into();
        app.selected = 0;
        start_command_palette(&mut app);
        assert_eq!(app.command_palette.as_ref().unwrap().matches.len(), 1);

        let child_index = app
            .rows()
            .iter()
            .position(|row| row.node == NodeId::from("b"))
            .unwrap();
        app.selected = child_index;
        start_command_palette(&mut app);
        assert!(app.command_palette.as_ref().unwrap().matches.is_empty());
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
            node.body = "echo hello-from-action".into();
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
    fn a_rhai_action_runs_in_process_without_spawning_a_subprocess() {
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
        app.selected = 1; // row "0/0" -> node "b"

        run_action(
            &mut app,
            &PositionId("0/0".into()),
            &PositionId("0/0".into()),
        );

        let output = app.action_output.as_ref().expect("action produced output");
        assert_eq!(output.interpreter, "rhai");
        assert_eq!(output.status, Some(0));
        assert!(output.text.contains("hello from rhai"), "{:?}", output.text);
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
    fn an_action_receives_the_previously_selected_node_as_cub_env_vars() {
        // Node "b" ("@action Greet") is the action being run, but node "c"
        // ("C") is what the user had selected before invoking it -- the env
        // vars must describe "c", the target, not "b", the action itself.
        let mut app = editing_app();
        {
            let node = app
                .document
                .outline
                .nodes
                .get_mut(&NodeId::from("b"))
                .unwrap();
            node.headline = "@action Greet".into();
            node.body =
                "echo $CUB_GNX/$CUB_PARENT_GNX/$CUB_HEADLINE/$CUB_POSITION/$CUB_PATH".into();
        }

        run_action(
            &mut app,
            &PositionId("0/0".into()),
            &PositionId("0/1".into()),
        );

        let output = app.action_output.as_ref().expect("action produced output");
        assert_eq!(output.text.trim(), "c/a/C/0/1/A/C");
    }

    #[test]
    fn a_root_target_has_no_cub_parent_gnx() {
        let mut app = editing_app();
        {
            let node = app
                .document
                .outline
                .nodes
                .get_mut(&NodeId::from("b"))
                .unwrap();
            node.headline = "@action Root".into();
            node.body = "echo \"[$CUB_PARENT_GNX]\"".into();
        }

        run_action(&mut app, &PositionId("0/0".into()), &PositionId("0".into()));

        let output = app.action_output.as_ref().expect("action produced output");
        assert_eq!(output.text.trim(), "[]");
    }

    #[test]
    fn an_apply_directive_applies_the_actions_stdout_to_the_outline() {
        let mut app = editing_app();
        {
            let node = app
                .document
                .outline
                .nodes
                .get_mut(&NodeId::from("b"))
                .unwrap();
            node.headline = "@action Add child".into();
            node.body = concat!(
                "@apply\n",
                "echo '{\"operations\":[{\"op\":\"insert-tree\",",
                "\"parent-headline\":\"A\",",
                "\"tree\":{\"New thing\":{\"_body\":\"hi\"}}}]}'",
            )
            .into();
        }
        app.selected = 1; // row "0/0" -> node "b"

        run_action(
            &mut app,
            &PositionId("0/0".into()),
            &PositionId("0/0".into()),
        );

        assert!(app.dirty, "applying an operation batch should mark dirty");
        assert!(app.status.contains("applied 1 operation"), "{}", app.status);
        assert!(
            app.document
                .outline
                .nodes
                .values()
                .any(|node| node.headline == "New thing"),
            "expected a new 'New thing' node in the outline"
        );

        let output = app.action_output.as_ref().expect("action produced output");
        assert_eq!(output.status, Some(0));
    }

    #[test]
    fn an_apply_directive_reports_invalid_json_without_touching_the_outline() {
        let mut app = editing_app();
        let node_count_before = app.document.outline.nodes.len();
        {
            let node = app
                .document
                .outline
                .nodes
                .get_mut(&NodeId::from("b"))
                .unwrap();
            node.headline = "@action Broken".into();
            node.body = "@apply\necho 'not json'".into();
        }
        app.selected = 1;

        run_action(
            &mut app,
            &PositionId("0/0".into()),
            &PositionId("0/0".into()),
        );

        assert!(!app.dirty);
        assert_eq!(app.document.outline.nodes.len(), node_count_before);
        assert!(
            app.status.contains("not a valid operation batch"),
            "{}",
            app.status
        );
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
            node.body = "echo hi".into();
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
        // picked from it. The env vars must describe "c" -- reproduces a bug
        // where they described "b" (the action itself) instead.
        let mut app = editing_app();
        {
            let node = app
                .document
                .outline
                .nodes
                .get_mut(&NodeId::from("b"))
                .unwrap();
            node.headline = "@action Greet".into();
            node.body = "echo $CUB_GNX".into();
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
        let updated = body_marker(true, true);
        assert_eq!(updated.content, "↑ ");
        assert_eq!(updated.style.fg, Some(Color::LightGreen));

        let updated_without_body = body_marker(false, true);
        assert_eq!(updated_without_body.content, "↑ ");
        assert_eq!(updated_without_body.style.fg, Some(Color::LightGreen));
    }

    #[test]
    fn shows_a_subtle_dot_only_for_nodes_with_body_content() {
        let populated = body_marker(true, false);
        assert_eq!(populated.content, "· ");
        assert_eq!(populated.style.fg, Some(Color::DarkGray));

        let empty = body_marker(false, false);
        assert_eq!(empty.content, "  ");
        assert_eq!(empty.style.fg, None);
    }

    #[test]
    fn editing_a_headline_initially_selects_and_replaces_it() {
        let mut app = editing_app();

        edit_headline(&mut app);
        assert!(app.input.as_ref().unwrap().selected);
        handle_headline_input(
            &mut app,
            KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::SHIFT),
        );

        let input = app.input.as_ref().unwrap();
        assert_eq!(input.value, "Z");
        assert_eq!(input.cursor, 1);
        assert!(!input.selected);
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
        app.input.as_mut().unwrap().value = "@file test.md".into();
        handle_headline_input(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        let file = &app.writable_external[&NodeId::from("a")];
        assert_eq!(file.path, PathBuf::from("test.md"));
        assert_eq!(file.start_delimiter, "#");
        assert_eq!(file.end_delimiter, "");
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
        assert_eq!(input.value, "");

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
        assert_eq!(input.value, "!A");
        assert_eq!(input.cursor, 2);
        assert!(!input.selected);
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
    fn cutting_the_sole_auto_occurrence_is_blocked() {
        let mut app = editing_app();
        app.derived_nodes.insert(NodeId::from("b"));
        app.derived_nodes.insert(NodeId::from("c"));

        cut_selected(&mut app);

        assert_eq!(app.document.outline.roots.len(), 1);
        assert_eq!(app.status, "@auto subtrees cannot be cut");
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
