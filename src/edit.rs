//! Interactive structural edits on an [`Outline`]: move a position among its
//! siblings or across levels, and copy/paste subtrees (as independent copies
//! or as clones). Pure outline manipulation -- callers decide which nodes
//! are editable (e.g. read-only derived content) before calling.

use std::collections::HashMap;

use thiserror::Error;

use crate::import::IdGenerator;
use crate::model::parse_path;
use crate::{Node, NodeId, Outline, Position, PositionId, referenced_nodes};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveDirection {
    /// Swap with the previous sibling.
    Up,
    /// Swap with the next sibling.
    Down,
    /// Promote: become the next sibling of the parent.
    Left,
    /// Demote: become the last child of the previous sibling.
    Right,
}

#[derive(Debug, Error, PartialEq)]
pub enum EditError {
    #[error("no such position: {0}")]
    NoPosition(String),
    #[error("node is already at the edge")]
    AtEdge,
    #[error("top-level nodes cannot be promoted")]
    TopLevel,
    #[error("no previous sibling to become parent")]
    NoPreviousSibling,
    #[error("this node cannot take new children")]
    RefusesChildren,
    #[error("cannot paste a clone inside its own tree")]
    CloneIntoSelf,
}

/// A copied set of subtrees together with the nodes they reference.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Subtree {
    pub roots: Vec<Position>,
    pub nodes: HashMap<NodeId, Node>,
}

fn split(id: &PositionId) -> Option<(Option<PositionId>, usize)> {
    let mut path = parse_path(id)?;
    let index = path.pop()?;
    let parent = (!path.is_empty()).then(|| join(None, &path));
    Some((parent, index))
}

fn join(parent: Option<&PositionId>, path: &[usize]) -> PositionId {
    let tail = path
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join("/");
    match parent {
        Some(parent) => PositionId(format!("{}/{tail}", parent.0)),
        None => PositionId(tail),
    }
}

fn child_id(parent: Option<&PositionId>, index: usize) -> PositionId {
    join(parent, &[index])
}

impl Outline {
    /// Moves the position `id` and returns where it ends up. For
    /// [`MoveDirection::Right`], `refuses_children` is asked about the
    /// previous sibling that would become the new parent.
    pub fn move_position(
        &mut self,
        id: &PositionId,
        direction: MoveDirection,
        refuses_children: &dyn Fn(&NodeId) -> bool,
    ) -> Result<PositionId, EditError> {
        let missing = || EditError::NoPosition(id.0.clone());
        let (parent, index) = split(id).ok_or_else(missing)?;
        if self.position(id).is_none() {
            return Err(missing());
        }
        match direction {
            MoveDirection::Up | MoveDirection::Down => {
                let siblings = self.children_mut(parent.as_ref()).ok_or_else(missing)?;
                let other = if direction == MoveDirection::Up {
                    index.checked_sub(1)
                } else {
                    (index + 1 < siblings.len()).then_some(index + 1)
                }
                .ok_or(EditError::AtEdge)?;
                siblings.swap(index, other);
                Ok(child_id(parent.as_ref(), other))
            }
            MoveDirection::Right => {
                let previous = index.checked_sub(1).ok_or(EditError::NoPreviousSibling)?;
                let siblings = self.children_mut(parent.as_ref()).ok_or_else(missing)?;
                if refuses_children(&siblings[previous].node) {
                    return Err(EditError::RefusesChildren);
                }
                let position = siblings.remove(index);
                let child_index = siblings[previous].children.len();
                siblings[previous].children.push(position);
                let previous_id = child_id(parent.as_ref(), previous);
                Ok(child_id(Some(&previous_id), child_index))
            }
            MoveDirection::Left => {
                let parent_id = parent.ok_or(EditError::TopLevel)?;
                let (grandparent, parent_index) = split(&parent_id).ok_or_else(missing)?;
                let position = self
                    .children_mut(Some(&parent_id))
                    .ok_or_else(missing)?
                    .remove(index);
                self.children_mut(grandparent.as_ref())
                    .ok_or_else(missing)?
                    .insert(parent_index + 1, position);
                Ok(child_id(grandparent.as_ref(), parent_index + 1))
            }
        }
    }

