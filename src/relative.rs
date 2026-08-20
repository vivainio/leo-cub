//! Parser for `@f` files: leo-cub's own "cub-1-thin" sentinel format (not an
//! official Leo version tag) that encodes outline depth relative to the
//! preceding node sentinel, and serializes a GNX only for nodes that need
//! persistent identity (the file root, clones, and nodes carrying user
//! attributes). See leo-editor issue #4928.
//!
//! Every other sentinel (`@+others`, `@+all`, `@+<<section>>`, `@@first`,
//! `@@last`, `@afterref`, `@verbatim`, `@@directive` escaping, `@-leo`) is
//! unchanged from the 5-thin grammar implemented in `derived.rs`; only the
//! per-node opening sentinel differs, so this module reuses `derived.rs`'s
//! body-reconstruction helpers rather than duplicating them.

use std::collections::{HashMap, HashSet};

use regex::Regex;

use crate::{
    Node, NodeId, Outline, Position, PositionId, SentinelError,
    derived::{
        append_body, children_at, leading_width, referenced_nodes, restore_expansion,
        sentinel_content, strip_indent,
    },
};

#[derive(Clone, Debug)]
pub struct RelativeFile {
    pub outline: Outline,
    pub root: NodeId,
    pub start_delimiter: String,
    pub end_delimiter: String,
    /// One-based sentinel line for each physical outline position.
    pub locations: HashMap<PositionId, usize>,
    /// Positions whose sentinel omitted a `[gnx]`; their `NodeId` is a
    /// synthetic placeholder that `merge_into` reconciles against whatever
    /// node already occupies the same structural position.
    anonymous: HashSet<NodeId>,
}

