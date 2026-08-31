//! Live expansion of `@auto-dir <dir-or-glob>`: enumerates files under a
//! resolved base path, tree-sitter-parses each match via [`AutoFile::parse`],
//! and assembles the results into one synthetic [`AutoFile`] whose root is
//! the `@auto-dir` node. Matches that share a subdirectory (relative to the
//! resolved search root) are nested under synthetic `@path <name>` container
//! nodes mirroring that directory structure, the same shape `cub import
//! --paths` builds by hand, rather than dumped as one flat list of siblings.
//! Because `AutoFile`'s fields are all `pub` and `merge_into` only cares
//! about their shape, this flows through the exact same merge/read-only/
//! write-back-exclusion machinery as a plain single-file `@auto` node -- no
//! other code needs to know `@auto-dir` produced it rather than a real
//! single file.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use globset::Glob;
use walkdir::WalkDir;

use crate::{AutoError, AutoFile, Node, NodeId, Outline, Position};

/// `resolved` is the directive's filename argument already joined onto its
/// `@path`-inherited base (the same resolution every derived directive gets
/// in `derived_jobs`), so it may be a bare directory (`src`), a single-level
/// glob (`src/*.rs`), or a recursive one (`src/**/*.rs`).
pub fn parse_dir(resolved: &Path, root: NodeId) -> Result<AutoFile, AutoError> {
    let (search_root, pattern, recursive) = split_pattern(resolved)?;
    let matcher = Glob::new(&pattern)
        .map_err(|error| AutoError::Dir(format!("{pattern}: {error}")))?
        .compile_matcher();

    let mut walker = WalkDir::new(&search_root).min_depth(1);
    if !recursive {
        walker = walker.max_depth(1);
    }
    let mut matches: Vec<PathBuf> = walker
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        // `.leo` files are outline XML, never a meaningful `@auto` source --
        // excluding them also keeps a pattern like `@auto-dir .` from trying
        // to tree-sitter-parse the very file the pattern is declared in.
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) != Some("leo"))
        .filter_map(|entry| {
            let relative = entry.path().strip_prefix(&search_root).ok()?.to_owned();
            matcher.is_match(&relative).then(|| entry.into_path())
        })
        .collect();
    matches.sort();

    let mut outline = Outline::default();
    let mut locations = HashMap::new();
    let mut file_paths = HashMap::new();
    let mut tree = DirTree::default();

    for path in &matches {
        let relative = path.strip_prefix(&search_root).unwrap_or(path);
        let relative_display = relative.display().to_string();
        // Just the bare filename: directory context now comes from the
        // synthetic `@path` ancestors built below, the same as a
        // hand-written `@path <dir>` / `@auto <name>` pair would carry it.
        // `collect_file` (inspect.rs) and `dynamic_source_location` (tui.rs)
        // both know to stop trusting plain `@path`-ancestor accumulation at
        // an `@auto-dir` boundary -- inspect.rs re-anchors there instead of
        // reconstructing the search root's own name, and tui.rs defers
        // entirely to `AutoFile::file_paths` (via `app.source_nodes`) rather
        // than guess.
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| relative_display.clone());
        let source = fs::read_to_string(path)
            .map_err(|error| AutoError::Dir(format!("{}: {error}", path.display())))?;
        let file_id = NodeId(format!("{}::auto-dir:{relative_display}", root.0));
        let file = AutoFile::parse(path, file_id.clone(), &source)?;

        locations.insert(file_id.clone(), 1);
        for (id, line) in &file.locations {
            locations.insert(id.clone(), *line);
        }
        for id in file.outline.nodes.keys() {
            file_paths.insert(id.clone(), path.clone());
        }

        let file_root_position = file.outline.roots[0].clone();
        for (id, mut node) in file.outline.nodes {
            if id == file_id {
                node.headline = format!("@auto {file_name}");
            }
            outline.nodes.insert(id, node);
        }

        let dirs: Vec<String> = relative
            .parent()
            .into_iter()
            .flat_map(|parent| parent.components())
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        tree.insert(&dirs, file_root_position);
    }

    let children = tree.flatten(&root, &mut outline);

    outline.nodes.insert(
        root.clone(),
        Node {
            id: root.clone(),
            headline: String::new(),
            body: String::new(),
            vnode_attributes: HashMap::new(),
            tnode_attributes: HashMap::new(),
        },
    );
    outline.roots.push(Position {
        node: root.clone(),
        children,
    });

    Ok(AutoFile {
        outline,
        root,
        locations,
        file_paths: Some(file_paths),
    })
}

