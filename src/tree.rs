use std::collections::HashMap;

use serde::Serialize;
use thiserror::Error;

use crate::import::IdGenerator;
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
