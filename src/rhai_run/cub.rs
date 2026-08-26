//! `cub::`-namespaced Rhai functions for cub's own outline *conventions* --
//! built on the plain `Doc`/`Node` API the parent module exposes, but
//! opinionated about node shape (here, the `@variables` tree
//! docs/workflows/scripting.md describes), unlike that API, which stays
//! convention-free. Kept in a namespaced module rather than more `Node`/
//! `Doc` methods, so conventions like this one don't pile onto types meant
//! to stay generic outline primitives -- and in its own file/submodule
//! since it's one narrow, independently growable slice of `rhai_run`
//! rather than its core plumbing.

use super::{Node, RhaiResult};
use rhai::{Dynamic, Engine};

/// Registers every `cub::` function into `engine`.
pub(super) fn register(engine: &mut Engine) {
    let mut module = rhai::Module::new();
    module.set_native_fn("variable", resolve_variable);
    let module: rhai::Shared<rhai::Module> = module.into();
    engine.register_static_module("cub", module);
}

/// One setting named `name` among `variables_node`'s children -- either
/// `name = value` in a setting's own headline, or a plain `name` headline
/// with the value in its body (both conventions `cub::variable` supports).
/// `None` when `variables_node` has no child matching `name` either way.
fn variable_among(variables_node: &mut Node, name: &str) -> RhaiResult<Option<String>> {
    for setting in variables_node.children()? {
        let mut setting = setting.cast::<Node>();
        let headline = setting.get_h()?;
        if let Some((key, value)) = headline.split_once('=') {
            if key.trim() == name {
                return Ok(Some(value.trim().to_owned()));
            }
        } else if headline.trim() == name {
            return Ok(Some(setting.get_b()?.trim().to_owned()));
        }
    }
    Ok(None)
}

/// `node`'s child headlined `@variables` (if it has one), searched for
/// `name` via `variable_among`.
fn variable_in_children(node: &mut Node, name: &str) -> RhaiResult<Option<String>> {
    for child in node.children()? {
        let mut child = child.cast::<Node>();
        if child.get_h()?.trim() == "@variables" {
            return variable_among(&mut child, name);
        }
    }
    Ok(None)
}

/// Resolves one named `@variables` setting for `target`: walks `target`
/// and its ancestors, nearest first (so different subtrees can each set
/// their own value for the same name -- e.g. `repo` -- without clobbering
/// each other), returning the first definition of `name` found among any
/// `@variables` child along the way. Falls back to any top-level root
/// itself headlined `@variables` if nothing turned up walking ancestors,
/// for an outline that keeps one outline-wide settings block rather than
/// scoping it to a subtree -- the only shape `@variables` supported
/// before this walk existed. `()` when `name` is set nowhere.
///
/// Deliberately resolves one name at a time rather than building a map of
/// every setting under whichever `@variables` node it finds (`gh_repo_flag`,
/// its only caller so far, only ever wants `repo`): no point parsing
/// settings nobody asked for, or walking past the first `@variables` that
/// actually sets the one being asked about.
fn resolve_variable(target: &mut Node, name: &str) -> RhaiResult<Dynamic> {
    let mut node = target.clone();
    loop {
        if let Some(value) = variable_in_children(&mut node, name)? {
            return Ok(Dynamic::from(value));
        }
        let parent = node.parent()?;
        if parent.gnx.is_empty() {
            break;
        }
        node = parent;
    }
    let mut doc = node.doc.clone();
    for root in doc.roots() {
        let Ok(gnx) = root.into_string() else {
            continue;
        };
        let mut root_node = Node {
            doc: doc.clone(),
            gnx,
            position: None,
        };
        if root_node.get_h()?.trim() == "@variables"
            && let Some(value) = variable_among(&mut root_node, name)?
        {
            return Ok(Dynamic::from(value));
        }
    }
    Ok(Dynamic::UNIT)
}
