//! Rhai scripting, shared by two entry points that both drive an outline
//! through the same [`Doc`] API so neither is a second-class citizen:
//!
//! - `cub run SCRIPT.rhai` ([`run`]): a scriptable, non-interactive
//!   replacement for the old jsonl/TUI-driven integration test suite. A
//!   script opens its own `.leo` file and asserts on the result, exercising
//!   the same library code `cub`'s other subcommands do, without a
//!   terminal.
//! - `@action` bodies ([`run_bound`]), run in-process from inside the TUI
//!   (see `tui::run_action`). Rather than opening a file, the action's
//!   `doc` is bound to the outline already open in the editor, and `target`
//!   is predefined as the gnx of the node the action was invoked on.

use std::collections::{BTreeMap, HashMap};
use std::process::Command;
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{Context, Result};
use leo::{
    LeoDocument, NodeId, Operation, OperationBatch, OriginalExternalState, Position, PositionId,
    WritableExternalFile, external_filename, external_format, load_derived_files, save_document,
    track_external_rename,
};
use regex::Regex;
use rhai::packages::Package;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Scope};

mod cub;

/// The outline handle a script gets back from `open(path)` (or, for a bound
/// `@action` script, from the predefined `doc`). Every method mutates or
/// reads the in-memory document; nothing touches disk until `save`/
/// `save_as` is called. Backed by `Rc<RefCell<Inner>>` rather than owning
/// `Inner` directly so that `Doc::clone()` -- including the clone every
/// `Node` handle (see below) carries -- aliases the same document instead
/// of deep-copying it: a `Node`'s `.h`/`.b` writes are then visible through
/// the original `doc` variable a script already has.
#[derive(Clone)]
pub(crate) struct Doc {
    inner: Rc<RefCell<Inner>>,
}

struct Inner {
    document: LeoDocument,
    path: PathBuf,
    /// Set by any method that mutates `document`. `run_bound` reads this
    /// afterwards so the TUI only marks the outline dirty (and drops
    /// layout/highlight caches) when a bound script actually changed
    /// something, rather than on every run.
    touched: bool,
    /// `@file`/`@thin`/`@file-thin`/`@f`/`@clean` nodes `open` loaded (or a
    /// script's `set_headline` turned into one -- see [`Doc::set_headline`]),
    /// keyed by root node id. Mirrors the TUI App's own field of the same
    /// name and drives `save`'s write-back the same way `tui::save` does:
    /// a node here whose live subtree has diverged from `original` gets
    /// rendered and written to `path` on save. Always empty for a `bind`ed
    /// (`@action`) `Doc` -- the TUI already tracks and writes those itself.
    writable_external: HashMap<NodeId, WritableExternalFile>,
    /// The on-disk shape (children/bodies/nodes) `writable_external` and
    /// `@auto` nodes had before their content was merged in, so `save` can
    /// restore it into the `.leo` XML being written -- derived/writable
    /// content lives in its own file, never baked into the `.leo` file.
    original_external: OriginalExternalState,
}

type RhaiResult<T> = Result<T, Box<EvalAltResult>>;

fn rhai_err(message: impl std::fmt::Display) -> Box<EvalAltResult> {
    message.to_string().into()
}

fn ids_to_array(ids: Vec<NodeId>) -> Array {
    ids.into_iter().map(|id| Dynamic::from(id.0)).collect()
}

fn json_value_to_dynamic(value: serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(b),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(Dynamic::from)
            .unwrap_or_else(|| Dynamic::from(n.as_f64().unwrap_or(0.0))),
        serde_json::Value::String(s) => Dynamic::from(s),
        serde_json::Value::Array(items) => Dynamic::from(
            items
                .into_iter()
                .map(json_value_to_dynamic)
                .collect::<Array>(),
        ),
        serde_json::Value::Object(fields) => {
            let mut map = rhai::Map::new();
            for (key, value) in fields {
                map.insert(key.into(), json_value_to_dynamic(value));
            }
            Dynamic::from(map)
        }
    }
}

/// Parses `json` (object, array, or scalar -- unlike Rhai's built-in
/// `parse_json`, which only accepts an object) into the matching Rhai
/// value. The obvious companion to `sh`: a script piping a subprocess's
/// stdout (`gh pr list --json ...`, ...) through here gets Rhai arrays/maps
/// back instead of having to pick the JSON apart as a string. Registered
/// under the same name as Rhai's built-in `parse_json`, replacing it.
fn parse_json(json: &str) -> RhaiResult<Dynamic> {
    serde_json::from_str(json)
        .map(json_value_to_dynamic)
        .map_err(rhai_err)
}

/// Reports whether `pattern` matches anywhere in `text`. Fails if `pattern`
/// isn't a valid regular expression -- the same syntax `cub inspect
/// --search` and `doc.find_h`/`doc.find_b` use.
fn regex_is_match(pattern: &str, text: &str) -> RhaiResult<bool> {
    let regex = Regex::new(pattern).map_err(rhai_err)?;
    Ok(regex.is_match(text))
}

/// Returns the leftmost match of `pattern` in `text`, or `()` when there is
/// none.
fn regex_find(pattern: &str, text: &str) -> RhaiResult<Dynamic> {
    let regex = Regex::new(pattern).map_err(rhai_err)?;
    Ok(regex
        .find(text)
        .map(|m| Dynamic::from(m.as_str().to_string()))
        .unwrap_or(Dynamic::UNIT))
}

/// Returns every non-overlapping match of `pattern` in `text`, left to
/// right, as an array of strings -- `[]` when there are none.
fn regex_find_all(pattern: &str, text: &str) -> RhaiResult<Array> {
    let regex = Regex::new(pattern).map_err(rhai_err)?;
    Ok(regex
        .find_iter(text)
        .map(|m| Dynamic::from(m.as_str().to_string()))
        .collect())
}

/// Returns the leftmost match's capture groups as an array of strings --
/// index 0 is the whole match, followed by one entry per `(...)` group,
/// with `()` for a group the match didn't reach (e.g. inside an
/// unmatched `|` alternative). Returns `()`, not `[]`, when `pattern`
/// doesn't match `text` at all, so callers can tell "no match" from
/// "matched, no groups".
fn regex_captures(pattern: &str, text: &str) -> RhaiResult<Dynamic> {
    let regex = Regex::new(pattern).map_err(rhai_err)?;
    Ok(match regex.captures(text) {
        Some(captures) => {
            let groups: Array = captures
                .iter()
                .map(|group| {
                    group
                        .map(|m| Dynamic::from(m.as_str().to_string()))
                        .unwrap_or(Dynamic::UNIT)
                })
                .collect();
            Dynamic::from(groups)
        }
        None => Dynamic::UNIT,
    })
}

/// Replaces the leftmost match of `pattern` in `text` with `replacement`
/// (`$1`-style references reach into capture groups, same as the `regex`
/// crate) and returns the result. Returns `text` unchanged when `pattern`
/// doesn't match.
fn regex_replace(pattern: &str, text: &str, replacement: &str) -> RhaiResult<String> {
    let regex = Regex::new(pattern).map_err(rhai_err)?;
    Ok(regex.replace(text, replacement).into_owned())
}

