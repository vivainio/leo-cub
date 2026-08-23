//! Loading `@auto`/`@file`/`@thin`/`@file-thin`/`@f`/`@clean` external-file
//! nodes into an outline. Shared by the TUI's own open/reload and by the
//! rhai `Doc` API's `open` (`cub run`), so a script sees exactly the same
//! merged document an interactive session would -- not the bare, unexpanded
//! XML `LeoDocument::open` returns on its own.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use crate::{
    AutoFile, DerivedFile, ExternalFormat, Node, NodeId, Outline, Position, PositionId,
    RelativeFile, WritableExternalFile, comment_delimiters, external_snapshot,
    format_for_directive, referenced_nodes,
};

/// What a [`load_derived_files`]/[`load_derived_jobs`] pass found: newly
/// merged content, source locations for jump-to-source, which nodes are
/// derived (read-only unless also writable-external), which external files
/// are writable, and the on-disk shape to restore before serializing the
/// `.leo` file itself (derived/writable content never gets baked into it).
#[derive(Default)]
pub struct LoadReport {
    pub loaded: usize,
    pub errors: Vec<String>,
    pub locations: HashMap<PositionId, SourceLocation>,
    pub node_locations: HashMap<NodeId, SourceLocation>,
    pub derived_nodes: HashSet<NodeId>,
    pub writable_external: HashMap<NodeId, WritableExternalFile>,
    pub original_children: HashMap<NodeId, Vec<Position>>,
    pub original_bodies: HashMap<NodeId, String>,
    pub original_nodes: HashMap<NodeId, Node>,
}

/// One external-file node to load: `root`/`position` identify it in the
/// outline, `path` is its resolved on-disk location, `auto` is whether it's
/// an `@auto`-family (transient, read-only) directive rather than a
/// writable one, and `directive` is the exact headline directive text.
pub struct DerivedJob {
    pub position: PositionId,
    pub path: PathBuf,
    pub auto: bool,
    pub directive: String,
    pub root: NodeId,
}

#[derive(Clone)]
pub struct SourceLocation {
    pub path: PathBuf,
    pub line: usize,
}

/// Every `@auto`/`@file`/`@thin`/`@file-thin`/`@f`/`@clean` node in
/// `outline`, loaded and merged in place. `outline_path` anchors relative
/// external paths (and `@path` ancestors) to the `.leo` file's own
/// directory.
pub fn load_derived_files(outline: &mut Outline, outline_path: &Path) -> LoadReport {
    let jobs = derived_jobs(outline, outline_path);
    load_derived_jobs(outline, jobs)
}

/// Runs a specific set of derived-file jobs against `outline`, rather than
/// every derived node in it. Used to fetch content for a handful of
/// freshly-created nodes without re-merging -- and so silently discarding
/// unsaved edits to -- every other derived node in the document, which a
/// full [`load_derived_files`] pass would do.
pub fn load_derived_jobs(outline: &mut Outline, jobs: Vec<DerivedJob>) -> LoadReport {
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

pub fn derived_filename(headline: &str) -> Option<(bool, &str, &str)> {
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
pub fn external_format(headline: &str) -> ExternalFormat {
    match headline.trim().split_once(char::is_whitespace) {
        Some((directive, _)) => format_for_directive(directive),
        None => ExternalFormat::Thin,
    }
}

pub fn external_filename(headline: &str) -> Option<&str> {
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

pub fn path_directive(text: &str) -> Option<String> {
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