    /// Copies the subtrees at `ids` (which must not nest inside each other).
    pub fn copy_subtrees(&self, ids: &[PositionId]) -> Subtree {
        let roots: Vec<Position> = ids
            .iter()
            .filter_map(|id| self.position(id).cloned())
            .collect();
        let nodes = referenced_nodes(&roots)
            .into_iter()
            .filter_map(|id| self.nodes.get(&id).cloned().map(|node| (id, node)))
            .collect();
        Subtree { roots, nodes }
    }

    /// Removes the subtrees at `ids` (which must not nest inside each
    /// other) and drops nodes no longer referenced anywhere.
    pub fn remove_subtrees(&mut self, ids: &[PositionId]) {
        let mut ordered: Vec<(Vec<usize>, &PositionId)> = ids
            .iter()
            .filter_map(|id| Some((parse_path(id)?, id)))
            .collect();
        // Back to front so earlier removals don't shift later paths.
        ordered.sort_by(|a, b| b.0.cmp(&a.0));
        for (_, id) in ordered {
            if let Some((parent, index)) = split(id)
                && let Some(siblings) = self.children_mut(parent.as_ref())
                && index < siblings.len()
            {
                siblings.remove(index);
            }
        }
        let referenced = referenced_nodes(&self.roots);
        self.nodes.retain(|id, _| referenced.contains(id));
    }