/// Same as `regex_replace`, but replaces every non-overlapping match
/// instead of only the leftmost.
fn regex_replace_all(pattern: &str, text: &str, replacement: &str) -> RhaiResult<String> {
    let regex = Regex::new(pattern).map_err(rhai_err)?;
    Ok(regex.replace_all(text, replacement).into_owned())
}

/// Runs `cmd` through `sh -c`, inheriting `cub`'s own working directory
/// unless `opts` overrides it with a `cwd` entry. Always succeeds and hands
/// back a `#{stdout, stderr, code}` map -- a nonzero exit is something the
/// caller decides how to handle, not something this function judges -- the
/// `Err` case is reserved for `sh` itself failing to launch (e.g. missing
/// from `PATH`).
fn run_shell(cmd: &str, opts: &rhai::Map) -> RhaiResult<rhai::Map> {
    let mut command = Command::new("sh");
    command.arg("-c").arg(cmd);
    if let Some(cwd) = opts.get("cwd") {
        command.current_dir(cwd.clone().into_string().map_err(|type_name| {
            rhai_err(format!("sh: `cwd` must be a string, got {type_name}"))
        })?);
    }
    let output = command
        .output()
        .map_err(|error| rhai_err(format!("failed to run '{cmd}': {error}")))?;
    let mut result = rhai::Map::new();
    result.insert(
        "stdout".into(),
        Dynamic::from(String::from_utf8_lossy(&output.stdout).into_owned()),
    );
    result.insert(
        "stderr".into(),
        Dynamic::from(String::from_utf8_lossy(&output.stderr).into_owned()),
    );
    result.insert(
        "code".into(),
        Dynamic::from(output.status.code().unwrap_or(-1) as i64),
    );
    Ok(result)
}

/// `sh(cmd)`: runs in `cub`'s own working directory.
fn sh_default_cwd(cmd: &str) -> RhaiResult<rhai::Map> {
    run_shell(cmd, &rhai::Map::new())
}

/// `sh(cmd, opts)`: `opts` is currently just `#{cwd: path}`, to override
/// the default working directory.
fn sh_with_opts(cmd: &str, opts: rhai::Map) -> RhaiResult<rhai::Map> {
    run_shell(cmd, &opts)
}

/// Reads an environment variable, `""` when unset -- lets a script build a
/// path under `$HOME` (or honor `$XDG_*`, `$EDITOR`, ...) without shelling
/// out to `sh("echo -n $HOME")` just to ask the process's own environment.
fn env_var(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

/// `root`'s subtree, depth-first and in outline order (`root` itself
/// first), paired with each occurrence's own `PositionId` -- the
/// exact-occurrence counterpart to `Outline::subtree_ids`, which only
/// knows node ids and can't tell two clones of the same node apart.
/// `own_position` is the index path `root` itself sits at.
fn subtree_with_positions(root: &Position, own_position: &str) -> Vec<(NodeId, PositionId)> {
    fn visit(position: &Position, path: String, out: &mut Vec<(NodeId, PositionId)>) {
        out.push((position.node.clone(), PositionId(path.clone())));
        for (i, child) in position.children.iter().enumerate() {
            visit(child, format!("{path}/{i}"), out);
        }
    }
    let mut out = Vec::new();
    visit(root, own_position.to_owned(), &mut out);
    out
}

fn find_node<'a>(document: &'a LeoDocument, gnx: &str) -> RhaiResult<&'a leo::Node> {
    document
        .outline
        .nodes
        .get(&NodeId(gnx.to_owned()))
        .ok_or_else(|| rhai_err(format!("node not found: {gnx}")))
}

fn find_node_mut<'a>(document: &'a mut LeoDocument, gnx: &str) -> RhaiResult<&'a mut leo::Node> {
    document
        .outline
        .nodes
        .get_mut(&NodeId(gnx.to_owned()))
        .ok_or_else(|| rhai_err(format!("node not found: {gnx}")))
}

impl Doc {
    fn new(
        document: LeoDocument,
        path: PathBuf,
        writable_external: HashMap<NodeId, WritableExternalFile>,
        original_external: OriginalExternalState,
    ) -> Doc {
        Doc {
            inner: Rc::new(RefCell::new(Inner {
                document,
                path,
                touched: false,
                writable_external,
                original_external,
            })),
        }
    }

    /// Binds an already-open document instead of reading one from disk --
    /// used to hand an `@action` script the outline the TUI already has in
    /// memory, so it sees in-progress edits and its mutations flow straight
    /// back into the editor instead of round-tripping through disk. The TUI
    /// already ran its own derived-file load on `document` and owns
    /// write-back for it via the App's own `writable_external`, so this
    /// `Doc` starts with none of its own -- a bound script's `save`/
    /// `save_as` only serializes the `.leo` XML, same as before.
    #[cfg(feature = "tui")]
    pub(crate) fn bind(document: LeoDocument, path: PathBuf) -> Doc {
        Doc::new(
            document,
            path,
            HashMap::new(),
            OriginalExternalState::default(),
        )
    }

    #[cfg(feature = "tui")]
    pub(crate) fn touched(&self) -> bool {
        self.inner.borrow().touched
    }

    #[cfg(feature = "tui")]
    pub(crate) fn into_document(self) -> LeoDocument {
        match Rc::try_unwrap(self.inner) {
            Ok(cell) => cell.into_inner().document,
            // Some other clone of this `Doc` (e.g. a `Node` a script never
            // let go of) is still alive; fall back to copying the document
            // out rather than losing it.
            Err(shared) => shared.borrow().document.clone(),
        }
    }

    /// Opens `path` the same way the TUI does: `@auto`/`@file`/`@thin`/
    /// `@file-thin`/`@f`/`@clean` nodes get their external content parsed
    /// and merged in immediately (see `leo::load_derived_files`), not left
    /// as bare, unexpanded headline-only nodes. `@file`/`@thin`/`@f`/
    /// `@clean` also get write-back tracking, so a later `save` renders and
    /// writes any that have diverged -- e.g. after `set_headline` promotes
    /// an `@auto` node to `@f`, `save` writes real sentinels to its file,
    /// exactly like renaming and saving in the TUI does.
    fn open(path: &str) -> RhaiResult<Doc> {
        let mut document = LeoDocument::open(path).map_err(rhai_err)?;
        let report = load_derived_files(&mut document.outline, Path::new(path));
        // A load failure (e.g. a file whose on-disk sentinel format no
        // longer matches what this node's headline expects -- which can
        // happen when two different .leo outlines share a gnx for the same
        // external file and a script converts/saves both in one run)
        // leaves that node's body/children exactly as they were before the
        // failed load, typically empty. Silently proceeding is dangerous:
        // a later `save()` would then happily render and write that empty
        // state, destroying whatever correct content was already on disk.
        // The TUI's own open surfaces the same report.errors as a status
        // message (see open_document in tui.rs); scripts get no status
        // bar, so this prints to stderr instead.
        for error in &report.errors {
            eprintln!("cub: {path}: {error}");
        }
        let original_external = OriginalExternalState {
            children: report.original_children,
            bodies: report.original_bodies,
            nodes: report.original_nodes,
        };
        Ok(Doc::new(
            document,
            PathBuf::from(path),
            report.writable_external,
            original_external,
        ))
    }

