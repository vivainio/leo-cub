//! Synchronize external Leo file nodes into an outline.

use std::{
    collections::{HashMap, HashSet},
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use serde::Serialize;
use thiserror::Error;

use crate::{
    DerivedFile, LeoDocument, Node, NodeId, Outline, Position, PositionId, RelativeFile,
    propagate_clean_changes, referenced_nodes,
};

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("unsupported @clean expansion in node {gnx}: {directive}")]
    UnsupportedExpansion { gnx: String, directive: String },
    #[error("no external node matches {0:?}")]
    NoMatch(String),
    #[error("no external node has GNX {0}")]
    NoGnx(String),
    #[error("cloned external node {gnx} resolves to multiple files: {paths:?}")]
    AmbiguousClone { gnx: String, paths: Vec<PathBuf> },
    #[error(transparent)]
    Sentinel(#[from] crate::SentinelError),
    #[error("sync produced an invalid outline: {0:?}")]
    InvalidOutline(Vec<String>),
    #[error("{path}: {source}")]
    Render {
        path: PathBuf,
        source: Box<SyncError>,
    },
    #[error("{path}: generated invalid {label} file: {source}")]
    GeneratedInvalid {
        path: PathBuf,
        label: &'static str,
        source: crate::SentinelError,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct SyncItem {
    pub gnx: NodeId,
    pub path: PathBuf,
    pub directive: String,
    pub changed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SyncReport {
    pub examined: usize,
    pub changed: usize,
    pub dry_run: bool,
    pub items: Vec<SyncItem>,
}

#[derive(Clone)]
struct Job {
    position: PositionId,
    gnx: NodeId,
    path: PathBuf,
    directive: String,
}

/// Sync selected external files into `document`, without writing it.
pub fn sync_document(
    document: &mut LeoDocument,
    outline_path: &Path,
    filename: Option<&str>,
    gnx: Option<&str>,
    dry_run: bool,
) -> Result<SyncReport, SyncError> {
    let jobs = select_jobs(
        external_jobs(&document.outline, outline_path),
        filename,
        gnx,
    )?;
    let mut next = document.clone();
    let mut items = Vec::new();
    for job in jobs {
        let before = next.outline.clone();
        let source = fs::read_to_string(&job.path).map_err(|source| SyncError::Io {
            path: job.path.clone(),
            source,
        })?;
        if job.directive == "@clean" {
            let (start, end) = comment_delimiters(&job.path);
            let private = render_private(&next.outline, &job.position, start, end)?;
            let updated = propagate_clean_changes(&source, &private, start, end);
            let parsed = DerivedFile::parse(&updated)?;
            parsed.merge_into(&mut next.outline, &job.position)?;
        } else if job.directive == "@f" {
            // Unlike @clean, @f files always carry sentinels, so there is no
            // public/private text to reconcile via Mulder/Ream -- just parse
            // and merge, reconciling gnx-less nodes against the outline's
            // current tree (see RelativeFile::merge_into).
            let parsed = RelativeFile::parse(&source)?;
            parsed.merge_into(&mut next.outline, &job.position)?;
        } else {
            // Leo reconstructs @file trees in memory, but their content remains
            // exclusively in the thin derived file. Validate the derived file and
            // its root GNX without materializing that transient tree in the .leo file.
            let parsed = DerivedFile::parse(&source)?;
            let target_node = next
                .outline
                .position(&job.position)
                .map(|target| target.node.clone())
                .ok_or_else(|| crate::SentinelError::PositionNotFound(job.position.0.clone()))?;
            if target_node != parsed.root {
                return Err(crate::SentinelError::RootMismatch {
                    outline: target_node.0.clone(),
                    derived: parsed.root.0,
                }
                .into());
            }
            if let Some(node) = next.outline.nodes.get_mut(&target_node) {
                node.vnode_attributes.remove("expanded");
            }
        }
        items.push(SyncItem {
            gnx: job.gnx,
            path: job.path,
            directive: job.directive,
            changed: before != next.outline,
        });
    }
    // Leo treats `_mod_time` as a session-only cache and discards values
    // serialized by older releases whenever an outline is refreshed and saved.
    for node in next.outline.nodes.values_mut() {
        node.tnode_attributes.remove("_mod_time");
    }
    let errors = next
        .outline
        .validate()
        .into_iter()
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        return Err(SyncError::InvalidOutline(errors));
    }
    let changed = items.iter().filter(|item| item.changed).count();
    if !dry_run {
        *document = next;
    }
    Ok(SyncReport {
        examined: items.len(),
        changed,
        dry_run,
        items,
    })
}

fn select_jobs(
    jobs: Vec<Job>,
    filename: Option<&str>,
    gnx: Option<&str>,
) -> Result<Vec<Job>, SyncError> {
    let mut by_gnx: HashMap<NodeId, Vec<Job>> = HashMap::new();
    for job in jobs {
        by_gnx.entry(job.gnx.clone()).or_default().push(job);
    }
    let mut unique = Vec::new();
    for (id, mut clones) in by_gnx {
        clones.sort_by(|a, b| a.path.cmp(&b.path));
        clones.dedup_by(|a, b| a.path == b.path);
        if clones.len() > 1 {
            return Err(SyncError::AmbiguousClone {
                gnx: id.0,
                paths: clones.into_iter().map(|job| job.path).collect(),
            });
        }
        unique.push(clones.remove(0));
    }
    unique.sort_by(|a, b| a.path.cmp(&b.path));
    if let Some(gnx) = gnx {
        let selected = unique
            .into_iter()
            .filter(|job| job.gnx.0 == gnx)
            .collect::<Vec<_>>();
        return (!selected.is_empty())
            .then_some(selected)
            .ok_or_else(|| SyncError::NoGnx(gnx.to_owned()));
    }
    if let Some(filename) = filename {
        let wanted = Path::new(filename);
        let selected = unique
            .into_iter()
            .filter(|job| job.path == wanted || job.path.ends_with(wanted))
            .collect::<Vec<_>>();
        return (!selected.is_empty())
            .then_some(selected)
            .ok_or_else(|| SyncError::NoMatch(filename.to_owned()));
    }
    Ok(unique)
}

/// The on-disk path a node's `@file`/`@thin`/`@file-thin`/`@clean`/`@f` body
/// syncs to, accounting for every ancestor `@path` directive -- the same
/// resolution `sync_document` uses to find each external file. `None` if
/// `gnx` isn't itself an external-file node.
pub fn external_file_path(outline: &Outline, outline_path: &Path, gnx: &NodeId) -> Option<PathBuf> {
    external_jobs(outline, outline_path)
        .into_iter()
        .find(|job| &job.gnx == gnx)
        .map(|job| job.path)
}

fn external_jobs(outline: &Outline, outline_path: &Path) -> Vec<Job> {
    fn visit(
        outline: &Outline,
        positions: &[Position],
        parent: &str,
        base: &Path,
        inherited_paths: &[String],
        jobs: &mut Vec<Job>,
    ) {
        for (index, position) in positions.iter().enumerate() {
            let id = if parent.is_empty() {
                index.to_string()
            } else {
                format!("{parent}/{index}")
            };
            let node = &outline.nodes[&position.node];
            let mut paths = inherited_paths.to_vec();
            if let Some(path) =
                path_directive(&node.headline).or_else(|| path_directive(&node.body))
            {
                paths.push(path);
            }
            if let Some((directive, filename)) = external_filename(&node.headline) {
                let mut path = base.to_path_buf();
                for component in inherited_paths {
                    path.push(component);
                }
                path.push(filename);
                jobs.push(Job {
                    position: PositionId(id.clone()),
                    gnx: position.node.clone(),
                    path,
                    directive: directive.to_owned(),
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

fn external_filename(headline: &str) -> Option<(&str, &str)> {
    let (directive, filename) = headline.trim().split_once(char::is_whitespace)?;
    matches!(
        directive,
        "@file" | "@thin" | "@file-thin" | "@clean" | "@f"
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

pub fn comment_delimiters(path: &Path) -> (&'static str, &'static str) {
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
        | "go" | "swift" | "kt" | "kts" | "cs" | "rhai" => ("//", ""),
        "html" | "htm" | "xml" | "xhtml" | "svg" => ("<!--", "-->"),
        "css" | "scss" | "less" => ("/*", "*/"),
        "sql" | "lua" => ("--", ""),
        "ini" | "cfg" => ("#", ""),
        // The private sentinel stream is an internal merge representation and
        // is never written to the external @clean file. A line-comment fallback
        // therefore supports plain-text and otherwise unknown extensions safely.
        _ => ("#", ""),
    }
}

/// Which sentinel writer/parser a directive's derived file uses. `@f` is the
/// only directive using the cub-1-thin relative-depth, optional-gnx grammar
/// (a leo-cub extension inspired by leo-editor issue #4928, not an official
/// Leo version tag); every other thin/file directive still uses the 5-thin
/// grammar in `derived.rs`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExternalFormat {
    /// 5-thin sentinels (`@file`/`@thin`/`@file-thin`): absolute `*N*`
    /// depth, every node keeps its gnx.
    Thin,
    /// cub-1-thin sentinels (`@f`, leo-cub's own format tag -- not an
    /// official Leo version): depth relative to the preceding node, gnx
    /// omitted except for the root, clones, and UA-bearing nodes.
    Relative,
}

pub fn format_for_directive(directive: &str) -> ExternalFormat {
    if directive == "@f" {
        ExternalFormat::Relative
    } else {
        ExternalFormat::Thin
    }
}

/// A `@file`/`@thin`/`@file-thin`/`@f` node whose subtree is authoritative
/// on disk. `original` is the snapshot captured when the file was last
/// loaded or written, used to detect whether the live outline has diverged
/// and needs writing back.
#[derive(Clone)]
pub struct WritableExternalFile {
    pub path: PathBuf,
    pub start_delimiter: String,
    pub end_delimiter: String,
    pub original: Outline,
    pub format: ExternalFormat,
}

/// The on-disk state of every writable external file at load time, captured
/// so a save can restore it into the outline before serializing the `.leo`
/// file -- the live tree only carries freshly generated derived content, not
/// what was last read from disk.
#[derive(Default)]
pub struct OriginalExternalState {
    pub children: HashMap<NodeId, Vec<Position>>,
    pub bodies: HashMap<NodeId, String>,
    /// Node entries for `children`'s subtree, captured at load time. A
    /// derived container's live children get replaced by freshly generated
    /// ones (auto.rs's merge prunes `outline.nodes` down to what the live
    /// tree references), so by save time the *original* child ids are often
    /// gone from `outline.nodes` even though `children` still points at
    /// them. Restoring `children` for serialization needs these node
    /// bodies/headlines to still resolve.
    pub nodes: HashMap<NodeId, Node>,
}

pub struct ExternalUpdate {
    pub root: NodeId,
    pub path: PathBuf,
    pub rendered: String,
    pub snapshot: Outline,
}

/// A read-only snapshot of the subtree rooted at `root`'s current position,
/// used both to detect whether a writable external file has diverged from
/// what was last read/written, and to restore that subtree's disk-only
/// structure before serializing the `.leo` file.
pub fn external_snapshot(outline: &Outline, root: &NodeId) -> Option<(PositionId, Outline)> {
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

/// Renders each writable external file whose live subtree has diverged from
/// `file.original`, without writing anything to disk. Callers should write
/// the returned updates (see [`write_external_updates`]) and then record
/// `update.snapshot` back as the new `original` for each entry.
pub fn prepare_external_updates(
    outline: &Outline,
    writable: &HashMap<NodeId, WritableExternalFile>,
) -> Result<Vec<ExternalUpdate>, SyncError> {
    let mut updates = Vec::new();
    for (root, file) in writable {
        let Some((position, snapshot)) = external_snapshot(outline, root) else {
            continue;
        };
        if snapshot == file.original {
            continue;
        }
        let rendered = match file.format {
            ExternalFormat::Relative => {
                let rendered = render_relative(
                    outline,
                    &position,
                    &file.start_delimiter,
                    &file.end_delimiter,
                )
                .map_err(|error| SyncError::Render {
                    path: file.path.clone(),
                    source: Box::new(error),
                })?;
                RelativeFile::parse(&rendered).map_err(|error| SyncError::GeneratedInvalid {
                    path: file.path.clone(),
                    label: "@f",
                    source: error,
                })?;
                rendered
            }
            ExternalFormat::Thin => {
                let rendered = render_thin(
                    outline,
                    &position,
                    &file.start_delimiter,
                    &file.end_delimiter,
                )
                .map_err(|error| SyncError::Render {
                    path: file.path.clone(),
                    source: Box::new(error),
                })?;
                DerivedFile::parse(&rendered).map_err(|error| SyncError::GeneratedInvalid {
                    path: file.path.clone(),
                    label: "thin",
                    source: error,
                })?;
                rendered
            }
        };
        updates.push(ExternalUpdate {
            root: root.clone(),
            path: file.path.clone(),
            rendered,
            snapshot,
        });
    }
    Ok(updates)
}

/// Writes every update to its target path, all-or-nothing: each file is
/// staged next to its target and only renamed into place once every staged
/// write has succeeded, so a mid-batch failure leaves none of the target
/// files touched.
pub fn write_external_updates(updates: &[ExternalUpdate]) -> Result<(), SyncError> {
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
        let permissions = fs::metadata(&update.path)
            .ok()
            .map(|metadata| metadata.permissions());
        let result = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .and_then(|mut file| {
                file.write_all(update.rendered.as_bytes())?;
                file.sync_all()
            })
            .and_then(|()| {
                permissions.map_or(Ok(()), |permissions| {
                    fs::set_permissions(&temporary, permissions)
                })
            });
        if let Err(error) = result {
            for path in &staged {
                let _ = fs::remove_file(path);
            }
            let _ = fs::remove_file(&temporary);
            return Err(SyncError::Io {
                path: update.path.clone(),
                source: error,
            });
        }
        staged.push(temporary);
    }
    for (update, temporary) in updates.iter().zip(&staged) {
        if let Err(error) = fs::rename(temporary, &update.path) {
            for path in &staged {
                let _ = fs::remove_file(path);
            }
            return Err(SyncError::Io {
                path: update.path.clone(),
                source: error,
            });
        }
    }
    Ok(())
}

/// Restores the disk-only children and bodies captured in `children`/`bodies`
/// (and their supporting `nodes` entries) into `outline`, undoing the
/// in-memory merge so the outline can be serialized to the `.leo` file
/// without persisting the freshly generated derived content.
pub fn restore_external_state(
    outline: &mut Outline,
    children: &HashMap<NodeId, Vec<Position>>,
    bodies: &HashMap<NodeId, String>,
    nodes: &HashMap<NodeId, Node>,
) {
    restore_derived_children(&mut outline.roots, children);
    // The restored children can reference ids that were pruned from
    // outline.nodes once the live tree stopped using them; reinstate them
    // from the load-time snapshot so serialization can still resolve them.
    for (id, node) in nodes {
        outline
            .nodes
            .entry(id.clone())
            .or_insert_with(|| node.clone());
    }
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

pub fn render_thin(
    outline: &Outline,
    target: &PositionId,
    start: &str,
    end: &str,
) -> Result<String, SyncError> {
    let root = outline
        .position(target)
        .ok_or_else(|| crate::SentinelError::PositionNotFound(target.0.clone()))?;
    let mut first = Vec::new();
    let mut last = Vec::new();
    collect_first_last(outline, root, &mut first, &mut last);
    let mut result = String::new();
    for line in first {
        result.push_str(line);
        result.push('\n');
    }
    result.push_str(&format!("{start}@+leo-ver=5-thin{end}\n"));
    render_position(outline, root, 1, "", start, end, &mut result);
    result.push_str(&format!("{start}@-leo{end}\n"));
    for line in last {
        result.push_str(line);
        result.push('\n');
    }
    Ok(result)
}

/// Render an `@f` derived file: like `render_thin`, but the per-node
/// sentinel encodes outline depth relative to the preceding sentinel and
/// omits `[gnx]` for nodes that don't need persistent identity (everything
/// except the root, clones, and nodes carrying user attributes). See
/// leo-editor issue #4928 and `RelativeFile`.
pub fn render_relative(
    outline: &Outline,
    target: &PositionId,
    start: &str,
    end: &str,
) -> Result<String, SyncError> {
    let root = outline
        .position(target)
        .ok_or_else(|| crate::SentinelError::PositionNotFound(target.0.clone()))?;
    let mut first = Vec::new();
    let mut last = Vec::new();
    collect_first_last(outline, root, &mut first, &mut last);
    let mut result = String::new();
    for line in first {
        result.push_str(line);
        result.push('\n');
    }
    result.push_str(&format!("{start}@+leo-ver=cub-1-thin{end}\n"));
    let protected = protected_node_ids(outline, root);
    let mut prev_level = None;
    render_position_relative(
        outline,
        root,
        1,
        &mut prev_level,
        &protected,
        "",
        start,
        end,
        &mut result,
    );
    result.push_str(&format!("{start}@-leo{end}\n"));
    for line in last {
        result.push_str(line);
        result.push('\n');
    }
    Ok(result)
}

/// Nodes an `@f` sentinel must serialize a `[gnx]` for: the file root, any
/// node whose id occupies more than one position anywhere in the whole
/// document (a clone -- scanned document-wide, since a node inside this
/// subtree could also be cloned elsewhere), and any node carrying user
/// attributes. Everything else can be reconstructed from structural position
/// alone. This is a conservative subset of the identity-preserving set
/// described in leo-editor issue #4928 -- it does not attempt to recognize
/// GNX values referenced from inside UA strings.
fn protected_node_ids(outline: &Outline, root: &Position) -> HashSet<NodeId> {
    let mut counts: HashMap<NodeId, usize> = HashMap::new();
    fn visit(positions: &[Position], counts: &mut HashMap<NodeId, usize>) {
        for position in positions {
            *counts.entry(position.node.clone()).or_default() += 1;
            visit(&position.children, counts);
        }
    }
    visit(&outline.roots, &mut counts);
    let mut protected: HashSet<NodeId> = counts
        .into_iter()
        .filter_map(|(id, count)| (count > 1).then_some(id))
        .collect();
    for (id, node) in &outline.nodes {
        if !node.vnode_attributes.is_empty() || !node.tnode_attributes.is_empty() {
            protected.insert(id.clone());
        }
    }
    protected.insert(root.node.clone());
    protected
}

#[allow(clippy::too_many_arguments)]
fn render_position_relative(
    outline: &Outline,
    position: &Position,
    level: usize,
    prev_level: &mut Option<usize>,
    protected: &HashSet<NodeId>,
    indent: &str,
    start: &str,
    end: &str,
    result: &mut String,
) {
    let node = &outline.nodes[&position.node];
    let token = match *prev_level {
        None => "0".to_owned(),
        Some(previous) if level == previous => String::new(),
        Some(previous) if level > previous => {
            let delta = level - previous;
            if delta == 1 {
                ">".to_owned()
            } else {
                format!(">{delta}")
            }
        }
        Some(previous) => {
            let delta = previous - level;
            if delta == 1 {
                "<".to_owned()
            } else {
                format!("<{delta}")
            }
        }
    };
    *prev_level = Some(level);
    let gnx = if protected.contains(&position.node) {
        format!("[{}] ", node.id.0)
    } else {
        String::new()
    };
    result.push_str(&format!(
        "{indent}{start}@{token} {gnx}{}{end}\n",
        node.headline
    ));
    let mut expanded = false;
    for line in node.body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.trim_end() == "@others" {
            expanded = true;
            let leading = &line[..line.len() - trimmed.len()];
            let child_indent = format!("{indent}{leading}");
            result.push_str(&format!("{indent}{leading}{start}@+others{end}\n"));
            for child in position
                .children
                .iter()
                .filter(|child| !is_section_node(outline, child))
            {
                render_position_relative(
                    outline,
                    child,
                    level + 1,
                    prev_level,
                    protected,
                    &child_indent,
                    start,
                    end,
                    result,
                );
            }
            result.push_str(&format!("{indent}{leading}{start}@-others{end}\n"));
        } else if trimmed.trim_end() == "@all" {
            expanded = true;
            let leading = &line[..line.len() - trimmed.len()];
            let child_indent = format!("{indent}{leading}");
            result.push_str(&format!("{child_indent}{start}@+all{end}\n"));
            for child in &position.children {
                render_position_relative(
                    outline,
                    child,
                    level + 1,
                    prev_level,
                    protected,
                    &child_indent,
                    start,
                    end,
                    result,
                );
            }
            result.push_str(&format!("{child_indent}{start}@-all{end}\n"));
        } else if let Some(section) = section_reference(trimmed.trim_end()) {
            expanded = true;
            let leading = &line[..line.len() - trimmed.len()];
            let child_indent = format!("{indent}{leading}");
            result.push_str(&format!("{child_indent}{start}@+{section}{end}\n"));
            if let Some(child) = position
                .children
                .iter()
                .find(|child| outline.nodes[&child.node].headline.trim() == section)
            {
                render_position_relative(
                    outline,
                    child,
                    level + 1,
                    prev_level,
                    protected,
                    &child_indent,
                    start,
                    end,
                    result,
                );
            }
            result.push_str(&format!("{child_indent}{start}@-{section}{end}\n"));
        } else if trimmed.starts_with("@first ") {
            result.push_str(&format!("{indent}{start}@@first{end}\n"));
        } else if trimmed.starts_with("@last ") {
            result.push_str(&format!("{indent}{start}@@last{end}\n"));
        } else if let Some(directive) = trimmed.strip_prefix('@') {
            result.push_str(&format!(
                "{indent}{start}@@{}{end}\n",
                directive.trim_end_matches(['\r', '\n'])
            ));
        } else {
            let rendered = format!("{indent}{line}");
            if is_sentinel_like(&rendered, start, end) {
                result.push_str(&format!("{indent}{start}@verbatim{end}\n"));
            }
            result.push_str(&rendered);
            if !line.ends_with('\n') {
                result.push('\n');
            }
        }
    }
    if !expanded && !position.children.is_empty() {
        result.push_str(&format!("{indent}{start}@+others{end}\n"));
        for child in position
            .children
            .iter()
            .filter(|child| !is_section_node(outline, child))
        {
            render_position_relative(
                outline,
                child,
                level + 1,
                prev_level,
                protected,
                indent,
                start,
                end,
                result,
            );
        }
        result.push_str(&format!("{indent}{start}@-others{end}\n"));
    }
}

fn render_private(
    outline: &Outline,
    target: &PositionId,
    start: &str,
    end: &str,
) -> Result<String, SyncError> {
    let root = outline
        .position(target)
        .ok_or_else(|| crate::SentinelError::PositionNotFound(target.0.clone()))?;
    ensure_supported_clean_tree(outline, root)?;
    render_thin(outline, target, start, end)
}

fn collect_first_last<'a>(
    outline: &'a Outline,
    position: &'a Position,
    first: &mut Vec<&'a str>,
    last: &mut Vec<&'a str>,
) {
    for line in outline.nodes[&position.node].body.lines() {
        if let Some(line) = line.strip_prefix("@first ") {
            first.push(line);
        } else if let Some(line) = line.strip_prefix("@last ") {
            last.push(line);
        }
    }
    for child in &position.children {
        collect_first_last(outline, child, first, last);
    }
}

fn ensure_supported_clean_tree(outline: &Outline, position: &Position) -> Result<(), SyncError> {
    let node = &outline.nodes[&position.node];
    for line in node.body.lines() {
        let line = line.trim();
        if line == "@all" || (line.starts_with("<<") && line.ends_with(">>")) {
            return Err(SyncError::UnsupportedExpansion {
                gnx: node.id.0.clone(),
                directive: line.to_owned(),
            });
        }
    }
    for child in &position.children {
        ensure_supported_clean_tree(outline, child)?;
    }
    Ok(())
}

fn render_position(
    outline: &Outline,
    position: &Position,
    level: usize,
    indent: &str,
    start: &str,
    end: &str,
    result: &mut String,
) {
    let node = &outline.nodes[&position.node];
    let stars = if level <= 5 {
        "*".repeat(level)
    } else {
        format!("*{level}*")
    };
    result.push_str(&format!(
        "{indent}{start}@+node:{}: {stars} {}{end}\n",
        node.id.0, node.headline
    ));
    let mut expanded = false;
    for line in node.body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.trim_end() == "@others" {
            expanded = true;
            let leading = &line[..line.len() - trimmed.len()];
            let child_indent = format!("{indent}{leading}");
            result.push_str(&format!("{indent}{leading}{start}@+others{end}\n"));
            for child in position
                .children
                .iter()
                .filter(|child| !is_section_node(outline, child))
            {
                render_position(outline, child, level + 1, &child_indent, start, end, result);
            }
            result.push_str(&format!("{indent}{leading}{start}@-others{end}\n"));
        } else if trimmed.trim_end() == "@all" {
            expanded = true;
            let leading = &line[..line.len() - trimmed.len()];
            let child_indent = format!("{indent}{leading}");
            result.push_str(&format!("{child_indent}{start}@+all{end}\n"));
            for child in &position.children {
                render_position(outline, child, level + 1, &child_indent, start, end, result);
            }
            result.push_str(&format!("{child_indent}{start}@-all{end}\n"));
        } else if let Some(section) = section_reference(trimmed.trim_end()) {
            expanded = true;
            let leading = &line[..line.len() - trimmed.len()];
            let child_indent = format!("{indent}{leading}");
            result.push_str(&format!("{child_indent}{start}@+{section}{end}\n"));
            if let Some(child) = position
                .children
                .iter()
                .find(|child| outline.nodes[&child.node].headline.trim() == section)
            {
                render_position(outline, child, level + 1, &child_indent, start, end, result);
            }
            result.push_str(&format!("{child_indent}{start}@-{section}{end}\n"));
        } else if trimmed.starts_with("@first ") {
            result.push_str(&format!("{indent}{start}@@first{end}\n"));
        } else if trimmed.starts_with("@last ") {
            result.push_str(&format!("{indent}{start}@@last{end}\n"));
        } else if let Some(directive) = trimmed.strip_prefix('@') {
            result.push_str(&format!(
                "{indent}{start}@@{}{end}\n",
                directive.trim_end_matches(['\r', '\n'])
            ));
        } else {
            let rendered = format!("{indent}{line}");
            if is_sentinel_like(&rendered, start, end) {
                result.push_str(&format!("{indent}{start}@verbatim{end}\n"));
            }
            result.push_str(&rendered);
            if !line.ends_with('\n') {
                result.push('\n');
            }
        }
    }
    if !expanded && !position.children.is_empty() {
        result.push_str(&format!("{indent}{start}@+others{end}\n"));
        for child in position
            .children
            .iter()
            .filter(|child| !is_section_node(outline, child))
        {
            render_position(outline, child, level + 1, indent, start, end, result);
        }
        result.push_str(&format!("{indent}{start}@-others{end}\n"));
    }
}

fn section_reference(line: &str) -> Option<&str> {
    (line.starts_with("<<") && line.ends_with(">>")).then_some(line)
}

fn is_section_node(outline: &Outline, position: &Position) -> bool {
    section_reference(outline.nodes[&position.node].headline.trim()).is_some()
}

fn is_sentinel_like(line: &str, start: &str, end: &str) -> bool {
    let line = line.trim();
    line.starts_with(&format!("{start}@")) && (end.is_empty() || line.ends_with(end))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> LeoDocument {
        let source = concat!(
            r#"<leo_file><vnodes><v t="r"><vh>@clean test.py</vh><v t="c"><vh>child</vh></v></v></vnodes><tnodes><t tx="r">root"#,
            "\n@",
            "others\ntail\n",
            r#"</t><t tx="c">child
</t></tnodes></leo_file>"#,
        );
        LeoDocument::parse(source).unwrap()
    }

    #[test]
    fn rhai_uses_double_slash_comments_not_the_hash_fallback() {
        // Rhai has no `#` line comment -- only `//` and `/* */` -- so a `@f`
        // rhai file sentineled with `#` would be invalid Rhai on write-back.
        assert_eq!(comment_delimiters(Path::new("demo.rhai")), ("//", ""));
    }

    #[test]
    fn renders_clean_tree_as_parseable_private_text() {
        let doc = document();
        let private = render_private(&doc.outline, &PositionId("0".into()), "#", "").unwrap();
        let parsed = DerivedFile::parse(&private).unwrap();
        assert_eq!(
            parsed.outline.nodes[&NodeId::from("r")].body,
            "root\n@others\ntail\n"
        );
        assert_eq!(parsed.outline.nodes[&NodeId::from("c")].body, "child\n");
    }

    #[test]
    fn renders_thin_section_all_and_first_last_expansions() {
        let source = concat!(
            "#!/usr/bin/env python\n",
            "#@+leo-ver=5-thin\n",
            "#@+node:r: * @file test.py\n",
            "#@@first\n",
            "#@+<<imports>>\n",
            "#@+node:s: ** <<imports>>\n",
            "import sys\n",
            "#@-<<imports>>\n",
            "#@+others\n",
            "#@+node:c: ** child\n",
            "#@+all\n",
            "#@+node:g: *3* grandchild\n",
            "body\n",
            "#@-all\n",
            "#@-others\n",
            "#@@last\n",
            "#@-leo\n",
            "# trailing\n",
        );
        let parsed = DerivedFile::parse(source).unwrap();
        let rendered = render_thin(&parsed.outline, &PositionId("0".into()), "#", "").unwrap();
        let reparsed = DerivedFile::parse(&rendered).unwrap();

        assert_eq!(reparsed.outline, parsed.outline);
        assert!(rendered.starts_with("#!/usr/bin/env python\n#@+leo-ver=5-thin\n"));
        assert!(rendered.ends_with("#@-leo\n# trailing\n"));
    }

    #[test]
    fn dry_run_does_not_mutate_document() {
        let mut doc = document();
        let before = doc.outline.clone();
        let dir = std::env::temp_dir().join(format!("leo-cub-sync-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let outline_path = dir.join("outline.leo");
        fs::write(dir.join("test.py"), "changed\nchild\ntail\n").unwrap();
        let report = sync_document(&mut doc, &outline_path, None, None, true).unwrap();
        assert_eq!(report.changed, 1);
        assert_eq!(doc.outline, before);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn file_sync_validates_without_materializing_the_derived_tree() {
        let source = concat!(
            r#"<leo_file><vnodes><v t="r"><vh>@file test.py</vh></v></vnodes>"#,
            r#"<tnodes><t tx="r"></t></tnodes></leo_file>"#,
        );
        let mut doc = LeoDocument::parse(source).unwrap();
        let before = doc.outline.clone();
        let dir =
            std::env::temp_dir().join(format!("leo-cub-file-sync-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let outline_path = dir.join("outline.leo");
        fs::write(
            dir.join("test.py"),
            "#@+leo-ver=5-thin\n#@+node:r: * @file test.py\n#@+others\n#@+node:c: ** child\nbody\n#@-others\n#@-leo\n",
        )
        .unwrap();
        let report = sync_document(&mut doc, &outline_path, None, None, false).unwrap();
        assert_eq!(report.changed, 0);
        assert_eq!(doc.outline, before);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn clean_sync_uses_fallback_delimiters_for_plain_text() {
        let source = concat!(
            r#"<leo_file><vnodes><v t="r"><vh>@clean test.txt</vh></v></vnodes>"#,
            r#"<tnodes><t tx="r">old\n</t></tnodes></leo_file>"#,
        );
        let mut doc = LeoDocument::parse(source).unwrap();
        let dir =
            std::env::temp_dir().join(format!("leo-cub-clean-sync-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let outline_path = dir.join("outline.leo");
        fs::write(dir.join("test.txt"), "new\n").unwrap();
        let report = sync_document(&mut doc, &outline_path, None, None, false).unwrap();
        assert_eq!(report.changed, 1);
        assert_eq!(doc.outline.nodes[&NodeId::from("r")].body, "new\n");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn render_relative_omits_gnx_except_for_protected_nodes() {
        let source = concat!(
            r#"<leo_file><vnodes><v t="r"><vh>@f test.py</vh>"#,
            r#"<v t="ord"><vh>ordinary</vh></v>"#,
            r#"<v t="clone1"><vh>cloned</vh></v>"#,
            r#"<v t="ua" custom="v"><vh>has ua</vh></v>"#,
            r#"</v><v t="clone1"></v></vnodes>"#,
            r#"<tnodes><t tx="r"></t><t tx="ord">a</t><t tx="clone1">b</t>"#,
            r#"<t tx="ua" custom="t">c</t></tnodes></leo_file>"#,
        );
        let doc = LeoDocument::parse(source).unwrap();
        let rendered = render_relative(&doc.outline, &PositionId("0".into()), "#", "").unwrap();

        assert!(rendered.contains("#@0 [r] @f test.py\n"));
        assert!(rendered.contains("#@> ordinary\n"));
        assert!(!rendered.contains("[ord]"));
        assert!(rendered.contains("[clone1] cloned\n"));
        assert!(rendered.contains("[ua] has ua\n"));

        let parsed = RelativeFile::parse(&rendered).unwrap();
        assert_eq!(parsed.root, NodeId::from("r"));
        assert_eq!(parsed.outline.roots[0].children.len(), 3);
        assert_eq!(
            parsed.outline.roots[0].children[1].node,
            NodeId::from("clone1")
        );
    }

    #[test]
    fn f_sync_reconciles_anonymous_nodes_and_keeps_clone_identity() {
        let source = concat!(
            r#"<leo_file><vnodes><v t="r"><vh>@f test.py</vh>"#,
            r#"<v t="ord"><vh>ordinary</vh></v>"#,
            r#"</v></vnodes>"#,
            r#"<tnodes><t tx="r"></t><t tx="ord">old body</t></tnodes></leo_file>"#,
        );
        let mut doc = LeoDocument::parse(source).unwrap();
        let dir = std::env::temp_dir().join(format!("leo-cub-f-sync-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let outline_path = dir.join("outline.leo");
        fs::write(
            dir.join("test.py"),
            "#@+leo-ver=cub-1-thin\n#@0 [r] @f test.py\n#@+others\n#@> ordinary\nnew body\n#@-others\n#@-leo\n",
        )
        .unwrap();
        let report = sync_document(&mut doc, &outline_path, None, None, false).unwrap();
        assert_eq!(report.changed, 1);
        assert_eq!(doc.outline.roots[0].children[0].node, NodeId::from("ord"));
        assert_eq!(doc.outline.nodes[&NodeId::from("ord")].body, "new body\n");
        let _ = fs::remove_dir_all(dir);
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
                Node {
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
            &HashMap::new(),
        );
        assert!(outline.nodes[&NodeId::from("file")].body.is_empty());
    }

    #[test]
    fn restoring_external_state_reinstates_pruned_original_child_nodes() {
        // A derived container's on-disk children (captured into
        // original_children/original_nodes before the fresh auto/derived
        // content is merged in) can reference node ids that auto.rs's
        // merge_into then prunes out of outline.nodes entirely, since the
        // live tree no longer uses them. restore_external_state reattaches
        // those original children before serializing, so it must also bring
        // their node entries back -- otherwise the resulting tree has
        // positions pointing at node ids missing from outline.nodes, and
        // serializing it panics.
        let mut document = LeoDocument::parse(
            r#"<leo_file><vnodes><v t="container"><vh>@auto x.py</vh><v t="fresh"><vh>fresh</vh></v></v></vnodes><tnodes><t tx="container"></t><t tx="fresh">fresh body</t></tnodes></leo_file>"#,
        )
        .unwrap();

        let old_child = NodeId::from("stale-on-disk-child");
        let original_children = HashMap::from([(
            NodeId::from("container"),
            vec![Position {
                node: old_child.clone(),
                children: vec![],
            }],
        )]);
        let original_nodes = HashMap::from([(
            old_child.clone(),
            Node {
                id: old_child.clone(),
                headline: "stale headline".into(),
                body: "stale body".into(),
                vnode_attributes: HashMap::new(),
                tnode_attributes: HashMap::new(),
            },
        )]);
        // Simulates merge_into's blanket prune: the live tree only
        // references "fresh", so the stale id is already gone from
        // outline.nodes even though original_children still points at it.
        assert!(!document.outline.nodes.contains_key(&old_child));

        restore_external_state(
            &mut document.outline,
            &original_children,
            &HashMap::new(),
            &original_nodes,
        );

        assert_eq!(
            document.outline.roots[0].children,
            vec![Position {
                node: old_child.clone(),
                children: vec![],
            }]
        );
        assert!(document.outline.nodes.contains_key(&old_child));
        assert!(document.to_xml().is_ok());
    }
}
