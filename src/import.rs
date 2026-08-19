use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use thiserror::Error;

use crate::{LeoDocument, Node, NodeId, Position};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportMode {
    Auto,
    Edit,
    Clean,
}

impl ImportMode {
    fn directive(self) -> &'static str {
        match self {
            Self::Auto => "@auto",
            Self::Edit => "@edit",
            Self::Clean => "@clean",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ImportOptions {
    pub mode: ImportMode,
    pub recursive: bool,
    pub paths: bool,
    pub parent: Option<NodeId>,
    pub dry_run: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImportItem {
    pub path: PathBuf,
    pub gnx: NodeId,
    pub headline: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImportReport {
    pub imported: usize,
    pub path_nodes: usize,
    pub dry_run: bool,
    pub items: Vec<ImportItem>,
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("directory import requires --recursive: {0}")]
    DirectoryNeedsRecursive(PathBuf),
    #[error("parent node not found: {0}")]
    ParentNotFound(String),
    #[error("import would create an invalid outline: {0}")]
    Invalid(String),
}

/// Import files as Leo external-file nodes. All input is read before the
/// document is changed, so an error can never leave a partial import.
pub fn import_files(
    document: &mut LeoDocument,
    outline_path: &Path,
    inputs: &[PathBuf],
    options: &ImportOptions,
) -> Result<ImportReport, ImportError> {
    if let Some(parent) = &options.parent
        && !document.outline.nodes.contains_key(parent)
    {
        return Err(ImportError::ParentNotFound(parent.0.clone()));
    }

    let outline_dir = outline_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut specs = Vec::new();
    for input in inputs {
        let metadata = fs::metadata(input).map_err(|source| ImportError::Io {
            path: input.clone(),
            source,
        })?;
        if metadata.is_dir() {
            if !options.recursive {
                return Err(ImportError::DirectoryNeedsRecursive(input.clone()));
            }
            collect_dir(input, input, options.paths, &mut specs)?;
        } else {
            let body = read_text(input)?;
            specs.push(Spec::File {
                path: input.clone(),
                relative: relative_path(outline_dir, input),
                body,
            });
        }
    }

    let mut next = document.outline.clone();
    let mut ids = IdGenerator::new(next.nodes.keys().cloned().collect());
    let mut items = Vec::new();
    let mut path_nodes = 0;
    let mut positions = Vec::new();
    for spec in specs {
        positions.push(build_spec(
            spec,
            &mut next.nodes,
            &mut ids,
            options.mode,
            outline_dir,
            &mut items,
            &mut path_nodes,
        ));
    }

    let children = children_for_parent(&mut next.roots, options.parent.as_ref())
        .ok_or_else(|| ImportError::ParentNotFound(options.parent.as_ref().unwrap().0.clone()))?;
    children.extend(positions);
    if let Some(error) = next.validate().first() {
        return Err(ImportError::Invalid(error.to_string()));
    }
    if !options.dry_run {
        document.outline = next;
    }
    Ok(ImportReport {
        imported: items.len(),
        path_nodes,
        dry_run: options.dry_run,
        items,
    })
}

enum Spec {
    File {
        path: PathBuf,
        relative: PathBuf,
        body: String,
    },
    Directory {
        name: String,
        children: Vec<Spec>,
    },
}

fn collect_dir(
    root: &Path,
    directory: &Path,
    paths: bool,
    output: &mut Vec<Spec>,
) -> Result<(), ImportError> {
    let mut entries = fs::read_dir(directory)
        .map_err(|source| ImportError::Io {
            path: directory.to_owned(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| ImportError::Io {
            path: directory.to_owned(),
            source,
        })?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut children = Vec::new();
    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| ImportError::Io {
            path: path.clone(),
            source,
        })?;
        if file_type.is_dir() {
            collect_dir(root, &path, paths, &mut children)?;
        } else if file_type.is_file() {
            children.push(Spec::File {
                body: read_text(&path)?,
                relative: if paths {
                    PathBuf::from(path.file_name().unwrap_or_default())
                } else {
                    path.strip_prefix(root).unwrap_or(&path).to_owned()
                },
                path,
            });
        }
    }
    if children.is_empty() {
        return Ok(());
    }
    if paths {
        output.push(Spec::Directory {
            name: directory
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            children,
        });
    } else {
        flatten_files(children, output);
    }
    Ok(())
}

fn flatten_files(specs: Vec<Spec>, output: &mut Vec<Spec>) {
    for spec in specs {
        match spec {
            Spec::File { .. } => output.push(spec),
            Spec::Directory { children, .. } => flatten_files(children, output),
        }
    }
}

fn build_spec(
    spec: Spec,
    nodes: &mut HashMap<NodeId, Node>,
    ids: &mut IdGenerator,
    mode: ImportMode,
    outline_dir: &Path,
    items: &mut Vec<ImportItem>,
    path_nodes: &mut usize,
) -> Position {
    match spec {
        Spec::File {
            path,
            relative,
            body,
        } => {
            let id = ids.next();
            let display = if relative.components().count() == 1 {
                relative
            } else {
                relative_path(outline_dir, &path)
            };
            let headline = format!("{} {}", mode.directive(), slash_path(&display));
            nodes.insert(
                id.clone(),
                Node {
                    id: id.clone(),
                    headline: headline.clone(),
                    // Leo reconstructs @auto bodies and descendants from the
                    // external source when the outline is loaded.
                    body: if mode == ImportMode::Auto {
                        String::new()
                    } else {
                        body
                    },
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                },
            );
            items.push(ImportItem {
                path,
                gnx: id.clone(),
                headline,
            });
            Position {
                node: id,
                children: vec![],
            }
        }
        Spec::Directory { name, children } => {
            *path_nodes += 1;
            let id = ids.next();
            nodes.insert(
                id.clone(),
                Node {
                    id: id.clone(),
                    headline: format!("@path {name}"),
                    body: String::new(),
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                },
            );
            let children = children
                .into_iter()
                .map(|child| build_spec(child, nodes, ids, mode, outline_dir, items, path_nodes))
                .collect();
            Position { node: id, children }
        }
    }
}

fn read_text(path: &Path) -> Result<String, ImportError> {
    fs::read_to_string(path).map_err(|source| ImportError::Io {
        path: path.to_owned(),
        source,
    })
}

fn relative_path(base: &Path, path: &Path) -> PathBuf {
    let base = fs::canonicalize(base).unwrap_or_else(|_| base.to_owned());
    let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
    pathdiff(&base, &path).unwrap_or(path)
}

fn pathdiff(base: &Path, path: &Path) -> Option<PathBuf> {
    let base: Vec<_> = base.components().collect();
    let path: Vec<_> = path.components().collect();
    if matches!((base.first(), path.first()), (Some(Component::Prefix(a)), Some(Component::Prefix(b))) if a != b)
    {
        return None;
    }
    let common = base.iter().zip(&path).take_while(|(a, b)| a == b).count();
    let mut result = PathBuf::new();
    for _ in common..base.len() {
        result.push("..");
    }
    for component in &path[common..] {
        result.push(component.as_os_str());
    }
    Some(result)
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn children_for_parent<'a>(
    roots: &'a mut Vec<Position>,
    parent: Option<&NodeId>,
) -> Option<&'a mut Vec<Position>> {
    let Some(parent) = parent else {
        return Some(roots);
    };
    fn find<'a>(positions: &'a mut [Position], id: &NodeId) -> Option<&'a mut Vec<Position>> {
        for position in positions {
            if &position.node == id {
                return Some(&mut position.children);
            }
            if let Some(found) = find(&mut position.children, id) {
                return Some(found);
            }
        }
        None
    }
    find(roots, parent)
}

