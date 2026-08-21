//! Rhai scripting, shared by two entry points that both drive an outline
//! through the same [`Doc`] API so neither is a second-class citizen:
//!
//! - `cub run SCRIPT.rhai` ([`run`]): a scriptable, non-interactive
//!   replacement for the old jsonl/TUI-driven integration test suite. A
//!   script opens its own `.leo` file and asserts on the result, exercising
//!   the same library code `cub`'s other subcommands do, without a
//!   terminal.
//! - `@action` bodies with `@language rhai` ([`run_bound`]), run in-process
//!   from inside the TUI (see `tui::run_action`). Rather than opening a
//!   file, the action's `doc` is bound to the outline already open in the
//!   editor, and `target` is predefined as the gnx of the node the action
//!   was invoked on -- the same role env vars like `CUB_GNX` play for
//!   subprocess-based actions.

use std::collections::BTreeMap;
#[cfg(feature = "tui")]
use std::{cell::RefCell, rc::Rc};
use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use leo::{LeoDocument, NodeId, Operation, OperationBatch};
#[cfg(feature = "tui")]
use rhai::Scope;
use rhai::{Array, Dynamic, Engine, EvalAltResult};

/// The outline handle a script gets back from `open(path)` (or, for a bound
/// `@action` script, from the predefined `doc`). Every method mutates or
/// reads the in-memory document; nothing touches disk until `save`/
/// `save_as` is called.
#[derive(Clone)]
pub(crate) struct Doc {
    document: LeoDocument,
    path: PathBuf,
    /// Set by any method that mutates `document`. `run_bound` reads this
    /// afterwards so the TUI only marks the outline dirty (and drops
    /// layout/highlight caches) when a bound script actually changed
    /// something, rather than on every run.
    touched: bool,
}

type RhaiResult<T> = Result<T, Box<EvalAltResult>>;

fn rhai_err(message: impl std::fmt::Display) -> Box<EvalAltResult> {
    message.to_string().into()
}

fn ids_to_array(ids: Vec<NodeId>) -> Array {
    ids.into_iter().map(|id| Dynamic::from(id.0)).collect()
}

impl Doc {
    /// Binds an already-open document instead of reading one from disk --
    /// used to hand an `@action` script the outline the TUI already has in
    /// memory, so it sees in-progress edits and its mutations flow straight
    /// back into the editor instead of round-tripping through disk.
    #[cfg(feature = "tui")]
    pub(crate) fn bind(document: LeoDocument, path: PathBuf) -> Doc {
        Doc {
            document,
            path,
            touched: false,
        }
    }

    #[cfg(feature = "tui")]
    pub(crate) fn into_document(self) -> LeoDocument {
        self.document
    }

    fn open(path: &str) -> RhaiResult<Doc> {
        let document = LeoDocument::open(path).map_err(rhai_err)?;
        Ok(Doc {
            document,
            path: PathBuf::from(path),
            touched: false,
        })
    }

    /// Ensures a slash-separated headline path exists (creating any missing
    /// segments, reusing existing ones) and returns its gnx.
    fn add(&mut self, path: &str) -> RhaiResult<String> {
        self.document
            .outline
            .add_headline_paths(&[path.to_owned()])
            .map_err(rhai_err)?;
        self.touched = true;
        self.gnx(path)
    }

    /// Resolves a slash-separated headline path to its gnx without creating
    /// anything; fails if the path doesn't exist or is ambiguous.
    fn gnx(&mut self, path: &str) -> RhaiResult<String> {
        self.document
            .outline
            .resolve_headline_path(path)
            .map(|id| id.0)
            .map_err(rhai_err)
    }

    /// The gnxs of the outline's top-level nodes, in outline order.
    fn roots(&mut self) -> Array {
        ids_to_array(self.document.outline.root_ids())
    }

    /// The gnxs of `gnx`'s children, in outline order (empty if it's a
    /// leaf); fails if `gnx` isn't a node in the outline.
    fn children(&mut self, gnx: &str) -> RhaiResult<Array> {
        self.node(gnx)?;
        Ok(ids_to_array(
            self.document
                .outline
                .children_of(&NodeId(gnx.to_owned()))
                .unwrap_or_default(),
        ))
    }

    /// The gnxs of `gnx` and everything under it, depth-first and in
    /// outline order (`gnx` itself first) -- the flattened equivalent of
    /// Leo's `p.self_and_subtree()`. Fails if `gnx` isn't a node in the
    /// outline.
    fn subtree(&mut self, gnx: &str) -> RhaiResult<Array> {
        self.document
            .outline
            .subtree_ids(&NodeId(gnx.to_owned()))
            .map(ids_to_array)
            .ok_or_else(|| rhai_err(format!("node not found: {gnx}")))
    }

    /// The gnxs of every node in the outline, depth-first and in outline
    /// order -- the flattened equivalent of Leo's `c.all_positions()`. A
    /// node cloned to more than one position appears once per occurrence.
    fn all(&mut self) -> Array {
        ids_to_array(self.document.outline.all_ids())
    }

    /// `gnx`'s parent's gnx, or `""` if `gnx` is a root; fails if `gnx`
    /// isn't a node in the outline.
    fn parent(&mut self, gnx: &str) -> RhaiResult<String> {
        self.node(gnx)?;
        Ok(self
            .document
            .outline
            .parent_of(&NodeId(gnx.to_owned()))
            .map(|id| id.0)
            .unwrap_or_default())
    }

    /// The slash-separated headline path from the root down to `gnx`,
    /// escaped the same way `doc.gnx(path)`/`doc.add(path)` expect, so it
    /// can be fed straight back into either of them.
    fn path(&mut self, gnx: &str) -> RhaiResult<String> {
        self.document
            .outline
            .headline_path_of(&NodeId(gnx.to_owned()))
            .ok_or_else(|| rhai_err(format!("node not found: {gnx}")))
    }

