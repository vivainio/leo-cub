use std::{
    collections::{HashMap, HashSet},
    env, fs,
    fs::OpenOptions,
    io,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use leo::{AutoFile, DerivedFile, LeoDocument, NodeId, Outline, Position, PositionId, render_thin};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

#[derive(Clone)]
struct Row {
    position: PositionId,
    node: NodeId,
    depth: usize,
    has_children: bool,
}

#[derive(Default)]
struct OriginalExternalState {
    children: HashMap<NodeId, Vec<Position>>,
    bodies: HashMap<NodeId, String>,
}

#[derive(Clone)]
struct WritableExternalFile {
    path: PathBuf,
    start_delimiter: String,
    end_delimiter: String,
    original: Outline,
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
    body_wrap: bool,
    body_full_width: bool,
    outline_full_width: bool,
    help: bool,
    status: String,
    flash: Option<(String, Instant)>,
    input: Option<HeadlineInput>,
    find: Option<FindInput>,
    dirty: bool,
    dirty_nodes: HashSet<NodeId>,
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
            body_wrap: false,
            body_full_width: false,
            outline_full_width: false,
            help: false,
            status,
            flash: None,
            input: None,
            find: None,
            dirty: false,
            dirty_nodes: HashSet::new(),
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
            self.body_scroll = 0;
            self.body_horizontal_scroll = 0;
        }
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
        if self.body_wrap {
            return;
        }
        self.body_horizontal_scroll = self
            .body_horizontal_scroll
            .saturating_add_signed(columns)
            .min(self.body_horizontal_scroll_max);
    }

    fn toggle_body_wrap(&mut self) {
        self.body_wrap = !self.body_wrap;
        self.body_scroll = 0;
        self.body_horizontal_scroll = 0;
        self.status = format!(
            "word wrap {}",
            if self.body_wrap {
                "enabled"
            } else {
                "disabled"
            }
        );
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

pub fn run(path: PathBuf, load_derived: bool) -> Result<()> {
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
    let mut app = App::new(
        document,
        path,
        status,
        source_locations,
        source_nodes,
        derived_nodes,
        writable_external,
        original_external,
        load_derived,
    );
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    result
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let event = event::read()?;
        if let Event::Mouse(mouse) = event {
            if app.input.is_none() && app.find.is_none() && !app.help {
                handle_mouse(app, terminal.size()?.into(), mouse);
            }
            continue;
        }
        let Event::Key(key) = event else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if app.input.is_some() {
            handle_headline_input(app, key);
            continue;
        }
        if app.find.is_some() {
            handle_find_input(app, key);
            continue;
        }
        if app.help {
            if matches!(
                key.code,
                KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Esc
            ) {
                app.help = false;
            }
            continue;
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
            continue;
        }
        match key.code {
            KeyCode::Char('c') if key.modifiers.is_empty() => copy_selected(app),
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
                    return Ok(());
                }
                app.quit_armed = true;
                app.status = "unsaved changes; press q again to discard, or Ctrl-S to save".into();
            }
            KeyCode::Char('o') => open_selected(terminal, app),
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
            KeyCode::Char('W') => app.toggle_body_wrap(),
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
            KeyCode::Down if app.body_full_width => app.scroll_body_lines(1),
            KeyCode::Up if app.body_full_width => app.scroll_body_lines(-1),
            KeyCode::Down => app.move_selection(1),
            KeyCode::Up => app.move_selection(-1),
            KeyCode::Right if app.body_full_width => app.scroll_body_horizontal(4),
            KeyCode::Left if app.body_full_width => app.scroll_body_horizontal(-4),
            KeyCode::Enter => open_selected(terminal, app),
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
                app.body_scroll = 0;
                app.body_horizontal_scroll = 0;
            }
            KeyCode::End => {
                app.selection_anchor = None;
                app.selected = app.rows().len().saturating_sub(1);
                app.body_scroll = 0;
                app.body_horizontal_scroll = 0;
            }
            KeyCode::PageUp => app.scroll_body(-1),
            KeyCode::PageDown => app.scroll_body(1),
            _ => {}
        }
    }
}

