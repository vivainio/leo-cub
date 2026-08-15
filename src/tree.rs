use std::{
    collections::{HashMap, HashSet},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use thiserror::Error;

use crate::{Node, NodeId, Outline, Position};

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
        Ok(selected.unwrap().node.clone())
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
        let mut ids = IdGenerator::new(next.nodes.keys().cloned().collect());
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
}

fn path_parts(path: &str) -> Result<Vec<&str>, HeadlinePathError> {
    if path.is_empty() {
        return Err(HeadlinePathError::Empty);
    }
    let parts: Vec<_> = path.split('/').collect();
    if parts.iter().any(|part| part.is_empty()) {
        return Err(HeadlinePathError::EmptyComponent(path.to_owned()));
    }
    Ok(parts)
}

fn ensure_path(
    siblings: &mut Vec<Position>,
    nodes: &mut HashMap<NodeId, Node>,
    ids: &mut IdGenerator,
    parts: &[&str],
    full_path: &str,
    created: &mut usize,
) -> Result<(), HeadlinePathError> {
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
        )?;
    }
    Ok(())
}

struct IdGenerator {
    used: HashSet<NodeId>,
    seconds: u64,
    nanos: u32,
    sequence: u64,
}

impl IdGenerator {
    fn new(used: HashSet<NodeId>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self {
            used,
            seconds: now.as_secs(),
            nanos: now.subsec_nanos(),
            sequence: 0,
        }
    }

    fn next(&mut self) -> NodeId {
        loop {
            self.sequence += 1;
            let id = NodeId(format!(
                "cub.{}.{}.{}",
                self.seconds, self.nanos, self.sequence
            ));
            if self.used.insert(id.clone()) {
                return id;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
