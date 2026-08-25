//! Synchronize external Leo file nodes into an outline.

use std::{
    collections::{HashMap, HashSet},
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use rayon::prelude::*;
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
    #[error("{path}: {source}")]
    Save {
        path: PathBuf,
        source: crate::LeoXmlError,
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

/// Starts (or updates) write-back tracking for `node` -- the bookkeeping a
/// headline rename into an external directive (`@file`/`@thin`/
/// `@file-thin`/`@f`/`@clean`) needs so a later [`save_document`] renders
/// and writes it. `path` is the caller's already-resolved on-disk path for
/// the new headline's filename; resolving it (including any ancestor
/// `@path` directives) is the caller's job, since that resolution differs
/// between the TUI (walks ancestors) and the rhai `Doc` API (doesn't).
/// Shared by `tui::handle_headline_input` and `Doc::set_headline` so both
/// track a rename identically.
pub fn track_external_rename(
    writable_external: &mut HashMap<NodeId, WritableExternalFile>,
    node: NodeId,
    path: PathBuf,
    format: ExternalFormat,
) {
    let (start_delimiter, end_delimiter) = comment_delimiters(&path);
    writable_external
        .entry(node)
        .and_modify(|file| {
            file.path = path.clone();
            file.start_delimiter = start_delimiter.to_owned();
            file.end_delimiter = end_delimiter.to_owned();
            file.format = format;
        })
        .or_insert(WritableExternalFile {
            path,
            start_delimiter: start_delimiter.to_owned(),
            end_delimiter: end_delimiter.to_owned(),
            original: Outline::default(),
            format,
        });
}

/// Writes any diverged `writable_external` entry to its own file with
/// sentinels, then serializes `document`'s outline to `.leo` XML at `path`
/// -- with derived/writable content restored to its pre-merge, on-disk
/// shape first (via `original_external`), so it isn't baked into the
/// `.leo` file itself. On success, updates each written entry's `original`
/// snapshot in place, so a later call only re-renders what changed since.
/// Shared by `tui::save` and the rhai `Doc` API's `save`/`save_as`, so a
/// script's `open`/`set_headline`/`save` produces the same on-disk result
/// the same steps in the TUI would.
pub fn save_document(
    document: &LeoDocument,
    path: &Path,
    writable_external: &mut HashMap<NodeId, WritableExternalFile>,
    original_external: &OriginalExternalState,
) -> Result<(), SyncError> {
    let external_updates = prepare_external_updates(&document.outline, writable_external)?;
    let mut persisted = document.clone();
    restore_external_state(
        &mut persisted.outline,
        &original_external.children,
        &original_external.bodies,
        &original_external.nodes,
    );
    let referenced = referenced_nodes(&persisted.outline.roots);
    persisted
        .outline
        .nodes
        .retain(|id, _| referenced.contains(id));
    write_external_updates(&external_updates)?;
    persisted.save(path).map_err(|source| SyncError::Save {
        path: path.to_path_buf(),
        source,
    })?;
    for update in external_updates {
        if let Some(file) = writable_external.get_mut(&update.root) {
            file.original = update.snapshot;
        }
    }
    Ok(())
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

/// Every position's id and node, indexed by node id for O(1) lookup --
/// built with a single walk of `outline.roots` so a caller checking many
/// roots (see `prepare_external_updates`) doesn't re-walk the whole tree
/// once per root. Borrows rather than clones each position during the
/// walk -- `Position` clones its whole child subtree, so eagerly cloning
/// every node visited (instead of just the ones a caller actually asks
/// for) would trade the old O(roots x tree size) search for an equally
/// bad O(tree size) of wasted deep clones. A node found at more than one
/// position (a clone) keeps only the first occurrence, matching the old
/// recursive-search behavior this replaced.
fn position_index(outline: &Outline) -> HashMap<NodeId, (PositionId, &Position)> {
    fn visit<'a>(
        positions: &'a [Position],
        parent: &str,
        index: &mut HashMap<NodeId, (PositionId, &'a Position)>,
    ) {
        for (i, position) in positions.iter().enumerate() {
            let id = if parent.is_empty() {
                i.to_string()
            } else {
                format!("{parent}/{i}")
            };
            index
                .entry(position.node.clone())
                .or_insert_with(|| (PositionId(id.clone()), position));
            visit(&position.children, &id, index);
        }
    }
    let mut index = HashMap::new();
    visit(&outline.roots, "", &mut index);
    index
}

/// A read-only snapshot of the subtree rooted at `root`'s current position,
/// used both to detect whether a writable external file has diverged from
/// what was last read/written, and to restore that subtree's disk-only
/// structure before serializing the `.leo` file.
pub fn external_snapshot(outline: &Outline, root: &NodeId) -> Option<(PositionId, Outline)> {
    snapshot_at(outline, &position_index(outline), root)
}

/// Same as [`external_snapshot`], but for a caller that already knows the
/// exact `PositionId` (e.g. having just merged a derived file in at that
/// position) -- an O(depth) direct lookup via [`Outline::position`] instead
/// of an O(tree size) search by node id. Loading a document's derived files
/// calls this once per external node, so using [`external_snapshot`]'s
/// by-id search there would cost O(external nodes x tree size) all over
/// again for the same reason `prepare_external_updates` used to.
pub fn external_snapshot_at(
    outline: &Outline,
    position_id: &PositionId,
) -> Option<(PositionId, Outline)> {
    let tree = outline.position(position_id)?;
    let ids = referenced_nodes(std::slice::from_ref(tree));
    let nodes = ids
        .into_iter()
        .filter_map(|id| outline.nodes.get(&id).cloned().map(|node| (id, node)))
        .collect();
    Some((
        position_id.clone(),
        Outline {
            roots: vec![tree.clone()],
            nodes,
        },
    ))
}

fn snapshot_at(
    outline: &Outline,
    index: &HashMap<NodeId, (PositionId, &Position)>,
    root: &NodeId,
) -> Option<(PositionId, Outline)> {
    let (position, tree) = index.get(root)?;
    let ids = referenced_nodes(std::slice::from_ref(tree));
    let nodes = ids
        .into_iter()
        .filter_map(|id| outline.nodes.get(&id).cloned().map(|node| (id, node)))
        .collect();
    Some((
        position.clone(),
        Outline {
            roots: vec![(*tree).clone()],
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
    let index = position_index(outline);
    // Every step here -- snapshotting, rendering, and validating the
    // rendered text -- only ever reads `outline`/`index`, so rendering each
    // writable root is independent of every other one and safe to run in
    // parallel.
    writable
        .par_iter()
        .filter_map(|(root, file)| render_update(outline, &index, root, file).transpose())
        .collect()
}

/// Renders `root`'s writable external file if its live subtree has diverged
/// from `file.original`, or returns `Ok(None)` if it's unchanged.
fn render_update(
    outline: &Outline,
    index: &HashMap<NodeId, (PositionId, &Position)>,
    root: &NodeId,
    file: &WritableExternalFile,
) -> Result<Option<ExternalUpdate>, SyncError> {
    let Some((position, snapshot)) = snapshot_at(outline, index, root) else {
        return Ok(None);
    };
    if snapshot == file.original {
        return Ok(None);
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
    Ok(Some(ExternalUpdate {
        root: root.clone(),
        path: file.path.clone(),
        rendered,
        snapshot,
    }))
}

/// Writes every update to its target path, all-or-nothing: each file is
/// staged next to its target and only renamed into place once every staged
/// write has succeeded, so a mid-batch failure leaves none of the target
/// files touched.
pub fn write_external_updates(updates: &[ExternalUpdate]) -> Result<(), SyncError> {
    let staged: Vec<PathBuf> = updates
        .iter()
        .enumerate()
        .map(|(index, update)| {
            let name = update
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("external");
            update
                .path
                .with_file_name(format!(".{name}.cub-save-{}-{index}", std::process::id()))
        })
        .collect();

    // Each update stages to its own pre-computed temp path, so staging is
    // independent per file -- and staging (open, write, fsync) is exactly
    // where wall-clock time goes for a large batch of external files.
    if let Err(error) = updates
        .par_iter()
        .zip(staged.par_iter())
        .try_for_each(|(update, temporary)| stage_update(update, temporary))
    {
        for path in &staged {
            let _ = fs::remove_file(path);
        }
        return Err(error);
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

/// Writes `update.rendered` to `temporary`, preserving the target's existing
/// permissions if it has any. Safe to run concurrently across updates: each
/// one owns a distinct pre-computed temp path.
fn stage_update(update: &ExternalUpdate, temporary: &Path) -> Result<(), SyncError> {
    let permissions = fs::metadata(&update.path)
        .ok()
        .map(|metadata| metadata.permissions());
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)
        .and_then(|mut file| {
            file.write_all(update.rendered.as_bytes())?;
            file.sync_all()
        })
        .and_then(|()| {
            permissions.map_or(Ok(()), |permissions| {
                fs::set_permissions(temporary, permissions)
            })
        })
        .map_err(|error| SyncError::Io {
            path: update.path.clone(),
            source: error,
        })
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
    render_position(outline, root, 1, false, "", start, end, &mut result);
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
        false,
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
    in_all: bool,
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

    if in_all {
        render_body_under_all(node, indent, start, end, result);
        for child in &position.children {
            render_position_relative(
                outline, child, level + 1, prev_level, protected, true, indent, start, end,
                result,
            );
        }
        return;
    }

    let mut expanded = false;
    for line in node.body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(others_tail) = directive_tail(trimmed.trim_end(), "@others") {
            expanded = true;
            let leading = &line[..line.len() - trimmed.len()];
            let child_indent = format!("{indent}{leading}");
            let suffix = if others_tail.is_empty() {
                String::new()
            } else {
                format!(" {others_tail}")
            };
            result.push_str(&format!("{indent}{leading}{start}@+others{suffix}{end}\n"));
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
                    false,
                    &child_indent,
                    start,
                    end,
                    result,
                );
            }
            result.push_str(&format!("{indent}{leading}{start}@-others{end}\n"));
        } else if let Some(all_tail) = directive_tail(trimmed.trim_end(), "@all") {
            expanded = true;
            let leading = &line[..line.len() - trimmed.len()];
            let child_indent = format!("{indent}{leading}");
            let suffix = if all_tail.is_empty() {
                String::new()
            } else {
                format!(" {all_tail}")
            };
            result.push_str(&format!("{child_indent}{start}@+all{suffix}{end}\n"));
            for child in &position.children {
                render_position_relative(
                    outline,
                    child,
                    level + 1,
                    prev_level,
                    protected,
                    true,
                    &child_indent,
                    start,
                    end,
                    result,
                );
            }
            result.push_str(&format!("{child_indent}{start}@-all{end}\n"));
        } else if let Some(section) = section_reference(trimmed.trim_end()) {
            // Deliberately does *not* set `expanded`: unlike `@others`/
            // `@all`, a bare section reference accounts for only the one
            // named child, not the rest -- real Leo's writer reaches any
            // further, non-section-shaped children of this same node via
            // a thread-order walk seeded by an *ancestor's* `@others`
            // (`at.putAtOthersLine`'s `moveToThreadNext` when a node's own
            // `putBody` reports no `@others`), not via anything in this
            // node's own body. The trailing implicit-children loop below
            // is what stands in for that walk here, and must still run.
            let leading = &line[..line.len() - trimmed.len()];
            let child_indent = format!("{indent}{leading}");
            result.push_str(&format!("{child_indent}{start}@+{section}{end}\n"));
            if let Some(child) = position
                .children
                .iter()
                .find(|child| headline_matches_section_reference(&outline.nodes[&child.node].headline, section))
            {
                render_position_relative(
                    outline,
                    child,
                    level + 1,
                    prev_level,
                    protected,
                    false,
                    &child_indent,
                    start,
                    end,
                    result,
                );
            }
            result.push_str(&format!("{child_indent}{start}@-{section}{end}\n"));
        } else if line.starts_with("@first ") {
            // Real Leo's `directiveKind4` (leoAtFile.py) only recognizes a
            // directive word -- including `@first`/`@last` -- when the `@`
            // is the line's very first character; every directive *except*
            // `@others`/`@all` requires column zero (those two alone skip
            // leading whitespace explicitly). So this checks the raw
            // `line`, not the whitespace-trimmed `trimmed` used above for
            // `@others`/`@all`: an *indented* `@first ...`/`@last ...` (a
            // docstring's usage example, say) isn't a directive at all and
            // must fall through to the plain-text `else` branch below,
            // unescaped -- unlike here, real Leo's own `@first`/`@last`
            // handling drops whatever follows the directive word, so this
            // form must stay column-0-only to avoid losing that text.
            result.push_str(&format!("{indent}{start}@@first{end}\n"));
        } else if line.starts_with("@last ") {
            result.push_str(&format!("{indent}{start}@@last{end}\n"));
        } else if line.strip_prefix('@').is_some_and(is_leo_directive_word) {
            // A bare `@`-prefixed body line (no comment prefix yet -- that
            // only gets added below) can never be mistaken for a real
            // sentinel on a later parse regardless of what follows the
            // `@`: every node-marker/sentinel pattern this crate parses
            // requires the comment prefix *before* the `@` (`#@+node:...`,
            // `#@0 ...`, ...), which a plain body line doesn't have until
            // the (unescaped) `else` branch below adds it back -- and
            // `is_sentinel_like` there already guards against a line that
            // coincidentally starts with the full comment-prefixed
            // sentinel shape. So the only real reason to escape here is a
            // genuine Leo directive name (`is_leo_directive_word`); no
            // separate `@0`/`@>`/`@<`-shaped collision check is needed.
            // And per the `@first`/`@last` comment above, this too must
            // check the raw (untrimmed) `line`, not `trimmed`: real Leo
            // doesn't treat an indented `@language`/`@path`/etc. line as a
            // directive either.
            let directive = line.strip_prefix('@').expect("checked above");
            result.push_str(&format!(
                "{indent}{start}@@{}{end}\n",
                directive.trim_end_matches(['\r', '\n'])
            ));
        } else {
            let rendered = if line == "\n" || line.is_empty() {
                line.to_string()
            } else {
                format!("{indent}{line}")
            };
            if is_sentinel_like(&rendered, start, end) {
                result.push_str(&format!("{indent}{start}@verbatim{end}\n"));
            }
            result.push_str(&rendered);
            if !line.ends_with('\n') {
                result.push('\n');
            }
        }
    }
    if !expanded {
        // No explicit `@others`/`@all`/section reference in the body:
        // emit children as a flat leveled sequence, same as real Leo does
        // for structural-only nesting. A synthetic `@+others` wrapper
        // isn't needed -- the parser attaches children purely from level
        // tokens (see `RelativeFile::parse`), not from wrapper sentinels
        // -- and adding one anyway just inflates the derived file with
        // noise relative to canonical Leo's own output for the same tree.
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
                false,
                indent,
                start,
                end,
                result,
            );
        }
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
    in_all: bool,
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

    if in_all {
        render_body_under_all(node, indent, start, end, result);
        for child in &position.children {
            render_position(outline, child, level + 1, true, indent, start, end, result);
        }
        return;
    }

    let mut expanded = false;
    for line in node.body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(others_tail) = directive_tail(trimmed.trim_end(), "@others") {
            expanded = true;
            let leading = &line[..line.len() - trimmed.len()];
            let child_indent = format!("{indent}{leading}");
            let suffix = if others_tail.is_empty() {
                String::new()
            } else {
                format!(" {others_tail}")
            };
            result.push_str(&format!("{indent}{leading}{start}@+others{suffix}{end}\n"));
            for child in position
                .children
                .iter()
                .filter(|child| !is_section_node(outline, child))
            {
                render_position(
                    outline,
                    child,
                    level + 1,
                    false,
                    &child_indent,
                    start,
                    end,
                    result,
                );
            }
            result.push_str(&format!("{indent}{leading}{start}@-others{end}\n"));
        } else if let Some(all_tail) = directive_tail(trimmed.trim_end(), "@all") {
            expanded = true;
            let leading = &line[..line.len() - trimmed.len()];
            let child_indent = format!("{indent}{leading}");
            let suffix = if all_tail.is_empty() {
                String::new()
            } else {
                format!(" {all_tail}")
            };
            result.push_str(&format!("{child_indent}{start}@+all{suffix}{end}\n"));
            for child in &position.children {
                render_position(
                    outline,
                    child,
                    level + 1,
                    true,
                    &child_indent,
                    start,
                    end,
                    result,
                );
            }
            result.push_str(&format!("{child_indent}{start}@-all{end}\n"));
        } else if let Some(section) = section_reference(trimmed.trim_end()) {
            // See the matching comment in `render_position_relative`:
            // deliberately doesn't set `expanded` -- a bare section
            // reference accounts for only its one named child, so the
            // trailing implicit-children loop must still run to reach any
            // further, non-section-shaped children of this node.
            let leading = &line[..line.len() - trimmed.len()];
            let child_indent = format!("{indent}{leading}");
            result.push_str(&format!("{child_indent}{start}@+{section}{end}\n"));
            if let Some(child) = position
                .children
                .iter()
                .find(|child| headline_matches_section_reference(&outline.nodes[&child.node].headline, section))
            {
                render_position(
                    outline,
                    child,
                    level + 1,
                    false,
                    &child_indent,
                    start,
                    end,
                    result,
                );
            }
            result.push_str(&format!("{child_indent}{start}@-{section}{end}\n"));
        } else if line.starts_with("@first ") {
            // See render_position_relative's matching branch: real Leo's
            // `directiveKind4` requires column zero for every directive
            // except `@others`/`@all`, so this (and the branches below)
            // check the raw `line`, not the whitespace-trimmed `trimmed`
            // used above for `@others`/`@all` -- an indented `@first ...`
            // is plain text, not a directive, and must fall through to the
            // unescaped `else` branch instead of losing its tail text.
            result.push_str(&format!("{indent}{start}@@first{end}\n"));
        } else if line.starts_with("@last ") {
            result.push_str(&format!("{indent}{start}@@last{end}\n"));
        } else if line.strip_prefix('@').is_some_and(is_leo_directive_word) {
            // See render_position_relative's matching branch: only a real
            // Leo directive name needs escaping here, not every `@`-led
            // line -- a bare `@`/`@data whatever` goes out unescaped,
            // matching `at.directiveKind4`.
            let directive = line.strip_prefix('@').expect("checked above");
            result.push_str(&format!(
                "{indent}{start}@@{}{end}\n",
                directive.trim_end_matches(['\r', '\n'])
            ));
        } else {
            let rendered = if line == "\n" || line.is_empty() {
                line.to_string()
            } else {
                format!("{indent}{line}")
            };
            if is_sentinel_like(&rendered, start, end) {
                result.push_str(&format!("{indent}{start}@verbatim{end}\n"));
            }
            result.push_str(&rendered);
            if !line.ends_with('\n') {
                result.push('\n');
            }
        }
    }
    if !expanded {
        // No explicit `@others`/`@all`/section reference in the body:
        // emit children as a flat leveled sequence, same as real Leo does
        // for structural-only nesting. A synthetic `@+others` wrapper
        // isn't needed -- the parser attaches children purely from level
        // tokens (see `DerivedFile::parse`), not from wrapper sentinels --
        // and adding one anyway just inflates the derived file with noise
        // relative to canonical Leo's own output for the same tree.
        for child in position
            .children
            .iter()
            .filter(|child| !is_section_node(outline, child))
        {
            render_position(outline, child, level + 1, false, indent, start, end, result);
        }
    }
}

fn section_reference(line: &str) -> Option<&str> {
    (line.starts_with("<<") && line.ends_with(">>")).then_some(line)
}

/// A headline denotes a section-definition node if it starts with `<<` and
/// closes with `>>` -- Leo allows (and the corpus uses) a trailing
/// disambiguating `" (filename.py)"` suffix after that close, e.g.
/// `<< define regexes >> (leoGlobals.py)`, so this checks for the `>>`
/// anywhere rather than requiring the headline to literally end with it
/// (unlike `section_reference`, which matches a body *reference* line that
/// carries no such suffix).
fn is_section_node(outline: &Outline, position: &Position) -> bool {
    let headline = outline.nodes[&position.node].headline.trim();
    headline.starts_with("<<") && headline.contains(">>")
}

/// Mirrors `VNode.matchHeadline` in canonical Leo's `leoNodes.py`: a
/// section reference (`<<name>>`, found verbatim in a body line) matches a
/// candidate child's headline case- and whitespace-insensitively, and as a
/// *prefix* -- so the headline may carry trailing text after the reference
/// (most commonly the `" (filename)"` disambiguator Leo appends when two
/// same-named sections exist in different files).
fn headline_matches_section_reference(headline: &str, reference: &str) -> bool {
    fn normalize(s: &str) -> String {
        s.chars()
            .filter(|c| *c != ' ' && *c != '\t')
            .flat_map(char::to_lowercase)
            .collect()
    }
    let headline = normalize(headline);
    let headline = headline.trim_start_matches('.');
    let reference = normalize(reference);
    headline.starts_with(&reference)
}

fn is_sentinel_like(line: &str, start: &str, end: &str) -> bool {
    let line = line.trim();
    line.starts_with(&format!("{start}@")) && (end.is_empty() || line.ends_with(end))
}

/// Leo's own recognized directive names (`g.globalDirectiveList` in
/// `leoGlobals.py`), used by `is_leo_directive_word` to tell an actual
/// directive (`@language python`) from body text that merely starts with
/// `@` (a Python decorator, an `@`-prefixed docstring line, ...).
const GLOBAL_DIRECTIVES: &[&str] = &[
    "all",
    "beautify",
    "c",
    "code",
    "color",
    "colorcache",
    "comment",
    "delims",
    "doc",
    "encoding",
    "first",
    "header",
    "ignore",
    "killbeautify",
    "killcolor",
    "language",
    "last",
    "lineending",
    "markup",
    "nobeautify",
    "nocolor-node",
    "nocolor",
    "noheader",
    "nowrap",
    "nopyflakes",
    "nosearch",
    "others",
    "pagewidth",
    "path",
    "quiet",
    "section-delims",
    "silent",
    "tabwidth",
    "unit",
    "verbose",
    "wrap",
];

/// Whether `after_at` (the text right after a body line's leading `@`)
/// would be misread as a node-boundary marker on a later parse: cub-1-thin
/// node markers are bare tokens -- `@0 `, `@`, `@< `, `@> `, or
/// `@<N `/`@>N ` (N = digits) -- before a space or end of line, unlike
/// classic Leo's `@+node:gnx: ...` markers, which a line merely starting
/// with a digit or `<`/`>` can never collide with.
fn is_cub_node_marker_collision(after_at: &str) -> bool {
    if after_at.is_empty() || after_at.starts_with(char::is_whitespace) {
        return true;
    }
    if let Some(rest) = after_at.strip_prefix('0') {
        return rest.is_empty() || rest.starts_with(char::is_whitespace);
    }
    for prefix in ['<', '>'] {
        if let Some(rest) = after_at.strip_prefix(prefix) {
            let digits_end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            let tail = &rest[digits_end..];
            return tail.is_empty() || tail.starts_with(char::is_whitespace);
        }
    }
    false
}

/// Whether `after_at` names a real Leo directive, mirroring classic Leo's
/// `at.directiveKind4` (`leoAtFile.py`): the leading word must be in
/// [`GLOBAL_DIRECTIVES`], *and* not immediately followed by `.` or `(` --
/// that combination marks a decorator or method call instead (`@cmd('x')`,
/// `@g.trace(...)`), which `directiveKind4` explicitly carves out to tell
/// Leo directives from Python decorators.
fn is_leo_directive_word(after_at: &str) -> bool {
    let word_end = after_at
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .unwrap_or(after_at.len());
    if word_end == 0 {
        return false;
    }
    let word = &after_at[..word_end];
    let tail = &after_at[word_end..];
    if tail.starts_with('.') || tail.starts_with('(') {
        return false;
    }
    GLOBAL_DIRECTIVES.contains(&word)
}

/// Whether `line` (a body line, trimmed of leading/trailing whitespace) is
/// an `@others`/`@all` directive -- optionally followed by trailing text,
/// e.g. `@others # helper functions` -- and if so, that trailing text
/// (`""` if there is none). Real Leo's own `others_pat`/`all_pat`
/// (`leoAtFile.py`) match `@(+|-)others\b(.*)` / `@(+|-)all\b(.*)`, a `\b`
/// word boundary rather than requiring the line to be exactly `@others`/
/// `@all` -- and `putAtOthersLine` echoes that trailing text back into the
/// `@+others`/`@+all` open sentinel it writes. Without this, a line like
/// `@others # helper functions` falls through to the generic `@`-escaping
/// path instead of expanding, silently promoting its children out to
/// top-level siblings instead of nesting them where the body asked.
fn directive_tail<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(keyword)?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then(|| rest.trim())
}

/// Renders a node's body while already inside an ancestor's `@all`
/// expansion. Real Leo's `putAtAllBody` (`leoAtFile.py`) writes body text
/// completely unconditionally there, with zero directive scanning -- `@all`
/// is already exhaustive, so a literal `@others`/`@language`/... appearing
/// in a descendant's body under it is just inert text, never a directive.
/// This mirrors that: no `@others`/`@all`/section-reference expansion, no
/// Leo-directive-name escaping -- only the escaping cub's own parser
/// genuinely needs regardless of context (a line that would collide with
/// cub's node-marker syntax, or one that already happens to start with the
/// sentinel prefix).
fn render_body_under_all(node: &Node, indent: &str, start: &str, end: &str, result: &mut String) {
    for line in node.body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed
            .strip_prefix('@')
            .is_some_and(is_cub_node_marker_collision)
        {
            let directive = trimmed.strip_prefix('@').expect("checked above");
            result.push_str(&format!(
                "{indent}{start}@@{}{end}\n",
                directive.trim_end_matches(['\r', '\n'])
            ));
        } else {
            let rendered = if line == "\n" || line.is_empty() {
                line.to_string()
            } else {
                format!("{indent}{line}")
            };
            if is_sentinel_like(&rendered, start, end) {
                result.push_str(&format!("{indent}{start}@verbatim{end}\n"));
            }
            result.push_str(&rendered);
            if !line.ends_with('\n') {
                result.push('\n');
            }
        }
    }
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
    fn render_thin_resolves_section_reference_with_filename_disambiguator_suffix() {
        // Leo disambiguates same-named sections across files by appending
        // " (filename)" to the *defining* node's headline while the body
        // reference stays bare (`<<sect>>`); `VNode.matchHeadline` strips
        // that suffix when matching a reference to its defining node.
        // Without the same tolerance the reference resolves to nothing
        // (renders empty) *and* the shape check that's supposed to keep
        // the section node out of `@others` also fails on the same
        // suffix, so the node's content leaks into `@others` under a
        // malformed marker instead -- or, when the headline additionally
        // differs in case from the reference (see the next test), is lost
        // from the file entirely. Matches leoGlobals.py's real
        // `<< define global decorator dicts >> (leoGlobals.py)`.
        let source = concat!(
            "#@+leo-ver=5-thin\n",
            "#@+node:r: * @file test.py\n",
            "#@+<<sect>>\n",
            "#@+node:s: ** <<sect>> (test.py)\n",
            "inner\n",
            "#@-<<sect>>\n",
            "#@+others\n",
            "#@+node:c: ** child\n",
            "body\n",
            "#@-others\n",
            "#@-leo\n",
        );
        let parsed = DerivedFile::parse(source).unwrap();
        let rendered = render_thin(&parsed.outline, &PositionId("0".into()), "#", "").unwrap();

        assert!(
            rendered.contains(concat!(
                "#@+<<sect>>\n",
                "#@+node:s: ** <<sect>> (test.py)\n",
                "inner\n",
                "#@-<<sect>>\n",
            )),
            "section content must render inline at its reference point, \
             not empty there and relocated under @others:\n{rendered}"
        );
        assert_eq!(
            rendered.matches("@+node:s:").count(),
            1,
            "the section-defining node must not be duplicated under @others:\n{rendered}"
        );

        let reparsed = DerivedFile::parse(&rendered).unwrap();
        assert_eq!(reparsed.outline, parsed.outline);
    }

    #[test]
    fn render_thin_resolves_section_reference_case_insensitively() {
        // `VNode.matchHeadline` lowercases both sides before comparing, so
        // a reference and its defining headline may differ in case (real
        // Leo corpus: body reference `<<define g.decorators>>` against
        // defining headline `<< define g.Decorators >>`). Here the section
        // node's shape check succeeds (headline ends with the bracket, no
        // suffix), correctly excluding it from `@others` -- so a case-only
        // match failure loses the section's content entirely, with no
        // trace anywhere in the rendered file.
        let source = concat!(
            "#@+leo-ver=5-thin\n",
            "#@+node:r: * @file test.py\n",
            "#@+<<Sect>>\n",
            "#@+node:s: ** <<sect>>\n",
            "inner\n",
            "#@-<<Sect>>\n",
            "#@+others\n",
            "#@+node:c: ** child\n",
            "body\n",
            "#@-others\n",
            "#@-leo\n",
        );
        let parsed = DerivedFile::parse(source).unwrap();
        let rendered = render_thin(&parsed.outline, &PositionId("0".into()), "#", "").unwrap();

        assert!(
            rendered.contains("inner\n"),
            "section content must not be dropped:\n{rendered}"
        );

        let reparsed = DerivedFile::parse(&rendered).unwrap();
        assert_eq!(reparsed.outline, parsed.outline);
    }

    #[test]
    fn render_thin_walks_children_past_a_lone_section_reference_with_no_others() {
        // A node's body can hold *only* a section reference (no
        // `@others`/`@all`) and still have further, non-section children
        // beyond the one the reference names -- real Leo's writer still
        // reaches those via a thread-order walk seeded by an *ancestor's*
        // `@others` (`at.putAtOthersLine`'s `moveToThreadNext`, taken
        // whenever a node's own `putBody` reports no `@others`), not
        // anything in this node's own body. A lone section reference must
        // not be treated as "fully accounts for all children" the way
        // `@others`/`@all` do -- only they can license skipping the
        // trailing implicit-children walk. Matches leoGlobals.py's real
        // `g.Urls & UNLs`, whose body is only `<<About clickable
        // links>>`, but which also parents 15 more nodes (g.computeFileUrl
        // and onward) that must still round-trip.
        let source = concat!(
            "#@+leo-ver=5-thin\n",
            "#@+node:r: * @file test.py\n",
            "#@+others\n",
            "#@+node:container: ** container\n",
            "#@+<<sect>>\n",
            "#@+node:s: *3* <<sect>>\n",
            "inner\n",
            "#@-<<sect>>\n",
            "#@+node:extra: *3* extra\n",
            "more\n",
            "#@-others\n",
            "#@-leo\n",
        );
        let parsed = DerivedFile::parse(source).unwrap();
        let rendered = render_thin(&parsed.outline, &PositionId("0".into()), "#", "").unwrap();

        assert!(
            rendered.contains("more\n"),
            "a non-section child past a lone section reference must not be dropped:\n{rendered}"
        );

        let reparsed = DerivedFile::parse(&rendered).unwrap();
        assert_eq!(reparsed.outline, parsed.outline);
    }

    #[test]
    fn render_thin_does_not_treat_an_indented_at_first_line_as_the_at_first_directive() {
        // Real Leo's `directiveKind4` (leoAtFile.py) requires the `@` to be
        // the line's very first character for every directive except
        // `@others`/`@all` -- an indented `@first ...` (a docstring's
        // usage example, say) is plain text, not the file-envelope
        // `@first` directive, and must render verbatim. The `@first`/
        // `@last` branches discard whatever follows the directive word
        // (matching real Leo's own `at.putDirective`, which does the same
        // for a *genuine* mid-body `@first`/`@last`), so mistaking an
        // indented line for one of these loses its text outright. Matches
        // a real docstring in leo-editor's own `helpCommands.py`.
        let source = concat!(
            "#@+leo-ver=5-thin\n",
            "#@+node:r: * @file test.py\n",
            "#@+others\n",
            "#@+node:c: ** child\n",
            "        @first #! /usr/bin/env python\n",
            "#@-others\n",
            "#@-leo\n",
        );
        let parsed = DerivedFile::parse(source).unwrap();
        let rendered = render_thin(&parsed.outline, &PositionId("0".into()), "#", "").unwrap();

        assert!(
            rendered.contains("        @first #! /usr/bin/env python\n"),
            "an indented, non-column-0 @first line is plain text, not the \
             @first directive, and must render verbatim:\n{rendered}"
        );

        let reparsed = DerivedFile::parse(&rendered).unwrap();
        assert_eq!(reparsed.outline, parsed.outline);
    }

    #[test]
    fn render_thin_does_not_pad_blank_lines_inside_indented_others() {
        // A blank line inside a body that renders under an indented
        // `@others` (e.g. a method body) must stay a bare `\n` -- not gain
        // the surrounding indent as trailing whitespace. See at.putCodeLine
        // in canonical Leo's leoAtFile.py: "Don't put any whitespace in
        // otherwise blank lines."
        let source = concat!(
            "#@+leo-ver=5-thin\n",
            "#@+node:r: * @file test.py\n",
            "class C:\n",
            "    #@+others\n",
            "#@+node:c: ** method\n",
            "    def method(self):\n",
            "\n",
            "        return 1\n",
            "    #@-others\n",
            "#@-leo\n",
        );
        let parsed = DerivedFile::parse(source).unwrap();
        let rendered = render_thin(&parsed.outline, &PositionId("0".into()), "#", "").unwrap();

        for line in rendered.lines() {
            assert!(
                line.is_empty() || !line.trim().is_empty(),
                "blank line was padded with indentation: {line:?}\nfull output:\n{rendered}"
            );
        }
        let reparsed = DerivedFile::parse(&rendered).unwrap();
        assert_eq!(reparsed.outline, parsed.outline);
    }

    #[test]
    fn render_thin_expands_an_others_line_with_trailing_text() {
        // Real Leo's `others_pat` (`leoAtFile.py`) matches `@others` with a
        // `\b` word boundary, not an exact-line match, so `@others #
        // helper functions` -- a common way to annotate an `@others` line
        // -- is still a real expansion point, and `putAtOthersLine` echoes
        // the trailing text back into the `@+others` sentinel it writes.
        // Requiring an exact `"@others"` match here used to miss that,
        // falling through to `@`-escaping and silently promoting the
        // nested function out to a top-level sibling instead of keeping it
        // nested -- exactly what leo-editor's own
        // leo/commands/checkerCommands.py (`find_long_lines`'s nested
        // `get_root`/`in_nopylint`) does.
        let source = concat!(
            "#@+leo-ver=5-thin\n",
            "#@+node:r: * @file test.py\n",
            "def outer():\n",
            "    #@+others # helper functions\n",
            "    #@+node:c: ** function: helper\n",
            "    def helper():\n",
            "        return 1\n",
            "    #@-others\n",
            "    return helper()\n",
            "#@-leo\n",
        );
        let parsed = DerivedFile::parse(source).unwrap();
        assert_eq!(
            parsed.outline.nodes[&NodeId::from("r")].body,
            "def outer():\n    @others # helper functions\n    return helper()\n"
        );

        let rendered = render_thin(&parsed.outline, &PositionId("0".into()), "#", "").unwrap();
        assert!(
            rendered.contains("    def helper():\n        return 1\n"),
            "expected helper() to stay nested (indented) under outer():\n{rendered}"
        );
        assert!(
            !rendered.contains("@@others"),
            "the tail-bearing @others line must expand, not fall through to @@-escaping:\n{rendered}"
        );
        assert!(
            rendered.contains("@+others # helper functions"),
            "expected the trailing text echoed back into the open sentinel:\n{rendered}"
        );

        let reparsed = DerivedFile::parse(&rendered).unwrap();
        assert_eq!(reparsed.outline, parsed.outline);
    }

    #[test]
    fn render_thin_omits_at_others_for_children_with_no_explicit_marker_in_body() {
        // Real Leo represents purely-structural nesting (no body text
        // asking for children at a specific point) as a flat sequence of
        // leveled `@+node:...` lines, no `@+others` wrapper -- and its own
        // parser attaches children from level numbers alone (see
        // `DerivedFile::parse`), so a synthetic wrapper here would just be
        // noise a canonical-Leo diff wouldn't have.
        let source = concat!(
            "#@+leo-ver=5-thin\n",
            "#@+node:r: * @file test.py\n",
            "#@+node:c1: ** child one\n",
            "line one\n",
            "#@+node:c2: ** child two\n",
            "line two\n",
            "#@-leo\n",
        );
        let parsed = DerivedFile::parse(source).unwrap();
        let rendered = render_thin(&parsed.outline, &PositionId("0".into()), "#", "").unwrap();

        assert!(
            !rendered.contains("@+others") && !rendered.contains("@-others"),
            "expected no @+others wrapper for implicit children:\n{rendered}"
        );
        let reparsed = DerivedFile::parse(&rendered).unwrap();
        assert_eq!(reparsed.outline, parsed.outline);
    }

    #[test]
    fn render_relative_omits_at_others_for_children_with_no_explicit_marker_in_body() {
        let source = concat!(
            r#"<leo_file><vnodes><v t="r"><vh>@f test.py</vh>"#,
            r#"<v t="c1"><vh>child one</vh></v>"#,
            r#"<v t="c2"><vh>child two</vh></v>"#,
            r#"</v></vnodes><tnodes><t tx="r"></t><t tx="c1">line one</t>"#,
            r#"<t tx="c2">line two</t></tnodes></leo_file>"#,
        );
        let doc = LeoDocument::parse(source).unwrap();
        let rendered = render_relative(&doc.outline, &PositionId("0".into()), "#", "").unwrap();

        assert!(
            !rendered.contains("@+others") && !rendered.contains("@-others"),
            "expected no @+others wrapper for implicit children:\n{rendered}"
        );
        let reparsed = RelativeFile::parse(&rendered).unwrap();
        let children = &reparsed.outline.roots[0].children;
        assert_eq!(children.len(), 2);
        assert_eq!(
            reparsed.outline.nodes[&children[0].node].body,
            "line one\n"
        );
        assert_eq!(
            reparsed.outline.nodes[&children[1].node].body,
            "line two\n"
        );
    }

    #[test]
    fn render_thin_treats_at_others_as_inert_text_under_an_active_at_all() {
        // Real Leo's `putAtAllBody` (`leoAtFile.py`) writes body text
        // completely unconditionally under an ancestor's `@all` -- no
        // directive scanning at all -- so a literal `@others` in a
        // descendant's body there is just inert text, not an expansion
        // point. `render_position` must match: it shouldn't treat it as a
        // directive just because the word matches.
        let source = concat!(
            r#"<leo_file><vnodes><v t="r"><vh>@file test.py</vh>"#,
            r#"<v t="c"><vh>class Foo</vh></v>"#,
            r#"</v></vnodes><tnodes><t tx="r">"#,
            "@all\n",
            r#"</t><t tx="c">"#,
            "class Foo:\n@others\n    pass\n",
            r#"</t></tnodes></leo_file>"#,
        );
        let doc = LeoDocument::parse(source).unwrap();
        let rendered = render_thin(&doc.outline, &PositionId("0".into()), "#", "").unwrap();

        assert!(
            rendered.contains("\nclass Foo:\n@others\n    pass\n"),
            "expected the literal @others line to survive as plain, unescaped body text:\n{rendered}"
        );
        assert!(
            !rendered.contains("#@+others") && !rendered.contains("#@@others"),
            "a literal @others under @all must not be treated as a directive:\n{rendered}"
        );

        let reparsed = DerivedFile::parse(&rendered).unwrap();
        assert_eq!(
            reparsed.outline.nodes[&NodeId::from("c")].body,
            "class Foo:\n@others\n    pass\n"
        );
    }

    #[test]
    fn render_relative_treats_at_others_as_inert_text_under_an_active_at_all() {
        let source = concat!(
            r#"<leo_file><vnodes><v t="r"><vh>@f test.py</vh>"#,
            r#"<v t="c"><vh>class Foo</vh></v>"#,
            r#"</v></vnodes><tnodes><t tx="r">"#,
            "@all\n",
            r#"</t><t tx="c">"#,
            "class Foo:\n@others\n    pass\n",
            r#"</t></tnodes></leo_file>"#,
        );
        let doc = LeoDocument::parse(source).unwrap();
        let rendered = render_relative(&doc.outline, &PositionId("0".into()), "#", "").unwrap();

        assert!(
            rendered.contains("\nclass Foo:\n@others\n    pass\n"),
            "expected the literal @others line to survive as plain, unescaped body text:\n{rendered}"
        );
        assert!(
            !rendered.contains("#@+others") && !rendered.contains("#@@others"),
            "a literal @others under @all must not be treated as a directive:\n{rendered}"
        );

        let reparsed = RelativeFile::parse(&rendered).unwrap();
        let child = &reparsed.outline.roots[0].children[0];
        assert_eq!(
            reparsed.outline.nodes[&child.node].body,
            "class Foo:\n@others\n    pass\n"
        );
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
    fn body_lines_starting_with_at_are_only_escaped_when_actually_directive_like() {
        // A Python decorator (`@cmd(...)`) or a dotted call (`@g.trace(...)`)
        // merely starts with '@' -- it isn't a Leo directive, and escaping
        // it as `@@cmd(...)` would comment it out, breaking the derived
        // file as standalone Python. A real directive (`@language`) still
        // needs escaping so it stays valid Python once commented -- bare
        // `@language python` isn't. Mirrors classic Leo's
        // `at.directiveKind4` disambiguation (`leoAtFile.py`).
        let source = concat!(
            r#"<leo_file><vnodes><v t="r"><vh>@f test.py</vh></v></vnodes>"#,
            r#"<tnodes><t tx="r">"#,
            "@cmd('buffer-copy')\n",
            "def f(): pass\n",
            "@g.commander_command('restart-leo')\n",
            "def g(): pass\n",
            "@language python\n",
            "</t></tnodes></leo_file>",
        );
        let doc = LeoDocument::parse(source).unwrap();
        let rendered = render_relative(&doc.outline, &PositionId("0".into()), "#", "").unwrap();

        assert!(
            rendered.contains("\n@cmd('buffer-copy')\n"),
            "decorator should be unescaped, live code:\n{rendered}"
        );
        assert!(
            rendered.contains("\n@g.commander_command('restart-leo')\n"),
            "dotted call should be unescaped, live code:\n{rendered}"
        );
        assert!(
            rendered.contains("#@@language python\n"),
            "a real directive still needs escaping to stay valid Python:\n{rendered}"
        );

        let reparsed = RelativeFile::parse(&rendered).unwrap();
        assert_eq!(
            reparsed.outline.nodes[&NodeId::from("r")].body,
            doc.outline.nodes[&NodeId::from("r")].body
        );
    }

    #[test]
    fn render_does_not_escape_bare_at_lines_that_are_not_real_directives() {
        // Both render_position (5-thin) and render_position_relative (@f)
        // used to also escape any body line `is_cub_node_marker_collision`
        // flagged -- an empty tail, or a bare `0`/`<N`/`>N` token after the
        // `@`, matching cub-1-thin's *own* `@0`/`@>`/`@<` marker grammar.
        // But a body line doesn't get the comment prefix until the
        // (unescaped) `else` branch adds it back, and every sentinel/node-
        // marker pattern this crate parses (5-thin *and* cub-1-thin)
        // requires that comment prefix *before* the `@` -- `#@+node:...`,
        // `#@0 ...` -- never a bare, unprefixed `@`. So a plain `@0 ...`
        // body line can't be mistaken for a marker on a later parse
        // regardless of format, and `is_sentinel_like` already guards the
        // one case that could -- a line that already starts with the full
        // comment-prefixed sentinel shape. Escaping should only fire for
        // an actual Leo directive name (`is_leo_directive_word`). Found
        // via a real cub/leo-editor round-trip: leo/commands/
        // checkerCommands.py has a docstring line that's literally a bare
        // `@` (documenting headline prefixes Leo's dubious-node checker
        // ignores), and real Leo writes it out completely unescaped.
        let source = concat!(
            r#"<leo_file><vnodes><v t="r"><vh>@file test.py</vh></v></vnodes>"#,
            r#"<tnodes><t tx="r">"#,
            "@\n",
            "@0 not a real directive\n",
            "@language python\n",
            "</t></tnodes></leo_file>",
        );
        let doc = LeoDocument::parse(source).unwrap();

        for (rendered, reparsed_body) in [
            {
                let rendered =
                    render_thin(&doc.outline, &PositionId("0".into()), "#", "").unwrap();
                let reparsed_body = DerivedFile::parse(&rendered).unwrap().outline.nodes
                    [&NodeId::from("r")]
                    .body
                    .clone();
                (rendered, reparsed_body)
            },
            {
                let rendered =
                    render_relative(&doc.outline, &PositionId("0".into()), "#", "").unwrap();
                let reparsed_body = RelativeFile::parse(&rendered).unwrap().outline.nodes
                    [&NodeId::from("r")]
                    .body
                    .clone();
                (rendered, reparsed_body)
            },
        ] {
            assert!(
                rendered.contains("\n@\n"),
                "a bare `@` can't collide with any format's marker grammar:\n{rendered}"
            );
            assert!(
                rendered.contains("\n@0 not a real directive\n"),
                "`@0 ...` isn't a directive or a real marker without the comment prefix:\n{rendered}"
            );
            assert!(
                rendered.contains("@@language python\n"),
                "a real directive still needs escaping to stay valid Python:\n{rendered}"
            );
            assert_eq!(reparsed_body, doc.outline.nodes[&NodeId::from("r")].body);
        }
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

    #[test]
    fn external_snapshot_at_matches_external_snapshot_for_a_nested_node() {
        // `external_snapshot_at` (an O(depth) lookup by known PositionId,
        // used when a caller -- like load_derived_jobs -- already knows
        // exactly where a node landed) must agree with `external_snapshot`
        // (an O(tree size) lookup by NodeId) for the same node, including
        // one nested a few levels deep rather than a root.
        let source = concat!(
            r#"<leo_file><vnodes><v t="r"><vh>root</vh>"#,
            r#"<v t="mid"><vh>mid</vh>"#,
            r#"<v t="leaf"><vh>@f leaf.py</vh>"#,
            r#"<v t="grandchild"><vh>grandchild</vh></v>"#,
            r#"</v></v></v></vnodes>"#,
            r#"<tnodes><t tx="r"></t><t tx="mid"></t><t tx="leaf"></t>"#,
            r#"<t tx="grandchild">body</t></tnodes></leo_file>"#,
        );
        let doc = LeoDocument::parse(source).unwrap();
        let leaf = NodeId::from("leaf");
        let leaf_position = PositionId("0/0/0".into());

        let by_id = external_snapshot(&doc.outline, &leaf).unwrap();
        let by_position = external_snapshot_at(&doc.outline, &leaf_position).unwrap();

        assert_eq!(by_id.0, leaf_position);
        assert_eq!(by_id, by_position);
        assert_eq!(by_id.1.roots[0].node, leaf);
        assert_eq!(
            by_id.1.roots[0].children[0].node,
            NodeId::from("grandchild")
        );
    }

    #[test]
    fn prepare_external_updates_finds_every_writable_root_via_one_shared_index() {
        // Regression test for the O(externals x tree size) `find()` this
        // module used to do once per writable root in `prepare_external_updates`
        // (and, before `external_snapshot_at` existed, once per root in
        // `load_derived_jobs` too): with several sibling `@f` roots, every
        // one of them must still be found and rendered when its body
        // diverges from `file.original`, not just the first one located.
        let source = concat!(
            r#"<leo_file><vnodes>"#,
            r#"<v t="a"><vh>@f a.py</vh></v>"#,
            r#"<v t="b"><vh>@f b.py</vh></v>"#,
            r#"<v t="c"><vh>@f c.py</vh></v>"#,
            r#"</vnodes>"#,
            r#"<tnodes><t tx="a">alpha</t><t tx="b">beta</t><t tx="c">gamma</t></tnodes>"#,
            r#"</leo_file>"#,
        );
        let doc = LeoDocument::parse(source).unwrap();
        let writable = HashMap::from([
            (
                NodeId::from("a"),
                WritableExternalFile {
                    path: PathBuf::from("a.py"),
                    start_delimiter: "#".into(),
                    end_delimiter: "".into(),
                    original: Outline::default(),
                    format: ExternalFormat::Relative,
                },
            ),
            (
                NodeId::from("b"),
                WritableExternalFile {
                    path: PathBuf::from("b.py"),
                    start_delimiter: "#".into(),
                    end_delimiter: "".into(),
                    original: Outline::default(),
                    format: ExternalFormat::Relative,
                },
            ),
            (
                NodeId::from("c"),
                WritableExternalFile {
                    path: PathBuf::from("c.py"),
                    start_delimiter: "#".into(),
                    end_delimiter: "".into(),
                    original: Outline::default(),
                    format: ExternalFormat::Relative,
                },
            ),
        ]);

        let updates = prepare_external_updates(&doc.outline, &writable).unwrap();
        let mut roots: Vec<_> = updates.iter().map(|update| update.root.0.clone()).collect();
        roots.sort();
        assert_eq!(roots, vec!["a", "b", "c"]);
    }
}