    /// Ensures a slash-separated headline path exists (creating any missing
    /// segments, reusing existing ones) and returns the leaf as a `Node`.
    fn ensure(&mut self, path: &str) -> RhaiResult<Node> {
        {
            let mut inner = self.inner.borrow_mut();
            inner
                .document
                .outline
                .add_headline_paths(&[path.to_owned()])
                .map_err(rhai_err)?;
            inner.touched = true;
        }
        let gnx = self.gnx(path)?;
        Ok(Node {
            doc: self.clone(),
            gnx,
            position: None,
        })
    }

    /// Resolves a slash-separated headline path to its gnx without creating
    /// anything; fails if the path doesn't exist or is ambiguous.
    fn gnx(&mut self, path: &str) -> RhaiResult<String> {
        self.inner
            .borrow()
            .document
            .outline
            .resolve_headline_path(path)
            .map(|id| id.0)
            .map_err(rhai_err)
    }

    /// The gnxs of the outline's top-level nodes, in outline order.
    fn roots(&mut self) -> Array {
        ids_to_array(self.inner.borrow().document.outline.root_ids())
    }

    fn children_ids(&self, gnx: &str) -> RhaiResult<Vec<NodeId>> {
        let inner = self.inner.borrow();
        find_node(&inner.document, gnx)?;
        Ok(inner
            .document
            .outline
            .children_of(&NodeId(gnx.to_owned()))
            .unwrap_or_default())
    }

    /// The gnxs of `gnx`'s children, in outline order (empty if it's a
    /// leaf); fails if `gnx` isn't a node in the outline.
    fn children(&mut self, gnx: &str) -> RhaiResult<Array> {
        Ok(ids_to_array(self.children_ids(gnx)?))
    }

    fn subtree_ids(&self, gnx: &str) -> RhaiResult<Vec<NodeId>> {
        self.inner
            .borrow()
            .document
            .outline
            .subtree_ids(&NodeId(gnx.to_owned()))
            .ok_or_else(|| rhai_err(format!("node not found: {gnx}")))
    }

    /// The gnxs of `gnx` and everything under it, depth-first and in
    /// outline order (`gnx` itself first) -- the flattened equivalent of
    /// Leo's `p.self_and_subtree()`. Fails if `gnx` isn't a node in the
    /// outline.
    fn subtree(&mut self, gnx: &str) -> RhaiResult<Array> {
        Ok(ids_to_array(self.subtree_ids(gnx)?))
    }

    /// The gnxs of every node in the outline, depth-first and in outline
    /// order -- the flattened equivalent of Leo's `c.all_positions()`. A
    /// node cloned to more than one position appears once per occurrence.
    fn all(&mut self) -> Array {
        ids_to_array(self.inner.borrow().document.outline.all_ids())
    }

    /// `gnx`'s parent's gnx, or `""` if `gnx` is a root; fails if `gnx`
    /// isn't a node in the outline.
    fn parent(&mut self, gnx: &str) -> RhaiResult<String> {
        let inner = self.inner.borrow();
        find_node(&inner.document, gnx)?;
        Ok(inner
            .document
            .outline
            .parent_of(&NodeId(gnx.to_owned()))
            .map(|id| id.0)
            .unwrap_or_default())
    }

    /// The slash-separated headline path from the root down to `gnx`,
    /// escaped the same way `doc.gnx(path)`/`doc.ensure(path)` expect, so it
    /// can be fed straight back into either of them.
    fn path(&mut self, gnx: &str) -> RhaiResult<String> {
        self.inner
            .borrow()
            .document
            .outline
            .headline_path_of(&NodeId(gnx.to_owned()))
            .ok_or_else(|| rhai_err(format!("node not found: {gnx}")))
    }

    /// The on-disk path `gnx`'s `@file`/`@thin`/`@file-thin`/`@clean`/`@f`
    /// body syncs to, resolved the same way `cub sync` finds it -- the
    /// outline's own directory plus every ancestor (and `gnx`'s own) `@path`
    /// directive, plus the filename in `gnx`'s headline. `""` if `gnx` isn't
    /// itself an external-file node (an ordinary node with a `@path`
    /// ancestor has no on-disk path of its own -- `@path` only names a
    /// directory for descendants that *are* external-file nodes). Fails if
    /// `gnx` isn't a node in the outline.
    fn file_path(&mut self, gnx: &str) -> RhaiResult<String> {
        let inner = self.inner.borrow();
        find_node(&inner.document, gnx)?;
        Ok(leo::external_file_path(
            &inner.document.outline,
            &inner.path,
            &NodeId(gnx.to_owned()),
        )
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default())
    }

    /// Wraps `gnx` as a `Node` bound to this `Doc` -- lets a script hold a
    /// handle that supports `.h`/`.b` property syntax and object-based
    /// traversal (`.parent()`, `.children()`) instead of re-passing `gnx`
    /// to a `Doc` method every time. Fails if `gnx` isn't a node in the
    /// outline.
    fn node(&mut self, gnx: &str) -> RhaiResult<Node> {
        find_node(&self.inner.borrow().document, gnx)?;
        Ok(Node {
            doc: self.clone(),
            gnx: gnx.to_owned(),
            position: None,
        })
    }

    /// Wraps the exact tree occurrence `position` (an index path like
    /// `"0/2/1"`, as returned by `p.position` or `doc.all_positions()`) as
    /// a `Node` -- like `doc.node(gnx)`, but anchored to *this* occurrence
    /// rather than falling back to the first one, so it stays correct for
    /// a node cloned to more than one place. Fails if `position` doesn't
    /// resolve to anything in the outline.
    fn node_at(&mut self, position: &str) -> RhaiResult<Node> {
        let position = PositionId(position.to_owned());
        let gnx = self
            .inner
            .borrow()
            .document
            .outline
            .position(&position)
            .map(|p| p.node.0.clone())
            .ok_or_else(|| rhai_err(format!("position not found: {}", position.0)))?;
        Ok(Node {
            doc: self.clone(),
            gnx,
            position: Some(position),
        })
    }

    /// Runs `pattern` (a regex -- the same syntax `cub inspect --search`
    /// takes) over the outline and keeps the matches `keep` accepts,
    /// deduped to one `Node` per node (its first occurrence, same as
    /// `doc.children`/`.parent` -- a node matched at more than one clone
    /// occurrence still yields once). Fails if `pattern` isn't valid.
    fn find_matching(
        &self,
        pattern: &str,
        keep: impl Fn(&leo::SearchMatch) -> bool,
    ) -> RhaiResult<Array> {
        let regex = Regex::new(pattern).map_err(rhai_err)?;
        let matches = leo::search_outline(&self.inner.borrow().document.outline, &[regex]);
        let mut seen = std::collections::BTreeSet::new();
        Ok(matches
            .into_iter()
            .filter(keep)
            .filter_map(|m| {
                let gnx = m.gnx.0;
                seen.insert(gnx.clone()).then(|| {
                    Dynamic::from(Node {
                        doc: self.clone(),
                        gnx,
                        position: None,
                    })
                })
            })
            .collect())
    }

    /// Nodes whose headline matches `pattern`, as `Node`s in outline
    /// order. See [`find_matching`](Doc::find_matching).
    fn find_h(&mut self, pattern: &str) -> RhaiResult<Array> {
        self.find_matching(pattern, |m| m.headline_match)
    }

