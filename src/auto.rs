//! Transient expansion of Leo `@auto` source files.

use std::{collections::HashMap, path::Path};

use thiserror::Error;
use tree_sitter::{Language, Node as TsNode, Parser};

use crate::{Node, NodeId, Outline, Position};

#[derive(Debug, Error)]
pub enum AutoError {
    #[error("no Tree-sitter auto importer for {0}")]
    Unsupported(String),
    #[error("could not load the {0} Tree-sitter grammar")]
    Grammar(&'static str),
    #[error("Tree-sitter did not produce a syntax tree")]
    Parse,
}

/// An in-memory `@auto` expansion. It is never serialized into the `.leo` file.
pub struct AutoFile {
    pub outline: Outline,
    pub root: NodeId,
    /// One-based source line for each generated node.
    pub locations: HashMap<NodeId, usize>,
}

#[derive(Clone, Copy)]
enum Flavor {
    Python,
    Rust,
}

#[derive(Debug)]
struct Block {
    kind: String,
    name: String,
    syntax_start: usize,
    body_start: usize,
    start: usize,
    end: usize,
    class_docstring: bool,
    children: Vec<Block>,
}

impl AutoFile {
    pub fn parse(path: &Path, root: NodeId, source: &str) -> Result<Self, AutoError> {
        let (flavor, language, language_name) = language_for(path)?;
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .map_err(|_| AutoError::Grammar(language_name))?;
        let tree = parser.parse(source, None).ok_or(AutoError::Parse)?;
        let line_starts = line_starts(source);
        let line_count = line_starts.len();
        let mut blocks = find_blocks(tree.root_node(), flavor, source, line_count);
        assign_owned_starts(&mut blocks, 0);
        let preamble_end = root_preamble_end(flavor, source, &blocks);
        if let Some(first) = blocks.first_mut() {
            first.start = preamble_end;
        }

        let mut outline = Outline::default();
        let mut locations = HashMap::new();
        let children = Builder {
            nodes: &mut outline.nodes,
            locations: &mut locations,
            root: &root,
            source,
            starts: &line_starts,
        }
        .build(&blocks, "", None);
        let body = if children.is_empty() {
            source.to_owned()
        } else {
            let tail_start = blocks.last().map_or(0, |block| block.end);
            let mut body = slice_lines(source, &line_starts, 0, preamble_end).to_owned();
            body.push_str("@others\n");
            body.push_str(slice_lines(source, &line_starts, tail_start, line_count));
            body
        };
        let mut body = body;
        if !body.is_empty() && !body.ends_with('\n') {
            body.push('\n');
        }
        body.push_str(&format!("@language {language_name}\n@tabwidth -4\n"));
        outline.nodes.insert(
            root.clone(),
            Node {
                id: root.clone(),
                headline: String::new(),
                body,
                vnode_attributes: HashMap::new(),
                tnode_attributes: HashMap::new(),
            },
        );
        outline.roots.push(Position {
            node: root.clone(),
            children,
        });
        move_leading_blank_lines(&mut outline);
        Ok(Self {
            outline,
            root,
            locations,
        })
    }

