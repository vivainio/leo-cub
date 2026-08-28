//! Live expansion of `@auto-dir <dir-or-glob>`: enumerates files under a
//! resolved base path, tree-sitter-parses each match via [`AutoFile::parse`],
//! and assembles the results into one synthetic [`AutoFile`] whose root is
//! the `@auto-dir` node and whose children are one per matched file. Because
//! `AutoFile`'s fields are all `pub` and `merge_into` only cares about their
//! shape, this flows through the exact same merge/read-only/write-back-
//! exclusion machinery as a plain single-file `@auto` node -- no other code
//! needs to know `@auto-dir` produced it rather than a real single file.

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
    let mut children = Vec::with_capacity(matches.len());

    for path in &matches {
        let relative = path.strip_prefix(&search_root).unwrap_or(path);
        let relative_display = relative.display().to_string();
        // `path` itself (not stripped to search_root) becomes the headline:
        // `collect_file` in inspect.rs matches `cub inspect <file>` purely
        // by suffix-comparing headline text against the on-disk outline, so
        // a per-file node needs the same directory context a hand-written
        // `@auto <path>` headline would carry, not just its bare filename.
        let path_display = path.display().to_string();
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
                node.headline = format!("@auto {path_display}");
            }
            outline.nodes.insert(id, node);
        }
        children.push(file_root_position);
    }

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

    /// Headlines of the top-level per-file children, with `dir` stripped
    /// back off so assertions read as the relative path a user would
    /// actually type -- the real headline carries the full resolved path
    /// (see the comment at its construction site), not just the filename.
    fn headlines(auto: &AutoFile, dir: &Path) -> Vec<String> {
        auto.outline.roots[0]
            .children
            .iter()
            .map(|position| {
                let headline = &auto.outline.nodes[&position.node].headline;
                let path = headline.strip_prefix("@auto ").unwrap();
                format!(
                    "@auto {}",
                    Path::new(path)
                        .strip_prefix(dir)
                        .unwrap_or(Path::new(path))
                        .display()
                )
            })
            .collect()
    }

    #[test]
    fn bare_directory_lists_immediate_files_non_recursively() {
        let dir = temp_dir("bare-dir");
        write(&dir, "a.py", "def a():\n    pass\n");
        write(&dir, "b.rs", "fn b() {}\n");
        write(&dir, "nested/c.py", "def c():\n    pass\n");

        let auto = parse_dir(&dir, NodeId::from("root")).unwrap();
        assert_eq!(headlines(&auto, &dir), vec!["@auto a.py", "@auto b.rs"]);
    }

    #[test]
    fn single_level_glob_filters_by_extension() {
        let dir = temp_dir("single-glob");
        write(&dir, "a.py", "def a():\n    pass\n");
        write(&dir, "b.rs", "fn b() {}\n");

        let auto = parse_dir(&dir.join("*.py"), NodeId::from("root")).unwrap();
        assert_eq!(headlines(&auto, &dir), vec!["@auto a.py"]);
    }

    #[test]
    fn recursive_glob_walks_subdirectories() {
        let dir = temp_dir("recursive-glob");
        write(&dir, "a.rs", "fn a() {}\n");
        write(&dir, "nested/b.rs", "fn b() {}\n");
        write(&dir, "nested/deeper/c.rs", "fn c() {}\n");

        let auto = parse_dir(&dir.join("**/*.rs"), NodeId::from("root")).unwrap();
        assert_eq!(
            headlines(&auto, &dir),
            vec![
                "@auto a.rs",
                "@auto nested/b.rs",
                "@auto nested/deeper/c.rs"
            ]
        );
    }

    #[test]
    fn leo_files_are_never_matched() {
        let dir = temp_dir("skip-leo");
        write(&dir, "outline.leo", "<leo_file></leo_file>");
        write(&dir, "a.py", "def a():\n    pass\n");

        let auto = parse_dir(&dir, NodeId::from("root")).unwrap();
        assert_eq!(headlines(&auto, &dir), vec!["@auto a.py"]);
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