    /// Nodes whose body matches `pattern`, as `Node`s in outline order.
    /// See [`find_matching`](Doc::find_matching).
    fn find_b(&mut self, pattern: &str) -> RhaiResult<Array> {
        self.find_matching(pattern, |m| !m.excerpts.is_empty())
    }

    fn headline(&mut self, gnx: &str) -> RhaiResult<String> {
        let inner = self.inner.borrow();
        Ok(find_node(&inner.document, gnx)?.headline.clone())
    }

    /// Sets `gnx`'s headline. When the new headline is itself an external
    /// directive (`@file`/`@thin`/`@file-thin`/`@f`/`@clean`), this also
    /// starts (or updates) write-back tracking for it -- the same thing the
    /// TUI's interactive headline rename does -- so a later `save` renders
    /// and writes it. That's what promotes an `@auto <path>` node (already
    /// carrying real content from `open`'s derived-file load) in place into
    /// a real `@f <path>` file: rename, then save. The path resolves every
    /// ancestor `@path` directive the same way `file_path`/
    /// `external_file_path` do, so a node several `@path` levels deep from
    /// the open `.leo` file lands where it actually belongs instead of next
    /// to the `.leo` file itself. `external_file_path` only recognizes the
    /// writable directives it covers (not the `@auto` family, which
    /// `external_filename` here also matches, for the promote-in-place case
    /// above) -- for anything it doesn't resolve, fall back to the flat
    /// `.leo`-directory-relative path this always used before.
    fn set_headline(&mut self, gnx: &str, text: &str) -> RhaiResult<()> {
        let mut inner = self.inner.borrow_mut();
        find_node_mut(&mut inner.document, gnx)?.headline = text.to_owned();
        inner.touched = true;
        if let Some(filename) = external_filename(text) {
            let node_id = NodeId(gnx.to_owned());
            let path = leo::external_file_path(&inner.document.outline, &inner.path, &node_id)
                .unwrap_or_else(|| {
                    let base = inner
                        .path
                        .parent()
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| PathBuf::from("."));
                    base.join(filename)
                });
            track_external_rename(
                &mut inner.writable_external,
                node_id,
                path,
                external_format(text),
            );
        }
        Ok(())
    }

    fn body(&mut self, gnx: &str) -> RhaiResult<String> {
        let inner = self.inner.borrow();
        Ok(find_node(&inner.document, gnx)?.body.clone())
    }

    fn set_body(&mut self, gnx: &str, text: &str) -> RhaiResult<()> {
        let mut inner = self.inner.borrow_mut();
        find_node_mut(&mut inner.document, gnx)?.body = text.to_owned();
        inner.touched = true;
        Ok(())
    }

    /// Inserts a new occurrence of `gnx` as the last child of `parent_gnx`.
    /// Fails if either `gnx` or `parent_gnx` isn't already a node in the
    /// outline -- nothing is created. Returns `gnx` unchanged, so the call
    /// can be chained. If a script only has a headline path for the parent,
    /// resolve it first with `doc.gnx(path)`/`doc.ensure(path)` rather than
    /// passing the path here directly: every `Doc` method takes a gnx, so
    /// there's exactly one place a path ever needs resolving.
    fn clone_node(&mut self, gnx: &str, parent_gnx: &str) -> RhaiResult<String> {
        self.clone_operation(gnx, parent_gnx, None)
    }

    /// The `clone_node(gnx, parent_gnx, index)` overload: inserts at a
    /// specific position among `parent_gnx`'s existing children instead of
    /// appending.
    fn clone_node_with_index(
        &mut self,
        gnx: &str,
        parent_gnx: &str,
        index: i64,
    ) -> RhaiResult<String> {
        self.clone_operation(gnx, parent_gnx, Some(index as usize))
    }

    fn clone_operation(
        &mut self,
        gnx: &str,
        parent_gnx: &str,
        index: Option<usize>,
    ) -> RhaiResult<String> {
        let batch = OperationBatch {
            operations: vec![Operation::Clone {
                parent: Some(NodeId(parent_gnx.to_owned())),
                parent_headline: None,
                index,
                node: NodeId(gnx.to_owned()),
            }],
            ..Default::default()
        };
        let mut inner = self.inner.borrow_mut();
        inner.document.outline.apply(&batch).map_err(rhai_err)?;
        inner.touched = true;
        Ok(gnx.to_owned())
    }

    /// Removes `gnx`'s defining (first-in-outline) occurrence and its whole
    /// subtree from the outline. If `gnx` is cloned elsewhere, those other
    /// occurrences are left in place -- the next one in outline order
    /// becomes the new defining occurrence. Fails if `gnx` isn't a node in
    /// the outline. A bare gnx can't name a specific clone occurrence --
    /// reach for [`Node::remove`] instead when a script holds a
    /// position-anchored handle (`p`, `doc.node_at(...)`, `.children()`,
    /// ...) and means "remove the one I'm looking at", not "remove
    /// whichever occurrence happens to be first".
    fn remove(&mut self, gnx: &str) -> RhaiResult<()> {
        let batch = OperationBatch {
            operations: vec![Operation::ReplaceTree {
                node: Some(NodeId(gnx.to_owned())),
                headline: None,
                tree: BTreeMap::new(),
            }],
            ..Default::default()
        };
        let mut inner = self.inner.borrow_mut();
        inner.document.outline.apply(&batch).map_err(rhai_err)?;
        inner.touched = true;
        Ok(())
    }

    fn render(&mut self) -> String {
        leo::render_compact(&self.inner.borrow().document.outline)
    }

    fn count(&mut self) -> i64 {
        self.inner.borrow().document.outline.nodes.len() as i64
    }

    fn validate(&mut self) -> Array {
        self.inner
            .borrow()
            .document
            .outline
            .validate()
            .into_iter()
            .map(|error| Dynamic::from(error.to_string()))
            .collect()
    }

    /// Applies a `cub apply`-style JSON operation batch and returns the
    /// report as a JSON string.
    fn apply(&mut self, json: &str) -> RhaiResult<String> {
        let batch: OperationBatch = serde_json::from_str(json).map_err(rhai_err)?;
        let mut inner = self.inner.borrow_mut();
        let report = inner.document.outline.apply(&batch).map_err(rhai_err)?;
        inner.touched = true;
        serde_json::to_string(&report).map_err(rhai_err)
    }

    fn save(&mut self) -> RhaiResult<()> {
        let path = self.inner.borrow().path.clone();
        self.save_to(&path)
    }

    fn save_as(&mut self, path: &str) -> RhaiResult<()> {
        self.save_to(Path::new(path))?;
        self.inner.borrow_mut().path = PathBuf::from(path);
        Ok(())
    }

    /// Writes any diverged `writable_external` file (see `set_headline`)
    /// out with its own sentinels first, then serializes `document` to
    /// `.leo` XML at `path`. Mirrors `tui::save` exactly (both call the
    /// same `leo::save_document`), so a script's `open` -> `set_headline`
    /// -> `save` produces the same on-disk result the same steps in the
    /// TUI would.
    fn save_to(&mut self, path: &Path) -> RhaiResult<()> {
        let mut inner = self.inner.borrow_mut();
        let Inner {
            document,
            writable_external,
            original_external,
            ..
        } = &mut *inner;
        save_document(&*document, path, writable_external, original_external).map_err(rhai_err)
    }

    /// The directory holding this `Doc`'s `.leo` file, or `None` if `path`
    /// is a bare filename with no directory component (in which case that
    /// directory is already `cub`'s own working directory).
    fn dir(&self) -> Option<PathBuf> {
        self.inner
            .borrow()
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(std::path::Path::to_path_buf)
    }

    /// `dir()` as a script-facing string, `"."` standing in for `None` --
    /// for building a path next to the open `.leo` file, e.g.
    /// `open_file(doc.dir() + "/notes.txt")` or `sh("cd " + shq(doc.dir())
    /// + " && ...")`.
    fn dir_string(&mut self) -> String {
        self.dir()
            .map(|dir| dir.display().to_string())
            .unwrap_or_else(|| ".".to_string())
    }
}