    pub fn merge_into(&self, target: &mut Outline, root_position: &crate::PositionId) -> bool {
        let Some(position) = target.position(root_position) else {
            return false;
        };
        let target_root = position.node.clone();
        if target_root != self.root {
            return false;
        }
        let generated = &self.outline.roots[0];
        target
            .nodes
            .get_mut(&self.root)
            .expect("validated auto root")
            .body
            .clone_from(&self.outline.nodes[&self.root].body);
        for (id, node) in &self.outline.nodes {
            if id != &self.root {
                target.nodes.insert(id.clone(), node.clone());
            }
        }
        if let Some(children) = target.children_mut(Some(root_position)) {
            children.clone_from(&generated.children);
        }
        let referenced = referenced_nodes(&target.roots);
        target.nodes.retain(|id, _| referenced.contains(id));
        true
    }
}

fn root_preamble_end(flavor: Flavor, source: &str, blocks: &[Block]) -> usize {
    let Some(first) = blocks.first() else {
        return 0;
    };
    if matches!(flavor, Flavor::Python) {
        return first.syntax_start;
    }
    let mut found_use = false;
    let mut end = 0;
    for (index, line) in source.lines().take(first.syntax_start).enumerate() {
        let line = line.trim();
        if line.starts_with("use ") {
            found_use = true;
            end = index + 1;
        } else if line.is_empty() || line.starts_with("//") || line.starts_with("/*") {
            if found_use || !line.is_empty() {
                end = index + 1;
            }
        } else {
            break;
        }
    }
    if found_use { end } else { 0 }
}

fn move_leading_blank_lines(outline: &mut Outline) {
    fn visit(nodes: &mut HashMap<NodeId, Node>, positions: &[Position]) {
        for index in 1..positions.len() {
            let current = positions[index].node.clone();
            let previous = positions[index - 1].node.clone();
            loop {
                let Some(first_len) = nodes[&current]
                    .body
                    .split_inclusive('\n')
                    .next()
                    .filter(|line| line.trim().is_empty())
                    .map(str::len)
                else {
                    break;
                };
                nodes.get_mut(&current).unwrap().body.drain(..first_len);
                nodes.get_mut(&previous).unwrap().body.push('\n');
            }
        }
        for position in positions {
            visit(nodes, &position.children);
        }
    }
    let children = outline.roots[0].children.clone();
    visit(&mut outline.nodes, &children);
}

fn language_for(path: &Path) -> Result<(Flavor, Language, &'static str), AutoError> {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
    {
        "py" | "pyw" => Ok((
            Flavor::Python,
            tree_sitter_python::LANGUAGE.into(),
            "python",
        )),
        "rs" => Ok((Flavor::Rust, tree_sitter_rust::LANGUAGE.into(), "rust")),
        extension => Err(AutoError::Unsupported(extension.to_owned())),
    }
}

fn find_blocks(parent: TsNode<'_>, flavor: Flavor, source: &str, line_count: usize) -> Vec<Block> {
    let mut cursor = parent.walk();
    let mut blocks = Vec::new();
    for child in parent.named_children(&mut cursor) {
        let structural = block_identity(child, flavor, source);
        if let Some((kind, name, structural_node)) = structural {
            let syntax_start = structural_node.start_position().row;
            let end_pos = structural_node.end_position();
            let end = (end_pos.row + usize::from(end_pos.column > 0)).min(line_count);
            let container = body_node(structural_node, flavor).unwrap_or(structural_node);
            let body_start = if container.start_position().row == syntax_start {
                (syntax_start + 1).min(end)
            } else {
                container.start_position().row
            };
            let children = if matches!(flavor, Flavor::Python)
                && actual_kind(structural_node) == "function_definition"
            {
                Vec::new()
            } else {
                find_blocks(container, flavor, source, line_count)
            };
            let class_docstring = matches!(flavor, Flavor::Python)
                && actual_kind(structural_node) == "class_definition"
                && python_suite_starts_with_string(container);
            blocks.push(Block {
                kind,
                name,
                syntax_start,
                body_start,
                start: syntax_start,
                end,
                class_docstring,
                children,
            });
        }
    }
    blocks
}

fn block_identity<'a>(
    node: TsNode<'a>,
    flavor: Flavor,
    source: &str,
) -> Option<(String, String, TsNode<'a>)> {
    let actual = if matches!(flavor, Flavor::Python) && node.kind() == "decorated_definition" {
        node.child_by_field_name("definition").unwrap_or(node)
    } else {
        node
    };
    let kind = match (flavor, actual.kind()) {
        (Flavor::Python, "class_definition") => "class",
        (Flavor::Python, "function_definition") => "def",
        (Flavor::Rust, "function_item") => "fn",
        (Flavor::Rust, "struct_item") => "struct",
        (Flavor::Rust, "enum_item") => "enum",
        (Flavor::Rust, "trait_item") => "trait",
        (Flavor::Rust, "impl_item") => "impl",
        (Flavor::Rust, "mod_item") => "mod",
        (Flavor::Rust, "macro_definition") => "macro",
        _ => return None,
    };
    if matches!(flavor, Flavor::Rust) && !leo_rust_block(actual, source) {
        return None;
    }
    let name = if actual.kind() == "impl_item" {
        impl_name(actual, source)
    } else {
        actual
            .child_by_field_name("name")
            .and_then(|name| name.utf8_text(source.as_bytes()).ok())
            .unwrap_or("unnamed")
            .to_owned()
    };
    Some((kind.to_owned(), name, node))
}