impl RelativeFile {
    /// Parse an `@f` derived file and reconstruct its logical outline.
    pub fn parse(source: &str) -> Result<Self, SentinelError> {
        let lines: Vec<&str> = source.split_inclusive('\n').collect();
        let (header_index, start_delimiter, end_delimiter) = scan_header(&lines)?;
        let sentinel_prefix = format!("{start_delimiter}@");
        let node_pattern = Regex::new(&format!(
            r"^(\s*){}@(0|[<>]\d*)? (?:\[([^\]]*)\] )?(.*){}\r?\n?$",
            regex::escape(&start_delimiter),
            regex::escape(&end_delimiter)
        ))
        .expect("generated node pattern");

        let mut outline = Outline::default();
        let mut root = None;
        let mut anonymous = HashSet::new();
        let mut level_paths: Vec<Vec<usize>> = Vec::new();
        let mut locations = HashMap::new();
        let mut current: Option<NodeId> = None;
        let mut indent = 0usize;
        let mut expansions: Vec<(NodeId, usize)> = Vec::new();
        let verbatim = format!("{start_delimiter}@verbatim{end_delimiter}");
        let mut verbatim_next = false;
        let mut after_ref = false;
        let mut first_index = 0usize;
        let mut last_count = 0usize;
        let mut tail_start = lines.len();
        let mut prev_level = 0usize;

        for (offset, original) in lines.iter().enumerate().skip(header_index + 1) {
            let line_number = offset + 1;
            let line = *original;
            let trimmed = line.trim();

            if verbatim_next {
                append_body(&mut outline, current.as_ref(), strip_indent(line, indent));
                verbatim_next = false;
                continue;
            }
            if after_ref {
                if let Some(id) = current.as_ref()
                    && let Some(node) = outline.nodes.get_mut(id)
                {
                    let joined = format!("{}{}", node.body.trim_end(), line);
                    node.body = joined;
                }
                after_ref = false;
                continue;
            }
            if trimmed == verbatim {
                verbatim_next = true;
                continue;
            }

            if let Some(captures) = node_pattern.captures(line) {
                let token = captures.get(2).map(|m| m.as_str());
                let gnx = captures.get(3).map(|m| m.as_str().to_owned());
                let headline = captures[4].to_owned();

                let level = if root.is_none() {
                    if token != Some("0") {
                        return Err(SentinelError::MalformedNode {
                            line: line_number,
                            text: trimmed.to_owned(),
                        });
                    }
                    if gnx.is_none() {
                        return Err(SentinelError::MalformedNode {
                            line: line_number,
                            text: trimmed.to_owned(),
                        });
                    }
                    1
                } else {
                    match token {
                        Some("0") => {
                            return Err(SentinelError::MalformedNode {
                                line: line_number,
                                text: trimmed.to_owned(),
                            });
                        }
                        None => prev_level,
                        Some(token) => {
                            let (sign, digits) = token.split_at(1);
                            let delta: usize = if digits.is_empty() {
                                1
                            } else {
                                digits.parse().map_err(|_| SentinelError::MalformedNode {
                                    line: line_number,
                                    text: trimmed.to_owned(),
                                })?
                            };
                            if sign == ">" {
                                prev_level + delta
                            } else {
                                prev_level
                                    .checked_sub(delta)
                                    .filter(|level| *level >= 1)
                                    .ok_or(SentinelError::DepthUnderflow { line: line_number })?
                            }
                        }
                    }
                };

                let path = if root.is_none() {
                    root = Some(NodeId(gnx.clone().expect("checked above")));
                    vec![0]
                } else {
                    level
                        .checked_sub(2)
                        .and_then(|i| level_paths.get(i))
                        .cloned()
                        .ok_or(SentinelError::MissingParent {
                            line: line_number,
                            level,
                        })?
                };

                // A synthetic id only needs to be unique as a HashMap key
                // during this parse; `merge_into` discards it in favor of
                // whatever node already occupies the same structural
                // position, so the line number (always unique per file) is
                // sufficient and avoids needing this position's own index
                // before it has been assigned one.
                let id = match &gnx {
                    Some(gnx) => NodeId(gnx.clone()),
                    None => {
                        let root_gnx = root.as_ref().expect("root set above").0.clone();
                        let id = NodeId(format!("{root_gnx}::f-anon:{line_number}"));
                        anonymous.insert(id.clone());
                        id
                    }
                };

                outline
                    .nodes
                    .entry(id.clone())
                    .and_modify(|node| {
                        node.headline.clone_from(&headline);
                        node.body.clear();
                    })
                    .or_insert_with(|| Node {
                        id: id.clone(),
                        headline: headline.clone(),
                        body: String::new(),
                        vnode_attributes: HashMap::new(),
                        tnode_attributes: HashMap::new(),
                    });

                let position = Position {
                    node: id.clone(),
                    children: Vec::new(),
                };
                let full_path = if level == 1 {
                    outline.roots.push(position);
                    vec![0]
                } else {
                    let children = children_at(&mut outline.roots, &path).ok_or(
                        SentinelError::MissingParent {
                            line: line_number,
                            level,
                        },
                    )?;
                    let index = children.len();
                    children.push(position);
                    let mut full_path = path;
                    full_path.push(index);
                    full_path
                };
                level_paths.truncate(level.saturating_sub(1));
                level_paths.push(full_path.clone());
                let position_id = PositionId(
                    full_path
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join("/"),
                );
                locations.insert(position_id, line_number);
                current = Some(id);
                indent = captures[1].len();
                prev_level = level;
                continue;
            }

            if !trimmed.starts_with(&sentinel_prefix) {
                append_body(&mut outline, current.as_ref(), strip_indent(line, indent));
                continue;
            }
            let inner = sentinel_content(trimmed, &start_delimiter, &end_delimiter);
            if inner == "@-leo" {
                tail_start = offset + 1;
                break;
            }
            if inner == "@afterref" {
                after_ref = true;
                continue;
            }
            if inner == "@@first" {
                if let Some(first) = lines
                    .get(first_index)
                    .filter(|_| first_index < header_index)
                {
                    append_body(&mut outline, current.as_ref(), &format!("@first {first}"));
                    first_index += 1;
                }
                continue;
            }
            if inner == "@@last" {
                last_count += 1;
                continue;
            }

            if let Some(tail) = inner.strip_prefix("@+others") {
                let id = current.clone().ok_or(SentinelError::MissingRoot)?;
                let leading = leading_width(line);
                let local = leading.saturating_sub(indent);
                append_body(
                    &mut outline,
                    Some(&id),
                    &format!("{}@others{}\n", &line[indent..indent + local], tail),
                );
                expansions.push((id, indent));
                indent = leading;
                continue;
            }
            if inner.starts_with("@-others") {
                (current, indent) = restore_expansion(&mut expansions, line_number)?;
                continue;
            }
            if let Some(tail) = inner.strip_prefix("@+all") {
                let id = current.clone().ok_or(SentinelError::MissingRoot)?;
                let leading = leading_width(line);
                let local = leading.saturating_sub(indent);
                append_body(
                    &mut outline,
                    Some(&id),
                    &format!("{}@all{}\n", &line[indent..indent + local], tail),
                );
                expansions.push((id, indent));
                indent = leading;
                continue;
            }
            if inner.starts_with("@-all") {
                (current, indent) = restore_expansion(&mut expansions, line_number)?;
                continue;
            }
            if let Some(section) = inner.strip_prefix("@+<<") {
                let id = current.clone().ok_or(SentinelError::MissingRoot)?;
                let section = section.strip_suffix(">>").unwrap_or(section);
                let leading = leading_width(line);
                let local = leading.saturating_sub(indent);
                append_body(
                    &mut outline,
                    Some(&id),
                    &format!("{}<<{}>>\n", &line[indent..indent + local], section),
                );
                expansions.push((id, indent));
                indent = leading;
                continue;
            }
            if inner.starts_with("@-<<") {
                (current, indent) = restore_expansion(&mut expansions, line_number)?;
                continue;
            }
            if let Some(directive) = inner.strip_prefix("@@") {
                append_body(&mut outline, current.as_ref(), &format!("@{directive}\n"));
            }
        }
        if verbatim_next {
            return Err(SentinelError::UnterminatedVerbatim);
        }
        if !expansions.is_empty() {
            return Err(SentinelError::UnterminatedExpansion);
        }
        let root = root.ok_or(SentinelError::MissingRoot)?;
        if last_count > 0 {
            for line in lines.iter().skip(tail_start).take(last_count) {
                append_body(
                    &mut outline,
                    current.as_ref(),
                    &format!("@last {}\n", line.trim_end()),
                );
            }
        }
        Ok(Self {
            outline,
            root,
            start_delimiter,
            end_delimiter,
            locations,
            anonymous,
        })
    }

