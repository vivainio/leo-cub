use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
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
    expanded: HashSet<PositionId>,
    selected: usize,
    status: String,
}

impl App {
    fn new(document: LeoDocument, status: String) -> Self {
        let expanded = document
            .outline
            .roots
            .iter()
            .enumerate()
            .map(|(index, _)| PositionId(index.to_string()))
            .collect();
        Self {
            document,
            expanded,
            selected: 0,
            status,
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
}

pub fn run(path: PathBuf, load_derived: bool) -> Result<()> {
    let mut document = LeoDocument::open(&path)?;
    let status = if load_derived {
        let report = load_derived_files(&mut document.outline, &path);
        if report.errors.is_empty() {
            format!("loaded {} derived file(s)", report.loaded)
        } else {
            format!(
                "loaded {}; {} error(s): {}",
                report.loaded,
                report.errors.len(),
                report.errors.join(" | ")
            )
        }
    } else {
        "derived files disabled".to_owned()
    };
    let mut app = App::new(document, status);
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
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
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

fn draw(frame: &mut ratatui::Frame<'_>, app: &App) {
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
                Span::raw(&node.headline),
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
            let mut text = Text::from(vec![
                Line::styled(
                    &node.headline,
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Line::from(format!("GNX: {}", node.id.0)),
                Line::from(format!("Position: {}", row.position.0)),
                Line::from(""),
            ]);
            text.extend(Text::from(node.body.as_str()));
            text
        })
        .unwrap_or_default();
    frame.render_widget(
        Paragraph::new(details).block(Block::default().title(" Node ").borders(Borders::ALL)),
        columns[1],
    );
    frame.render_widget(
        Paragraph::new(format!(
            "↑/k ↓/j   →/l expand   ←/h collapse   q quit   [{}]",
            app.status
        ))
        .style(Style::default().fg(Color::DarkGray)),
        areas[1],
    );
}

#[derive(Default)]
struct LoadReport {
    loaded: usize,
    errors: Vec<String>,
}

struct DerivedJob {
    position: PositionId,
    path: PathBuf,
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
                derived
                    .merge_into(outline, &job.position)
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(()) => report.loaded += 1,
            Err(error) => report.errors.push(format!("{label}: {error}")),
        }
    }
    report
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

    #[test]
    fn recognizes_only_sentinelled_file_headlines() {
        assert_eq!(thin_filename("@file src/main.rs"), Some("src/main.rs"));
        assert_eq!(thin_filename("@file \"src/main.rs\""), Some("src/main.rs"));
        assert_eq!(thin_filename("@clean src/main.rs"), None);
        assert_eq!(thin_filename("ordinary"), None);
    }

    #[test]
    fn extracts_leo_path_directives() {
        assert_eq!(
            path_directive("@language rust\n@path <src/core>\n"),
            Some("src/core".into())
        );
    }

    #[test]
    fn expands_checked_out_leo_reference_when_available() {
        let path = Path::new("/home/v/r/ref/leo-editor/leo/core/LeoPyRef.leo");
        if !path.exists() {
            return;
        }
        let mut document = LeoDocument::open(path).unwrap();
        let report = load_derived_files(&mut document.outline, path);
        assert!(report.loaded > 40, "{:?}", report.errors);
        let at_file = document
            .outline
            .nodes
            .get(&NodeId::from("ekr.20150323150718.1"))
            .unwrap();
        assert_eq!(at_file.headline, "@file leoAtFile.py");
        assert!(
            document
                .outline
                .nodes
                .contains_key(&NodeId::from("ekr.20041005105605.2"))
        );
    }
}
