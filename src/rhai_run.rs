//! `cub run <script.rhai>`: a scriptable, non-interactive replacement for
//! the old jsonl/TUI-driven integration test suite. Instead of scripting
//! keypresses against the TUI, a test script drives an outline directly
//! through a small [`Doc`] API and asserts on the result -- so it exercises
//! the same library code `cub`'s other subcommands do, without a terminal.

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use leo::{LeoDocument, NodeId, OperationBatch};
use rhai::{Array, Dynamic, Engine, EvalAltResult};

/// The outline handle a script gets back from `open(path)`. Every method
/// mutates or reads the in-memory document; nothing touches disk until
/// `save`/`save_as` is called.
#[derive(Clone)]
struct Doc {
    document: LeoDocument,
    path: PathBuf,
}

type RhaiResult<T> = Result<T, Box<EvalAltResult>>;

fn rhai_err(message: impl std::fmt::Display) -> Box<EvalAltResult> {
    message.to_string().into()
}

impl Doc {
    fn open(path: &str) -> RhaiResult<Doc> {
        let document = LeoDocument::open(path).map_err(rhai_err)?;
        Ok(Doc {
            document,
            path: PathBuf::from(path),
        })
    }

    /// Ensures a slash-separated headline path exists (creating any missing
    /// segments, reusing existing ones) and returns its gnx.
    fn add(&mut self, path: &str) -> RhaiResult<String> {
        self.document
            .outline
            .add_headline_paths(&[path.to_owned()])
            .map_err(rhai_err)?;
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
        Ok(())
    }

    fn body(&mut self, gnx: &str) -> RhaiResult<String> {
        Ok(self.node(gnx)?.body.clone())
    }

    fn set_body(&mut self, gnx: &str, text: &str) -> RhaiResult<()> {
        self.node_mut(gnx)?.body = text.to_owned();
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

fn build_engine() -> Engine {
    let mut engine = Engine::new();
    engine.on_print(|s| println!("{s}"));
    engine.on_debug(|s, source, pos| match source {
        Some(source) => eprintln!("{source} @ {pos:?} | {s}"),
        None => eprintln!("{pos:?} | {s}"),
    });

    engine.register_type_with_name::<Doc>("Doc");
    engine.register_fn("open", Doc::open);
    engine.register_fn("add", Doc::add);
    engine.register_fn("gnx", Doc::gnx);
    engine.register_fn("headline", Doc::headline);
    engine.register_fn("set_headline", Doc::set_headline);
    engine.register_fn("body", Doc::body);
    engine.register_fn("set_body", Doc::set_body);
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

    engine
}

/// Runs a Rhai test script; returns an error (nonzero exit) if it fails to
/// parse, throws, or fails an `assert`/`assert_eq`.
pub fn run(script_path: &std::path::Path) -> Result<()> {
    let source = fs::read_to_string(script_path)
        .with_context(|| format!("read script {}", script_path.display()))?;
    let engine = build_engine();
    let _: Dynamic = engine
        .eval(&source)
        .map_err(|error| anyhow::anyhow!("{error}"))
        .with_context(|| format!("run {}", script_path.display()))?;
    Ok(())
}