    /// Transactionally replace the descendants and bodies represented by a
    /// derived file. Unlike `DerivedFile::merge_into`, a position whose
    /// sentinel omitted a `[gnx]` keeps whatever `NodeId` already occupies
    /// the same structural position in `outline`, so anonymous nodes retain
    /// their prior identity across edits instead of getting a synthetic one
    /// every sync. A newly inserted anonymous position (no prior occupant at
    /// that path) keeps its synthetic placeholder id.
    pub fn merge_into(
        &self,
        outline: &mut Outline,
        target: &PositionId,
    ) -> Result<(), SentinelError> {
        let target_node = outline
            .position(target)
            .map(|p| p.node.clone())
            .ok_or_else(|| SentinelError::PositionNotFound(target.0.clone()))?;
        if target_node != self.root {
            return Err(SentinelError::RootMismatch {
                outline: target_node.0,
                derived: self.root.0.clone(),
            });
        }
        let mut next = outline.clone();
        let existing_children = outline
            .position(target)
            .map(|position| position.children.clone())
            .unwrap_or_default();
        next.nodes
            .get_mut(&self.root)
            .expect("validated target")
            .body
            .clone_from(&self.outline.nodes[&self.root].body);
        let parsed_root = &self.outline.roots[0];
        let children = reconcile_children(
            &parsed_root.children,
            &existing_children,
            &self.outline.nodes,
            &self.anonymous,
            &mut next.nodes,
        );
        next.children_mut(Some(target))
            .expect("validated target")
            .clone_from(&children);
        let referenced = referenced_nodes(&next.roots);
        next.nodes.retain(|id, _| referenced.contains(id));
        *outline = next;
        Ok(())
    }
}

fn reconcile_children(
    parsed: &[Position],
    existing: &[Position],
    parsed_nodes: &HashMap<NodeId, Node>,
    anonymous: &HashSet<NodeId>,
    nodes: &mut HashMap<NodeId, Node>,
) -> Vec<Position> {
    parsed
        .iter()
        .enumerate()
        .map(|(index, child)| {
            let existing_child = existing.get(index);
            let parsed_node = &parsed_nodes[&child.node];
            let real_id = if anonymous.contains(&child.node) {
                existing_child.map_or_else(|| child.node.clone(), |found| found.node.clone())
            } else {
                child.node.clone()
            };
            if let Some(node) = nodes.get_mut(&real_id) {
                node.headline.clone_from(&parsed_node.headline);
                node.body.clone_from(&parsed_node.body);
            } else {
                nodes.insert(
                    real_id.clone(),
                    Node {
                        id: real_id.clone(),
                        headline: parsed_node.headline.clone(),
                        body: parsed_node.body.clone(),
                        vnode_attributes: HashMap::new(),
                        tnode_attributes: HashMap::new(),
                    },
                );
            }
            let grandchildren = reconcile_children(
                &child.children,
                existing_child.map_or(&[][..], |found| &found.children),
                parsed_nodes,
                anonymous,
                nodes,
            );
            Position {
                node: real_id,
                children: grandchildren,
            }
        })
        .collect()
}

fn scan_header(lines: &[&str]) -> Result<(usize, String, String), SentinelError> {
    let pattern =
        Regex::new(r"^(.+)@\+leo-ver=cub-1-thin(?:-encoding=.*?,\.)?(.*)\r?\n?$").unwrap();
    for (index, line) in lines.iter().enumerate() {
        if let Some(captures) = pattern.captures(line) {
            return Ok((index, captures[1].to_owned(), captures[2].to_owned()));
        }
    }
    Err(SentinelError::MissingHeader)
}