    /// Pastes `subtree` as children of `parent` (or as roots) starting at
    /// `index`, returning the first pasted position. With `as_clone` the
    /// pasted positions share the original node ids; otherwise every node
    /// gets a fresh id and is independent.
    pub fn paste_subtree(
        &mut self,
        parent: Option<&PositionId>,
        index: usize,
        subtree: &Subtree,
        as_clone: bool,
    ) -> Result<PositionId, EditError> {
        let bad = || EditError::NoPosition(parent.map(|p| p.0.clone()).unwrap_or_default());
        if as_clone
            && let Some(parent) = parent
            && let Some(position) = self.position(parent)
            && subtree.nodes.contains_key(&position.node)
        {
            return Err(EditError::CloneIntoSelf);
        }
        let len = self.children_mut(parent).ok_or_else(bad)?.len();
        let index = index.min(len);
        let pasted = if as_clone {
            for (id, node) in &subtree.nodes {
                self.nodes.entry(id.clone()).or_insert_with(|| node.clone());
            }
            subtree.roots.clone()
        } else {
            let mut ids =
                IdGenerator::with_prefix("cub".into(), self.nodes.keys().cloned().collect());
            let remap: HashMap<NodeId, NodeId> = subtree
                .nodes
                .keys()
                .map(|old| (old.clone(), ids.next()))
                .collect();
            for (old, node) in &subtree.nodes {
                let id = remap[old].clone();
                let mut node = node.clone();
                node.id.clone_from(&id);
                self.nodes.insert(id, node);
            }
            fn visit(position: &Position, remap: &HashMap<NodeId, NodeId>) -> Position {
                Position {
                    node: remap[&position.node].clone(),
                    children: position.children.iter().map(|c| visit(c, remap)).collect(),
                }
            }
            subtree.roots.iter().map(|r| visit(r, &remap)).collect()
        };
        self.children_mut(parent)
            .ok_or_else(bad)?
            .splice(index..index, pasted);
        Ok(child_id(parent, index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outline() -> Outline {
        let mut outline = Outline::default();
        for id in ["a", "b", "c", "d"] {
            outline.nodes.insert(
                NodeId(id.into()),
                Node {
                    id: NodeId(id.into()),
                    headline: id.into(),
                    body: format!("body {id}"),
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                },
            );
        }
        let leaf = |id: &str| Position {
            node: NodeId(id.into()),
            children: vec![],
        };
        // a(b), c, d
        outline.roots = vec![
            Position {
                node: NodeId("a".into()),
                children: vec![leaf("b")],
            },
            leaf("c"),
            leaf("d"),
        ];
        outline
    }

    fn pid(s: &str) -> PositionId {
        PositionId(s.into())
    }

    fn roots(outline: &Outline) -> Vec<&str> {
        outline.roots.iter().map(|p| p.node.0.as_str()).collect()
    }

    #[test]
    fn move_up_and_down_swap_siblings() {
        let mut o = outline();
        let at = o
            .move_position(&pid("1"), MoveDirection::Up, &|_| false)
            .unwrap();
        assert_eq!(at, pid("0"));
        assert_eq!(roots(&o), ["c", "a", "d"]);
        let at = o
            .move_position(&pid("0"), MoveDirection::Down, &|_| false)
            .unwrap();
        assert_eq!(at, pid("1"));
        assert_eq!(
            o.move_position(&pid("0"), MoveDirection::Up, &|_| false),
            Err(EditError::AtEdge)
        );
        assert_eq!(
            o.move_position(&pid("2"), MoveDirection::Down, &|_| false),
            Err(EditError::AtEdge)
        );
    }

    #[test]
    fn demote_then_promote_round_trips() {
        let mut o = outline();
        let at = o
            .move_position(&pid("1"), MoveDirection::Right, &|_| false)
            .unwrap();
        assert_eq!(at, pid("0/1"));
        assert_eq!(roots(&o), ["a", "d"]);
        let at = o
            .move_position(&at, MoveDirection::Left, &|_| false)
            .unwrap();
        assert_eq!(at, pid("1"));
        assert_eq!(roots(&o), ["a", "c", "d"]);
        assert!(o.validate().is_empty());
    }

    #[test]
    fn demote_respects_refusal_and_edges() {
        let mut o = outline();
        assert_eq!(
            o.move_position(&pid("1"), MoveDirection::Right, &|n| n.0 == "a"),
            Err(EditError::RefusesChildren)
        );
        assert_eq!(
            o.move_position(&pid("0"), MoveDirection::Right, &|_| false),
            Err(EditError::NoPreviousSibling)
        );
        assert_eq!(
            o.move_position(&pid("0"), MoveDirection::Left, &|_| false),
            Err(EditError::TopLevel)
        );
        assert!(matches!(
            o.move_position(&pid("9"), MoveDirection::Up, &|_| false),
            Err(EditError::NoPosition(_))
        ));
    }

    #[test]
    fn copy_paste_makes_independent_nodes() {
        let mut o = outline();
        let copied = o.copy_subtrees(&[pid("0")]);
        assert_eq!(copied.nodes.len(), 2);
        let at = o.paste_subtree(None, 3, &copied, false).unwrap();
        assert_eq!(at, pid("3"));
        assert_eq!(o.roots.len(), 4);
        assert_eq!(o.nodes.len(), 6);
        assert_ne!(o.roots[3].node, NodeId("a".into()));
        assert_eq!(o.nodes[&o.roots[3].node].body, "body a");
        assert!(o.validate().is_empty());
    }

    #[test]
    fn paste_as_clone_shares_nodes_and_refuses_self_nesting() {
        let mut o = outline();
        let copied = o.copy_subtrees(&[pid("1")]);
        o.paste_subtree(Some(&pid("0")), 0, &copied, true).unwrap();
        assert_eq!(o.nodes.len(), 4);
        assert_eq!(o.roots[0].children[0].node, NodeId("c".into()));
        // pasting c's clone beneath c itself would loop
        assert_eq!(
            o.paste_subtree(Some(&pid("1")), 0, &copied, true),
            Err(EditError::CloneIntoSelf)
        );
        assert!(o.validate().is_empty());
    }

    #[test]
    fn remove_subtrees_handles_several_and_prunes_nodes() {
        let mut o = outline();
        o.remove_subtrees(&[pid("0"), pid("2")]);
        assert_eq!(roots(&o), ["c"]);
        assert_eq!(o.nodes.len(), 1);
        assert!(o.validate().is_empty());
    }
}
