use std::{
    collections::{HashMap, HashSet},
    env, fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use leo::{DerivedFile, LeoDocument, NodeId, Outline, Position, PositionId};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

#[derive(Clone)]
struct Row {
    position: PositionId,
    node: NodeId,
    depth: usize,
    has_children: bool,
}

struct App {
    document: LeoDocument,
    path: PathBuf,
    expanded: HashSet<PositionId>,
    selected: usize,
    status: String,
    input: Option<HeadlineInput>,
    dirty: bool,
    quit_armed: bool,
    source_locations: HashMap<PositionId, SourceLocation>,
    source_nodes: HashMap<NodeId, SourceLocation>,
    derived_nodes: HashSet<NodeId>,
    derived_roots: HashMap<NodeId, Vec<Position>>,
    #[cfg(feature = "syntax")]
    syntax: crate::syntax::SyntaxHighlighter,
    #[cfg(feature = "syntax")]
    syntax_enabled: bool,
    #[cfg(feature = "syntax")]
    highlight_cache: HashMap<PositionId, Text<'static>>,
}

impl App {
    fn new(
        document: LeoDocument,
        path: PathBuf,
        status: String,
        source_locations: HashMap<PositionId, SourceLocation>,
        source_nodes: HashMap<NodeId, SourceLocation>,
        derived_nodes: HashSet<NodeId>,
        derived_roots: HashMap<NodeId, Vec<Position>>,
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
            status,
            input: None,
            dirty: false,
            quit_armed: false,
            source_locations,
            source_nodes,
            derived_nodes,
            derived_roots,
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
        let len = self.rows().len();
        if len == 0 {
            self.selected = 0;
            return;
        }
        self.selected = self.selected.saturating_add_signed(delta).min(len - 1);
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

    fn editable(&mut self, row: &Row) -> bool {
        if self.derived_nodes.contains(&row.node) {
            self.status = "derived descendants are read-only; press o to edit the source".into();
            false
        } else {
            true
        }
    }
}

struct HeadlineInput {
    node: NodeId,
    value: String,
    original: String,
    inserted_position: Option<PositionId>,
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
    let (status, source_locations, source_nodes, derived_nodes, derived_roots) = if load_derived {
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
            report.original_children,
        )
    } else {
        (
            "derived files disabled".to_owned(),
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
        )
    };
    let mut app = App::new(
        document,
        path,
        status,
        source_locations,
        source_nodes,
        derived_nodes,
        derived_roots,
    );
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if app.input.is_some() {
            handle_headline_input(app, key);
            continue;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') => save(app),
                KeyCode::Char('i') | KeyCode::Tab => insert_headline(app),
                KeyCode::Char('h') | KeyCode::Backspace => edit_headline(app),
                KeyCode::Up => move_selected(app, MoveDirection::Up),
                KeyCode::Down => move_selected(app, MoveDirection::Down),
                KeyCode::Left => move_selected(app, MoveDirection::Left),
                KeyCode::Right => move_selected(app, MoveDirection::Right),
                _ => {}
            }
            continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if !app.dirty || app.quit_armed {
                    return Ok(());
                }
                app.quit_armed = true;
                app.status = "unsaved changes; press q again to discard, or Ctrl-S to save".into();
            }
            KeyCode::Char('o') => open_selected(terminal, app),
            KeyCode::Tab => insert_headline(app),
            KeyCode::Backspace => edit_headline(app),
            #[cfg(feature = "syntax")]
            KeyCode::Char('y') => {
                app.syntax_enabled = !app.syntax_enabled;
                app.status = format!(
                    "syntax highlighting {}",
                    if app.syntax_enabled { "on" } else { "off" }
                );
            }
            KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => app.toggle(true),
            KeyCode::Left | KeyCode::Char('h') => app.toggle(false),
            KeyCode::Home => app.selected = 0,
            KeyCode::End => app.selected = app.rows().len().saturating_sub(1),
            _ => {}
        }
    }
}

fn handle_headline_input(app: &mut App, key: KeyEvent) {
    let Some(input) = app.input.as_mut() else {
        return;
    };
    match key.code {
        KeyCode::Enter => {
            let headline = input.value.trim().to_owned();
            if headline.is_empty() {
                app.status = "headline may not be empty".into();
                return;
            }
            app.document
                .outline
                .nodes
                .get_mut(&input.node)
                .expect("edited node exists")
                .headline = headline;
            app.input = None;
            app.dirty = true;
            app.quit_armed = false;
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
            input.value.pop();
        }
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            input.value.push(character);
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
        original,
        inserted_position: None,
    });
    app.status = "editing headline: Enter accepts, Esc cancels".into();
}

fn insert_headline(app: &mut App) {
    let Some(row) = app.selected_row() else {
        return;
    };
    if !app.editable(&row) || position_contains_derived(app, &row.position) {
        app.status = "cannot insert beside an external derived subtree".into();
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
        inserted_position: Some(inserted),
    });
    app.status = "new headline: type a name, Enter accepts, Esc cancels".into();
}