/// Groups per-file [`Position`]s by the subdirectory (relative to the
/// search root) each match came from, so [`DirTree::flatten`] can turn that
/// grouping into a nested tree of synthetic `@path <name>` nodes instead of
/// one flat sibling list. Directories are keyed by name in `dir_index` so a
/// later match under an already-seen subdirectory extends its existing
/// subtree rather than creating a duplicate.
#[derive(Default)]
struct DirTree {
    entries: Vec<DirEntry>,
    dir_index: HashMap<String, usize>,
}

enum DirEntry {
    Dir(String, DirTree),
    File(Position),
}

impl DirTree {
    /// `dirs` is the match's containing-directory path split into
    /// components, relative to the search root -- empty for a file that
    /// sits directly in the search root itself.
    fn insert(&mut self, dirs: &[String], file: Position) {
        let Some((first, rest)) = dirs.split_first() else {
            self.entries.push(DirEntry::File(file));
            return;
        };
        let index = match self.dir_index.get(first) {
            Some(&index) => index,
            None => {
                let index = self.entries.len();
                self.entries
                    .push(DirEntry::Dir(first.clone(), DirTree::default()));
                self.dir_index.insert(first.clone(), index);
                index
            }
        };
        let DirEntry::Dir(_, subtree) = &mut self.entries[index] else {
            unreachable!("dir_index only ever indexes Dir entries");
        };
        subtree.insert(rest, file);
    }

    /// Consumes the tree, inserting a [`Node`] into `outline` for every
    /// synthetic directory and returning the top-level children (a mix of
    /// per-file positions and directory positions) in first-seen order --
    /// which, since callers insert matches in sorted order, is the same
    /// order a flat listing would have used.
    fn flatten(self, root: &NodeId, outline: &mut Outline) -> Vec<Position> {
        self.flatten_at(root, &PathBuf::new(), outline)
    }

    fn flatten_at(self, root: &NodeId, dir_relative: &Path, outline: &mut Outline) -> Vec<Position> {
        self.entries
            .into_iter()
            .map(|entry| match entry {
                DirEntry::File(position) => position,
                DirEntry::Dir(name, subtree) => {
                    let dir_relative = dir_relative.join(&name);
                    let dir_id = NodeId(format!(
                        "{}::auto-dir:path:{}",
                        root.0,
                        dir_relative.display()
                    ));
                    let children = subtree.flatten_at(root, &dir_relative, outline);
                    outline.nodes.insert(
                        dir_id.clone(),
                        Node {
                            id: dir_id.clone(),
                            headline: format!("@path {name}"),
                            body: String::new(),
                            vnode_attributes: HashMap::new(),
                            tnode_attributes: HashMap::new(),
                        },
                    );
                    Position {
                        node: dir_id,
                        children,
                    }
                }
            })
            .collect()
    }
}

