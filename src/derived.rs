//! Parser for Leo 5 thin derived files.

use std::collections::{HashMap, HashSet};

use regex::Regex;
use thiserror::Error;

use crate::{Node, NodeId, Outline, Position, PositionId};

#[derive(Clone, Debug)]
pub struct DerivedFile {
    pub outline: Outline,
    pub root: NodeId,
    pub start_delimiter: String,
    pub end_delimiter: String,
    /// One-based sentinel line for each physical outline position.
    pub locations: HashMap<PositionId, usize>,
}

#[derive(Debug, Error, PartialEq)]
pub enum SentinelError {
    #[error("no @+leo header sentinel found")]
    MissingHeader,
    #[error("derived file contains no @+node sentinel")]
    MissingRoot,
    #[error("line {line}: malformed node sentinel: {text}")]
    MalformedNode { line: usize, text: String },
    #[error("line {line}: node level {level} has no parent")]
    MissingParent { line: usize, level: usize },
    #[error("line {line}: unmatched closing expansion sentinel")]
    UnmatchedExpansion { line: usize },
    #[error("unterminated expansion sentinel")]
    UnterminatedExpansion,
    #[error("unterminated @verbatim sentinel")]
    UnterminatedVerbatim,
    #[error("root GNX mismatch: outline has {outline}, derived file has {derived}")]
    RootMismatch { outline: String, derived: String },
    #[error("position not found: {0}")]
    PositionNotFound(String),
}

impl DerivedFile {
    /// Parse a Leo 5 thin derived file and reconstruct its logical outline.
    pub fn parse(source: &str) -> Result<Self, SentinelError> {
        let lines: Vec<&str> = source.split_inclusive('\n').collect();
        let (header_index, start_delimiter, end_delimiter) = scan_header(&lines)?;
        let sentinel_prefix = format!("{start_delimiter}@");
        let node_pattern = Regex::new(&format!(
            r"^(\s*){}@\+node:([^:]+): (.*){}\r?\n?$",
            regex::escape(&start_delimiter),
            regex::escape(&end_delimiter)
        ))
        .expect("generated node pattern");

        let mut outline = Outline::default();
        let mut root = None;
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
                let gnx = NodeId(captures[2].to_owned());
                let descriptor = &captures[3];
                let (level_token, headline) =
                    descriptor
                        .split_once(' ')
                        .ok_or_else(|| SentinelError::MalformedNode {
                            line: line_number,
                            text: trimmed.to_owned(),
                        })?;
                let level =
                    parse_level(level_token).ok_or_else(|| SentinelError::MalformedNode {
                        line: line_number,
                        text: trimmed.to_owned(),
                    })?;
                if level == 0 {
                    return Err(SentinelError::MalformedNode {
                        line: line_number,
                        text: trimmed.to_owned(),
                    });
                }
                let headline = headline.to_owned();
                outline
                    .nodes
                    .entry(gnx.clone())
                    .and_modify(|node| {
                        node.headline.clone_from(&headline);
                        node.body.clear();
                    })
                    .or_insert_with(|| Node {
                        id: gnx.clone(),
                        headline,
                        body: String::new(),
                        vnode_attributes: HashMap::new(),
                        tnode_attributes: HashMap::new(),
                    });

                let position = Position {
                    node: gnx.clone(),
                    children: Vec::new(),
                };
                let path = if root.is_none() {
                    if level != 1 {
                        return Err(SentinelError::MissingParent {
                            line: line_number,
                            level,
                        });
                    }
                    root = Some(gnx.clone());
                    outline.roots.push(position);
                    vec![0]
                } else {
                    let parent_path = level
                        .checked_sub(2)
                        .and_then(|i| level_paths.get(i))
                        .cloned()
                        .ok_or(SentinelError::MissingParent {
                            line: line_number,
                            level,
                        })?;
                    let children = children_at(&mut outline.roots, &parent_path).ok_or(
                        SentinelError::MissingParent {
                            line: line_number,
                            level,
                        },
                    )?;
                    let index = children.len();
                    children.push(position);
                    let mut path = parent_path;
                    path.push(index);
                    path
                };
                level_paths.truncate(level.saturating_sub(1));
                level_paths.push(path);
                let position_id = PositionId(
                    level_paths
                        .last()
                        .expect("just pushed")
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join("/"),
                );
                locations.insert(position_id, line_number);
                current = Some(gnx);
                indent = captures[1].len();
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
        })
    }

    /// Transactionally replace the descendants and bodies represented by a
    /// derived file. The `@file` root headline remains owned by the `.leo` file.
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
        let derived_root = &self.outline.roots[0];
        let root_body = self.outline.nodes[&self.root].body.clone();
        next.nodes
            .get_mut(&self.root)
            .expect("validated target")
            .body = root_body;
        for (id, node) in &self.outline.nodes {
            if id != &self.root {
                if let Some(existing) = next.nodes.get_mut(id) {
                    existing.headline.clone_from(&node.headline);
                    existing.body.clone_from(&node.body);
                } else {
                    next.nodes.insert(id.clone(), node.clone());
                }
            }
        }
        next.children_mut(Some(target))
            .expect("validated target")
            .clone_from(&derived_root.children);
        let referenced = referenced_nodes(&next.roots);
        next.nodes.retain(|id, _| referenced.contains(id));
        *outline = next;
        Ok(())
    }
}

