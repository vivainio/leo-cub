use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub String);

impl From<&str> for NodeId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Identifies one occurrence in the outline, including cloned occurrences.
/// It is an index path from the hidden root (`0/2/1`).
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PositionId(pub String);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub headline: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub vnode_attributes: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tnode_attributes: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub node: NodeId,
    #[serde(default)]
    pub children: Vec<Position>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Outline {
    pub nodes: HashMap<NodeId, Node>,
    pub roots: Vec<Position>,
}

#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    #[error("position {position} references missing node {node}")]
    MissingNode { position: String, node: String },
    #[error("node table key {key} does not match node id {node}")]
    MismatchedNodeId { key: String, node: String },
    #[error("node {0} is not referenced by any position")]
    OrphanNode(String),
}

impl Outline {
    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        for (key, node) in &self.nodes {
            if key != &node.id {
                errors.push(ValidationError::MismatchedNodeId {
                    key: key.0.clone(),
                    node: node.id.0.clone(),
                });
            }
        }
        let mut referenced = HashSet::new();
        fn walk(
            p: &Position,
            path: String,
            nodes: &HashMap<NodeId, Node>,
            seen: &mut HashSet<NodeId>,
            errors: &mut Vec<ValidationError>,
        ) {
            if !nodes.contains_key(&p.node) {
                errors.push(ValidationError::MissingNode {
                    position: path.clone(),
                    node: p.node.0.clone(),
                });
            }
            seen.insert(p.node.clone());
            for (i, child) in p.children.iter().enumerate() {
                walk(child, format!("{path}/{i}"), nodes, seen, errors);
            }
        }
        for (i, root) in self.roots.iter().enumerate() {
            walk(
                root,
                i.to_string(),
                &self.nodes,
                &mut referenced,
                &mut errors,
            );
        }
        for id in self.nodes.keys() {
            if !referenced.contains(id) {
                errors.push(ValidationError::OrphanNode(id.0.clone()));
            }
        }
        errors
    }

    pub fn position(&self, id: &PositionId) -> Option<&Position> {
        let indices = parse_path(id)?;
        let mut p = self.roots.get(*indices.first()?)?;
        for index in &indices[1..] {
            p = p.children.get(*index)?;
        }
        Some(p)
    }

    pub(crate) fn children_mut(
        &mut self,
        parent: Option<&PositionId>,
    ) -> Option<&mut Vec<Position>> {
        let Some(parent) = parent else {
            return Some(&mut self.roots);
        };
        let indices = parse_path(parent)?;
        let mut p = self.roots.get_mut(*indices.first()?)?;
        for index in &indices[1..] {
            p = p.children.get_mut(*index)?;
        }
        Some(&mut p.children)
    }
}

fn parse_path(id: &PositionId) -> Option<Vec<usize>> {
    if id.0.is_empty() {
        return None;
    }
    id.0.split('/').map(|s| s.parse().ok()).collect()
}