/// Splits a resolved path into a concrete, existing search-root directory
/// and a glob pattern relative to it, plus whether the pattern needs a
/// recursive walk (contains `**`). A path with no glob metacharacters
/// resolves to `(path, "*", false)` when it's an existing directory --
/// matching the one-level, non-recursive listing behaviour of the existing
/// `@path` "import" TUI command -- or `(parent, filename, false)` when it's
/// an existing single file.
fn split_pattern(resolved: &Path) -> Result<(PathBuf, String, bool), AutoError> {
    fn has_glob_chars(text: &str) -> bool {
        text.contains(['*', '?', '['])
    }

    // Collapse literal `.` components (e.g. from an `@auto-dir .` argument
    // joined onto a `@path`-inherited base) so `search_root` -- and every
    // per-file headline built from it -- doesn't carry a stray `/./`.
    let normalized: PathBuf = resolved
        .components()
        .filter(|component| !matches!(component, std::path::Component::CurDir))
        .collect();
    let resolved = if normalized.as_os_str().is_empty() {
        Path::new(".")
    } else {
        normalized.as_path()
    };

    let components: Vec<_> = resolved.components().collect();
    let split_at = components
        .iter()
        .position(|component| has_glob_chars(&component.as_os_str().to_string_lossy()));

    let (search_root, pattern) = match split_at {
        Some(index) => {
            let search_root: PathBuf = components[..index].iter().collect();
            let pattern = components[index..]
                .iter()
                .map(|component| component.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            (search_root, pattern)
        }
        None if resolved.is_dir() => (resolved.to_path_buf(), "*".to_owned()),
        None if resolved.is_file() => (
            resolved
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
            resolved
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
        ),
        None => {
            return Err(AutoError::Dir(format!(
                "{}: no such file or directory",
                resolved.display()
            )));
        }
    };
    let recursive = pattern.contains("**");
    Ok((search_root, pattern, recursive))
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
            "leo-cub-auto-dir-{name}-{}-{}",
            std::process::id(),
            now.as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write(dir: &Path, relative: &str, contents: &str) {
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    /// Headlines of the top-level children, in order.
    fn headlines(auto: &AutoFile) -> Vec<String> {
        auto.outline.roots[0]
            .children
            .iter()
            .map(|position| auto.outline.nodes[&position.node].headline.clone())
            .collect()
    }

    #[test]
    fn bare_directory_lists_immediate_files_non_recursively() {
        let dir = temp_dir("bare-dir");
        write(&dir, "a.py", "def a():\n    pass\n");
        write(&dir, "b.rs", "fn b() {}\n");
        write(&dir, "nested/c.py", "def c():\n    pass\n");

        let auto = parse_dir(&dir, NodeId::from("root")).unwrap();
        assert_eq!(headlines(&auto), vec!["@auto a.py", "@auto b.rs"]);
    }

    #[test]
    fn single_level_glob_filters_by_extension() {
        let dir = temp_dir("single-glob");
        write(&dir, "a.py", "def a():\n    pass\n");
        write(&dir, "b.rs", "fn b() {}\n");

        let auto = parse_dir(&dir.join("*.py"), NodeId::from("root")).unwrap();
        assert_eq!(headlines(&auto), vec!["@auto a.py"]);
    }

    /// Indented rendering of the whole tree's headlines (both `@path` and
    /// `@auto` nodes), so nesting produced by recursive matches shows up as
    /// indentation rather than being flattened away.
    fn render_tree(auto: &AutoFile) -> String {
        fn render(auto: &AutoFile, positions: &[Position], depth: usize, output: &mut String) {
            for position in positions {
                let headline = &auto.outline.nodes[&position.node].headline;
                output.push_str(&"  ".repeat(depth));
                output.push_str(headline);
                output.push('\n');
                render(auto, &position.children, depth + 1, output);
            }
        }
        let mut output = String::new();
        render(auto, &auto.outline.roots[0].children, 0, &mut output);
        output
    }

    #[test]
    fn recursive_glob_walks_subdirectories() {
        let dir = temp_dir("recursive-glob");
        write(&dir, "a.rs", "fn a() {}\n");
        write(&dir, "nested/b.rs", "fn b() {}\n");
        write(&dir, "nested/deeper/c.rs", "fn c() {}\n");

        let auto = parse_dir(&dir.join("**/*.rs"), NodeId::from("root")).unwrap();
        assert_eq!(
            render_tree(&auto),
            "@auto a.rs\n\
             @path nested\n\
             \x20\x20@auto b.rs\n\
             \x20\x20@path deeper\n\
             \x20\x20\x20\x20@auto c.rs\n"
        );
    }

    #[test]
    fn leo_files_are_never_matched() {
        let dir = temp_dir("skip-leo");
        write(&dir, "outline.leo", "<leo_file></leo_file>");
        write(&dir, "a.py", "def a():\n    pass\n");

        let auto = parse_dir(&dir, NodeId::from("root")).unwrap();
        assert_eq!(headlines(&auto), vec!["@auto a.py"]);
    }

    #[test]
    fn matched_files_are_structurally_parsed() {
        let dir = temp_dir("structural");
        write(&dir, "a.py", "class C:\n    def m(self):\n        pass\n");

        let auto = parse_dir(&dir, NodeId::from("root")).unwrap();
        let file_position = &auto.outline.roots[0].children[0];
        let grandchild = &file_position.children[0];
        assert_eq!(auto.outline.nodes[&grandchild.node].headline, "class C");
    }
}
