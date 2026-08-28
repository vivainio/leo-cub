//! Transient expansion of Leo `@auto` source files.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use streaming_iterator::StreamingIterator;
use thiserror::Error;
use tree_sitter::{Language, Node as TsNode, Parser, Query, QueryCursor};

use crate::{Node, NodeId, Outline, Position, referenced_nodes};

#[derive(Debug, Error)]
pub enum AutoError {
    #[error("no Tree-sitter auto importer for {0}")]
    Unsupported(String),
    #[error("could not load the {0} Tree-sitter grammar")]
    Grammar(&'static str),
    #[error("Tree-sitter did not produce a syntax tree")]
    Parse,
    #[error("invalid {0} outline query: {1}")]
    Query(&'static str, String),
    #[error("{0}")]
    Dir(String),
}

/// An in-memory `@auto` expansion. It is never serialized into the `.leo` file.
pub struct AutoFile {
    pub outline: Outline,
    pub root: NodeId,
    /// One-based source line for each generated node.
    pub locations: HashMap<NodeId, usize>,
    /// The on-disk file each generated node came from, when it differs from
    /// the single file a plain `@auto` job read (`job.path`) -- set by
    /// [`crate::auto_dir::parse_dir`], whose per-node source spans several
    /// files rather than one. `None` for an ordinary single-file `@auto`.
    pub file_paths: Option<HashMap<NodeId, PathBuf>>,
}

#[derive(Clone, Copy)]
enum Flavor {
    Markdown,
    Python,
    Rust,
    /// Rhai has no Tree-sitter grammar of its own, but its `fn` declarations
    /// parse cleanly under Rust's grammar (Rhai is a syntactic subset for
    /// that one construct), so this flavor reuses [`tree_sitter_rust`] and
    /// [`block_identity`] restricts recognition to `function_item` only --
    /// Rhai has no structs/traits/impls/mods for the other Rust kinds to
    /// (mis)match against.
    Rhai,
    Static(&'static StaticLanguage),
}

struct StaticLanguage {
    query: &'static str,
    prefixes: &'static [(&'static str, &'static str)],
    /// Block kinds (post-`prefixes`/`leo.kind` resolution) whose headline
    /// should also show a trivial value, when the block's content is just
    /// one plain-text child (no nested elements) -- e.g. `param diagnose :=
    /// true` for `<xsl:param name="diagnose">true</xsl:param>`. Empty for
    /// languages where declarations aren't XML elements with text content.
    trivial_value_kinds: &'static [&'static str],
}

#[derive(Debug)]
struct StaticMatch {
    name: String,
    /// Overrides `config.prefixes`-derived kind when the query itself
    /// determines the block kind (e.g. an XML tag name), rather than the
    /// language having one Tree-sitter node kind per declaration form.
    kind: Option<String>,
    /// The XML attribute `name` was taken from (e.g. `match`, `name`,
    /// `test`). `find_static_blocks` uses this to relabel `match`-derived
    /// blocks, since a match pattern isn't a name the way `name`/`test` are.
    attr: Option<String>,
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

#[derive(Debug)]
struct MarkdownEvent {
    level: usize,
    name: String,
    start: usize,
    body_start: usize,
    noheader: bool,
}

#[derive(Debug)]
struct MarkdownBlock {
    level: usize,
    name: String,
    body: String,
    line: usize,
    children: Vec<usize>,
}

impl AutoFile {
    pub fn parse(path: &Path, root: NodeId, source: &str) -> Result<Self, AutoError> {
        Self::parse_with_directive(path, root, source, None)
    }

    pub fn parse_with_directive(
        path: &Path,
        root: NodeId,
        source: &str,
        directive: Option<&str>,
    ) -> Result<Self, AutoError> {
        let (flavor, language, language_name) =
            if matches!(directive, Some("@auto-md" | "@auto-markdown")) {
                (Flavor::Markdown, tree_sitter_md::LANGUAGE.into(), "md")
            } else {
                match language_for(path) {
                    Ok(language) => language,
                    Err(AutoError::Unsupported(_)) => return Ok(parse_plain(root, source)),
                    Err(error) => return Err(error),
                }
            };
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .map_err(|_| AutoError::Grammar(language_name))?;
        let tree = parser.parse(source, None).ok_or(AutoError::Parse)?;
        if matches!(flavor, Flavor::Markdown) {
            return parse_markdown(tree.root_node(), root, source);
        }
        if let Flavor::Static(config) = flavor {
            return parse_static(
                tree.root_node(),
                root,
                source,
                &language,
                language_name,
                config,
            );
        }
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
            file_paths: None,
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

/// Leo's fallback for an `@auto` file without an importer: keep the complete
/// external file in the root node instead of inventing an outline structure.
fn parse_plain(root: NodeId, source: &str) -> AutoFile {
    let mut outline = Outline::default();
    outline.nodes.insert(
        root.clone(),
        Node {
            id: root.clone(),
            headline: String::new(),
            body: source.to_owned(),
            vnode_attributes: HashMap::new(),
            tnode_attributes: HashMap::new(),
        },
    );
    outline.roots.push(Position {
        node: root.clone(),
        children: Vec::new(),
    });
    AutoFile {
        outline,
        root,
        locations: HashMap::new(),
        file_paths: None,
    }
}

fn parse_static(
    syntax_root: TsNode<'_>,
    root: NodeId,
    source: &str,
    language: &Language,
    language_name: &'static str,
    config: &'static StaticLanguage,
) -> Result<AutoFile, AutoError> {
    let query = Query::new(language, config.query)
        .map_err(|error| AutoError::Query(language_name, error.to_string()))?;
    let capture_names = query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, syntax_root, source.as_bytes());
    let mut structural = HashMap::new();
    while let Some(query_match) = matches.next() {
        let mut node = None;
        let mut name = None;
        let mut kind = None;
        let mut attr = None;
        for capture in query_match.captures {
            match capture_names[capture.index as usize] {
                "leo.node" => node = Some(capture.node),
                "leo.name" => name = capture.node.utf8_text(source.as_bytes()).ok().map(unquote),
                "leo.kind" => {
                    kind = capture
                        .node
                        .utf8_text(source.as_bytes())
                        .ok()
                        .map(local_name)
                }
                "leo.attr" => {
                    attr = capture
                        .node
                        .utf8_text(source.as_bytes())
                        .ok()
                        .map(str::to_owned)
                }
                _ => {}
            }
        }
        if let (Some(node), Some(name)) = (node, name) {
            structural.insert(
                (node.start_byte(), node.end_byte()),
                StaticMatch { name, kind, attr },
            );
        }
    }

    let starts = line_starts(source);
    let line_count = starts.len();
    let mut blocks = find_static_blocks(syntax_root, source, line_count, config, &structural);
    assign_owned_starts(&mut blocks, 0);
    let preamble_end = blocks.first().map_or(0, |block| block.syntax_start);
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
        starts: &starts,
    }
    .build(&blocks, "", None);
    let mut body = if children.is_empty() {
        source.to_owned()
    } else {
        let tail_start = blocks.last().map_or(0, |block| block.end);
        let mut body = slice_lines(source, &starts, 0, preamble_end).to_owned();
        body.push_str("@others\n");
        body.push_str(slice_lines(source, &starts, tail_start, line_count));
        body
    };
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
    Ok(AutoFile {
        outline,
        root,
        locations,
        file_paths: None,
    })
}

fn find_static_blocks(
    parent: TsNode<'_>,
    source: &str,
    line_count: usize,
    config: &StaticLanguage,
    structural: &HashMap<(usize, usize), StaticMatch>,
) -> Vec<Block> {
    let mut cursor = parent.walk();
    let mut blocks = Vec::new();
    for child in parent.named_children(&mut cursor) {
        if let Some(item) = structural.get(&(child.start_byte(), child.end_byte())) {
            let syntax_start = child.start_position().row;
            let end_pos = child.end_position();
            let end = (end_pos.row + usize::from(end_pos.column > 0)).min(line_count);
            let container = child.child_by_field_name("body").unwrap_or(child);
            let children = find_static_blocks(container, source, line_count, config, structural);
            // The body's opening delimiter belongs to the declaration node,
            // including languages such as C# that put `{` on its own line. But
            // when a nested block shares that same line (e.g. a single-line
            // `class C { m() {} }`), skipping to the next line would run past
            // the nested block's own content, leaving it with an empty body;
            // anchor to the first nested block's actual line instead.
            let body_start = children
                .first()
                .map_or((container.start_position().row + 1).min(end), |first| {
                    first.syntax_start
                });
            // A `match` pattern isn't a name the way `name`/`test` are (an
            // `xsl:template match="foo"` isn't a template *named* foo), so
            // label those blocks by the attribute itself rather than the tag.
            let kind = if item.attr.as_deref() == Some("match") {
                "match".to_owned()
            } else {
                item.kind.clone().unwrap_or_else(|| {
                    config
                        .prefixes
                        .iter()
                        .find_map(|(kind, prefix)| (*kind == child.kind()).then_some(*prefix))
                        .unwrap_or(child.kind())
                        .to_owned()
                })
            };
            let name = if children.is_empty() && config.trivial_value_kinds.contains(&kind.as_str())
            {
                xml_trivial_value(child, source).map_or_else(
                    || item.name.clone(),
                    |value| format!("{} := {value}", item.name),
                )
            } else {
                item.name.clone()
            };
            blocks.push(Block {
                kind,
                name,
                syntax_start,
                body_start,
                start: syntax_start,
                end,
                class_docstring: false,
                children,
            });
        } else {
            blocks.extend(find_static_blocks(
                child, source, line_count, config, structural,
            ));
        }
    }
    blocks
}

fn parse_markdown(
    syntax_root: TsNode<'_>,
    root: NodeId,
    source: &str,
) -> Result<AutoFile, AutoError> {
    let starts = line_starts(source);
    let mut events = Vec::new();
    collect_markdown_events(syntax_root, source, &starts, &mut events);
    collect_leo_markdown_extensions(source, &mut events);
    events.sort_by_key(|event| event.start);
    events.dedup_by_key(|event| event.start);

    let mut blocks = Vec::<MarkdownBlock>::new();
    let mut roots = Vec::new();
    let mut stack = Vec::<usize>::new();
    if !source.is_empty() && events.first().is_none_or(|event| event.start > 0) {
        let end = events.first().map_or(starts.len(), |event| event.start);
        blocks.push(MarkdownBlock {
            level: 1,
            name: "!Declarations".to_owned(),
            body: slice_lines(source, &starts, 0, end).to_owned(),
            line: 1,
            children: vec![],
        });
        roots.push(0);
    }
    for (event_index, event) in events.iter().enumerate() {
        let body_end = events
            .get(event_index + 1)
            .map_or(starts.len(), |next| next.start);
        stack.truncate(event.level.saturating_sub(1));
        while stack.len() < event.level.saturating_sub(1) {
            let level = stack.len() + 1;
            let placeholder = blocks.len();
            blocks.push(MarkdownBlock {
                level,
                name: format!("placeholder level {level}"),
                body: String::new(),
                line: event.start + 1,
                children: vec![],
            });
            attach_markdown_block(&mut blocks, &mut roots, &stack, placeholder);
            stack.push(placeholder);
        }
        let index = blocks.len();
        let mut body = if event.noheader {
            String::from("@noheader\n")
        } else {
            String::new()
        };
        body.push_str(slice_lines(source, &starts, event.body_start, body_end));
        blocks.push(MarkdownBlock {
            level: event.level,
            name: event.name.clone(),
            body,
            line: event.start + 1,
            children: vec![],
        });
        attach_markdown_block(&mut blocks, &mut roots, &stack, index);
        stack.push(index);
    }

    let mut outline = Outline::default();
    let mut locations = HashMap::new();
    let positions = roots
        .into_iter()
        .map(|index| markdown_position(index, &blocks, &root, &mut outline.nodes, &mut locations))
        .collect();
    outline.nodes.insert(
        root.clone(),
        Node {
            id: root.clone(),
            headline: String::new(),
            body: "@language md\n@tabwidth -4\n".to_owned(),
            vnode_attributes: HashMap::new(),
            tnode_attributes: HashMap::new(),
        },
    );
    outline.roots.push(Position {
        node: root.clone(),
        children: positions,
    });
    Ok(AutoFile {
        outline,
        root,
        locations,
        file_paths: None,
    })
}

/// Supplement CommonMark's syntax tree with Leo-specific Markdown rules.
/// Leo accepts `#Heading` without a separating space and intentionally treats
/// only backtick fences as code fences.
fn collect_leo_markdown_extensions(source: &str, events: &mut Vec<MarkdownEvent>) {
    let lines = source.split_inclusive('\n').collect::<Vec<_>>();
    let mut in_backtick_fence = false;
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if line.starts_with("```") {
            in_backtick_fence = !in_backtick_fence;
            index += 1;
            continue;
        }
        if in_backtick_fence {
            index += 1;
            continue;
        }
        if let Some((level, name)) = noheader_marker(line) {
            events.push(MarkdownEvent {
                level,
                name,
                start: index,
                body_start: index + 1,
                noheader: true,
            });
        } else {
            let hashes = line.bytes().take_while(|byte| *byte == b'#').count();
            let name = line[hashes..].trim();
            if line.ends_with('\n') && hashes > 0 && !name.is_empty() {
                events.push(MarkdownEvent {
                    level: hashes,
                    name: name.to_owned(),
                    start: index,
                    body_start: index + 1,
                    noheader: false,
                });
            } else if index + 1 < lines.len() {
                let underline = lines[index + 1].trim();
                if !line.trim().is_empty()
                    && underline.len() >= 4
                    && (underline.bytes().all(|byte| byte == b'=')
                        || underline.bytes().all(|byte| byte == b'-'))
                {
                    events.push(MarkdownEvent {
                        level: usize::from(underline.starts_with('-')) + 1,
                        name: line.trim().to_owned(),
                        start: index,
                        body_start: index + 2,
                        noheader: false,
                    });
                    index += 1;
                }
            }
        }
        index += 1;
    }
}

fn attach_markdown_block(
    blocks: &mut [MarkdownBlock],
    roots: &mut Vec<usize>,
    stack: &[usize],
    index: usize,
) {
    if let Some(parent) = stack.last() {
        blocks[*parent].children.push(index);
    } else {
        roots.push(index);
    }
}

fn markdown_position(
    index: usize,
    blocks: &[MarkdownBlock],
    root: &NodeId,
    nodes: &mut HashMap<NodeId, Node>,
    locations: &mut HashMap<NodeId, usize>,
) -> Position {
    let block = &blocks[index];
    let id = NodeId(format!(
        "{}::auto-md:{}:{}",
        root.0, block.line, block.level
    ));
    let children = block
        .children
        .iter()
        .map(|child| markdown_position(*child, blocks, root, nodes, locations))
        .collect();
    nodes.insert(
        id.clone(),
        Node {
            id: id.clone(),
            headline: block.name.clone(),
            body: block.body.clone(),
            vnode_attributes: HashMap::new(),
            tnode_attributes: HashMap::new(),
        },
    );
    locations.insert(id.clone(), block.line);
    Position { node: id, children }
}

fn collect_markdown_events(
    node: TsNode<'_>,
    source: &str,
    starts: &[usize],
    events: &mut Vec<MarkdownEvent>,
) {
    if node.kind() == "fenced_code_block" || node.start_position().column > 0 {
        return;
    }
    match node.kind() {
        "atx_heading" => {
            let row = node.start_position().row;
            let line = slice_lines(source, starts, row, row + 1);
            if line.ends_with('\n') {
                let hashes = line.bytes().take_while(|byte| *byte == b'#').count();
                let name = line[hashes..].trim().to_owned();
                if hashes > 0 && !name.is_empty() {
                    events.push(MarkdownEvent {
                        level: hashes,
                        name,
                        start: row,
                        body_start: node.end_position().row,
                        noheader: false,
                    });
                }
            }
            return;
        }
        "setext_heading" => {
            let row = node.start_position().row;
            let underline_row = node.end_position().row.saturating_sub(1);
            let title = slice_lines(source, starts, row, row + 1).trim();
            let underline = slice_lines(source, starts, underline_row, underline_row + 1).trim();
            if underline.len() >= 4
                && !title.is_empty()
                && underline.bytes().all(|byte| matches!(byte, b'=' | b'-'))
            {
                events.push(MarkdownEvent {
                    level: usize::from(underline.starts_with('-')) + 1,
                    name: title.to_owned(),
                    start: row,
                    body_start: underline_row + 1,
                    noheader: false,
                });
            }
            return;
        }
        "html_block" => {
            let row = node.start_position().row;
            let text = slice_lines(source, starts, row, node.end_position().row);
            if let Some((level, name)) = noheader_marker(text) {
                events.push(MarkdownEvent {
                    level,
                    name,
                    start: row,
                    body_start: node.end_position().row,
                    noheader: true,
                });
            }
            return;
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_markdown_events(child, source, starts, events);
    }
}

fn noheader_marker(text: &str) -> Option<(usize, String)> {
    let text = text.trim();
    let inner = text.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
    let rest = inner.strip_prefix("leo-noheader level=")?;
    let (level, headline) = rest.split_once(" headline=")?;
    Some((level.parse().ok()?, percent_decode(headline.trim())))
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
        {
            result.push(high * 16 + low);
            index += 3;
        } else {
            result.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn root_preamble_end(flavor: Flavor, source: &str, blocks: &[Block]) -> usize {
    let Some(first) = blocks.first() else {
        return 0;
    };
    if matches!(flavor, Flavor::Python) {
        return first.syntax_start;
    }
    // Rhai has no `use` statement -- `import ... as ...;` and top-level
    // `const` declarations (see e.g. scripts/github.rhai's `COMMANDS`) play
    // the same "preamble, not a block" role instead.
    let import_prefixes: &[&str] = if matches!(flavor, Flavor::Rhai) {
        &["import ", "const "]
    } else {
        &["use "]
    };
    let mut found_use = false;
    let mut end = 0;
    // A multi-line `use std::{ ... };` (or Rhai's `const X = #{ ... };`)
    // continues past its own opening line -- track brace balance so the
    // scan doesn't stop partway through one and strand the rest of it as
    // a stray prefix glued onto whatever the first real block turns out
    // to be, instead of staying part of the preamble.
    let mut brace_depth: i32 = 0;
    for (index, line) in source.lines().take(first.syntax_start).enumerate() {
        let line = line.trim();
        if brace_depth > 0 {
            brace_depth += brace_delta(line);
            end = index + 1;
        } else if import_prefixes
            .iter()
            .any(|prefix| line.starts_with(prefix))
        {
            found_use = true;
            end = index + 1;
            brace_depth += brace_delta(line);
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

fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |depth, ch| match ch {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

fn move_leading_blank_lines(outline: &mut Outline) {
    fn visit(nodes: &mut HashMap<NodeId, Node>, positions: &[Position]) {
        for index in 1..positions.len() {
            let current = positions[index].node.clone();
            let previous = positions[index - 1].node.clone();
            while let Some(first_len) = nodes[&current]
                .body
                .split_inclusive('\n')
                .next()
                .filter(|line| line.trim().is_empty())
                .map(str::len)
            {
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

const C_SHARP: StaticLanguage = StaticLanguage {
    query: r#"
        (namespace_declaration name: (_) @leo.name) @leo.node
        (class_declaration name: (_) @leo.name) @leo.node
        (struct_declaration name: (_) @leo.name) @leo.node
        (interface_declaration name: (_) @leo.name) @leo.node
        (enum_declaration name: (_) @leo.name) @leo.node
        (record_declaration name: (_) @leo.name) @leo.node
        (method_declaration name: (_) @leo.name) @leo.node
        (constructor_declaration name: (_) @leo.name) @leo.node
    "#,
    prefixes: &[
        ("namespace_declaration", "namespace"),
        ("class_declaration", "class"),
        ("struct_declaration", "struct"),
        ("interface_declaration", "interface"),
        ("enum_declaration", "enum"),
        ("record_declaration", "record"),
        ("method_declaration", "method"),
        ("constructor_declaration", "constructor"),
    ],
    trivial_value_kinds: &[],
};

const JAVASCRIPT: StaticLanguage = StaticLanguage {
    query: r#"
        (class_declaration name: (_) @leo.name) @leo.node
        (function_declaration name: (_) @leo.name) @leo.node
        (generator_function_declaration name: (_) @leo.name) @leo.node
        (method_definition name: (_) @leo.name) @leo.node
    "#,
    prefixes: &[
        ("class_declaration", "class"),
        ("function_declaration", "function"),
        ("generator_function_declaration", "function"),
        ("method_definition", "method"),
    ],
    trivial_value_kinds: &[],
};

const TYPESCRIPT: StaticLanguage = StaticLanguage {
    query: r#"
        (class_declaration name: (_) @leo.name) @leo.node
        (abstract_class_declaration name: (_) @leo.name) @leo.node
        (interface_declaration name: (_) @leo.name) @leo.node
        (enum_declaration name: (_) @leo.name) @leo.node
        (type_alias_declaration name: (_) @leo.name) @leo.node
        (internal_module name: (_) @leo.name) @leo.node
        (function_declaration name: (_) @leo.name) @leo.node
        (generator_function_declaration name: (_) @leo.name) @leo.node
        (method_definition name: (_) @leo.name) @leo.node
    "#,
    prefixes: &[
        ("class_declaration", "class"),
        ("abstract_class_declaration", "abstract class"),
        ("interface_declaration", "interface"),
        ("enum_declaration", "enum"),
        ("type_alias_declaration", "type"),
        ("internal_module", "namespace"),
        ("function_declaration", "function"),
        ("generator_function_declaration", "function"),
        ("method_definition", "method"),
    ],
    trivial_value_kinds: &[],
};

const GO: StaticLanguage = StaticLanguage {
    query: r#"
        (type_spec name: (_) @leo.name) @leo.node
        (function_declaration name: (_) @leo.name) @leo.node
        (method_declaration name: (_) @leo.name) @leo.node
    "#,
    prefixes: &[
        ("type_spec", "type"),
        ("function_declaration", "func"),
        ("method_declaration", "method"),
    ],
    trivial_value_kinds: &[],
};

/// Strips an XML namespace prefix (`xsl:template` -> `template`) so tag
/// names read like the other languages' declaration keywords.
fn local_name(qualified: &str) -> String {
    qualified.rsplit(':').next().unwrap_or(qualified).to_owned()
}

/// For a `param`/`variable`-like element whose entire content is one run of
/// plain text (no nested elements, comments, or processing instructions),
/// returns that text normalized to one line -- e.g. the `true` in
/// `<xsl:param name="diagnose">true</xsl:param>`. `None` for anything with
/// element content (a complex default that can't sit on a headline).
fn xml_trivial_value(node: TsNode<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let content = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == "content")?;
    let mut inner = content.walk();
    let mut named = content.named_children(&mut inner);
    let only = named.next()?;
    if named.next().is_some() || only.kind() != "CharData" {
        return None;
    }
    let text = only.utf8_text(source.as_bytes()).ok()?;
    let value = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!value.is_empty()).then_some(value)
}

/// Strips the surrounding quotes Tree-sitter's XML grammar keeps on
/// `AttValue` nodes, so an attribute value reads as plain text. XSLT
/// expressions (`test`, `match`, `select`, ...) are also often hand-wrapped
/// across source lines with heavy indentation, which would otherwise leak
/// into the headline as a multi-line value full of stray whitespace; collapse
/// all whitespace runs to single spaces so the headline stays one line.
fn unquote(value: &str) -> String {
    let inner = value
        .strip_prefix('"')
        .or_else(|| value.strip_prefix('\''))
        .unwrap_or(value);
    let inner = inner
        .strip_suffix('"')
        .or_else(|| inner.strip_suffix('\''))
        .unwrap_or(inner);
    inner.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// XSLT stylesheets are XML, so Tree-sitter's generic XML grammar has no
/// dedicated node kind per element (everything is `element`); the block
/// kind and name instead come straight from the query, keyed off whichever
/// declaration-identifying attribute (`name`, `match`, `test`, `id`) the
/// matched element carries. Elements without one of those attributes (e.g.
/// literal result elements, `<xsl:otherwise>`) are left as plain body text.
const XSLT: StaticLanguage = StaticLanguage {
    query: r#"
        (
          (element
            (STag
              (Name) @leo.kind
              (Attribute
                (Name) @leo.attr
                (AttValue) @leo.name
              )
            )
          ) @leo.node
          (#any-of? @leo.attr "name" "match" "test" "id")
        )
    "#,
    prefixes: &[],
    trivial_value_kinds: &["param", "variable"],
};

fn language_for(path: &Path) -> Result<(Flavor, Language, &'static str), AutoError> {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "py" | "pyw" => Ok((
            Flavor::Python,
            tree_sitter_python::LANGUAGE.into(),
            "python",
        )),
        "md" | "rmd" => Ok((Flavor::Markdown, tree_sitter_md::LANGUAGE.into(), "md")),
        "rs" => Ok((Flavor::Rust, tree_sitter_rust::LANGUAGE.into(), "rust")),
        "rhai" => Ok((Flavor::Rhai, tree_sitter_rust::LANGUAGE.into(), "rhai")),
        "cs" => Ok((
            Flavor::Static(&C_SHARP),
            tree_sitter_c_sharp::LANGUAGE.into(),
            "csharp",
        )),
        "go" => Ok((Flavor::Static(&GO), tree_sitter_go::LANGUAGE.into(), "go")),
        "xsl" | "xslt" => Ok((
            Flavor::Static(&XSLT),
            tree_sitter_xml::LANGUAGE_XML.into(),
            "xslt",
        )),
        "js" | "jsx" | "mjs" | "cjs" => Ok((
            Flavor::Static(&JAVASCRIPT),
            tree_sitter_javascript::LANGUAGE.into(),
            "javascript",
        )),
        "ts" | "mts" | "cts" => Ok((
            Flavor::Static(&TYPESCRIPT),
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            "typescript",
        )),
        "tsx" => Ok((
            Flavor::Static(&TYPESCRIPT),
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            "typescript",
        )),
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
        (Flavor::Rhai, "function_item") => "fn",
        _ => return None,
    };
    if matches!(flavor, Flavor::Rust | Flavor::Rhai) && !leo_rust_block(actual, source) {
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

/// Leo's Rust importer (and, via [`Flavor::Rhai`], Rhai's) recognizes only
/// declarations whose opening-brace line ends in `{`. In particular, it
/// deliberately does not split semicolon items or one-line `{}` bodies.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(auto: &AutoFile) -> Vec<(usize, String, String)> {
        fn visit(
            outline: &Outline,
            positions: &[Position],
            depth: usize,
            rows: &mut Vec<(usize, String, String)>,
        ) {
            for position in positions {
                let node = &outline.nodes[&position.node];
                rows.push((depth, node.headline.clone(), node.body.clone()));
                visit(outline, &position.children, depth + 1, rows);
            }
        }
        let mut result = Vec::new();
        visit(&auto.outline, &auto.outline.roots, 0, &mut result);
        result
    }

    #[test]
    fn markdown_matches_leo_heading_hierarchy_and_bodies() {
        let source = "Decl line.\n# Header\n\nAfter header text\n\n## Subheader\n\nNot an underline\n\n----------------\n\nAfter subheader text\n\n# Last header: no text\n";
        let auto = AutoFile::parse(Path::new("x.md"), NodeId::from("root"), source).unwrap();
        assert_eq!(
            rows(&auto),
            vec![
                (0, String::new(), "@language md\n@tabwidth -4\n".into()),
                (1, "!Declarations".into(), "Decl line.\n".into()),
                (1, "Header".into(), "\nAfter header text\n\n".into()),
                (
                    2,
                    "Subheader".into(),
                    "\nNot an underline\n\n----------------\n\nAfter subheader text\n\n".into(),
                ),
                (1, "Last header: no text".into(), String::new()),
            ]
        );
    }

    #[test]
    fn unsupported_languages_fall_back_to_a_plain_root_node() {
        let source = "package:\n  name: leo\n";
        let auto = AutoFile::parse(Path::new("meta.yaml"), NodeId::from("root"), source).unwrap();
        assert_eq!(rows(&auto), vec![(0, String::new(), source.to_owned())]);
        assert!(auto.locations.is_empty());
    }

    #[test]
    fn query_languages_create_structural_outlines() {
        let cases = [
            (
                "x.cs",
                "namespace N { class C { C() {} void M() {} } }\n",
                vec!["namespace N", "class C", "constructor C", "method M"],
            ),
            (
                "x.js",
                "class C { m() {} }\nfunction f() {}\n",
                vec!["class C", "method m", "function f"],
            ),
            (
                "x.ts",
                "interface I { m(): void }\ntype T = string;\nfunction f(): void {}\n",
                vec!["interface I", "type T", "function f"],
            ),
            (
                "x.go",
                "package p\ntype S struct{}\nfunc (S) M() {}\nfunc F() {}\n",
                vec!["type S", "method M", "func F"],
            ),
            (
                "x.xslt",
                "<xsl:stylesheet version=\"1.0\">\n  <xsl:template match=\"/\">\n    <xsl:if test=\"@x\">y</xsl:if>\n  </xsl:template>\n  <xsl:function name=\"f\">\n    <xsl:param name=\"p\"/>\n  </xsl:function>\n</xsl:stylesheet>\n",
                vec!["match /", "if @x", "function f"],
            ),
        ];
        for (path, source, expected) in cases {
            let auto = AutoFile::parse(Path::new(path), NodeId::from("root"), source).unwrap();
            let headlines = rows(&auto)
                .into_iter()
                .skip(1)
                .map(|(_, headline, _)| headline)
                .collect::<Vec<_>>();
            assert_eq!(headlines, expected, "{path}");
        }
    }

    #[test]
    fn xslt_attribute_values_are_normalized_to_a_single_line_headline() {
        // Hand-wrapped, padded attribute values (common in real-world XSLT)
        // must not leak their raw whitespace or line breaks into a headline,
        // which is expected to render as one line.
        let source = "<xsl:stylesheet version=\"1.0\">\n  <xsl:template match=\"/\">\n    <xsl:if test=\" $terminate = 'assert' \">y</xsl:if>\n    <xsl:if test=\"doc-available(\n                      resolve-uri($x))\">y</xsl:if>\n  </xsl:template>\n</xsl:stylesheet>\n";
        let auto = AutoFile::parse(Path::new("x.xslt"), NodeId::from("root"), source).unwrap();
        let headlines = rows(&auto)
            .into_iter()
            .skip(1)
            .map(|(_, headline, _)| headline)
            .collect::<Vec<_>>();
        assert_eq!(
            headlines,
            vec![
                "match /",
                "if $terminate = 'assert'",
                "if doc-available( resolve-uri($x))",
            ]
        );
    }

    #[test]
    fn xslt_template_match_is_headlined_by_the_pattern_not_the_tag() {
        // `xsl:template match="foo"` isn't a template *named* foo -- "foo" is
        // a match pattern, not an identifier -- so it should read "match
        // foo". A `name=` template genuinely has a name, so it keeps
        // "template <name>".
        let source = "<xsl:stylesheet version=\"1.0\">\n  <xsl:template match=\"cd\">\n    <p/>\n  </xsl:template>\n  <xsl:template name=\"do-foo\">\n    <p/>\n  </xsl:template>\n</xsl:stylesheet>\n";
        let auto = AutoFile::parse(Path::new("x.xslt"), NodeId::from("root"), source).unwrap();
        let headlines = rows(&auto)
            .into_iter()
            .skip(1)
            .map(|(_, headline, _)| headline)
            .collect::<Vec<_>>();
        assert_eq!(headlines, vec!["match cd", "template do-foo"]);
    }

    #[test]
    fn xslt_trivial_param_and_variable_values_fold_into_the_headline() {
        let source = "<xsl:stylesheet version=\"1.0\">\n  <xsl:param name=\"diagnose\">true</xsl:param>\n  <xsl:variable name=\"x\"><b>bold</b></xsl:variable>\n  <xsl:param name=\"empty\"></xsl:param>\n  <xsl:template match=\"/\">\n    <p/>\n  </xsl:template>\n</xsl:stylesheet>\n";
        let auto = AutoFile::parse(Path::new("x.xslt"), NodeId::from("root"), source).unwrap();
        let headlines = rows(&auto)
            .into_iter()
            .skip(1)
            .map(|(_, headline, _)| headline)
            .collect::<Vec<_>>();
        assert_eq!(
            headlines,
            vec![
                "param diagnose := true",
                // Element content (not plain text) can't fold into a headline.
                "variable x",
                // Empty content has no trivial value to show.
                "param empty",
                "match /",
            ]
        );
    }

    #[test]
    fn nested_single_line_static_blocks_keep_their_body() {
        // A nested block sharing its declaration's line (e.g. a one-line
        // `class C { m() {} }`) used to compute its owned line range as
        // starting *after* that line, past its own content, leaving its
        // body completely empty even though the source clearly has one.
        // Line-based slicing can't perfectly split two constructs that
        // share one physical line, but it must not drop the nested one.
        let auto = AutoFile::parse(
            Path::new("x.js"),
            NodeId::from("root"),
            "class C { m() { return 1; } }\n",
        )
        .unwrap();
        let bodies: Vec<_> = rows(&auto)
            .into_iter()
            .skip(1)
            .map(|(_, headline, body)| (headline, body))
            .collect();
        let method = bodies
            .iter()
            .find(|(headline, _)| headline == "method m")
            .expect("method m should still be present");
        assert!(
            !method.1.is_empty(),
            "nested single-line block body should not be empty"
        );
        assert!(method.1.contains("return 1"));
    }

    #[test]
    fn markdown_matches_leo_fences_setext_and_noheader_nodes() {
        let source = "Top\n====\n\n```python\n# not a heading\n```\n<!-- leo-noheader level=2 headline=Hidden%20child -->\nbody\n#### Deep\ntail\n";
        let auto = AutoFile::parse(Path::new("x.Rmd"), NodeId::from("root"), source).unwrap();
        assert_eq!(
            rows(&auto),
            vec![
                (0, String::new(), "@language md\n@tabwidth -4\n".into()),
                (
                    1,
                    "Top".into(),
                    "\n```python\n# not a heading\n```\n".into()
                ),
                (2, "Hidden child".into(), "@noheader\nbody\n".into()),
                (3, "placeholder level 3".into(), String::new()),
                (4, "Deep".into(), "tail\n".into()),
            ]
        );
    }

    #[test]
    fn markdown_matches_leo_non_commonmark_headings_and_plain_documents() {
        let source = "#Header\nbody\n~~~\n#Heading inside tilde fence\n~~~\n";
        let auto = AutoFile::parse(Path::new("x.md"), NodeId::from("root"), source).unwrap();
        let result = rows(&auto);
        assert_eq!(result[1].1, "Header");
        assert_eq!(result[2].1, "Heading inside tilde fence");

        let plain = AutoFile::parse(
            Path::new("x.md"),
            NodeId::from("plain"),
            "just text\nmore text\n",
        )
        .unwrap();
        assert_eq!(
            rows(&plain),
            vec![
                (0, String::new(), "@language md\n@tabwidth -4\n".into()),
                (1, "!Declarations".into(), "just text\nmore text\n".into()),
            ]
        );
    }

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
    fn expands_rhai_functions_and_folds_const_declarations_into_the_preamble() {
        let source = "// header comment\nconst COMMANDS = [\"a\"];\n\n/// docs\nfn a(doc, target) {\n    doc.sh(\"x\");\n}\n\nfn helper(s) {\n    s\n}\n";
        let auto = AutoFile::parse(Path::new("x.rhai"), NodeId::from("root"), source).unwrap();
        let root = &auto.outline.roots[0];
        assert_eq!(auto.outline.nodes[&root.children[0].node].headline, "fn a");
        assert_eq!(
            auto.outline.nodes[&root.children[1].node].headline,
            "fn helper"
        );
        assert_eq!(
            auto.outline.nodes[&root.node].body,
            "// header comment\nconst COMMANDS = [\"a\"];\n\n/// docs\n@others\n@language rhai\n@tabwidth -4\n"
        );
    }

    #[test]
    fn folds_multiline_const_declarations_into_the_rhai_preamble() {
        // Same bug as `preserves_multiline_use_blocks_in_the_preamble`
        // below, for Rhai's "preamble, not a block" role: a brace-grouped
        // `const` continues past its own opening line.
        let source = "const COMMANDS = #{\n    a: \"one\",\n    b: \"two\",\n};\n\nfn helper(s) {\n    s\n}\n";
        let auto = AutoFile::parse(Path::new("x.rhai"), NodeId::from("root"), source).unwrap();
        let root = &auto.outline.roots[0];
        assert_eq!(
            auto.outline.nodes[&root.children[0].node].headline,
            "fn helper"
        );
        assert_eq!(
            auto.outline.nodes[&root.node].body,
            "const COMMANDS = #{\n    a: \"one\",\n    b: \"two\",\n};\n\n@others\n@language rhai\n@tabwidth -4\n"
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

    #[test]
    fn preserves_multiline_use_blocks_in_the_preamble() {
        // `root_preamble_end` used to stop the preamble scan at the first
        // line that didn't itself start with "use " -- which, for a
        // brace-grouped `use std::{ ... };`, is its very next line. The
        // rest of that statement then got glued onto whatever the first
        // real block happened to be, instead of staying in the preamble.
        let source = "use std::process::Command;\nuse std::{\n    fs,\n    path::PathBuf,\n};\n\npub fn f() {\n    let _ = 1;\n}\n\npub fn g() {\n    let _ = 2;\n}\n";
        let auto = AutoFile::parse(Path::new("x.rs"), NodeId::from("root"), source).unwrap();
        assert_eq!(
            auto.outline.nodes[&auto.root].body,
            "use std::process::Command;\nuse std::{\n    fs,\n    path::PathBuf,\n};\n\n@others\n@language rust\n@tabwidth -4\n"
        );
    }
}