/// A `Doc` node bound to one `gnx`, returned by `doc.node(gnx)`. Property
/// access (`.h`, `.b`) and traversal (`.parent()`, `.children()`) all read
/// and write through the same `Doc` the handle was made from, so `n.h =
/// "x"` and `doc.headline(gnx)` see the same data, and `n.children()` hands
/// back further `Node`s rather than bare gnx strings.
///
/// `gnx` stays the handle's identity -- it's what `.h`/`.b`/`.gnx` read and
/// write, and it's what stays valid if the outline changes around it, same
/// as always. `position`, when known, is *additional*: a snapshot of which
/// exact tree occurrence this handle came from, so a node cloned to more
/// than one place can still be told apart from its other clones. It's
/// `None` for a handle made from a bare gnx (`doc.node(gnx)`, `find_h`,
/// …), where only the node id is known and position-sensitive operations
/// (`.path()`, `.parent()`, …) fall back to the first occurrence, same as
/// `doc.path(gnx)` today. `.parent()`/`.children()`/`.subtree()` carry a
/// known `position` forward to the `Node`s they return, so once a script
/// has one positioned handle, everything derived from it stays anchored to
/// the right occurrence too.
#[derive(Clone)]
pub(crate) struct Node {
    doc: Doc,
    gnx: String,
    position: Option<PositionId>,
}

impl Node {
    fn gnx(&mut self) -> String {
        self.gnx.clone()
    }

    fn get_h(&mut self) -> RhaiResult<String> {
        self.doc.headline(&self.gnx)
    }

    fn set_h(&mut self, text: String) -> RhaiResult<()> {
        self.doc.set_headline(&self.gnx, &text)
    }

    fn get_b(&mut self) -> RhaiResult<String> {
        self.doc.body(&self.gnx)
    }

    fn set_b(&mut self, text: String) -> RhaiResult<()> {
        self.doc.set_body(&self.gnx, &text)
    }

    /// The exact tree occurrence this handle is anchored to, as an index
    /// path (`"0/2/1"`), or `""` if this handle only knows a bare gnx --
    /// see the type doc for what that means for `.path()`/`.parent()`/etc.
    /// A snapshot from when the handle was made, not re-validated against
    /// later tree edits -- like `gnx`, but unlike `gnx` it isn't kept
    /// correct if the outline changes underneath it, so treat it as a hint
    /// rather than something to hold onto across mutations.
    fn get_position(&mut self) -> String {
        self.position
            .as_ref()
            .map_or_else(String::new, |p| p.0.clone())
    }

    /// The parent `Node`, or a `Node` wrapping `""` if this one is a root
    /// (matching `Doc::parent`'s contract) -- further property or
    /// traversal access on that empty-gnx `Node` then fails the same way
    /// it would for any nonexistent gnx. When this handle knows its exact
    /// `position`, the parent is read off that occurrence directly instead
    /// of `Doc::parent`'s first-occurrence search, so it's still correct
    /// for a node cloned to more than one place.
    fn parent(&mut self) -> RhaiResult<Node> {
        if let Some(position) = &self.position {
            let inner = self.doc.inner.borrow();
            let outline = &inner.document.outline;
            return Ok(match outline.parent_position(position) {
                Some(parent_position) => {
                    let gnx = outline
                        .position(&parent_position)
                        .map(|p| p.node.0.clone())
                        .unwrap_or_default();
                    Node {
                        doc: self.doc.clone(),
                        gnx,
                        position: Some(parent_position),
                    }
                }
                None => Node {
                    doc: self.doc.clone(),
                    gnx: String::new(),
                    position: None,
                },
            });
        }
        let parent_gnx = self.doc.parent(&self.gnx)?;
        Ok(Node {
            doc: self.doc.clone(),
            gnx: parent_gnx,
            position: None,
        })
    }

    fn children(&mut self) -> RhaiResult<Array> {
        if let Some(position) = &self.position {
            let inner = self.doc.inner.borrow();
            let children = inner
                .document
                .outline
                .position(position)
                .ok_or_else(|| rhai_err(format!("position not found: {}", position.0)))?
                .children
                .iter()
                .enumerate()
                .map(|(i, child)| {
                    Dynamic::from(Node {
                        doc: self.doc.clone(),
                        gnx: child.node.0.clone(),
                        position: Some(PositionId(format!("{}/{i}", position.0))),
                    })
                })
                .collect();
            return Ok(children);
        }
        Ok(self
            .doc
            .children_ids(&self.gnx)?
            .into_iter()
            .map(|id| {
                Dynamic::from(Node {
                    doc: self.doc.clone(),
                    gnx: id.0,
                    position: None,
                })
            })
            .collect())
    }

    /// This `Node` and its ancestors, nearest first, ending at the root --
    /// the upward counterpart to `subtree()` (this node and its
    /// descendants). Just `parent()` called repeatedly until it wraps `""`
    /// (the root-has-no-parent sentinel `parent()` itself returns), so it
    /// inherits the same position-aware-when-possible correctness for
    /// cloned content.
    fn parents(&mut self) -> RhaiResult<Array> {
        let mut result: Array = vec![Dynamic::from(self.clone())];
        let mut current = self.clone();
        loop {
            let next = current.parent()?;
            if next.gnx.is_empty() {
                break;
            }
            result.push(Dynamic::from(next.clone()));
            current = next;
        }
        Ok(result)
    }

    /// This `Node` and everything under it, depth-first and in outline
    /// order (this node itself first) -- the `Node`-returning equivalent
    /// of `doc.subtree(gnx)`.
    fn subtree(&mut self) -> RhaiResult<Array> {
        if let Some(position) = &self.position {
            let inner = self.doc.inner.borrow();
            let root = inner
                .document
                .outline
                .position(position)
                .ok_or_else(|| rhai_err(format!("position not found: {}", position.0)))?;
            let items = subtree_with_positions(root, &position.0);
            drop(inner);
            return Ok(items
                .into_iter()
                .map(|(id, position)| {
                    Dynamic::from(Node {
                        doc: self.doc.clone(),
                        gnx: id.0,
                        position: Some(position),
                    })
                })
                .collect());
        }
        Ok(self
            .doc
            .subtree_ids(&self.gnx)?
            .into_iter()
            .map(|id| {
                Dynamic::from(Node {
                    doc: self.doc.clone(),
                    gnx: id.0,
                    position: None,
                })
            })
            .collect())
    }

