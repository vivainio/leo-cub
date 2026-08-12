//! Synchronize external Leo file nodes into an outline.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use thiserror::Error;

use crate::{
    DerivedFile, LeoDocument, NodeId, Outline, Position, PositionId, propagate_clean_changes,
};

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("unsupported comment delimiters for @clean file: {0}")]
    UnsupportedCleanFile(PathBuf),
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
        let parsed = if job.directive == "@clean" {
            let (start, end) = comment_delimiters(&job.path)
                .ok_or_else(|| SyncError::UnsupportedCleanFile(job.path.clone()))?;
            let private = render_private(&next.outline, &job.position, start, end)?;
            let updated = propagate_clean_changes(&source, &private, start, end);
            DerivedFile::parse(&updated)?
        } else {
            DerivedFile::parse(&source)?
        };
        parsed.merge_into(&mut next.outline, &job.position)?;
        items.push(SyncItem {
            gnx: job.gnx,
            path: job.path,
            directive: job.directive,
            changed: before != next.outline,
        });
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

fn comment_delimiters(path: &Path) -> Option<(&'static str, &'static str)> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "py" | "pyw" | "sh" | "bash" | "zsh" | "fish" | "rb" | "pl" | "pm" | "r" | "toml"
        | "yaml" | "yml" => Some(("#", "")),
        "rs" | "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "java" | "js" | "jsx" | "ts" | "tsx"
        | "go" | "swift" | "kt" | "kts" | "cs" => Some(("//", "")),
        "html" | "htm" | "xml" | "xhtml" | "svg" => Some(("<!--", "-->")),
        "css" | "scss" | "less" => Some(("/*", "*/")),
        "sql" | "lua" => Some(("--", "")),
        "ini" | "cfg" => Some(("#", "")),
        _ => None,
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
    let mut result = format!("{start}@+leo-ver=5-thin{end}\n");
    render_position(outline, root, 1, "", start, end, &mut result);
    result.push_str(&format!("{start}@-leo{end}\n"));
    Ok(result)
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
            for child in &position.children {
                render_position(outline, child, level + 1, &child_indent, start, end, result);
            }
            result.push_str(&format!("{indent}{leading}{start}@-others{end}\n"));
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
        for child in &position.children {
            render_position(outline, child, level + 1, indent, start, end, result);
        }
        result.push_str(&format!("{indent}{start}@-others{end}\n"));
    }
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
}