/// Leo's Rust importer recognizes only declarations whose opening-brace line
/// ends in `{`. In particular, it deliberately does not split semicolon items
/// or one-line `{}` bodies.
fn leo_rust_block(node: TsNode<'_>, source: &str) -> bool {
    let Some(body) = node.child_by_field_name("body") else {
        return false;
    };
    let row = body.start_position().row;
    source
        .lines()
        .nth(row)
        .is_some_and(|line| line.trim_end().ends_with('{'))
}

fn impl_name(node: TsNode<'_>, source: &str) -> String {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("impl");
    text.split('{')
        .next()
        .unwrap_or(text)
        .trim_start_matches("impl")
        .trim()
        .to_owned()
}

fn body_node(node: TsNode<'_>, flavor: Flavor) -> Option<TsNode<'_>> {
    let node = if matches!(flavor, Flavor::Python) && node.kind() == "decorated_definition" {
        node.child_by_field_name("definition").unwrap_or(node)
    } else {
        node
    };
    node.child_by_field_name("body")
}

fn actual_kind(node: TsNode<'_>) -> &str {
    if node.kind() == "decorated_definition" {
        node.child_by_field_name("definition")
            .map_or(node.kind(), |definition| definition.kind())
    } else {
        node.kind()
    }
}

fn python_suite_starts_with_string(body: TsNode<'_>) -> bool {
    let mut cursor = body.walk();
    body.named_children(&mut cursor).next().is_some_and(|node| {
        node.kind() == "expression_statement"
            && node
                .named_child(0)
                .is_some_and(|child| child.kind() == "string")
    })
}

fn assign_owned_starts(blocks: &mut [Block], region_start: usize) {
    let mut previous_end = region_start;
    for block in blocks {
        block.start = previous_end;
        assign_owned_starts(&mut block.children, block.body_start);
        if block.class_docstring
            && let Some(first) = block.children.first_mut()
        {
            first.start = first.syntax_start;
        }
        previous_end = block.end;
    }
}

struct Builder<'a> {
    nodes: &'a mut HashMap<NodeId, Node>,
    locations: &'a mut HashMap<NodeId, usize>,
    root: &'a NodeId,
    source: &'a str,
    starts: &'a [usize],
}

impl Builder<'_> {
    fn build(
        &mut self,
        blocks: &[Block],
        remove_indent: &str,
        parent_class: Option<&str>,
    ) -> Vec<Position> {
        blocks
            .iter()
            .map(|block| {
                let id = NodeId(format!(
                    "{}::auto:{}:{}:{}",
                    self.root.0, block.syntax_start, block.end, block.kind
                ));
                let child_indent = common_child_indent(&block.children, self.source, self.starts);
                let children = self.build(
                    &block.children,
                    &child_indent,
                    (block.kind == "class")
                        .then_some(block.name.as_str())
                        .or(parent_class),
                );
                let raw_body = if block.children.is_empty() {
                    slice_lines(self.source, self.starts, block.start, block.end).to_owned()
                } else {
                    let first = block.children.first().unwrap().start;
                    let last = block.children.last().unwrap().end;
                    let mut value =
                        slice_lines(self.source, self.starts, block.start, first).to_owned();
                    value.push_str(&child_indent);
                    value.push_str("@others\n");
                    value.push_str(slice_lines(self.source, self.starts, last, block.end));
                    value
                };
                let body = dedent(&raw_body, remove_indent);
                self.nodes.insert(
                    id.clone(),
                    Node {
                        id: id.clone(),
                        headline: headline(block, parent_class),
                        body,
                        vnode_attributes: HashMap::new(),
                        tnode_attributes: HashMap::new(),
                    },
                );
                self.locations.insert(id.clone(), block.syntax_start + 1);
                Position { node: id, children }
            })
            .collect()
    }
}