fn move_selected(app: &mut App, direction: MoveDirection) {
    let Some(row) = app.selected_row() else {
        return;
    };
    if !app.editable(&row) || position_contains_derived(app, &row.position) {
        app.status = "external derived subtrees cannot be moved".into();
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
    app.quit_armed = false;
    select_position(app, &target);
    app.status = "node moved (Ctrl-S to save)".into();
}

fn save(app: &mut App) {
    let mut persisted = app.document.clone();
    restore_derived_children(&mut persisted.outline.roots, &app.derived_roots);
    let referenced = referenced_nodes(&persisted.outline.roots);
    persisted
        .outline
        .nodes
        .retain(|id, _| referenced.contains(id));
    match persisted.save(&app.path) {
        Ok(()) => {
            app.dirty = false;
            app.quit_armed = false;
            app.status = format!("saved {}", app.path.display());
        }
        Err(error) => app.status = format!("save failed: {error}"),
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

fn position_contains_derived(app: &App, id: &PositionId) -> bool {
    app.document
        .outline
        .position(id)
        .is_some_and(|position| subtree_contains(&position.children, &app.derived_nodes))
}

fn subtree_contains(positions: &[Position], nodes: &HashSet<NodeId>) -> bool {
    positions.iter().any(|position| {
        nodes.contains(&position.node) || subtree_contains(&position.children, nodes)
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
    if let Some(index) = app.rows().iter().position(|row| &row.position == id) {
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

fn draw(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(1)])
        .split(frame.area());
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(areas[0]);
    let rows = app.rows();
    let items: Vec<_> = rows
        .iter()
        .map(|row| {
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
                format!(" ×{clone_count}")
            } else {
                String::new()
            };
            ListItem::new(Line::from(vec![
                Span::raw("  ".repeat(row.depth)),
                Span::raw(marker),
                Span::raw(
                    app.input
                        .as_ref()
                        .filter(|input| input.node == row.node)
                        .map_or(node.headline.as_str(), |input| input.value.as_str()),
                ),
                Span::raw(
                    app.input
                        .as_ref()
                        .filter(|input| input.node == row.node)
                        .map_or("", |_| "▏"),
                ),
                Span::styled(clone, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();
    let mut state = ListState::default().with_selected((!rows.is_empty()).then_some(app.selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().title(" Outline ").borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        columns[0],
        &mut state,
    );

    let details = rows
        .get(app.selected)
        .map(|row| {
            let node = &app.document.outline.nodes[&row.node];
            let headline = node.headline.clone();
            let gnx = node.id.0.clone();
            let mut text = Text::from(vec![
                Line::styled(headline, Style::default().add_modifier(Modifier::BOLD)),
                Line::from(format!("GNX: {gnx}")),
                Line::from(format!("Position: {}", row.position.0)),
                Line::from(""),
            ]);
            text.extend(body_text(app, row));
            text
        })
        .unwrap_or_default();
    frame.render_widget(
        Paragraph::new(details).block(Block::default().title(" Node ").borders(Borders::ALL)),
        columns[1],
    );
    frame.render_widget(
        Paragraph::new(format!("{}   [{}]", controls(), app.status))
            .style(Style::default().fg(Color::DarkGray)),
        areas[1],
    );
}

fn controls() -> &'static str {
    #[cfg(feature = "syntax")]
    return "Ctrl-I new  Ctrl-H rename  Ctrl-↑↓←→ move  Ctrl-S save  o open  y syntax  q quit";
    #[cfg(not(feature = "syntax"))]
    "Ctrl-I new  Ctrl-H rename  Ctrl-↑↓←→ move  Ctrl-S save  o open  q quit"
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
        if let Some(value) = crate::syntax::language_directive(&node.body) {
            language = Some(value.to_owned());
        }
        if let Some(filename) = external_filename(&node.headline) {
            source_path = Some(PathBuf::from(filename));
        }
    }

    (language, source_path)
}

#[derive(Default)]
struct LoadReport {
    loaded: usize,
    errors: Vec<String>,
    locations: HashMap<PositionId, SourceLocation>,
    node_locations: HashMap<NodeId, SourceLocation>,
    derived_nodes: HashSet<NodeId>,
    original_children: HashMap<NodeId, Vec<Position>>,
}

struct DerivedJob {
    position: PositionId,
    path: PathBuf,
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
            .and_then(|source| DerivedFile::parse(&source).map_err(|error| error.to_string()))
            .and_then(|derived| {
                let root_node = outline
                    .position(&job.position)
                    .map(|position| position.node.clone())
                    .ok_or_else(|| "derived root position disappeared".to_owned())?;
                let original_children = outline
                    .position(&job.position)
                    .map(|position| position.children.clone())
                    .unwrap_or_default();
                derived
                    .merge_into(outline, &job.position)
                    .map_err(|error| error.to_string())?;
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
                report
                    .original_children
                    .entry(root_node)
                    .or_insert(original_children);
                report.derived_nodes.extend(
                    derived
                        .outline
                        .nodes
                        .keys()
                        .filter(|id| **id != derived.root)
                        .cloned(),
                );
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
    let Some(location) = app
        .source_locations
        .get(&row.position)
        .or_else(|| app.source_nodes.get(&row.node))
        .cloned()
    else {
        app.status = "selected node has no external source location".to_owned();
        return;
    };
    if let Err(error) = suspend_and_open(terminal, &location) {
        app.status = format!("open failed: {error}");
    } else {
        app.status = format!("opened {}:{}", location.path.display(), location.line);
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
        "hx" | "helix" | "kak" => {
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
            if let Some(filename) = thin_filename(&node.headline) {
                let mut path = base.to_path_buf();
                for component in inherited_paths {
                    path.push(component);
                }
                path.push(filename);
                jobs.push(DerivedJob {
                    position: PositionId(position_id.clone()),
                    path,
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

fn thin_filename(headline: &str) -> Option<&str> {
    let (directive, filename) = headline.trim().split_once(char::is_whitespace)?;
    matches!(directive, "@file" | "@thin" | "@file-thin")
        .then(|| strip_path_cruft(filename))
        .filter(|filename| !filename.is_empty())
}

fn external_filename(headline: &str) -> Option<&str> {
    let (directive, filename) = headline.trim().split_once(char::is_whitespace)?;
    matches!(directive, "@file" | "@thin" | "@file-thin" | "@clean")
        .then(|| strip_path_cruft(filename))
        .filter(|filename| !filename.is_empty())
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
}