    /// The slash-separated headline path down to this occurrence. Uses the
    /// exact `position` when known, so it names *this* clone rather than
    /// whichever occurrence of `gnx` happens to be first in outline order.
    fn path(&mut self) -> RhaiResult<String> {
        match &self.position {
            Some(position) => self
                .doc
                .inner
                .borrow()
                .document
                .outline
                .headline_path_at(position)
                .ok_or_else(|| rhai_err(format!("position not found: {}", position.0))),
            None => self.doc.path(&self.gnx),
        }
    }

    fn file_path(&mut self) -> RhaiResult<String> {
        self.doc.file_path(&self.gnx)
    }

    /// Removes this handle's exact occurrence and its subtree, leaving every
    /// other clone of the same node untouched -- including its defining
    /// occurrence, if this isn't it. Needs a known `position` (built by
    /// walking the tree -- `p`, `.parent()`, `.children()`, `.subtree()`,
    /// `doc.node_at(...)`) to know which occurrence that is; a handle made
    /// from a bare gnx (`doc.node(gnx)`, `find_h`/`find_b`) falls back to
    /// [`Doc::remove`]'s defining-occurrence behavior, the same fallback
    /// `.parent()`/`.path()` use when `position` isn't known.
    fn remove(&mut self) -> RhaiResult<()> {
        let Some(position) = self.position.clone() else {
            return self.doc.remove(&self.gnx);
        };
        let batch = OperationBatch {
            operations: vec![Operation::Remove { position }],
            ..Default::default()
        };
        let mut inner = self.doc.inner.borrow_mut();
        inner.document.outline.apply(&batch).map_err(rhai_err)?;
        inner.touched = true;
        Ok(())
    }

    fn describe(&mut self) -> String {
        format!("Node({})", self.gnx)
    }
}

/// Best-effort equality across the handful of scalar types a test script is
/// likely to compare (`assert_eq(doc.count(), 3)`, `assert_eq(doc.headline(gnx),
/// "C")`, ...); anything else falls back to string comparison.
fn dynamic_eq(a: &Dynamic, b: &Dynamic) -> bool {
    if let (Some(a), Some(b)) = (a.as_int().ok(), b.as_int().ok()) {
        return a == b;
    }
    if let (Some(a), Some(b)) = (a.as_float().ok(), b.as_float().ok()) {
        return a == b;
    }
    if let (Some(a), Some(b)) = (a.as_bool().ok(), b.as_bool().ok()) {
        return a == b;
    }
    a.to_string() == b.to_string()
}

/// Registers the `Doc` API and assertion helpers shared by every Rhai entry
/// point; callers differ only in how output is captured and how (or
/// whether) a script obtains its `Doc`.
fn register_doc_api(engine: &mut Engine) {
    // Rhai's defaults (32 levels of expression nesting inside a function,
    // 64 at a script's top level) are sized to guard against pathological
    // or malicious input, not ordinary code -- a multi-line string build
    // (each `+` nests one level deeper than the last) inside a function
    // with a couple of levels of `if`/`for` around it can hit 32 easily.
    // Matching the function limit to the top-level one avoids that for
    // any reasonably-sized `@import`ed command, while still bounding
    // pathological nesting well short of a stack overflow.
    engine.set_max_expr_depths(64, 64);
    engine.register_type_with_name::<Doc>("Doc");
    engine.register_fn("open", Doc::open);
    engine.register_fn("ensure", Doc::ensure);
    engine.register_fn("gnx", Doc::gnx);
    engine.register_fn("roots", Doc::roots);
    engine.register_fn("children", Doc::children);
    engine.register_fn("subtree", Doc::subtree);
    engine.register_fn("all", Doc::all);
    engine.register_fn("parent", Doc::parent);
    engine.register_fn("path", Doc::path);
    engine.register_fn("file_path", Doc::file_path);
    engine.register_fn("node", Doc::node);
    engine.register_fn("node_at", Doc::node_at);
    engine.register_fn("find_h", Doc::find_h);
    engine.register_fn("find_b", Doc::find_b);
    engine.register_fn("headline", Doc::headline);
    engine.register_fn("set_headline", Doc::set_headline);
    engine.register_fn("body", Doc::body);
    engine.register_fn("set_body", Doc::set_body);
    engine.register_fn("clone_node", Doc::clone_node);
    engine.register_fn("clone_node", Doc::clone_node_with_index);
    engine.register_fn("remove", Doc::remove);
    engine.register_fn("render", Doc::render);
    engine.register_fn("count", Doc::count);
    engine.register_fn("validate", Doc::validate);
    engine.register_fn("apply", Doc::apply);
    engine.register_fn("save", Doc::save);
    engine.register_fn("save_as", Doc::save_as);
    engine.register_fn("dir", Doc::dir_string);
    engine.register_fn("parse_json", parse_json);
    engine.register_fn("sh", sh_default_cwd);
    engine.register_fn("sh", sh_with_opts);
    engine.register_fn("env_var", env_var);
    engine.register_fn("regex_is_match", regex_is_match);
    engine.register_fn("regex_find", regex_find);
    engine.register_fn("regex_find_all", regex_find_all);
    engine.register_fn("regex_captures", regex_captures);
    engine.register_fn("regex_replace", regex_replace);
    engine.register_fn("regex_replace_all", regex_replace_all);

    // rhai-fs: open_file/read_string/write/read_dir/create_dir/... on plain
    // path strings, global rather than `Doc`-scoped since a script may need
    // to touch files outside the outline entirely. Paths are resolved
    // against `cub`'s own working directory, same as any other Rust path --
    // use `doc.dir()` to build one next to the open `.leo` file instead.
    rhai_fs::FilesystemPackage::new().register_into_engine(engine);

    engine.register_type_with_name::<Node>("Node");
    engine.register_get_set("h", Node::get_h, Node::set_h);
    engine.register_get_set("b", Node::get_b, Node::set_b);
    engine.register_fn("parent", Node::parent);
    engine.register_fn("parents", Node::parents);
    engine.register_fn("children", Node::children);
    engine.register_fn("subtree", Node::subtree);
    engine.register_get("gnx", Node::gnx);
    engine.register_get("position", Node::get_position);
    engine.register_fn("path", Node::path);
    engine.register_fn("file_path", Node::file_path);
    engine.register_fn("remove", Node::remove);
    engine.register_fn("to_string", Node::describe);

    // `cub::`-namespaced functions for cub's own outline conventions --
    // see rhai_run/cub.rs.
    cub::register(engine);

    engine.register_fn("assert", |cond: bool| -> RhaiResult<()> {
        if cond {
            Ok(())
        } else {
            Err(rhai_err("assertion failed"))
        }
    });
    engine.register_fn("assert", |cond: bool, msg: &str| -> RhaiResult<()> {
        if cond {
            Ok(())
        } else {
            Err(rhai_err(format!("assertion failed: {msg}")))
        }
    });
    engine.register_fn("assert_eq", |a: Dynamic, b: Dynamic| -> RhaiResult<()> {
        if dynamic_eq(&a, &b) {
            Ok(())
        } else {
            Err(rhai_err(format!("assertion failed: {a} != {b}")))
        }
    });
}

