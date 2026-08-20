//! Select logical subtrees for machine-readable inspection.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;
use serde::Serialize;
use thiserror::Error;

use crate::{AutoFile, DerivedFile, NodeId, Outline, Position, PositionId};

#[derive(Clone, Copy, Debug)]
pub enum InspectSelector<'a> {
    Gnx(&'a str),
    Position(&'a PositionId),
    File(&'a str),
    Headline(&'a str),
}

#[derive(Debug, Error, PartialEq)]
pub enum InspectError {
    #[error("no subtree has GNX {0}")]
    NoGnx(String),
    #[error("no subtree exists at position {0}")]
    NoPosition(String),
    #[error("no external subtree matches {0:?}")]
    NoFile(String),
    #[error("{0}")]
    Headline(String),
    #[error("failed to parse derived file {path}: {message}")]
    Derived { path: PathBuf, message: String },
    #[error("failed to parse @auto file {path}: {message}")]
    Auto { path: PathBuf, message: String },
    #[error(
        "{parent} has more than one child headlined {headline:?}; json-tree requires unique sibling headlines"
    )]
    DuplicateHeadline { parent: String, headline: String },
    #[error("node {gnx} is headlined {headline:?}, which collides with a json-tree reserved key")]
    ReservedHeadline { gnx: String, headline: String },
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
    enum Loaded {
        Derived(DerivedFile),
        Auto(AutoFile),
    }
    let mut cache: HashMap<(PathBuf, NodeId), Option<Loaded>> = HashMap::new();
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
        let cache_key = (job.path.clone(), job.root.clone());
        if !cache.contains_key(&cache_key) {
            let parsed = match fs::read_to_string(&job.path) {
                Ok(source) => {
                    let content_matches = match filter {
                        ExternalFilter::Search(patterns) => patterns.iter().any(|pattern| {
                            source
                                .lines()
                                .any(|line| raw_line_might_match(line, pattern))
                        }),
                        ExternalFilter::Gnx(gnx) => {
                            job.directive.starts_with("@auto") || source.contains(gnx)
                        }
                        ExternalFilter::File(_) => true,
                    };
                    if content_matches {
                        if job.directive.starts_with("@auto") {
                            Some(Loaded::Auto(
                                AutoFile::parse_with_directive(
                                    &job.path,
                                    job.root.clone(),
                                    &source,
                                    Some(&job.directive),
                                )
                                .map_err(|error| {
                                    InspectError::Auto {
                                        path: job.path.clone(),
                                        message: error.to_string(),
                                    }
                                })?,
                            ))
                        } else {
                            Some(Loaded::Derived(DerivedFile::parse(&source).map_err(
                                |error| InspectError::Derived {
                                    path: job.path.clone(),
                                    message: error.to_string(),
                                },
                            )?))
                        }
                    } else {
                        None
                    }
                }
                Err(_) => None,
            };
            cache.insert(cache_key.clone(), parsed);
        }
        if let Some(expansion) = cache.get(&cache_key).and_then(Option::as_ref) {
            match expansion {
                Loaded::Derived(derived) => {
                    derived
                        .merge_into(outline, &job.position)
                        .map_err(|error| InspectError::Derived {
                            path: job.path.clone(),
                            message: error.to_string(),
                        })?
                }
                Loaded::Auto(auto) => {
                    if !auto.merge_into(outline, &job.position) {
                        return Err(InspectError::Auto {
                            path: job.path.clone(),
                            message: "auto root position disappeared".to_owned(),
                        });
                    }
                }
            }
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
    directive: String,
    root: NodeId,
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
                    directive: directive.to_owned(),
                    root: position.node.clone(),
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

/// Render only the outline hierarchy as a Markdown list.
///
/// A vnode that occurs more than once is shown at its first occurrence and
/// later occurrences are marked as clones. Clone descendants are omitted,
/// matching `render_compact`'s identity-aware rendering.
pub fn render_outline(outline: &Outline) -> String {
    render_outline_with_options(outline, None, false, &[])
}

/// Render an outline with an optional current occurrence and disclosure state.
pub fn render_outline_with_options(
    outline: &Outline,
    current: Option<&PositionId>,
    collapsed: bool,
    expanded: &[PositionId],
) -> String {
    fn escape_headline(headline: &str) -> String {
        headline
            .chars()
            .flat_map(|character| {
                if matches!(character, '\\' | '*' | '_' | '[' | ']' | '`') {
                    vec!['\\', character]
                } else {
                    vec![character]
                }
            })
            .collect()
    }

    fn is_descendant(path: &str, ancestor: &str) -> bool {
        path == ancestor
            || path
                .strip_prefix(ancestor)
                .is_some_and(|rest| rest.starts_with('/'))
    }

    #[allow(clippy::too_many_arguments)]
    fn visit(
        outline: &Outline,
        positions: &[Position],
        parent: &str,
        depth: usize,
        emitted: &mut HashSet<NodeId>,
        output: &mut String,
        current: Option<&str>,
        collapsed: bool,
        expanded: &[PositionId],
        html_details: bool,
    ) {
        for (index, position) in positions.iter().enumerate() {
            let is_last = index + 1 == positions.len();
            let path = if parent.is_empty() {
                index.to_string()
            } else {
                format!("{parent}/{index}")
            };
            let Some(node) = outline.nodes.get(&position.node) else {
                writeln!(
                    output,
                    "{}- <missing node {}>",
                    "  ".repeat(depth),
                    position.node.0
                )
                .unwrap();
                continue;
            };
            let indent = "  ".repeat(depth);
            let is_current = current == Some(path.as_str());
            let is_ancestor =
                current.is_some_and(|value| value != path && is_descendant(value, &path));
            let body_marker = if node.body.trim().is_empty() {
                String::new()
            } else {
                r#" <span class="leo-body-dot" aria-label="has body text" title="has body text"></span>"#
                    .to_owned()
            };
            let headline = format!("{}{}", escape_headline(&node.headline), body_marker);
            let headline = if is_current {
                format!(
                    r#"<span class="leo-outline__current" data-position="{path}" aria-current="page">{headline} <span class="leo-current-label" aria-label="current" title="current"></span></span>"#
                )
            } else if is_ancestor {
                format!(r#"<span class="leo-outline__ancestor">{headline}</span>"#)
            } else {
                headline
            };
            let is_clone = !emitted.insert(position.node.clone());
            if html_details {
                let item_class = if is_last {
                    r#" class="leo-outline__last""#
                } else {
                    ""
                };
                writeln!(output, "{indent}<li{item_class}>").unwrap();
                if !position.children.is_empty() && !is_clone {
                    let is_open = !collapsed
                        || expanded
                            .iter()
                            .any(|target| is_descendant(target.0.as_str(), &path))
                        || current.is_some_and(|value| is_descendant(value, &path));
                    let open = if is_open { " open" } else { "" };
                    writeln!(
                        output,
                        r#"{indent}  <details class="leo-outline__node" data-position="{path}"{open}>"#
                    )
                    .unwrap();
                    writeln!(output, "{indent}    <summary>{headline}</summary>").unwrap();
                    writeln!(output, "{indent}    <ul>").unwrap();
                    visit(
                        outline,
                        &position.children,
                        &path,
                        depth + 1,
                        emitted,
                        output,
                        current,
                        collapsed,
                        expanded,
                        html_details,
                    );
                    writeln!(output, "{indent}    </ul>").unwrap();
                    writeln!(output, "{indent}  </details>").unwrap();
                } else {
                    let suffix = if is_clone { " ↪ clone" } else { "" };
                    writeln!(output, "{indent}  {headline}{suffix}").unwrap();
                }
                writeln!(output, "{indent}</li>").unwrap();
                continue;
            }
            let suffix = if is_clone { " ↪ clone" } else { "" };
            writeln!(output, "{indent}- {headline}{suffix}").unwrap();
            if is_clone {
                continue;
            }
            visit(
                outline,
                &position.children,
                &path,
                depth + 1,
                emitted,
                output,
                current,
                collapsed,
                expanded,
                html_details,
            );
        }
    }

    let mut output = String::new();
    let html_details = collapsed || !expanded.is_empty();
    visit(
        outline,
        &outline.roots,
        "",
        0,
        &mut HashSet::new(),
        &mut output,
        current.map(|value| value.0.as_str()),
        collapsed,
        expanded,
        html_details,
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
        InspectSelector::Headline(path) => vec![
            outline
                .resolve_headline_position(path)
                .cloned()
                .map_err(|error| InspectError::Headline(error.to_string()))?,
        ],
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

/// A `json-tree` node: metadata under reserved `_gnx`/`_body` keys, children
/// addressable by their own headline via `#[serde(flatten)]`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct JsonTreeNode {
    #[serde(rename = "_gnx")]
    pub gnx: String,
    #[serde(rename = "_body")]
    pub body: String,
    #[serde(flatten)]
    pub children: JsonTree,
}

/// Root of a `json-tree` view: outline headlines mapped to their subtrees.
pub type JsonTree = BTreeMap<String, JsonTreeNode>;

/// Reshape `outline` into a tree addressable by headline path instead of by
/// position index or GNX (handy for `nu`'s `get "Headline" | get "Child"`
/// chaining). Every `get` step is guaranteed to return one node record, so
/// sibling headlines must be unique and none may collide with the reserved
/// `_gnx`/`_body` keys; either violation fails the whole conversion rather
/// than silently degrading one node to an array or an error sentinel.
pub fn json_tree(outline: &Outline) -> Result<JsonTree, InspectError> {
    fn build(
        outline: &Outline,
        positions: &[Position],
        parent: &str,
    ) -> Result<JsonTree, InspectError> {
        let mut children = JsonTree::new();
        for position in positions {
            let Some(node) = outline.nodes.get(&position.node) else {
                continue;
            };
            let headline = node.headline.clone();
            if headline == "_gnx" || headline == "_body" {
                return Err(InspectError::ReservedHeadline {
                    gnx: position.node.0.clone(),
                    headline,
                });
            }
            let tree_node = JsonTreeNode {
                gnx: position.node.0.clone(),
                body: node.body.clone(),
                children: build(outline, &position.children, &position.node.0)?,
            };
            if children.insert(headline.clone(), tree_node).is_some() {
                return Err(InspectError::DuplicateHeadline {
                    parent: if parent.is_empty() {
                        "the outline root".to_owned()
                    } else {
                        format!("node {parent}")
                    },
                    headline,
                });
            }
        }
        Ok(children)
    }

    build(outline, &outline.roots, "")
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
    matches!(
        directive,
        "@file" | "@thin" | "@file-thin" | "@clean" | "@auto" | "@auto-md" | "@auto-markdown"
    )
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
    fn headline_selects_only_the_one_occurrence_it_names() {
        let selected =
            select_subtrees(&outline(), InspectSelector::Headline("paths/@file main.rs")).unwrap();
        assert_eq!(selected.roots.len(), 1);
        assert_eq!(selected.roots[0].node, NodeId("file".into()));
        assert!(selected.nodes.contains_key(&NodeId("child".into())));
    }

    #[test]
    fn headline_reports_not_found_and_ambiguous_paths() {
        let error = select_subtrees(&outline(), InspectSelector::Headline("nope")).unwrap_err();
        assert!(matches!(error, InspectError::Headline(_)));
        assert!(error.to_string().contains("not found"));
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
    fn outline_renders_headlines_and_marks_clones() {
        let rendered = render_outline(&outline());
        assert_eq!(
            rendered,
            "- paths <span class=\"leo-body-dot\" aria-label=\"has body text\" title=\"has body text\"></span>\n  - @file main.rs\n    - child <span class=\"leo-body-dot\" aria-label=\"has body text\" title=\"has body text\"></span>\n- @file main.rs ↪ clone\n- other\n"
        );
    }

    #[test]
    fn outline_escapes_markdown_headline_punctuation() {
        let mut outline = outline();
        outline
            .nodes
            .get_mut(&NodeId("other".into()))
            .unwrap()
            .headline = "[literal] *headline*".into();
        assert!(render_outline(&outline).contains(r"- \[literal\] \*headline\*"));
    }

    #[test]
    fn outline_marks_current_path() {
        let rendered =
            render_outline_with_options(&outline(), Some(&PositionId("0/0/0".into())), false, &[]);
        assert!(rendered.contains(
            r#"<span class="leo-outline__ancestor">paths <span class="leo-body-dot" aria-label="has body text" title="has body text"></span></span>"#
        ));
        assert!(rendered.contains(
            r#"<span class="leo-outline__current" data-position="0/0/0" aria-current="page">child <span class="leo-body-dot" aria-label="has body text" title="has body text"></span> <span class="leo-current-label" aria-label="current" title="current"></span></span>"#
        ));
    }

    #[test]
    fn outline_collapses_unselected_branches_and_opens_current_path() {
        let mut outline = outline();
        outline.roots[2].children.push(Position {
            node: NodeId("child".into()),
            children: vec![],
        });
        let rendered =
            render_outline_with_options(&outline, Some(&PositionId("0/0/0".into())), true, &[]);
        assert!(rendered.contains(r#"<details class="leo-outline__node" data-position="0" open>"#));
        assert!(
            rendered.contains(r#"<details class="leo-outline__node" data-position="0/0" open>"#)
        );
        assert!(rendered.contains(r#"<details class="leo-outline__node" data-position="2">"#));
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

    #[test]
    fn lazily_expands_auto_files_with_tree_sitter() {
        let root = NodeId("auto-root".into());
        let mut outline = Outline {
            nodes: [(
                root.clone(),
                Node {
                    id: root.clone(),
                    headline: "@auto child.py".into(),
                    body: String::new(),
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                },
            )]
            .into_iter()
            .collect(),
            roots: vec![Position {
                node: root.clone(),
                children: vec![],
            }],
        };
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("cub-auto-inspect-{unique}"));
        fs::create_dir(&directory).unwrap();
        fs::write(
            directory.join("child.py"),
            "class Target:\n    def needle(self):\n        pass\n",
        )
        .unwrap();

        let count = load_matching_external_files(
            &mut outline,
            &directory.join("outline.leo"),
            ExternalFilter::File("child.py"),
        )
        .unwrap();

        assert_eq!(count, 1);
        assert_eq!(
            outline.nodes[&root].body,
            "@others\n@language python\n@tabwidth -4\n"
        );
        let class = &outline.roots[0].children[0];
        assert_eq!(outline.nodes[&class.node].headline, "class Target");
        assert_eq!(
            outline.nodes[&class.children[0].node].headline,
            "Target.needle"
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn json_tree_addresses_nodes_by_headline_path() {
        let tree = json_tree(&outline()).unwrap();
        let file_node = tree.get("@file main.rs").unwrap();
        assert_eq!(file_node.gnx, "file");
        let child_node = file_node.children.get("child").unwrap();
        assert_eq!(child_node.gnx, "child");
        assert_eq!(child_node.body, "body");
        assert!(child_node.children.is_empty());
    }

    #[test]
    fn json_tree_serializes_metadata_and_children_at_one_level() {
        let mut single = Outline {
            nodes: HashMap::new(),
            roots: vec![Position {
                node: NodeId("only".into()),
                children: vec![],
            }],
        };
        single.nodes.insert(
            NodeId("only".into()),
            Node {
                id: NodeId("only".into()),
                headline: "Only".into(),
                body: "text".into(),
                vnode_attributes: HashMap::new(),
                tnode_attributes: HashMap::new(),
            },
        );
        let tree = json_tree(&single).unwrap();
        let value = serde_json::to_value(&tree).unwrap();
        assert_eq!(
            value,
            serde_json::json!({ "Only": { "_gnx": "only", "_body": "text" } })
        );
    }

    #[test]
    fn json_tree_rejects_duplicate_sibling_headlines() {
        let mut outline = outline();
        outline
            .nodes
            .get_mut(&NodeId("other".into()))
            .unwrap()
            .headline = "paths".into();
        let error = json_tree(&outline).unwrap_err();
        assert_eq!(
            error,
            InspectError::DuplicateHeadline {
                parent: "the outline root".to_owned(),
                headline: "paths".to_owned(),
            }
        );
    }

    #[test]
    fn json_tree_rejects_headline_colliding_with_reserved_key() {
        let mut outline = outline();
        outline
            .nodes
            .get_mut(&NodeId("other".into()))
            .unwrap()
            .headline = "_gnx".into();
        let error = json_tree(&outline).unwrap_err();
        assert_eq!(
            error,
            InspectError::ReservedHeadline {
                gnx: "other".to_owned(),
                headline: "_gnx".to_owned(),
            }
        );
    }

    #[test]
    fn recognizes_and_expands_auto_md_directives() {
        let root = NodeId("md-root".into());
        let mut outline = Outline {
            nodes: [(
                root.clone(),
                Node {
                    id: root.clone(),
                    headline: "@auto-md notes.txt".into(),
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
        };
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("cub-auto-md-{unique}"));
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("notes.txt"), "# Notes\nbody\n").unwrap();
        assert_eq!(
            load_matching_external_files(
                &mut outline,
                &directory.join("outline.leo"),
                ExternalFilter::File("notes.txt"),
            )
            .unwrap(),
            1
        );
        assert_eq!(
            outline.nodes[&outline.roots[0].children[0].node].headline,
            "Notes"
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
