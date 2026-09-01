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

use rayon::prelude::*;

use crate::{
    AutoFile, DerivedFile, ExternalFormat, Node, NodeId, Outline, Position, PositionId,
    RelativeFile, WritableExternalFile, comment_delimiters, external_snapshot_at,
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
/// A directive's file already read from disk and parsed against a snapshot
/// of `outline` taken before any job in the batch merged into it -- the
/// output of the read+parse phase of [`load_derived_jobs`], which runs in
/// parallel across jobs since none of it touches `outline` mutably.
enum ParsedFile {
    Auto(AutoFile),
    Relative(RelativeFile),
    Derived(DerivedFile),
    /// An `@edit` file's raw content: no sentinels, no structure, and (like
    /// real Leo's `readOneAtEditNode`) no children -- the whole file is the
    /// node's body verbatim.
    Edit(String),
}

/// What [`load_derived_jobs`] reads from `outline` before a job's own merge
/// can overwrite it. Captured per job during the parallel prepare phase, not
/// the later sequential merge phase, so it always reflects pre-load state --
/// even for a clone occurrence whose sibling job merges first.
struct JobPrep {
    root_node: NodeId,
    original_children: Vec<Position>,
    original_body: String,
    original_nodes: HashMap<NodeId, Node>,
}

enum JobOutcome {
    /// Not `@auto` and the file doesn't exist yet: recorded as writable with
    /// no prior content, nothing to read or parse.
    Missing(DerivedJob),
    Parsed {
        job: DerivedJob,
        prep: JobPrep,
        parsed: Box<ParsedFile>,
    },
    Failed {
        label: String,
        error: String,
    },
}

pub fn load_derived_jobs(outline: &mut Outline, jobs: Vec<DerivedJob>) -> LoadReport {
    let mut report = LoadReport::default();
    // Read+parse every job's file up front, in parallel: this is the
    // I/O- and CPU-heavy part, and it only ever reads `outline` (to snapshot
    // pre-load state), never mutates it. The actual merge into `outline`
    // stays a plain sequential loop below since it has to.
    let outcomes: Vec<JobOutcome> = jobs
        .into_par_iter()
        .map(|job| prepare_job(outline, job))
        .collect();
    for outcome in outcomes {
        match outcome {
            JobOutcome::Missing(job) => {
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
            }
            JobOutcome::Failed { label, error } => {
                report.errors.push(format!("{label}: {error}"));
            }
            JobOutcome::Parsed { job, prep, parsed } => {
                match merge_parsed(outline, &mut report, &job, prep, *parsed) {
                    Ok(()) => report.loaded += 1,
                    Err(error) => report
                        .errors
                        .push(format!("{}: {error}", job.path.display())),
                }
            }
        }
    }
    report
}

/// Reads and parses one job's file against a read-only view of `outline`.
/// Safe to run concurrently across jobs: it never mutates `outline`, only
/// snapshots the pre-load state each job's later merge needs.
fn prepare_job(outline: &Outline, job: DerivedJob) -> JobOutcome {
    let label = job.path.display().to_string();
    // `@auto-dir`'s `job.path` is a directory or glob pattern, not a
    // readable file, so it must skip the generic `fs::read_to_string` below
    // entirely -- not just the single-file `@auto` parse branch further
    // down, which is the only other place `job.auto` is inspected.
    let is_auto_dir = job.directive == "@auto-dir";
    if !job.auto && !is_auto_dir && !job.path.exists() {
        return JobOutcome::Missing(job);
    }
    let Some(root_node) = outline
        .position(&job.position)
        .map(|position| position.node.clone())
    else {
        return JobOutcome::Failed {
            label,
            error: "derived root position disappeared".to_owned(),
        };
    };
    let original_children = outline
        .position(&job.position)
        .map(|position| position.children.clone())
        .unwrap_or_default();
    let original_body = outline.nodes[&root_node].body.clone();
    // Captured before merge_into prunes outline.nodes down to what the
    // freshly generated tree references: these ids otherwise vanish from
    // outline.nodes even though original_children (restored just before
    // serializing) still points at them.
    let original_nodes: HashMap<NodeId, Node> = referenced_nodes(&original_children)
        .into_iter()
        .filter_map(|id| outline.nodes.get(&id).cloned().map(|node| (id, node)))
        .collect();

    let parsed = if is_auto_dir {
        match crate::auto_dir::parse_dir(&job.path, job.root.clone()) {
            Ok(auto) => ParsedFile::Auto(auto),
            Err(error) => {
                return JobOutcome::Failed {
                    label,
                    error: error.to_string(),
                };
            }
        }
    } else {
        let source = match fs::read_to_string(&job.path) {
            Ok(source) => source,
            Err(error) => {
                return JobOutcome::Failed {
                    label,
                    error: error.to_string(),
                };
            }
        };
        if job.auto {
            match AutoFile::parse_with_directive(
                &job.path,
                job.root.clone(),
                &source,
                Some(&job.directive),
            ) {
                Ok(auto) => ParsedFile::Auto(auto),
                Err(error) => {
                    return JobOutcome::Failed {
                        label,
                        error: error.to_string(),
                    };
                }
            }
        } else if job.directive == "@f" {
            match RelativeFile::parse(&source) {
                Ok(derived) => ParsedFile::Relative(derived),
                Err(error) => {
                    return JobOutcome::Failed {
                        label,
                        error: error.to_string(),
                    };
                }
            }
        } else if job.directive == "@edit" {
            ParsedFile::Edit(source)
        } else {
            match DerivedFile::parse(&source) {
                Ok(derived) => ParsedFile::Derived(derived),
                Err(error) => {
                    return JobOutcome::Failed {
                        label,
                        error: error.to_string(),
                    };
                }
            }
        }
    };

    JobOutcome::Parsed {
        job,
        prep: JobPrep {
            root_node,
            original_children,
            original_body,
            original_nodes,
        },
        parsed: Box::new(parsed),
    }
}

/// Merges one already-parsed job into `outline` and records it in `report`.
/// Sequential by nature: it mutates the shared `Outline`.
fn merge_parsed(
    outline: &mut Outline,
    report: &mut LoadReport,
    job: &DerivedJob,
    prep: JobPrep,
    parsed: ParsedFile,
) -> Result<(), String> {
    match parsed {
        ParsedFile::Auto(auto) => {
            // `@auto-dir`'s nodes come from several files rather than the
            // one `job.path` names -- `auto.file_paths` carries the real
            // per-node source in that case, so jump-to-source lands on the
            // matched file instead of the directory/glob itself.
            let path_for = |id: &NodeId| {
                auto.file_paths
                    .as_ref()
                    .and_then(|paths| paths.get(id))
                    .cloned()
                    .unwrap_or_else(|| job.path.clone())
            };
            if !auto.merge_into(outline, &job.position) {
                return Err("auto root position disappeared".to_owned());
            }
            // An `@auto-dir` root isn't backed by any single file --
            // `job.path` is the directory/glob argument, not something `o`
            // can open -- so unlike a plain `@auto <path>` root, it gets no
            // location entry; opening it falls through to a body edit
            // instead of a misleading "open failed" on a glob pattern.
            if auto.file_paths.is_none() {
                report
                    .node_locations
                    .entry(auto.root.clone())
                    .or_insert(SourceLocation {
                        path: path_for(&auto.root),
                        line: 1,
                    });
            }
            for (id, line) in &auto.locations {
                report
                    .node_locations
                    .entry(id.clone())
                    .or_insert(SourceLocation {
                        path: path_for(id),
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
        }
        ParsedFile::Relative(derived) => {
            derived
                .merge_into(outline, &job.position)
                .map_err(|error| error.to_string())?;
            let original = external_snapshot_at(outline, &job.position)
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
        }
        ParsedFile::Derived(derived) => {
            derived
                .merge_into(outline, &job.position)
                .map_err(|error| error.to_string())?;
            let original = external_snapshot_at(outline, &job.position)
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
        ParsedFile::Edit(source) => {
            // Matches real Leo's `readOneAtEditNode`/`writeOneAtEditNode`
            // (leoAtFile.py): an `@edit` node is a flat body with no
            // children. Unlike real Leo, which silently deletes any
            // children on read, this refuses to load instead -- consistent
            // with how cub already refuses other structurally-invalid
            // states rather than discarding a user's work.
            let has_children = outline
                .position(&job.position)
                .is_some_and(|position| !position.children.is_empty());
            if has_children {
                return Err("@edit nodes must not have children".to_owned());
            }
            let node = outline
                .nodes
                .get_mut(&job.root)
                .ok_or_else(|| "edit root node disappeared".to_owned())?;
            node.body = source;
            let original = external_snapshot_at(outline, &job.position)
                .map(|(_, snapshot)| snapshot)
                .ok_or_else(|| "merged external root disappeared".to_owned())?;
            let (start_delimiter, end_delimiter) = comment_delimiters(&job.path);
            report.writable_external.insert(
                job.root.clone(),
                WritableExternalFile {
                    path: job.path.clone(),
                    start_delimiter: start_delimiter.to_owned(),
                    end_delimiter: end_delimiter.to_owned(),
                    original,
                    format: ExternalFormat::Edit,
                },
            );
        }
    }
    report
        .original_children
        .entry(prep.root_node)
        .or_insert(prep.original_children);
    report
        .original_bodies
        .entry(job.root.clone())
        .or_insert(prep.original_body);
    for (id, node) in prep.original_nodes {
        report.original_nodes.entry(id).or_insert(node);
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

pub fn derived_filename(headline: &str) -> Option<(bool, &str, &str)> {
    let (directive, filename) = headline.trim().split_once(char::is_whitespace)?;
    matches!(
        directive,
        "@file"
            | "@thin"
            | "@file-thin"
            | "@f"
            | "@clean"
            | "@edit"
            | "@auto"
            | "@auto-md"
            | "@auto-markdown"
            | "@auto-dir"
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
            | "@edit"
            | "@auto"
            | "@auto-md"
            | "@auto-markdown"
            | "@auto-dir"
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!(
            "leo-cub-derive-load-{name}-{}-{}",
            std::process::id(),
            now.as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    /// End-to-end wiring test for `@auto-dir`: `prepare_job`'s directory
    /// branch and `merge_parsed`'s `Auto` arm have to cooperate to expand a
    /// directory node into one child per matched file, mark every generated
    /// node read-only (`derived_nodes`), and snapshot enough pre-load state
    /// (`original_bodies`/`original_children`) for `sync::restore_external_state`
    /// to later exclude it from the serialized `.leo` XML, exactly like a
    /// plain `@auto` node.
    #[test]
    fn auto_dir_job_expands_matched_files_as_read_only_children() {
        let dir = temp_dir("wiring");
        fs::write(dir.join("a.py"), "def a():\n    pass\n").unwrap();
        fs::write(dir.join("b.rs"), "fn b() {}\n").unwrap();

        let mut outline = Outline {
            nodes: [(
                NodeId::from("dir-node"),
                Node {
                    id: NodeId::from("dir-node"),
                    headline: format!("@auto-dir {}", dir.display()),
                    body: String::new(),
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                },
            )]
            .into_iter()
            .collect(),
            roots: vec![Position {
                node: NodeId::from("dir-node"),
                children: vec![],
            }],
        };

        let outline_path = dir.join("outline.leo");
        let report = load_derived_files(&mut outline, &outline_path);

        assert!(report.errors.is_empty(), "{:?}", report.errors);
        let children = &outline.roots[0].children;
        assert_eq!(children.len(), 2);
        // Headline carries the full resolved path (not just the bare
        // filename) so `cub inspect <file>`'s suffix matching still finds
        // it -- strip the temp dir prefix back off for a readable assert.
        let headlines: Vec<_> = children
            .iter()
            .map(|position| {
                let headline = outline.nodes[&position.node].headline.clone();
                let path = headline.strip_prefix("@auto ").unwrap();
                format!(
                    "@auto {}",
                    Path::new(path)
                        .strip_prefix(&dir)
                        .unwrap_or(Path::new(path))
                        .display()
                )
            })
            .collect();
        assert_eq!(headlines, vec!["@auto a.py", "@auto b.rs"]);
        for position in children {
            assert!(report.derived_nodes.contains(&position.node));
        }
        // The `@auto-dir` root itself isn't backed by any single file --
        // `job.path` is the directory argument, not something `o` can
        // open -- so it must get no jump-to-source entry at all, rather
        // than one pointing at that directory.
        assert!(
            !report
                .node_locations
                .contains_key(&NodeId::from("dir-node"))
        );
        assert!(
            report
                .original_bodies
                .contains_key(&NodeId::from("dir-node"))
        );
        assert_eq!(
            report.original_children[&NodeId::from("dir-node")],
            Vec::<Position>::new()
        );
    }

    /// `o` (open in editor) reads `App::source_nodes`, populated verbatim
    /// from `report.node_locations` -- so this is the data that decides
    /// whether opening an `@auto-dir` descendant lands on the right file
    /// *and* the right line. Each matched file's node ids must map to
    /// *that* file, not the directory `job.path` itself names, and each
    /// declaration's line must be relative to its own file, not offset by
    /// the other files aggregated alongside it.
    #[test]
    fn auto_dir_node_locations_point_at_the_matched_files_not_the_directory() {
        // The Rust importer only recognizes a function whose brace is on
        // its own line (`leo_rust_block` in auto.rs) -- a one-line `fn a()
        // {}` body doesn't qualify -- and won't split a file with only one
        // recognized declaration into a child at all. Two multi-line
        // functions per file satisfies both.
        let dir = temp_dir("locations");
        fs::write(
            dir.join("a.rs"),
            "fn a() {\n    let _ = 1;\n}\nfn a2() {\n    let _ = 2;\n}\n",
        )
        .unwrap();
        fs::write(
            dir.join("b.rs"),
            "// leading comment\nfn b() {\n    let _ = 1;\n}\nfn b2() {\n    let _ = 2;\n}\n",
        )
        .unwrap();

        let mut outline = Outline {
            nodes: [(
                NodeId::from("dir-node"),
                Node {
                    id: NodeId::from("dir-node"),
                    headline: format!("@auto-dir {}", dir.display()),
                    body: String::new(),
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                },
            )]
            .into_iter()
            .collect(),
            roots: vec![Position {
                node: NodeId::from("dir-node"),
                children: vec![],
            }],
        };
        let outline_path = dir.join("outline.leo");
        let report = load_derived_files(&mut outline, &outline_path);
        assert!(report.errors.is_empty(), "{:?}", report.errors);

        let file_children = &outline.roots[0].children;
        assert_eq!(file_children.len(), 2);
        for position in file_children {
            let headline = &outline.nodes[&position.node].headline;
            let expected_path = if headline.ends_with("a.rs") {
                dir.join("a.rs")
            } else {
                dir.join("b.rs")
            };
            let file_root_location = report
                .node_locations
                .get(&position.node)
                .unwrap_or_else(|| panic!("no location recorded for {headline}"));
            assert_eq!(file_root_location.path, expected_path);
            assert_eq!(file_root_location.line, 1);

            // The declaration inside each file must resolve to that same
            // file, at the line it actually appears on within it -- "fn b"
            // sits on line 2 of b.rs, not on some line offset by a.rs's
            // own content being aggregated into the same job.
            let declaration = position
                .children
                .first()
                .unwrap_or_else(|| panic!("{headline} produced no structural children"));
            let declaration_location = report
                .node_locations
                .get(&declaration.node)
                .unwrap_or_else(|| panic!("no location recorded for a declaration in {headline}"));
            assert_eq!(declaration_location.path, expected_path);
            let expected_line = if headline.ends_with("a.rs") { 1 } else { 2 };
            assert_eq!(declaration_location.line, expected_line);
        }
    }
}