fn handle_mouse(app: &mut App, area: Rect, mouse: MouseEvent) {
    if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
        return;
    }
    let content_height = area.height.saturating_sub(1);
    let content = Rect::new(area.x, area.y, area.width, content_height);
    let columns = content_columns(content, app);
    let outline = columns[0];
    if app.body_full_width
        || mouse.column < outline.x
        || mouse.column >= outline.right()
        || mouse.row < outline.y.saturating_add(1)
        || mouse.row >= outline.bottom().saturating_sub(1)
    {
        return;
    }
    let row = app.outline_scroll + usize::from(mouse.row - outline.y - 1);
    if row < app.rows().len() {
        app.selection_anchor = None;
        if row != app.selected {
            app.body_scroll = 0;
            app.body_horizontal_scroll = 0;
        }
        app.selected = row;
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

fn reveal_and_select(app: &mut App, position: &PositionId) {
    let components = position.0.split('/').collect::<Vec<_>>();
    for end in 1..components.len() {
        app.expanded.insert(PositionId(components[..end].join("/")));
    }
    select_position(app, position);
}

fn handle_headline_input(app: &mut App, key: KeyEvent) {
    let Some(input) = app.input.as_mut() else {
        return;
    };
    match key.code {
        KeyCode::Enter => {
            let headline = input.value.trim().to_owned();
            let node_id = input.node.clone();
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
                    .or_insert(WritableExternalFile {
                        path,
                        start_delimiter: start_delimiter.to_owned(),
                        end_delimiter: end_delimiter.to_owned(),
                        original: Outline::default(),
                    });
            }
            app.dirty_nodes.insert(node_id);
            app.input = None;
            app.dirty = true;
            app.quit_armed = false;
            #[cfg(feature = "syntax")]
            app.highlight_cache.clear();
            app.status = "headline changed (Ctrl-S to save)".into();
        }
        KeyCode::Esc => {
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
            app.status = "headline edit cancelled".into();
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
    app.status = "new headline: type a name, Enter accepts, Esc cancels".into();
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
    if rows.iter().any(|row| {
        app.readonly_derived(&row.node) || position_contains_readonly_derived(app, &row.position)
    }) {
        app.status = "@auto subtrees cannot be cut".into();
        return;
    }
    copy_selected(app);
    rows.sort_by_key(|row| path_indices(&row.position));
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
    let (parent, insert_at) = if let Some(row) = app.selected_row() {
        if !app.editable(&row) || position_contains_readonly_derived(app, &row.position) {
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
        let Some(siblings) = children_mut(&mut app.document.outline, parent.as_ref()) else {
            return;
        };
        let count = clipboard.roots.len();
        siblings.splice(insert_at..insert_at, clipboard.roots);
        let target = join_position(parent.as_ref(), insert_at);
        app.dirty = true;
        app.quit_armed = false;
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
    let Some(siblings) = children_mut(&mut app.document.outline, parent.as_ref()) else {
        return;
    };
    let count = pasted.len();
    siblings.splice(insert_at..insert_at, pasted);
    let target = join_position(parent.as_ref(), insert_at);
    app.dirty = true;
    app.quit_armed = false;
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
    if !app.editable(&row) || position_contains_readonly_derived(app, &row.position) {
        app.status = "@auto subtrees cannot be moved".into();
        return;
    }
    let Some((parent, index)) = split_position(&row.position) else {
        return;
    };
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
            siblings[previous].children.push(position);
            let previous_path = join_position(parent.as_ref(), previous);
            app.expanded.insert(previous_path.clone());
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
    select_position(app, &target);
    app.status = "node moved (Ctrl-S to save)".into();
}

fn move_selected_block(app: &mut App, direction: MoveDirection, rows: Vec<Row>) {
    if rows.iter().any(|row| {
        app.readonly_derived(&row.node) || position_contains_readonly_derived(app, &row.position)
    }) {
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
            siblings[previous].children.extend(block);
            let parent_path = join_position(parent.as_ref(), previous);
            app.expanded.insert(parent_path.clone());
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
    select_position(app, &first);
    let first_index = app.selected;
    select_position(app, &last);
    app.selection_anchor = Some(first_index);
    app.status = format!("{count} nodes moved (Ctrl-S to save)");
}

fn save(app: &mut App) {
    let external_updates = match prepare_external_updates(app) {
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

struct ExternalUpdate {
    root: NodeId,
    path: PathBuf,
    rendered: String,
    snapshot: Outline,
}

fn prepare_external_updates(app: &App) -> Result<Vec<ExternalUpdate>, String> {
    let mut updates = Vec::new();
    for (root, file) in &app.writable_external {
        let Some((position, snapshot)) = external_snapshot(&app.document.outline, root) else {
            continue;
        };
        if snapshot == file.original {
            continue;
        }
        let rendered = render_thin(
            &app.document.outline,
            &position,
            &file.start_delimiter,
            &file.end_delimiter,
        )
        .map_err(|error| format!("{}: {error}", file.path.display()))?;
        DerivedFile::parse(&rendered).map_err(|error| {
            format!(
                "{}: generated invalid thin file: {error}",
                file.path.display()
            )
        })?;
        updates.push(ExternalUpdate {
            root: root.clone(),
            path: file.path.clone(),
            rendered,
            snapshot,
        });
    }
    Ok(updates)
}

fn write_external_updates(updates: &[ExternalUpdate]) -> Result<(), String> {
    let mut staged = Vec::new();
    for (index, update) in updates.iter().enumerate() {
        let name = update
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("external");
        let temporary = update
            .path
            .with_file_name(format!(".{name}.cub-save-{}-{index}", std::process::id()));
        let permissions = fs::metadata(&update.path).map(|metadata| metadata.permissions());
        let result = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .and_then(|mut file| {
                file.write_all(update.rendered.as_bytes())?;
                file.sync_all()
            })
            .and_then(|()| {
                permissions.and_then(|permissions| fs::set_permissions(&temporary, permissions))
            });
        if let Err(error) = result {
            for path in &staged {
                let _ = fs::remove_file(path);
            }
            let _ = fs::remove_file(&temporary);
            return Err(format!("{}: {error}", update.path.display()));
        }
        staged.push(temporary);
    }
    for (update, temporary) in updates.iter().zip(&staged) {
        if let Err(error) = fs::rename(temporary, &update.path) {
            for path in &staged {
                let _ = fs::remove_file(path);
            }
            return Err(format!("{}: {error}", update.path.display()));
        }
    }
    Ok(())
}

fn external_snapshot(outline: &Outline, root: &NodeId) -> Option<(PositionId, Outline)> {
    fn find(positions: &[Position], parent: &str, root: &NodeId) -> Option<(PositionId, Position)> {
        for (index, position) in positions.iter().enumerate() {
            let id = if parent.is_empty() {
                index.to_string()
            } else {
                format!("{parent}/{index}")
            };
            if &position.node == root {
                return Some((PositionId(id), position.clone()));
            }
            if let Some(found) = find(&position.children, &id, root) {
                return Some(found);
            }
        }
        None
    }
    let (position, tree) = find(&outline.roots, "", root)?;
    let ids = referenced_nodes(std::slice::from_ref(&tree));
    let nodes = ids
        .into_iter()
        .filter_map(|id| outline.nodes.get(&id).cloned().map(|node| (id, node)))
        .collect();
    Some((
        position,
        Outline {
            roots: vec![tree],
            nodes,
        },
    ))
}

fn reload(app: &mut App) {
    if app.dirty && !app.reload_armed {
        app.reload_armed = true;
        app.status =
            "unsaved changes; press Ctrl-R again to discard and reload, or Ctrl-S to save".into();
        return;
    }

    let selected_node = app.selected_row().map(|row| row.node);
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
    if let Some(node) = selected_node {
        if let Some(index) = app.rows().iter().position(|row| row.node == node) {
            app.selected = index;
        } else {
            app.selected = app.selected.min(app.rows().len().saturating_sub(1));
        }
    }
    app.status = format!("reloaded {} ({derived_status})", app.path.display());
}

fn restore_external_state(
    outline: &mut Outline,
    children: &HashMap<NodeId, Vec<Position>>,
    bodies: &HashMap<NodeId, String>,
) {
    restore_derived_children(&mut outline.roots, children);
    for (id, body) in bodies {
        if let Some(node) = outline.nodes.get_mut(id) {
            node.body.clone_from(body);
        }
    }
}

fn restore_derived_children(
    positions: &mut [Position],
    originals: &HashMap<NodeId, Vec<Position>>,
) {
    for position in positions {
        if let Some(children) = originals.get(&position.node) {
            position.children.clone_from(children);
        } else {
            restore_derived_children(&mut position.children, originals);
        }
    }
}

fn referenced_nodes(positions: &[Position]) -> HashSet<NodeId> {
    let mut result = HashSet::new();
    fn visit(positions: &[Position], result: &mut HashSet<NodeId>) {
        for position in positions {
            result.insert(position.node.clone());
            visit(&position.children, result);
        }
    }
    visit(positions, &mut result);
    result
}

fn position_contains_readonly_derived(app: &App, id: &PositionId) -> bool {
    app.document
        .outline
        .position(id)
        .is_some_and(|position| subtree_contains_readonly(app, &position.children))
}

fn subtree_contains_readonly(app: &App, positions: &[Position]) -> bool {
    positions.iter().any(|position| {
        app.readonly_derived(&position.node) || subtree_contains_readonly(app, &position.children)
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
    if app.body_full_width {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(0), Constraint::Percentage(100)])
            .split(area)
            .to_vec()
    } else if app.outline_full_width {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100), Constraint::Length(0)])
            .split(area)
            .to_vec()
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(area)
            .to_vec()
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(1)])
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
            let clone = if clone_count > 1 {
                format!(" ⧉×{clone_count}")
            } else {
                String::new()
            };
            let input = app.input.as_ref().filter(|input| input.node == row.node);
            let mut spans = vec![Span::raw("  ".repeat(row.depth)), Span::raw(marker)];
            spans.push(dirty_marker(app.dirty_nodes.contains(&row.node)));
            spans.push(body_marker(!node.body.trim().is_empty()));
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
        let node_block = Block::default()
            .title(node_title(app.body_wrap))
            .borders(Borders::ALL);
        let node_area = node_block.inner(columns[1]);
        frame.render_widget(node_block, columns[1]);
        if let Some(row) = rows.get(app.selected) {
            let body = body_text(app, row);
            let body_width = body.width();
            let mut paragraph = Paragraph::new(body);
            if app.body_wrap {
                paragraph = paragraph.wrap(Wrap { trim: false });
            }
            app.body_page_size = usize::from(node_area.height).max(1);
            let body_height = paragraph.line_count(node_area.width);
            app.body_scroll_max = body_height.saturating_sub(app.body_page_size);
            app.body_horizontal_scroll_max = if app.body_wrap {
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
    let mut status = vec![Span::styled(
        format!(
            "{}   [",
            controls(app.body_full_width, app.outline_full_width)
        ),
        Style::default().fg(Color::DarkGray),
    )];
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
    status.push(Span::styled("]", Style::default().fg(Color::DarkGray)));
    frame.render_widget(Paragraph::new(Line::from(status)), areas[1]);
    if let Some(find) = &app.find {
        let width = frame.area().width.saturating_sub(4).min(72);
        let shown = find.matches.len().min(5);
        let height = (3 + shown as u16).min(frame.area().height.saturating_sub(2));
        let area = Rect::new(
            frame.area().x + (frame.area().width.saturating_sub(width)) / 2,
            frame.area().y + 1,
            width,
            height,
        );
        let count = if find.query.is_empty() {
            String::new()
        } else if find.matches.is_empty() {
            "no matches".into()
        } else {
            format!("{} of {}", find.active + 1, find.matches.len())
        };
        let rows = all_rows(&app.document.outline);
        let first = find.active.saturating_sub(4);
        let mut lines = vec![Line::from(format!("> {}▏", find.query))];
        lines.extend(find.matches.iter().enumerate().skip(first).take(5).map(
            |(index, position)| {
                let row = rows
                    .iter()
                    .find(|row| &row.position == position)
                    .expect("matched position exists");
                let marker = if index == find.active { "› " } else { "  " };
                let mut spans = vec![Span::raw(marker)];
                spans.extend(headline_spans(
                    &app.document.outline.nodes[&row.node].headline,
                ));
                Line::from(spans)
            },
        ));
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(lines).block(
                Block::default()
                    .title(format!(" Find headline  {count} "))
                    .borders(Borders::ALL),
            ),
            area,
        );
    }
    if app.help {
        draw_help(frame, app.body_full_width, app.outline_full_width);
    }
}

fn dirty_marker(dirty: bool) -> Span<'static> {
    if dirty {
        Span::styled("* ", Style::default().fg(Color::LightRed))
    } else {
        Span::raw("  ")
    }
}

fn body_marker(has_body: bool) -> Span<'static> {
    if has_body {
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
        return "? help  arrows scroll  W wrap  c/x/v/V tree  f split  F outline  o open/edit  Ctrl-P find  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  y syntax  q quit";
        #[cfg(not(feature = "syntax"))]
        return "? help  arrows scroll  W wrap  c/x/v/V tree  f split  F outline  o open/edit  Ctrl-P find  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  q quit";
    }
    if outline_full_width {
        return "? help  arrows navigate  W wrap  c/x/v/V tree  F split view  o open/edit  Ctrl-P find  i new  h rename  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  q quit";
    }
    #[cfg(feature = "syntax")]
    return "? help  arrows navigate  PgUp/PgDn body  W wrap  c/x/v/V tree  f body  F outline  o open/edit  Ctrl-P find  i new  h rename  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  y syntax  q quit";
    #[cfg(not(feature = "syntax"))]
    "? help  arrows navigate  PgUp/PgDn body  W wrap  c/x/v/V tree  f body  F outline  o open/edit  Ctrl-P find  i new  h rename  Ctrl-↑↓←→ move  Ctrl-R reload  Ctrl-S save  q quit"
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
            Line::from("c/x              Copy/cut selected trees"),
            Line::from("v / Shift-V      Paste copy / paste clone"),
            Line::from("i                Insert a sibling"),
            Line::from("h                Rename the headline"),
            Line::from("Ctrl-↑↓←→        Move selected tree(s)"),
            Line::from("Ctrl-P           Find a headline"),
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
            Line::from("Shift-W          Toggle body word wrap"),
            Line::from("c/x/v/V          Copy/cut/paste/clone"),
            Line::from("Ctrl-P           Find a headline"),
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
            Line::from("PageUp/PageDown  Scroll the body pane"),
            Line::from("Shift-W          Toggle body word wrap"),
            Line::from("Ctrl-P           Find a headline"),
            Line::from("i                Insert a sibling"),
            Line::from("h                Rename the headline"),
            Line::from("c                Copy selected tree"),
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

fn body_text(app: &mut App, row: &Row) -> Text<'static> {
    let body = app.document.outline.nodes[&row.node].body.clone();
    #[cfg(feature = "syntax")]
    if app.syntax_enabled {
        if let Some(cached) = app.highlight_cache.get(&row.position) {
            return cached.clone();
        }
        let (inherited_language, external_path) =
            syntax_context(&app.document.outline, &row.position);
        let source_path = app
            .source_locations
            .get(&row.position)
            .map(|location| location.path.as_path())
            .or(external_path.as_deref());
        let highlighted =
            app.syntax
                .highlight_with_language(&body, source_path, inherited_language.as_deref());
        app.highlight_cache
            .insert(row.position.clone(), highlighted.clone());
        return highlighted;
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
    let mut report = LoadReport::default();
    for job in jobs {
        let label = job.path.display().to_string();
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
    match edit_body_in_temp_file(terminal, &app.document.outline.nodes[&row.node].body) {
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

fn edit_body_in_temp_file(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    original: &str,
) -> Result<Option<String>> {
    let path = unique_body_temp_path();
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

fn unique_body_temp_path() -> PathBuf {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    env::temp_dir().join(format!(
        "leo-cub-body-{}-{}-{}.txt",
        std::process::id(),
        duration.as_secs(),
        duration.subsec_nanos()
    ))
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
        "@file" | "@thin" | "@file-thin" | "@auto" | "@auto-md" | "@auto-markdown"
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

#[cfg(test)]
fn thin_filename(headline: &str) -> Option<&str> {
    derived_filename(headline).and_then(|(auto, _, filename)| (!auto).then_some(filename))
}

fn external_filename(headline: &str) -> Option<&str> {
    let (directive, filename) = headline.trim().split_once(char::is_whitespace)?;
    matches!(
        directive,
        "@file" | "@thin" | "@file-thin" | "@clean" | "@auto" | "@auto-md" | "@auto-markdown"
    )
    .then(|| strip_path_cruft(filename))
    .filter(|filename| !filename.is_empty())
}

fn comment_delimiters(path: &Path) -> (&'static str, &'static str) {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "py" | "pyw" | "sh" | "bash" | "zsh" | "fish" | "rb" | "pl" | "pm" | "r" | "toml"
        | "yaml" | "yml" => ("#", ""),
        "rs" | "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "java" | "js" | "jsx" | "ts" | "tsx"
        | "go" | "swift" | "kt" | "kts" | "cs" => ("//", ""),
        "html" | "htm" | "xml" | "xhtml" | "svg" => ("<!--", "-->"),
        "css" | "scss" | "less" => ("/*", "*/"),
        "sql" | "lua" => ("--", ""),
        "ini" | "cfg" => ("#", ""),
        _ => ("#", ""),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn shows_a_subtle_dot_only_for_nodes_with_body_content() {
        let populated = body_marker(true);
        assert_eq!(populated.content, "· ");
        assert_eq!(populated.style.fg, Some(Color::DarkGray));

        let empty = body_marker(false);
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
    fn restores_external_children_before_serializing() {
        let mut roots = vec![Position {
            node: NodeId::from("file"),
            children: vec![Position {
                node: NodeId::from("derived"),
                children: vec![],
            }],
        }];
        let originals = HashMap::from([(NodeId::from("file"), Vec::new())]);
        restore_derived_children(&mut roots, &originals);
        assert!(roots[0].children.is_empty());
    }

    #[test]
    fn restores_auto_root_body_before_serializing() {
        let mut outline = Outline {
            nodes: [(
                NodeId::from("file"),
                leo::Node {
                    id: NodeId::from("file"),
                    headline: "@auto x.py".into(),
                    body: "generated @others body".into(),
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                },
            )]
            .into_iter()
            .collect(),
            roots: vec![Position {
                node: NodeId::from("file"),
                children: vec![],
            }],
        };
        restore_external_state(
            &mut outline,
            &HashMap::new(),
            &HashMap::from([(NodeId::from("file"), String::new())]),
        );
        assert!(outline.nodes[&NodeId::from("file")].body.is_empty());
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
    fn paste_as_clone_retains_node_identities() {
        let mut app = editing_app();
        copy_selected(&mut app);
        paste_tree(&mut app, true);

        assert_eq!(app.document.outline.roots.len(), 2);
        assert_eq!(app.document.outline.roots[0], app.document.outline.roots[1]);
        assert_eq!(clone_count(&app.document.outline, &NodeId::from("a")), 2);
        assert_eq!(app.document.outline.nodes.len(), 3);
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
}