    fn node(&self, gnx: &str) -> RhaiResult<&leo::Node> {
        self.document
            .outline
            .nodes
            .get(&NodeId(gnx.to_owned()))
            .ok_or_else(|| rhai_err(format!("node not found: {gnx}")))
    }

    fn node_mut(&mut self, gnx: &str) -> RhaiResult<&mut leo::Node> {
        self.document
            .outline
            .nodes
            .get_mut(&NodeId(gnx.to_owned()))
            .ok_or_else(|| rhai_err(format!("node not found: {gnx}")))
    }

    fn headline(&mut self, gnx: &str) -> RhaiResult<String> {
        Ok(self.node(gnx)?.headline.clone())
    }

    fn set_headline(&mut self, gnx: &str, text: &str) -> RhaiResult<()> {
        self.node_mut(gnx)?.headline = text.to_owned();
        self.touched = true;
        Ok(())
    }

    fn body(&mut self, gnx: &str) -> RhaiResult<String> {
        Ok(self.node(gnx)?.body.clone())
    }

    fn set_body(&mut self, gnx: &str, text: &str) -> RhaiResult<()> {
        self.node_mut(gnx)?.body = text.to_owned();
        self.touched = true;
        Ok(())
    }

    /// Inserts a new occurrence of `gnx` as the last child of `parent_gnx`.
    /// Fails if either `gnx` or `parent_gnx` isn't already a node in the
    /// outline -- nothing is created. Returns `gnx` unchanged, so the call
    /// can be chained. If a script only has a headline path for the parent,
    /// resolve it first with `doc.gnx(path)`/`doc.add(path)` rather than
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
        self.document.outline.apply(&batch).map_err(rhai_err)?;
        self.touched = true;
        Ok(gnx.to_owned())
    }

    /// Removes `gnx`'s defining occurrence and its whole subtree from the
    /// outline. If `gnx` is cloned elsewhere, those other occurrences are
    /// left in place -- the next one in outline order becomes the new
    /// defining occurrence. Fails if `gnx` isn't a node in the outline.
    fn remove(&mut self, gnx: &str) -> RhaiResult<()> {
        let batch = OperationBatch {
            operations: vec![Operation::ReplaceTree {
                node: Some(NodeId(gnx.to_owned())),
                headline: None,
                tree: BTreeMap::new(),
            }],
            ..Default::default()
        };
        self.document.outline.apply(&batch).map_err(rhai_err)?;
        self.touched = true;
        Ok(())
    }

    fn render(&mut self) -> String {
        leo::render_compact(&self.document.outline)
    }

    fn count(&mut self) -> i64 {
        self.document.outline.nodes.len() as i64
    }

    fn validate(&mut self) -> Array {
        self.document
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
        let report = self.document.outline.apply(&batch).map_err(rhai_err)?;
        self.touched = true;
        serde_json::to_string(&report).map_err(rhai_err)
    }

    fn save(&mut self) -> RhaiResult<()> {
        self.document.save(&self.path).map_err(rhai_err)
    }

    fn save_as(&mut self, path: &str) -> RhaiResult<()> {
        self.document.save(path).map_err(rhai_err)?;
        self.path = PathBuf::from(path);
        Ok(())
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
    engine.register_type_with_name::<Doc>("Doc");
    engine.register_fn("open", Doc::open);
    engine.register_fn("add", Doc::add);
    engine.register_fn("gnx", Doc::gnx);
    engine.register_fn("roots", Doc::roots);
    engine.register_fn("children", Doc::children);
    engine.register_fn("subtree", Doc::subtree);
    engine.register_fn("all", Doc::all);
    engine.register_fn("parent", Doc::parent);
    engine.register_fn("path", Doc::path);
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
/// parse, throws, or fails an `assert`/`assert_eq`.
pub fn run(script_path: &std::path::Path) -> Result<()> {
    let source = fs::read_to_string(script_path)
        .with_context(|| format!("read script {}", script_path.display()))?;
    let mut engine = Engine::new();
    engine.on_print(|s| println!("{s}"));
    engine.on_debug(|s, source, pos| match source {
        Some(source) => eprintln!("{source} @ {pos:?} | {s}"),
        None => eprintln!("{pos:?} | {s}"),
    });
    register_doc_api(&mut engine);
    let _: Dynamic = engine
        .eval(&source)
        .map_err(|error| anyhow::anyhow!("{error}"))
        .with_context(|| format!("run {}", script_path.display()))?;
    Ok(())
}

/// What running a bound `@action` rhai script produced: the document as it
/// stood after the script ran (mutated in place if the script touched
/// `doc`, unchanged otherwise), plus output shaped like a subprocess's exit
/// status/stdout/stderr so callers don't need a separate code path from the
/// interpreter-based action runner.
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
/// predefined as `target_gnx` -- the gnx of the node the action was invoked
/// on. This is what makes `@action` rhai bodies see the same `Doc` API
/// `cub run` scripts get, instead of the print-only sandbox they used to be
/// limited to.
#[cfg(feature = "tui")]
pub(crate) fn run_bound(
    document: LeoDocument,
    path: PathBuf,
    target_gnx: &str,
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

    let mut scope = Scope::new();
    scope.push("doc", Doc::bind(document, path.clone()));
    scope.push_constant("target", target_gnx.to_owned());

    let eval_result = engine.eval_with_scope::<Dynamic>(&mut scope, source);
    let (document, touched) = match scope.get_value::<Doc>("doc") {
        Some(doc) => {
            let touched = doc.touched;
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
