use std::collections::HashMap;

use serde::Serialize;
use thiserror::Error;

use crate::import::IdGenerator;
use crate::model::parse_path;
use crate::{Node, NodeId, Outline, Position, PositionId};

#[derive(Clone, Debug, Serialize)]
pub struct AddPathsReport {
    pub created: usize,
    pub paths: Vec<String>,
}

#[derive(Debug, Error, PartialEq)]
pub enum HeadlinePathError {
    #[error("headline path is empty")]
    Empty,
    #[error("headline path contains an empty component: {0}")]
    EmptyComponent(String),
    #[error("headline path not found: {0}")]
    NotFound(String),
    #[error("headline path is ambiguous: {0}")]
    Ambiguous(String),
}

impl Outline {
    /// Resolve a slash-separated path of headlines to one node occurrence.
    pub fn resolve_headline_path(&self, path: &str) -> Result<NodeId, HeadlinePathError> {
        self.resolve_headline_position(path)
            .map(|position| position.node.clone())
    }

    /// Resolve a slash-separated path of headlines to one node occurrence,
    /// returning the full `Position` (including its children) rather than
    /// just its node id.
    pub fn resolve_headline_position(&self, path: &str) -> Result<&Position, HeadlinePathError> {
        let parts = path_parts(path)?;
        let mut siblings = self.roots.as_slice();
        let mut selected = None;
        for part in parts {
            let matches: Vec<_> = siblings
                .iter()
                .filter(|position| self.nodes[&position.node].headline == part)
                .collect();
            match matches.as_slice() {
                [] => return Err(HeadlinePathError::NotFound(path.to_owned())),
                [position] => {
                    selected = Some(*position);
                    siblings = &position.children;
                }
                _ => return Err(HeadlinePathError::Ambiguous(path.to_owned())),
            }
        }
        Ok(selected.unwrap())
    }

    /// Ensure slash-separated headline paths exist, reusing existing nodes.
    pub fn add_headline_paths(
        &mut self,
        paths: &[String],
    ) -> Result<AddPathsReport, HeadlinePathError> {
        let parsed = paths
            .iter()
            .map(|path| path_parts(path).map(|parts| (path, parts)))
            .collect::<Result<Vec<_>, _>>()?;
        let mut next = self.clone();
        let mut ids =
            IdGenerator::with_prefix("cub".to_owned(), next.nodes.keys().cloned().collect());
        let mut created = 0;
        for (path, parts) in parsed {
            ensure_path(
                &mut next.roots,
                &mut next.nodes,
                &mut ids,
                &parts,
                path,
                &mut created,
            )?;
        }
        *self = next;
        Ok(AddPathsReport {
            created,
            paths: paths.to_vec(),
        })
    }

    /// Resolve a single slash-separated headline path, creating any missing
    /// segments (reusing existing ones) the same way `add_headline_paths`
    /// does, and return the leaf node's id.
    pub(crate) fn ensure_headline_path(
        &mut self,
        path: &str,
        ids: &mut IdGenerator,
    ) -> Result<NodeId, HeadlinePathError> {
        let parts = path_parts(path)?;
        let mut created = 0;
        ensure_path(
            &mut self.roots,
            &mut self.nodes,
            ids,
            &parts,
            path,
            &mut created,
        )
    }

    /// The gnxs of the outline's top-level nodes, in outline order.
    pub fn root_ids(&self) -> Vec<NodeId> {
        self.roots
            .iter()
            .map(|position| position.node.clone())
            .collect()
    }