fn scan_header(lines: &[&str]) -> Result<(usize, String, String), SentinelError> {
    let pattern =
        Regex::new(r"^(.+)@\+leo(?:-ver=\d+)?(?:-thin)?(?:-encoding=.*?,\.)?(.*)\r?\n?$").unwrap();
    for (index, line) in lines.iter().enumerate() {
        if let Some(captures) = pattern.captures(line) {
            return Ok((index, captures[1].to_owned(), captures[2].to_owned()));
        }
    }
    Err(SentinelError::MissingHeader)
}

fn parse_level(token: &str) -> Option<usize> {
    let inner = token.strip_prefix('*')?;
    if inner.is_empty() {
        return Some(1);
    }
    if inner.bytes().all(|byte| byte == b'*') {
        return Some(1 + inner.len());
    }
    inner.strip_suffix('*')?.parse().ok()
}

fn sentinel_content<'a>(line: &'a str, start: &str, end: &str) -> &'a str {
    line.strip_prefix(start)
        .and_then(|s| s.strip_suffix(end))
        .unwrap_or(line)
}

fn leading_width(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

fn strip_indent(line: &str, indent: usize) -> &str {
    if indent > 0
        && line.len() > indent
        && line.as_bytes()[..indent]
            .iter()
            .all(u8::is_ascii_whitespace)
    {
        &line[indent..]
    } else {
        line
    }
}

fn append_body(outline: &mut Outline, id: Option<&NodeId>, text: &str) {
    if let Some(node) = id.and_then(|id| outline.nodes.get_mut(id)) {
        node.body.push_str(text);
    }
}

fn restore_expansion(
    stack: &mut Vec<(NodeId, usize)>,
    line: usize,
) -> Result<(Option<NodeId>, usize), SentinelError> {
    stack
        .pop()
        .map(|(id, indent)| (Some(id), indent))
        .ok_or(SentinelError::UnmatchedExpansion { line })
}

fn children_at<'a>(roots: &'a mut [Position], path: &[usize]) -> Option<&'a mut Vec<Position>> {
    let mut position = roots.get_mut(*path.first()?)?;
    for index in &path[1..] {
        position = position.children.get_mut(*index)?;
    }
    Some(&mut position.children)
}

fn referenced_nodes(positions: &[Position]) -> HashSet<NodeId> {
    fn visit(items: &[Position], result: &mut HashSet<NodeId>) {
        for position in items {
            result.insert(position.node.clone());
            visit(&position.children, result);
        }
    }
    let mut result = HashSet::new();
    visit(positions, &mut result);
    result
}
