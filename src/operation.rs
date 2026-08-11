use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Node, NodeId, Outline, Position, PositionId};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Target {
    Position { position: PositionId },
    Node { node: NodeId },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum Operation {
    Insert {
        parent: Option<PositionId>,
        index: Option<usize>,
        node: Node,
    },
    Clone {
        parent: Option<PositionId>,
        index: Option<usize>,
        node: NodeId,
    },
    SetHeadline {
        node: NodeId,
        headline: String,
        expected: Option<String>,
    },
    SetBody {
        node: NodeId,
        body: String,
        expected: Option<String>,
    },
    Remove {
        position: PositionId,
    },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OperationBatch {
    pub operations: Vec<Operation>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApplyReport {
    pub applied: usize,
}

#[derive(Debug, Error)]
pub enum ApplyError {
    #[error("node already exists: {0}")]
    DuplicateNode(String),
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("position not found: {0}")]
    PositionNotFound(String),
    #[error("index {index} is out of range (length {len})")]
    BadIndex { index: usize, len: usize },
    #[error("precondition failed for {field} of node {node}")]
    Precondition { node: String, field: &'static str },
    #[error("batch would create an invalid outline: {0}")]
    Invalid(String),
}

impl Outline {
    /// Applies all operations to a copy and commits only if every operation succeeds.
    pub fn apply(&mut self, batch: &OperationBatch) -> Result<ApplyReport, ApplyError> {
        let mut next = self.clone();
        for op in &batch.operations {
            next.apply_one(op)?;
        }
        if let Some(error) = next.validate().first() {
            return Err(ApplyError::Invalid(error.to_string()));
        }
        *self = next;
        Ok(ApplyReport {
            applied: batch.operations.len(),
        })
    }

    fn apply_one(&mut self, op: &Operation) -> Result<(), ApplyError> {
        match op {
            Operation::Insert {
                parent,
                index,
                node,
            } => {
                if self.nodes.contains_key(&node.id) {
                    return Err(ApplyError::DuplicateNode(node.id.0.clone()));
                }
                let id = node.id.clone();
                let children = self.children_mut(parent.as_ref()).ok_or_else(|| {
                    ApplyError::PositionNotFound(parent.as_ref().unwrap().0.clone())
                })?;
                let at = index.unwrap_or(children.len());
                if at > children.len() {
                    return Err(ApplyError::BadIndex {
                        index: at,
                        len: children.len(),
                    });
                }
                children.insert(
                    at,
                    Position {
                        node: id.clone(),
                        children: vec![],
                    },
                );
                self.nodes.insert(id, node.clone());
            }
            Operation::Clone {
                parent,
                index,
                node,
            } => {
                if !self.nodes.contains_key(node) {
                    return Err(ApplyError::NodeNotFound(node.0.clone()));
                }
                let children = self.children_mut(parent.as_ref()).ok_or_else(|| {
                    ApplyError::PositionNotFound(parent.as_ref().unwrap().0.clone())
                })?;
                let at = index.unwrap_or(children.len());
                if at > children.len() {
                    return Err(ApplyError::BadIndex {
                        index: at,
                        len: children.len(),
                    });
                }
                children.insert(
                    at,
                    Position {
                        node: node.clone(),
                        children: vec![],
                    },
                );
            }
            Operation::SetHeadline {
                node,
                headline,
                expected,
            } => set_text(self.nodes.get_mut(node), node, expected, headline, true)?,
            Operation::SetBody {
                node,
                body,
                expected,
            } => set_text(self.nodes.get_mut(node), node, expected, body, false)?,
            Operation::Remove { position } => {
                let mut path: Vec<usize> = position
                    .0
                    .split('/')
                    .map(str::parse)
                    .collect::<Result<_, _>>()
                    .map_err(|_| ApplyError::PositionNotFound(position.0.clone()))?;
                let index = path
                    .pop()
                    .ok_or_else(|| ApplyError::PositionNotFound(position.0.clone()))?;
                let parent = if path.is_empty() {
                    None
                } else {
                    Some(PositionId(
                        path.iter()
                            .map(usize::to_string)
                            .collect::<Vec<_>>()
                            .join("/"),
                    ))
                };
                let children = self
                    .children_mut(parent.as_ref())
                    .ok_or_else(|| ApplyError::PositionNotFound(position.0.clone()))?;
                if index >= children.len() {
                    return Err(ApplyError::PositionNotFound(position.0.clone()));
                }
                children.remove(index);
                let used: std::collections::HashSet<_> =
                    positions(&self.roots).map(|p| p.node.clone()).collect();
                self.nodes.retain(|id, _| used.contains(id));
            }
        }
        Ok(())
    }
}

fn set_text(
    node: Option<&mut Node>,
    id: &NodeId,
    expected: &Option<String>,
    value: &str,
    headline: bool,
) -> Result<(), ApplyError> {
    let node = node.ok_or_else(|| ApplyError::NodeNotFound(id.0.clone()))?;
    let current = if headline {
        &mut node.headline
    } else {
        &mut node.body
    };
    if expected.as_ref().is_some_and(|e| e != current) {
        return Err(ApplyError::Precondition {
            node: id.0.clone(),
            field: if headline { "headline" } else { "body" },
        });
    }
    *current = value.to_owned();
    Ok(())
}

fn positions(items: &[Position]) -> Box<dyn Iterator<Item = &Position> + '_> {
    Box::new(
        items
            .iter()
            .flat_map(|p| std::iter::once(p).chain(positions(&p.children))),
    )
}
