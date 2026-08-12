//! Select logical subtrees for machine-readable inspection.

use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;
use serde::Serialize;
use thiserror::Error;

use crate::{DerivedFile, NodeId, Outline, Position, PositionId};

#[derive(Clone, Copy, Debug)]
pub enum InspectSelector<'a> {
    Gnx(&'a str),
    Position(&'a PositionId),
    File(&'a str),
}

#[derive(Debug, Error, PartialEq)]
pub enum InspectError {
    #[error("no subtree has GNX {0}")]
    NoGnx(String),
    #[error("no subtree exists at position {0}")]
    NoPosition(String),
    #[error("no external subtree matches {0:?}")]
    NoFile(String),
    #[error("failed to parse derived file {path}: {message}")]
    Derived { path: PathBuf, message: String },
}

pub enum ExternalFilter<'a> {
    Search(&'a [Regex]),
    Gnx(&'a str),
    File(&'a str),
}

/// Lazily merge thin external trees whose raw files might satisfy `filter`.
/// Missing external files are ignored, matching Leo's handling of stale links.
pub fn load_matching_external_files(
    outline: &mut Outline,
    outline_path: &Path,
    filter: ExternalFilter<'_>,
) -> Result<usize, InspectError> {
    let jobs = external_jobs(outline, outline_path);
    let mut cache: HashMap<PathBuf, Option<DerivedFile>> = HashMap::new();
    let mut loaded = 0;
    for job in jobs {
        let path_matches = match filter {
            ExternalFilter::File(wanted) => {
                job.path == Path::new(wanted) || job.path.ends_with(wanted)
            }
            _ => true,
        };
        if !path_matches {
            continue;
        }
        if !cache.contains_key(&job.path) {
            let parsed = match fs::read_to_string(&job.path) {
                Ok(source) => {
                    let content_matches = match filter {
                        ExternalFilter::Search(patterns) => patterns.iter().any(|pattern| {
                            source
                                .lines()
                                .any(|line| raw_line_might_match(line, pattern))
                        }),
                        ExternalFilter::Gnx(gnx) => source.contains(gnx),
                        ExternalFilter::File(_) => true,
                    };
                    if content_matches {
                        Some(DerivedFile::parse(&source).map_err(|error| {
                            InspectError::Derived {
                                path: job.path.clone(),
                                message: error.to_string(),
                            }
                        })?)
                    } else {
                        None
                    }
                }
                Err(_) => None,
            };
            cache.insert(job.path.clone(), parsed);
        }
        if let Some(derived) = cache.get(&job.path).and_then(Option::as_ref) {
            derived
                .merge_into(outline, &job.position)
                .map_err(|error| InspectError::Derived {
                    path: job.path.clone(),
                    message: error.to_string(),
                })?;
            loaded += 1;
        }
    }
    Ok(loaded)
}

fn raw_line_might_match(line: &str, pattern: &Regex) -> bool {
    pattern.is_match(line)
        || pattern.is_match(line.trim_start())
        || line
            .split_once("@+node:")
            .and_then(|(_, sentinel)| sentinel.split_once(": "))
            .and_then(|(_, descriptor)| descriptor.split_once(' '))
            .is_some_and(|(_, headline)| pattern.is_match(headline))
}

struct ExternalJob {
    position: PositionId,
    path: PathBuf,
}

fn external_jobs(outline: &Outline, outline_path: &Path) -> Vec<ExternalJob> {
    fn visit(
        outline: &Outline,
        positions: &[Position],
        parent: &str,
        base: &Path,
        inherited_paths: &[String],
        jobs: &mut Vec<ExternalJob>,
    ) {
        for (index, position) in positions.iter().enumerate() {
            let id = if parent.is_empty() {
                index.to_string()
            } else {
                format!("{parent}/{index}")
            };
            let Some(node) = outline.nodes.get(&position.node) else {
                continue;
            };
            let mut paths = inherited_paths.to_vec();
            if let Some(path) =
                path_directive(&node.headline).or_else(|| path_directive(&node.body))
            {
                paths.push(path);
            }
            if let Some((directive, filename)) = external_file(&node.headline)
                && directive != "@clean"
            {
                let mut path = base.to_path_buf();
                for component in inherited_paths {
                    path.push(component);
                }
                path.push(filename);
                jobs.push(ExternalJob {
                    position: PositionId(id.clone()),
                    path,
                });
            }
            visit(outline, &position.children, &id, base, &paths, jobs);
        }
    }

    let mut jobs = Vec::new();
    visit(
        outline,
        &outline.roots,
        "",
        outline_path.parent().unwrap_or_else(|| Path::new(".")),
        &[],
        &mut jobs,
    );
    jobs
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SearchExcerpt {
    pub start_line: usize,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SearchMatch {
    pub position: PositionId,
    pub gnx: NodeId,
    pub headline: String,
    pub headline_match: bool,
    pub excerpts: Vec<SearchExcerpt>,
}

/// Search headlines and body lines, returning two lines around matches.
/// Multiple patterns have OR semantics.
pub fn search_outline(outline: &Outline, patterns: &[Regex]) -> Vec<SearchMatch> {
    const CONTEXT: usize = 2;

    fn visit(
        outline: &Outline,
        positions: &[Position],
        parent: &str,
        patterns: &[Regex],
        matches: &mut Vec<SearchMatch>,
    ) {
        for (index, position) in positions.iter().enumerate() {
            let path = if parent.is_empty() {
                index.to_string()
            } else {
                format!("{parent}/{index}")
            };
            let Some(node) = outline.nodes.get(&position.node) else {
                continue;
            };
            let lines = node.body.lines().collect::<Vec<_>>();
            let matched_lines = lines
                .iter()
                .enumerate()
                .filter_map(|(index, line)| {
                    patterns
                        .iter()
                        .any(|pattern| pattern.is_match(line))
                        .then_some(index)
                })
                .collect::<Vec<_>>();
            let excerpts = merge_excerpts(&lines, &matched_lines, CONTEXT);
            let headline_match = patterns
                .iter()
                .any(|pattern| pattern.is_match(&node.headline));
            if headline_match || !excerpts.is_empty() {
                matches.push(SearchMatch {
                    position: PositionId(path.clone()),
                    gnx: node.id.clone(),
                    headline: node.headline.clone(),
                    headline_match,
                    excerpts,
                });
            }
            visit(outline, &position.children, &path, patterns, matches);
        }
    }

    let mut matches = Vec::new();
    visit(outline, &outline.roots, "", patterns, &mut matches);
    matches
}

fn merge_excerpts(lines: &[&str], matched: &[usize], context: usize) -> Vec<SearchExcerpt> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for &line in matched {
        let start = line.saturating_sub(context);
        let end = (line + context + 1).min(lines.len());
        if let Some(last) = ranges.last_mut()
            && start <= last.1
        {
            last.1 = last.1.max(end);
        } else {
            ranges.push((start, end));
        }
    }
    ranges
        .into_iter()
        .map(|(start, end)| SearchExcerpt {
            start_line: start + 1,
            lines: lines[start..end]
                .iter()
                .map(|line| (*line).to_owned())
                .collect(),
        })
        .collect()
}

pub fn render_search_compact(matches: &[SearchMatch]) -> String {
    let mut output = String::new();
    for result in matches {
        writeln!(
            output,
            "{} {} {}",
            result.position.0, result.gnx.0, result.headline
        )
        .unwrap();
        for (excerpt_index, excerpt) in result.excerpts.iter().enumerate() {
            if excerpt_index > 0 {
                writeln!(output, "| ...").unwrap();
            }
            for (offset, line) in excerpt.lines.iter().enumerate() {
                writeln!(output, "| {}: {line}", excerpt.start_line + offset).unwrap();
            }
        }
    }
    output
}

/// Render positions, vnode identity, headlines, and bodies without JSON syntax.
/// A cloned vnode's content is emitted once; later occurrences use `=GNX`.
pub fn render_compact(outline: &Outline) -> String {
    fn visit(
        outline: &Outline,
        positions: &[Position],
        parent: &str,
        depth: usize,
        emitted: &mut HashSet<NodeId>,
        output: &mut String,
    ) {
        for (index, position) in positions.iter().enumerate() {
            let path = if parent.is_empty() {
                index.to_string()
            } else {
                format!("{parent}/{index}")
            };
            let indent = "  ".repeat(depth);
            if !emitted.insert(position.node.clone()) {
                writeln!(output, "{indent}{path} ={}", position.node.0).unwrap();
                continue;
            }
            let Some(node) = outline.nodes.get(&position.node) else {
                writeln!(output, "{indent}{path} {} <missing>", position.node.0).unwrap();
                continue;
            };
            writeln!(output, "{indent}{path} {} {}", node.id.0, node.headline).unwrap();
            for line in node.body.lines() {
                writeln!(output, "{indent}| {line}").unwrap();
            }
            visit(
                outline,
                &position.children,
                &path,
                depth + 1,
                emitted,
                output,
            );
        }
    }

    let mut output = String::new();
    visit(
        outline,
        &outline.roots,
        "",
        0,
        &mut HashSet::new(),
        &mut output,
    );
    output
}

/// Return an outline whose roots are the occurrences selected from `outline`.
///
/// GNX selection intentionally returns every occurrence of a cloned vnode.
/// The node table contains only vnodes referenced by the returned subtrees.
pub fn select_subtrees(
    outline: &Outline,
    selector: InspectSelector<'_>,
) -> Result<Outline, InspectError> {
    let roots = match selector {
        InspectSelector::Gnx(gnx) => {
            let mut matches = Vec::new();
            collect_gnx(&outline.roots, gnx, &mut matches);
            if matches.is_empty() {
                return Err(InspectError::NoGnx(gnx.to_owned()));
            }
            matches
        }
        InspectSelector::Position(position) => vec![
            outline
                .position(position)
                .cloned()
                .ok_or_else(|| InspectError::NoPosition(position.0.clone()))?,
        ],
        InspectSelector::File(filename) => {
            let mut matches = Vec::new();
            collect_file(
                outline,
                &outline.roots,
                &[],
                Path::new(filename),
                &mut matches,
            );
            if matches.is_empty() {
                return Err(InspectError::NoFile(filename.to_owned()));
            }
            matches
        }
    };

    let mut referenced = HashSet::new();
    for root in &roots {
        collect_ids(root, &mut referenced);
    }
    let nodes = outline
        .nodes
        .iter()
        .filter(|(id, _)| referenced.contains(*id))
        .map(|(id, node)| (id.clone(), node.clone()))
        .collect();
    Ok(Outline { nodes, roots })
}

fn collect_gnx(positions: &[Position], gnx: &str, matches: &mut Vec<Position>) {
    for position in positions {
        if position.node.0 == gnx {
            matches.push(position.clone());
        } else {
            collect_gnx(&position.children, gnx, matches);
        }
    }
}

fn collect_file(
    outline: &Outline,
    positions: &[Position],
    inherited_paths: &[String],
    wanted: &Path,
    matches: &mut Vec<Position>,
) {
    for position in positions {
        let Some(node) = outline.nodes.get(&position.node) else {
            continue;
        };
        let mut paths = inherited_paths.to_vec();
        if let Some(path) = path_directive(&node.headline).or_else(|| path_directive(&node.body)) {
            paths.push(path);
        }
        if let Some(filename) = external_filename(&node.headline) {
            let mut candidate = PathBuf::new();
            for component in inherited_paths {
                candidate.push(component);
            }
            candidate.push(filename);
            if candidate == wanted || candidate.ends_with(wanted) {
                matches.push(position.clone());
                continue;
            }
        }
        collect_file(outline, &position.children, &paths, wanted, matches);
    }
}

fn collect_ids(position: &Position, ids: &mut HashSet<NodeId>) {
    ids.insert(position.node.clone());
    for child in &position.children {
        collect_ids(child, ids);
    }
}

fn external_filename(headline: &str) -> Option<&str> {
    external_file(headline).map(|(_, filename)| filename)
}

fn external_file(headline: &str) -> Option<(&str, &str)> {
    let (directive, filename) = headline.trim().split_once(char::is_whitespace)?;
    matches!(directive, "@file" | "@thin" | "@file-thin" | "@clean")
        .then(|| (directive, strip_path_cruft(filename)))
        .filter(|(_, filename)| !filename.is_empty())
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

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::Node;

    use super::*;

    fn outline() -> Outline {
        let nodes = [
            ("path", "paths", "@path src"),
            ("file", "@file main.rs", ""),
            ("child", "child", "body"),
            ("other", "other", ""),
        ]
        .into_iter()
        .map(|(id, headline, body)| {
            let id = NodeId(id.into());
            (
                id.clone(),
                Node {
                    id,
                    headline: headline.into(),
                    body: body.into(),
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                },
            )
        })
        .collect();
        let file = Position {
            node: NodeId("file".into()),
            children: vec![Position {
                node: NodeId("child".into()),
                children: vec![],
            }],
        };
        Outline {
            nodes,
            roots: vec![
                Position {
                    node: NodeId("path".into()),
                    children: vec![file.clone()],
                },
                file,
                Position {
                    node: NodeId("other".into()),
                    children: vec![],
                },
            ],
        }
    }

    #[test]
    fn gnx_returns_all_clone_occurrences_and_only_subtree_nodes() {
        let selected = select_subtrees(&outline(), InspectSelector::Gnx("file")).unwrap();
        assert_eq!(selected.roots.len(), 2);
        assert_eq!(selected.nodes.len(), 2);
        assert!(selected.nodes.contains_key(&NodeId("child".into())));
    }

    #[test]
    fn file_matches_resolved_or_basename_path() {
        let selected = select_subtrees(&outline(), InspectSelector::File("src/main.rs")).unwrap();
        assert_eq!(selected.roots.len(), 1);
        let selected = select_subtrees(&outline(), InspectSelector::File("main.rs")).unwrap();
        assert_eq!(selected.roots.len(), 2);
    }

    #[test]
    fn position_selects_one_occurrence() {
        let selected = select_subtrees(
            &outline(),
            InspectSelector::Position(&PositionId("0/0".into())),
        )
        .unwrap();
        assert_eq!(selected.roots.len(), 1);
        assert_eq!(selected.roots[0].node.0, "file");
    }

    #[test]
    fn compact_includes_bodies_and_abbreviates_clones() {
        let rendered = render_compact(&outline());
        assert!(rendered.contains("0 path paths\n| @path src\n"));
        assert!(rendered.contains("  0/0 file @file main.rs\n"));
        assert!(rendered.contains("  | body\n"));
        assert!(rendered.contains("1 =file\n"));
    }

    #[test]
    fn search_returns_merged_line_numbered_excerpts() {
        let mut outline = outline();
        outline.nodes.get_mut(&NodeId("child".into())).unwrap().body =
            "zero\none\nneedle a\nthree\nneedle b\nfive\nsix\nseven\nneedle c".into();
        let matches = search_outline(&outline, &[Regex::new("needle [ab]").unwrap()]);
        assert_eq!(matches.len(), 2); // The child occurs below both file clones.
        assert_eq!(matches[0].position.0, "0/0/0");
        assert_eq!(matches[0].excerpts.len(), 1);
        assert_eq!(matches[0].excerpts[0].start_line, 1);
        assert_eq!(matches[0].excerpts[0].lines.len(), 7);
        let rendered = render_search_compact(&matches[..1]);
        assert!(rendered.contains("| 3: needle a\n"));
    }

    #[test]
    fn multiple_search_patterns_use_or_semantics() {
        let matches = search_outline(
            &outline(),
            &[Regex::new("^paths$").unwrap(), Regex::new("body").unwrap()],
        );
        assert_eq!(matches.len(), 3);
        assert!(matches[0].headline_match);
        assert!(!matches[1].headline_match);
    }

    #[test]
    fn lazily_loads_external_file_for_search_and_gnx() {
        fn external_outline() -> Outline {
            let root = NodeId("root".into());
            Outline {
                nodes: [(
                    root.clone(),
                    Node {
                        id: root.clone(),
                        headline: "@file child.py".into(),
                        body: String::new(),
                        vnode_attributes: HashMap::new(),
                        tnode_attributes: HashMap::new(),
                    },
                )]
                .into_iter()
                .collect(),
                roots: vec![Position {
                    node: root,
                    children: vec![],
                }],
            }
        }

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("cub-inspect-{unique}"));
        fs::create_dir(&directory).unwrap();
        fs::write(
            directory.join("child.py"),
            "#@+leo-ver=5-thin\n#@+node:root: * @file child.py\n#@+node:target: ** class Target\nneedle\n#@-leo\n",
        )
        .unwrap();
        let outline_path = directory.join("outline.leo");

        let mut by_search = external_outline();
        let anchored = Regex::new("^class Target$").unwrap();
        assert_eq!(
            load_matching_external_files(
                &mut by_search,
                &outline_path,
                ExternalFilter::Search(&[anchored]),
            )
            .unwrap(),
            1
        );
        assert!(by_search.nodes.contains_key(&NodeId("target".into())));

        let mut by_gnx = external_outline();
        load_matching_external_files(&mut by_gnx, &outline_path, ExternalFilter::Gnx("target"))
            .unwrap();
        assert!(select_subtrees(&by_gnx, InspectSelector::Gnx("target")).is_ok());
        fs::remove_dir_all(directory).unwrap();
    }
}