fn headline(block: &Block, parent_class: Option<&str>) -> String {
    if block.kind != "def" {
        return format!("{} {}", block.kind, block.name);
    }
    parent_class.map_or_else(
        || format!("function: {}", block.name),
        |class| format!("{class}.{}", block.name),
    )
}

fn common_child_indent(blocks: &[Block], source: &str, starts: &[usize]) -> String {
    blocks
        .iter()
        .filter_map(|block| {
            let start = line_offset(source, starts, block.syntax_start);
            let end = line_offset(source, starts, block.syntax_start + 1);
            source.get(start..end)
        })
        .map(|line| {
            line.chars()
                .take_while(|c| matches!(c, ' ' | '\t'))
                .collect::<String>()
        })
        .min_by_key(String::len)
        .unwrap_or_default()
}

fn dedent(text: &str, indent: &str) -> String {
    if indent.is_empty() {
        return text.to_owned();
    }
    text.split_inclusive('\n')
        .map(|line| line.strip_prefix(indent).unwrap_or(line))
        .collect()
}

fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(source.match_indices('\n').map(|(index, _)| index + 1));
    starts
}

fn slice_lines<'a>(source: &'a str, starts: &[usize], start: usize, end: usize) -> &'a str {
    &source[line_offset(source, starts, start)..line_offset(source, starts, end)]
}

fn line_offset(source: &str, starts: &[usize], line: usize) -> usize {
    starts.get(line).copied().unwrap_or(source.len())
}

fn referenced_nodes(positions: &[Position]) -> std::collections::HashSet<NodeId> {
    fn visit(positions: &[Position], result: &mut std::collections::HashSet<NodeId>) {
        for position in positions {
            result.insert(position.node.clone());
            visit(&position.children, result);
        }
    }
    let mut result = std::collections::HashSet::new();
    visit(positions, &mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_nested_python_blocks() {
        let source = "import os\n\nclass C:\n    \"\"\"docs\"\"\"\n    def one(self):\n        return 1\n\ndef top():\n    return 2\n";
        let auto = AutoFile::parse(Path::new("x.py"), NodeId::from("root"), source).unwrap();
        let root = &auto.outline.roots[0];
        assert_eq!(root.children.len(), 2);
        assert_eq!(
            auto.outline.nodes[&root.children[0].node].headline,
            "class C"
        );
        assert_eq!(root.children[0].children.len(), 1);
        assert_eq!(
            auto.outline.nodes[&root.node].body,
            "import os\n\n@others\n@language python\n@tabwidth -4\n"
        );
        assert_eq!(
            auto.outline.nodes[&root.children[0].node].body,
            "class C:\n    \"\"\"docs\"\"\"\n    @others\n\n"
        );
        assert_eq!(
            auto.outline.nodes[&root.children[0].children[0].node].headline,
            "C.one"
        );
        assert_eq!(
            auto.outline.nodes[&root.children[0].children[0].node].body,
            "def one(self):\n    return 1\n"
        );
        assert_eq!(
            auto.outline.nodes[&root.children[1].node].headline,
            "function: top"
        );
    }

    #[test]
    fn expands_rust_items_and_impl_methods() {
        let source = "use std::fmt;\n\npub struct S;\n\nimpl S {\n    fn f(&self) {}\n}\n";
        let auto = AutoFile::parse(Path::new("x.rs"), NodeId::from("root"), source).unwrap();
        let root = &auto.outline.roots[0];
        assert_eq!(
            auto.outline.nodes[&root.children[0].node].headline,
            "impl S"
        );
        assert!(root.children[0].children.is_empty());
        assert_eq!(
            auto.outline.nodes[&root.node].body,
            "use std::fmt;\n\n@others\n@language rust\n@tabwidth -4\n"
        );
        assert_eq!(
            auto.outline.nodes[&root.children[0].node].body,
            "pub struct S;\n\nimpl S {\n    fn f(&self) {}\n}\n"
        );
    }
}