/// Runs a Rhai test script; returns an error (nonzero exit) if it fails to
/// parse, throws, or fails an `assert`/`assert_eq`. `args` -- whatever the
/// CLI passed after the script path -- is predefined for it as `ARGS`, an
/// array of strings (`[]` when none were given).
pub fn run(script_path: &std::path::Path, args: &[String]) -> Result<()> {
    let source = fs::read_to_string(script_path)
        .with_context(|| format!("read script {}", script_path.display()))?;
    let mut engine = Engine::new();
    engine.on_print(|s| println!("{s}"));
    engine.on_debug(|s, source, pos| match source {
        Some(source) => eprintln!("{source} @ {pos:?} | {s}"),
        None => eprintln!("{pos:?} | {s}"),
    });
    register_doc_api(&mut engine);
    let mut scope = Scope::new();
    scope.push_constant(
        "ARGS",
        args.iter().cloned().map(Dynamic::from).collect::<Array>(),
    );
    let _: Dynamic = engine
        .eval_with_scope(&mut scope, &source)
        .map_err(|error| anyhow::anyhow!("{error}"))
        .with_context(|| format!("run {}", script_path.display()))?;
    Ok(())
}

/// What running a bound `@action` rhai script produced: the document as it
/// stood after the script ran (mutated in place if the script touched
/// `doc`, unchanged otherwise), plus an exit status/stdout/stderr shape the
/// caller can render the same way regardless of how the script failed.
#[cfg(feature = "tui")]
pub(crate) struct BoundOutcome {
    pub(crate) document: LeoDocument,
    /// Whether the script called a `doc` method that mutates the outline
    /// (`add`, `set_headline`, `set_body`, `apply`) -- lets the caller skip
    /// marking the outline dirty for read-only or failed-before-mutating
    /// scripts.
    pub(crate) touched: bool,
    pub(crate) status: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

/// Runs `source` as a Rhai script with `doc` predefined and bound to
/// `document` (rather than a script calling `open()` itself), and `target`
/// predefined as the gnx of `target_position` -- the exact occurrence the
/// action was invoked on. This is what makes `@action` rhai bodies see the
/// same `Doc` API `cub run` scripts get, instead of the print-only sandbox
/// they used to be limited to.
#[cfg(feature = "tui")]
pub(crate) fn run_bound(
    document: LeoDocument,
    path: PathBuf,
    target_position: &PositionId,
    source: &str,
) -> BoundOutcome {
    let output = Rc::new(RefCell::new(String::new()));
    let print_output = output.clone();
    let debug_output = output.clone();

    let mut engine = Engine::new();
    engine.on_print(move |s| {
        let mut output = print_output.borrow_mut();
        output.push_str(s);
        output.push('\n');
    });
    engine.on_debug(move |s, source, pos| {
        let mut output = debug_output.borrow_mut();
        match source {
            Some(source) => output.push_str(&format!("{source} @ {pos:?} | {s}\n")),
            None => output.push_str(&format!("{pos:?} | {s}\n")),
        }
    });
    register_doc_api(&mut engine);

    // A defensive clone: if the script rebinds `doc` to something other
    // than a `Doc` (`let doc = 5;`), the scope lookup below can't recover
    // the mutated document, so this original stands in rather than losing
    // the outline entirely.
    let original = document.clone();

    let mut doc = Doc::bind(document, path.clone());
    // `p` is the same handle `doc.node_at(target)` would hand back --
    // predefined so scripts (and REPL snippets) can write `p.h`/`p.b`
    // instead of resolving it every time, and anchored to the exact
    // occurrence the caller had selected (not just its gnx), so `p.path()`/
    // `p.parent()` stay correct even if that node is cloned elsewhere.
    // Absent only if `target_position` somehow doesn't resolve, which
    // `run_bound`'s callers don't currently allow.
    let node = doc.node_at(&target_position.0).ok();
    let target_gnx = node.as_ref().map_or_else(String::new, |n| n.gnx.clone());

    let mut scope = Scope::new();
    scope.push("doc", doc);
    scope.push_constant("target", target_gnx);
    if let Some(node) = node {
        scope.push("p", node);
    }

    let eval_result = engine.eval_with_scope::<Dynamic>(&mut scope, source);
    let (document, touched) = match scope.get_value::<Doc>("doc") {
        Some(doc) => {
            let touched = doc.touched();
            (doc.into_document(), touched)
        }
        None => (original, false),
    };

    match eval_result {
        Ok(_) => BoundOutcome {
            document,
            touched,
            status: Some(0),
            stdout: output.borrow().clone(),
            stderr: String::new(),
        },
        Err(error) => BoundOutcome {
            document,
            touched,
            status: Some(1),
            stdout: output.borrow().clone(),
            stderr: error.to_string(),
        },
    }
}

/// One function an `@import`ed script's `COMMANDS` map names as directly
/// runnable -- see [`discover_commands`].
#[cfg(feature = "tui")]
pub(crate) struct ImportedCommand {
    pub(crate) name: String,
    /// The one-line description `COMMANDS` gave this command, if its value
    /// wasn't empty. Shown in the action palette so a command's purpose
    /// doesn't require opening the script to check.
    pub(crate) doc: Option<String>,
}

/// Reads back the `COMMANDS` map a script at `script_path` declares,
/// without invoking anything in it: compiles the script, runs only its
/// top-level `let`/`const` statements (registering its `fn`s along the
/// way, same as any script load), then reads `COMMANDS` out of the
/// resulting scope. `COMMANDS` is `#{ fn_name: "one-line description", ...
/// }`, naming the functions meant to be exposed as zero-input commands --
/// runnable with just `doc` -- in the action palette, each paired with the
/// description shown there; everything else the script defines is a
/// library helper, or a function that needs more input than the palette
/// can supply yet, and stays invisible there.
///
/// Returns an empty list -- not an error -- for a script that parses fine
/// but declares no `COMMANDS` (most imports imported only for their
/// `available_commands`, or with none at all, look like this). `Err`
/// covers the cases actually worth surfacing to the user: the file can't
/// be read, doesn't parse, or throws while its top level runs -- e.g. a
/// `fn` named after a Rhai reserved word breaks the whole script's
/// `COMMANDS` (and `available_commands`) silently unless a caller shows
/// this message somewhere.
#[cfg(feature = "tui")]
pub(crate) fn discover_commands(
    script_path: &std::path::Path,
) -> Result<Vec<ImportedCommand>, String> {
    let source = fs::read_to_string(script_path)
        .map_err(|error| format!("{}: {error}", script_path.display()))?;
    let mut engine = Engine::new();
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    register_doc_api(&mut engine);
    let ast = engine
        .compile(&source)
        .map_err(|error| format!("{}: {error}", script_path.display()))?;
    let mut scope = Scope::new();
    let _: Dynamic = engine
        .eval_ast_with_scope(&mut scope, &ast)
        .map_err(|error| format!("{}: {error}", script_path.display()))?;
    Ok(commands_from_map(
        scope.get_value::<rhai::Map>("COMMANDS").unwrap_or_default(),
    ))
}

/// Converts a `#{ fn_name: "description", ... }` map -- the shape both
/// `COMMANDS` and `available_commands`'s return value use -- into
/// `ImportedCommand`s. A non-string or empty description becomes `None`
/// rather than an empty label in the palette.
#[cfg(feature = "tui")]
fn commands_from_map(map: rhai::Map) -> Vec<ImportedCommand> {
    map.into_iter()
        .map(|(name, doc)| ImportedCommand {
            name: name.to_string(),
            doc: doc
                .into_immutable_string()
                .ok()
                .map(|doc| doc.to_string())
                .filter(|doc| !doc.is_empty()),
        })
        .collect()
}

/// Calls `available_commands(doc, target)` in the script at `script_path`,
/// if it defines one, to get commands whose palette availability depends
/// on the current selection -- the dynamic counterpart to the static
/// `COMMANDS` map `discover_commands` reads. `document` is cloned before
/// binding, so `available_commands` runs against a throwaway copy: nothing
/// it does (it's meant to be a pure query, but nothing stops a script from
/// calling a mutating `Doc` method) touches the outline actually open in
/// the TUI, unlike a real command run via [`run_command`].
///
/// A script simply not defining `available_commands` -- true for most
/// imports, since it's opt-in -- returns an empty list, not an `Err`;
/// that's the expected, silent case. `Err` is for the same "worth telling
/// someone" failures `discover_commands` reports (unreadable, doesn't
/// parse, throws at the top level), plus `available_commands` itself
/// throwing or not returning a map -- both signal a bug in that function
/// specifically, since its mere presence was already confirmed. Also `Err`
/// if `target_position` doesn't resolve, though callers don't currently
/// invoke this without a live selection.
#[cfg(feature = "tui")]
pub(crate) fn discover_available_commands(
    document: &LeoDocument,
    path: PathBuf,
    script_path: &std::path::Path,
    target_position: &PositionId,
) -> Result<Vec<ImportedCommand>, String> {
    let source = fs::read_to_string(script_path)
        .map_err(|error| format!("{}: {error}", script_path.display()))?;
    let mut engine = Engine::new();
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    register_doc_api(&mut engine);
    let ast = engine
        .compile(&source)
        .map_err(|error| format!("{}: {error}", script_path.display()))?;
    let mut scope = Scope::new();
    let _: Dynamic = engine
        .eval_ast_with_scope(&mut scope, &ast)
        .map_err(|error| format!("{}: {error}", script_path.display()))?;
    let mut doc = Doc::bind(document.clone(), path);
    let target = doc
        .node_at(&target_position.0)
        .map_err(|error| format!("{}: {error}", script_path.display()))?;
    match engine.call_fn::<rhai::Map>(&mut scope, &ast, "available_commands", (doc, target)) {
        Ok(result) => Ok(commands_from_map(result)),
        Err(error) if matches!(*error, EvalAltResult::ErrorFunctionNotFound(..)) => Ok(Vec::new()),
        Err(error) => Err(format!(
            "{}: available_commands: {error}",
            script_path.display()
        )),
    }
}

/// Runs the function `fn_name` from the script at `script_path` with
/// `(doc, target)` as its two arguments -- `doc` bound to `document`,
/// `target` the *`Node`* for the gnx selected when the command was invoked
/// from the palette (not a bare gnx string -- unlike `@action`'s
/// predefined `target`/`p` pair, a command has only one slot for "the
/// current position", so it gets the more useful of the two forms
/// directly; `target.gnx` recovers the string if a script needs it) --
/// what the action palette does for a command an `@import`ed script's
/// `COMMANDS` array named (see [`discover_commands`]). Mirrors
/// [`run_bound`]'s output/mutation contract, but calls one named function
/// instead of evaluating a whole script body -- and since a plain rhai
/// `fn` can't see the caller's scope the way a whole evaluated script body
/// can, `target` has to be passed as a real argument here rather than
/// predefined in `scope` the way `run_bound` does it. Fails (rather than
/// calling `fn_name` at all) if `target_position` no longer resolves to a
/// node in `document`.
#[cfg(feature = "tui")]
pub(crate) fn run_command(
    document: LeoDocument,
    path: PathBuf,
    script_path: &std::path::Path,
    target_position: &PositionId,
    fn_name: &str,
) -> BoundOutcome {
    let source = match fs::read_to_string(script_path) {
        Ok(source) => source,
        Err(error) => {
            return BoundOutcome {
                document,
                touched: false,
                status: Some(1),
                stdout: String::new(),
                stderr: format!("read {}: {error}", script_path.display()),
            };
        }
    };

    let output = Rc::new(RefCell::new(String::new()));
    let print_output = output.clone();
    let debug_output = output.clone();

    let mut engine = Engine::new();
    engine.on_print(move |s| {
        let mut output = print_output.borrow_mut();
        output.push_str(s);
        output.push('\n');
    });
    engine.on_debug(move |s, source, pos| {
        let mut output = debug_output.borrow_mut();
        match source {
            Some(source) => output.push_str(&format!("{source} @ {pos:?} | {s}\n")),
            None => output.push_str(&format!("{pos:?} | {s}\n")),
        }
    });
    register_doc_api(&mut engine);

    let ast = match engine.compile(&source) {
        Ok(ast) => ast,
        Err(error) => {
            return BoundOutcome {
                document,
                touched: false,
                status: Some(1),
                stdout: output.borrow().clone(),
                stderr: format!("{}: {error}", script_path.display()),
            };
        }
    };

    let mut scope = Scope::new();
    // Runs the script's top-level `let`/`const` statements (`COMMANDS`
    // among them) before the call below -- consistent with how a normal
    // module load would make those available to the function it calls.
    if let Err(error) = engine.eval_ast_with_scope::<Dynamic>(&mut scope, &ast) {
        return BoundOutcome {
            document,
            touched: false,
            status: Some(1),
            stdout: output.borrow().clone(),
            stderr: error.to_string(),
        };
    }

    let mut doc = Doc::bind(document, path);
    let target = match doc.node_at(&target_position.0) {
        Ok(node) => node,
        Err(error) => {
            return BoundOutcome {
                document: doc.into_document(),
                touched: false,
                status: Some(1),
                stdout: output.borrow().clone(),
                stderr: error.to_string(),
            };
        }
    };
    let call_result = engine.call_fn::<Dynamic>(&mut scope, &ast, fn_name, (doc.clone(), target));
    let touched = doc.touched();
    let document = doc.into_document();

    match call_result {
        Ok(_) => BoundOutcome {
            document,
            touched,
            status: Some(0),
            stdout: output.borrow().clone(),
            stderr: String::new(),
        },
        Err(error) => BoundOutcome {
            document,
            touched,
            status: Some(1),
            stdout: output.borrow().clone(),
            stderr: error.to_string(),
        },
    }
}

#[cfg(all(test, feature = "tui"))]
mod tests {
    use super::discover_commands;
    use std::path::Path;

    /// `scripts/claude-history.rhai` (the repo's `@import`-shaped demo, see
    /// demos/scripting.leo) must keep exposing exactly its one intended
    /// palette command -- not the private `first_text`/`excerpt` helpers it
    /// also defines -- so the palette entry `demos/scripting.leo`'s
    /// `@import` node expects doesn't silently disappear or multiply.
    #[test]
    fn claude_history_script_exposes_only_its_import_command() {
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/claude-history.rhai");
        let names: Vec<String> = discover_commands(&script)
            .expect("claude-history.rhai should compile and declare COMMANDS")
            .into_iter()
            .map(|command| command.name)
            .collect();
        assert_eq!(names, vec!["import_claude_history".to_string()]);
    }
}