    /// The gnxs of `node`'s children, in outline order, or `None` if `node`
    /// isn't reachable from any root. Found via the first occurrence
    /// located by a depth-first search from the roots; since a clone's
    /// children are the same regardless of which occurrence you start
    /// from, which occurrence is "first" doesn't change the result.
    pub fn children_of(&self, node: &NodeId) -> Option<Vec<NodeId>> {
        fn find<'a>(positions: &'a [Position], node: &NodeId) -> Option<&'a [Position]> {
            for position in positions {
                if &position.node == node {
                    return Some(&position.children);
                }
                if let Some(found) = find(&position.children, node) {
                    return Some(found);
                }
            }
            None
        }
        find(&self.roots, node).map(|children| {
            children
                .iter()
                .map(|position| position.node.clone())
                .collect()
        })
    }

    /// The gnx of `node`'s parent, or `None` if `node` is a root or isn't
    /// reachable from any root. Uses the same first-occurrence search as
    /// [`Outline::children_of`].
    pub fn parent_of(&self, node: &NodeId) -> Option<NodeId> {
        let chain = self.ancestor_chain(node)?;
        (chain.len() >= 2).then(|| chain[chain.len() - 2].clone())
    }

    /// The slash-separated headline path from a root down to `node`
    /// (inclusive), escaped the same way [`Outline::resolve_headline_path`]
    /// expects, so `outline.headline_path_of(id)` and
    /// `outline.resolve_headline_path(path)` round-trip. `None` if `node`
    /// isn't reachable from any root.
    pub fn headline_path_of(&self, node: &NodeId) -> Option<String> {
        let chain = self.ancestor_chain(node)?;
        Some(
            chain
                .iter()
                .map(|id| escape_headline_path_component(&self.nodes[id].headline))
                .collect::<Vec<_>>()
                .join("/"),
        )
    }

    /// The slash-separated headline path down to the exact occurrence
    /// `position` names, escaped the same way [`Outline::headline_path_of`]
    /// is -- the exact-occurrence counterpart to `headline_path_of`, which
    /// can only resolve a node id's *first* occurrence, indistinguishable
    /// from any of its other clones. `None` if `position` doesn't resolve
    /// to anything in the outline.
    pub fn headline_path_at(&self, position: &PositionId) -> Option<String> {
        let chain = self.ancestor_chain_at(position)?;
        Some(
            chain
                .iter()
                .map(|id| escape_headline_path_component(&self.nodes[id].headline))
                .collect::<Vec<_>>()
                .join("/"),
        )
    }

    /// The chain of gnxs from a root down to `position` (inclusive), read
    /// directly off `position`'s indices -- the exact-occurrence
    /// counterpart to [`Outline::ancestor_chain`]. `None` if `position`
    /// doesn't resolve to anything in the outline.
    fn ancestor_chain_at(&self, position: &PositionId) -> Option<Vec<NodeId>> {
        let indices = parse_path(position)?;
        let mut chain = Vec::new();
        let mut p = self.roots.get(*indices.first()?)?;
        chain.push(p.node.clone());
        for index in &indices[1..] {
            p = p.children.get(*index)?;
            chain.push(p.node.clone());
        }
        Some(chain)
    }

    /// The chain of gnxs from a root down to `node` (inclusive), via the
    /// first occurrence found by a depth-first search from the roots, or
    /// `None` if `node` isn't reachable from any root.
    fn ancestor_chain(&self, node: &NodeId) -> Option<Vec<NodeId>> {
        fn find(positions: &[Position], node: &NodeId, chain: &mut Vec<NodeId>) -> bool {
            for position in positions {
                chain.push(position.node.clone());
                if &position.node == node || find(&position.children, node, chain) {
                    return true;
                }
                chain.pop();
            }
            false
        }
        let mut chain = Vec::new();
        find(&self.roots, node, &mut chain).then_some(chain)
    }

    /// The gnxs of `node` and everything under it, depth-first and in
    /// outline order (`node` itself first) -- the same shape as Leo's
    /// `p.self_and_subtree()`. `None` if `node` isn't reachable from any
    /// root. Uses the same first-occurrence search as
    /// [`Outline::children_of`].
    pub fn subtree_ids(&self, node: &NodeId) -> Option<Vec<NodeId>> {
        fn find<'a>(positions: &'a [Position], node: &NodeId) -> Option<&'a Position> {
            for position in positions {
                if &position.node == node {
                    return Some(position);
                }
                if let Some(found) = find(&position.children, node) {
                    return Some(found);
                }
            }
            None
        }
        let mut out = Vec::new();
        collect_preorder(find(&self.roots, node)?, &mut out);
        Some(out)
    }

    /// The gnxs of every node in the outline, depth-first and in outline
    /// order -- the same shape as Leo's `c.all_positions()`. A node cloned
    /// to more than one position appears once per occurrence, matching
    /// Leo's generator (which yields positions, not distinct vnodes).
    pub fn all_ids(&self) -> Vec<NodeId> {
        let mut out = Vec::new();
        for root in &self.roots {
            collect_preorder(root, &mut out);
        }
        out
    }
}

fn collect_preorder(position: &Position, out: &mut Vec<NodeId>) {
    out.push(position.node.clone());
    for child in &position.children {
        collect_preorder(child, out);
    }
}

/// Escapes a headline for use as one path component: `\` becomes `\\` and
/// `/` becomes `\/`, the inverse of what `path_parts` unescapes.
pub fn escape_headline_path_component(headline: &str) -> String {
    headline.replace('\\', "\\\\").replace('/', "\\/")
}