pub(crate) struct IdGenerator {
    prefix: String,
    used: HashSet<NodeId>,
    seconds: u64,
    nanos: u32,
    sequence: u64,
}

impl IdGenerator {
    fn new(used: HashSet<NodeId>) -> Self {
        Self::with_prefix("cub".to_owned(), used)
    }

    pub(crate) fn with_prefix(prefix: String, used: HashSet<NodeId>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self {
            prefix,
            used,
            seconds: now.as_secs(),
            nanos: now.subsec_nanos(),
            sequence: 0,
        }
    }

    pub(crate) fn next(&mut self) -> NodeId {
        loop {
            self.sequence += 1;
            let id = NodeId(format!(
                "{}.{}.{}.{}",
                self.prefix, self.seconds, self.nanos, self.sequence
            ));
            if self.used.insert(id.clone()) {
                return id;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?><leo_file><leo_header file_format="2"/><globals/><vnodes><v t="root"><vh>Root</vh></v></vnodes><tnodes><t tx="root"></t></tnodes></leo_file>"#;

    fn temp_dir(name: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!(
            "leo-cub-import-{name}-{}-{}",
            std::process::id(),
            now.as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn recursive_import_builds_path_nodes_and_basename_file_nodes() {
        let dir = temp_dir("paths");
        let src = dir.join("src");
        fs::create_dir_all(src.join("nested")).unwrap();
        fs::write(src.join("main.rs"), "fn main() {}\n").unwrap();
        fs::write(src.join("nested/lib.rs"), "pub fn f() {}\n").unwrap();
        let mut document = LeoDocument::parse(SAMPLE).unwrap();

        let report = import_files(
            &mut document,
            &dir.join("project.leo"),
            &[src],
            &ImportOptions {
                mode: ImportMode::Auto,
                recursive: true,
                paths: true,
                parent: Some(NodeId("root".into())),
                dry_run: false,
            },
        )
        .unwrap();

        assert_eq!(report.imported, 2);
        assert_eq!(report.path_nodes, 2);
        let headlines: HashSet<_> = document
            .outline
            .nodes
            .values()
            .map(|node| node.headline.as_str())
            .collect();
        assert!(headlines.contains("@path src"));
        assert!(headlines.contains("@path nested"));
        assert!(headlines.contains("@auto main.rs"));
        assert!(headlines.contains("@auto lib.rs"));
        assert!(document.outline.validate().is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn no_paths_flattens_files_and_dry_run_does_not_mutate() {
        let dir = temp_dir("flat");
        let src = dir.join("src");
        fs::create_dir_all(src.join("nested")).unwrap();
        fs::write(src.join("nested/note.txt"), "hello\n").unwrap();
        let mut document = LeoDocument::parse(SAMPLE).unwrap();
        let before = document.outline.clone();

        let report = import_files(
            &mut document,
            &dir.join("project.leo"),
            &[src],
            &ImportOptions {
                mode: ImportMode::Clean,
                recursive: true,
                paths: false,
                parent: None,
                dry_run: true,
            },
        )
        .unwrap();

        assert_eq!(report.items[0].headline, "@clean src/nested/note.txt");
        assert_eq!(report.path_nodes, 0);
        assert_eq!(document.outline, before);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn auto_import_defers_source_expansion_until_load() {
        let dir = temp_dir("auto-deferred");
        let source = dir.join("sample.py");
        fs::write(&source, "def f():\n    pass\n").unwrap();
        let mut document = LeoDocument::parse(SAMPLE).unwrap();
        let report = import_files(
            &mut document,
            &dir.join("project.leo"),
            &[source],
            &ImportOptions {
                mode: ImportMode::Auto,
                recursive: false,
                paths: false,
                parent: None,
                dry_run: false,
            },
        )
        .unwrap();
        assert!(document.outline.nodes[&report.items[0].gnx].body.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }
}