/// Split a slash-separated headline path into its components. `\/` is a
/// literal slash and `\\` a literal backslash, so a headline that itself
/// contains a `/` (a branch-name-style PR title, say) can still be written
/// as one path component; any other backslash is kept as-is.
fn path_parts(path: &str) -> Result<Vec<String>, HeadlinePathError> {
    if path.is_empty() {
        return Err(HeadlinePathError::Empty);
    }
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if matches!(chars.peek(), Some('/') | Some('\\')) => {
                current.push(chars.next().unwrap());
            }
            '/' => {
                parts.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    parts.push(current);
    if parts.iter().any(|part| part.is_empty()) {
        return Err(HeadlinePathError::EmptyComponent(path.to_owned()));
    }
    Ok(parts)
}

fn ensure_path(
    siblings: &mut Vec<Position>,
    nodes: &mut HashMap<NodeId, Node>,
    ids: &mut IdGenerator,
    parts: &[String],
    full_path: &str,
    created: &mut usize,
) -> Result<NodeId, HeadlinePathError> {
    let matches: Vec<_> = siblings
        .iter()
        .enumerate()
        .filter(|(_, position)| nodes[&position.node].headline == parts[0])
        .map(|(index, _)| index)
        .collect();
    let index = match matches.as_slice() {
        [] => {
            let id = ids.next();
            nodes.insert(
                id.clone(),
                Node {
                    id: id.clone(),
                    headline: parts[0].to_owned(),
                    body: String::new(),
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                },
            );
            siblings.push(Position {
                node: id,
                children: vec![],
            });
            *created += 1;
            siblings.len() - 1
        }
        [index] => *index,
        _ => return Err(HeadlinePathError::Ambiguous(full_path.to_owned())),
    };
    if parts.len() > 1 {
        ensure_path(
            &mut siblings[index].children,
            nodes,
            ids,
            &parts[1..],
            full_path,
            created,
        )
    } else {
        Ok(siblings[index].node.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_children_parent_roots_and_reconstructs_headline_paths() {
        let mut outline = Outline::default();
        outline
            .add_headline_paths(&[
                "Project/Tasks/First task".into(),
                "Project/Notes".into(),
                r"Project/Tasks/fix a\/b bug".into(),
                "Standalone".into(),
            ])
            .unwrap();

        let project = outline.resolve_headline_path("Project").unwrap();
        let tasks = outline.resolve_headline_path("Project/Tasks").unwrap();
        let notes = outline.resolve_headline_path("Project/Notes").unwrap();
        let first_task = outline
            .resolve_headline_path("Project/Tasks/First task")
            .unwrap();
        let standalone = outline.resolve_headline_path("Standalone").unwrap();

        assert_eq!(
            outline.root_ids(),
            vec![project.clone(), standalone.clone()]
        );
        assert_eq!(
            outline.children_of(&project),
            Some(vec![tasks.clone(), notes])
        );
        assert_eq!(outline.children_of(&first_task), Some(vec![]));
        assert_eq!(outline.children_of(&NodeId::from("missing")), None);

        assert_eq!(outline.parent_of(&first_task), Some(tasks));
        assert_eq!(outline.parent_of(&project), None);
        assert_eq!(outline.parent_of(&NodeId::from("missing")), None);

        assert_eq!(
            outline.headline_path_of(&first_task).as_deref(),
            Some("Project/Tasks/First task")
        );
        // Round-trips even through a component that itself needed escaping.
        let escaped = outline
            .resolve_headline_path(r"Project/Tasks/fix a\/b bug")
            .unwrap();
        let path = outline.headline_path_of(&escaped).unwrap();
        assert_eq!(outline.resolve_headline_path(&path).unwrap(), escaped);
        assert_eq!(outline.headline_path_of(&NodeId::from("missing")), None);
    }

    #[test]
    fn headline_path_at_and_parent_position_resolve_the_exact_clone_occurrence() {
        let mut outline = Outline::default();
        outline
            .add_headline_paths(&["A/Shared".into(), "A/C".into()])
            .unwrap();
        let shared = outline.resolve_headline_path("A/Shared").unwrap();

        // Clone `shared` as a child of `c`, so it occurs at both "0/0"
        // (its original spot) and "0/1/0".
        outline.roots[0].children[1].children.push(Position {
            node: shared.clone(),
            children: vec![],
        });

        assert_eq!(
            outline
                .headline_path_at(&PositionId("0/0".into()))
                .as_deref(),
            Some("A/Shared")
        );
        // Told apart by position alone -- same node id, different path.
        assert_eq!(
            outline
                .headline_path_at(&PositionId("0/1/0".into()))
                .as_deref(),
            Some("A/C/Shared")
        );
        assert_eq!(outline.headline_path_at(&PositionId("9/9".into())), None);

        assert_eq!(
            outline.parent_position(&PositionId("0/1/0".into())),
            Some(PositionId("0/1".into()))
        );
        assert_eq!(outline.parent_position(&PositionId("0".into())), None);

        // `headline_path_of`'s first-occurrence search can't tell the two
        // occurrences apart -- it always resolves to the first one.
        assert_eq!(
            outline.headline_path_of(&shared).as_deref(),
            Some("A/Shared")
        );
    }

    #[test]
    fn subtree_and_all_ids_walk_depth_first_in_outline_order() {
        let mut outline = Outline::default();
        outline
            .add_headline_paths(&[
                "Project/Tasks/First task".into(),
                "Project/Tasks/Second task".into(),
                "Project/Notes".into(),
                "Standalone".into(),
            ])
            .unwrap();

        let project = outline.resolve_headline_path("Project").unwrap();
        let tasks = outline.resolve_headline_path("Project/Tasks").unwrap();
        let notes = outline.resolve_headline_path("Project/Notes").unwrap();
        let first_task = outline
            .resolve_headline_path("Project/Tasks/First task")
            .unwrap();
        let second_task = outline
            .resolve_headline_path("Project/Tasks/Second task")
            .unwrap();
        let standalone = outline.resolve_headline_path("Standalone").unwrap();

        assert_eq!(
            outline.subtree_ids(&project),
            Some(vec![
                project.clone(),
                tasks.clone(),
                first_task.clone(),
                second_task.clone(),
                notes.clone()
            ])
        );
        assert_eq!(
            outline.subtree_ids(&first_task),
            Some(vec![first_task.clone()])
        );
        assert_eq!(outline.subtree_ids(&NodeId::from("missing")), None);

        assert_eq!(
            outline.all_ids(),
            vec![project, tasks, first_task, second_task, notes, standalone]
        );
    }

    #[test]
    fn adds_shared_prefixes_and_resolves_paths() {
        let mut outline = Outline::default();
        let report = outline
            .add_headline_paths(&["Project/Tasks/First task".into(), "Project/Notes".into()])
            .unwrap();
        assert_eq!(report.created, 4);
        let project = outline.resolve_headline_path("Project").unwrap();
        let tasks = outline.resolve_headline_path("Project/Tasks").unwrap();
        assert_ne!(project, tasks);
        assert_eq!(
            outline
                .add_headline_paths(&["Project/Notes".into()])
                .unwrap()
                .created,
            0
        );
        assert!(outline.validate().is_empty());
    }

    #[test]
    fn rejects_ambiguous_and_malformed_paths() {
        let mut outline = Outline::default();
        outline.add_headline_paths(&["A".into()]).unwrap();
        let original = outline.roots[0].clone();
        outline.roots.push(original);
        assert_eq!(
            outline.resolve_headline_path("A"),
            Err(HeadlinePathError::Ambiguous("A".into()))
        );
        assert!(matches!(
            outline.add_headline_paths(&["A/B".into()]),
            Err(HeadlinePathError::Ambiguous(path)) if path == "A/B"
        ));
        assert_eq!(
            outline.resolve_headline_path("A//B"),
            Err(HeadlinePathError::EmptyComponent("A//B".into()))
        );
    }

    #[test]
    fn backslash_escapes_a_literal_slash_or_backslash_in_a_component() {
        let mut outline = Outline::default();
        outline
            .add_headline_paths(&[r"Imports/PRs/fix a\/b bug".into()])
            .unwrap();
        let prs = outline.resolve_headline_path("Imports/PRs").unwrap();
        assert_eq!(outline.roots[0].children[0].node, prs);
        let leaf = &outline.nodes[&outline.roots[0].children[0].children[0].node];
        assert_eq!(leaf.headline, "fix a/b bug");
        assert_eq!(
            outline
                .resolve_headline_path(r"Imports/PRs/fix a\/b bug")
                .unwrap(),
            leaf.id
        );

        outline
            .add_headline_paths(&[r"Escaped\\slash".into()])
            .unwrap();
        assert_eq!(
            outline.roots[1].node,
            outline.resolve_headline_path(r"Escaped\\slash").unwrap()
        );
        assert_eq!(
            outline.nodes[&outline.roots[1].node].headline,
            r"Escaped\slash"
        );

        // A backslash not followed by "/" or "\\" is kept as a literal
        // character, not treated as the start of an escape.
        outline.add_headline_paths(&[r"Plain\path".into()]).unwrap();
        assert_eq!(
            outline.nodes[&outline.roots[2].node].headline,
            r"Plain\path"
        );
    }
}
