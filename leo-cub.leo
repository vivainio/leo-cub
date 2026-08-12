<?xml version="1.0" encoding="utf-8"?>
<!-- Created by Leo: https://leo-editor.github.io/leo-editor/leo_toc.html -->
<leo_file xmlns:leo="https://leo-editor.github.io/leo-editor/namespaces/leo-python-editor/1.1" >
<leo_header file_format="2"/>
<globals/>
<preferences/>
<find_panel_settings/>
<vnodes>
<v t="ekr.20260812052828.2"><vh>Startup</vh>
<v t="ekr.20260812052904.1"><vh> Recursive import script</vh></v>
</v>
<v t="ekr.20260812053055.2"><vh>leo-cub/src</vh>
<v t="ekr.20260812053055.3"><vh>@clean src/derived.rs</vh>
<v t="ekr.20260812053055.4"><vh>struct DerivedFile</vh></v>
<v t="ekr.20260812053055.5"><vh>enum SentinelError</vh></v>
<v t="ekr.20260812053055.6"><vh>impl DerivedFile</vh>
<v t="ekr.20260812053055.16"><vh>fn parse</vh></v>
<v t="ekr.20260812053055.17"><vh>fn merge_into</vh></v>
</v>
<v t="ekr.20260812053055.7"><vh>fn scan_header</vh></v>
<v t="ekr.20260812053055.8"><vh>fn parse_level</vh></v>
<v t="ekr.20260812053055.9"><vh>fn sentinel_content</vh></v>
<v t="ekr.20260812053055.10"><vh>fn leading_width</vh></v>
<v t="ekr.20260812053055.11"><vh>fn strip_indent</vh></v>
<v t="ekr.20260812053055.12"><vh>fn append_body</vh></v>
<v t="ekr.20260812053055.13"><vh>fn restore_expansion</vh></v>
<v t="ekr.20260812053055.14"><vh>fn children_at</vh></v>
<v t="ekr.20260812053055.15"><vh>fn referenced_nodes</vh>
<v t="ekr.20260812053055.18"><vh>fn visit</vh></v>
</v>
</v>
<v t="ekr.20260812053055.19"><vh>@clean src/lib.rs</vh></v>
<v t="ekr.20260812053055.20"><vh>@clean src/main.rs</vh>
<v t="ekr.20260812053055.21"><vh>struct Cli</vh></v>
<v t="ekr.20260812053055.22"><vh>enum Command</vh></v>
<v t="ekr.20260812053055.23"><vh>fn main</vh></v>
<v t="ekr.20260812053055.24"><vh>fn count_positions</vh></v>
</v>
<v t="ekr.20260812053055.25"><vh>@clean src/model.rs</vh>
<v t="ekr.20260812053055.26"><vh>impl From for NodeId</vh>
<v t="ekr.20260812053055.33"><vh>fn from</vh></v>
</v>
<v t="ekr.20260812053055.27"><vh>struct Node</vh></v>
<v t="ekr.20260812053055.28"><vh>struct Position</vh></v>
<v t="ekr.20260812053055.29"><vh>struct Outline</vh></v>
<v t="ekr.20260812053055.30"><vh>enum ValidationError</vh></v>
<v t="ekr.20260812053055.31"><vh>impl Outline</vh>
<v t="ekr.20260812053055.34"><vh>fn validate</vh>
<v t="ekr.20260812053055.37"><vh>fn walk</vh></v>
</v>
<v t="ekr.20260812053055.35"><vh>fn position</vh></v>
<v t="ekr.20260812053055.36"><vh>fn children_mut</vh></v>
</v>
<v t="ekr.20260812053055.32"><vh>fn parse_path</vh></v>
</v>
<v t="ekr.20260812053055.38"><vh>@clean src/operation.rs</vh>
<v t="ekr.20260812053055.39"><vh>enum Target</vh></v>
<v t="ekr.20260812053055.40"><vh>enum Operation</vh></v>
<v t="ekr.20260812053055.41"><vh>struct OperationBatch</vh></v>
<v t="ekr.20260812053055.42"><vh>struct ApplyReport</vh></v>
<v t="ekr.20260812053055.43"><vh>enum ApplyError</vh></v>
<v t="ekr.20260812053055.44"><vh>impl Outline</vh>
<v t="ekr.20260812053055.47"><vh>fn apply</vh></v>
<v t="ekr.20260812053055.48"><vh>fn apply_one</vh></v>
</v>
<v t="ekr.20260812053055.45"><vh>fn set_text</vh></v>
<v t="ekr.20260812053055.46"><vh>fn positions</vh></v>
</v>
<v t="ekr.20260812053055.49"><vh>@clean src/syntax.rs</vh>
<v t="ekr.20260812053055.50"><vh>unnamed use</vh></v>
<v t="ekr.20260812053055.51"><vh>unnamed use</vh></v>
<v t="ekr.20260812053055.52"><vh>struct SyntaxHighlighter</vh></v>
<v t="ekr.20260812053055.53"><vh>impl SyntaxHighlighter</vh>
<v t="ekr.20260812053055.56"><vh>fn new</vh></v>
<v t="ekr.20260812053055.57"><vh>fn highlight</vh></v>
<v t="ekr.20260812053055.58"><vh>fn syntax_for</vh></v>
</v>
<v t="ekr.20260812053055.54"><vh>fn language_directive</vh></v>
<v t="ekr.20260812053055.55"><vh>mod tests</vh>
<v t="ekr.20260812053055.59"><vh>fn finds_leo_language_directives</vh></v>
<v t="ekr.20260812053055.60"><vh>fn highlighting_preserves_line_structure_and_whitespace</vh></v>
</v>
</v>
<v t="ekr.20260812053055.61"><vh>@clean src/tui.rs</vh>
<v t="ekr.20260812053055.62"><vh>unnamed use</vh></v>
<v t="ekr.20260812053055.63"><vh>unnamed use</vh></v>
<v t="ekr.20260812053055.64"><vh>unnamed use</vh></v>
<v t="ekr.20260812053055.65"><vh>struct Row</vh></v>
<v t="ekr.20260812053055.66"><vh>struct App</vh></v>
<v t="ekr.20260812053055.67"><vh>impl App</vh>
<v t="ekr.20260812053055.103"><vh>fn new</vh></v>
<v t="ekr.20260812053055.104"><vh>fn rows</vh>
<v t="ekr.20260812053055.117"><vh>fn visit</vh></v>
</v>
<v t="ekr.20260812053055.105"><vh>fn move_selection</vh></v>
<v t="ekr.20260812053055.106"><vh>fn toggle</vh></v>
<v t="ekr.20260812053055.107"><vh>fn selected_row</vh></v>
<v t="ekr.20260812053055.108"><vh>fn editable</vh></v>
</v>
<v t="ekr.20260812053055.68"><vh>struct HeadlineInput</vh></v>
<v t="ekr.20260812053055.69"><vh>enum MoveDirection</vh></v>
<v t="ekr.20260812053055.70"><vh>fn run</vh></v>
<v t="ekr.20260812053055.71"><vh>fn event_loop</vh></v>
<v t="ekr.20260812053055.72"><vh>fn handle_headline_input</vh></v>
<v t="ekr.20260812053055.73"><vh>fn edit_headline</vh></v>
<v t="ekr.20260812053055.74"><vh>fn insert_headline</vh></v>
<v t="ekr.20260812053055.75"><vh>fn move_selected</vh></v>
<v t="ekr.20260812053055.76"><vh>fn save</vh></v>
<v t="ekr.20260812053055.77"><vh>fn restore_derived_children</vh></v>
<v t="ekr.20260812053055.78"><vh>fn referenced_nodes</vh>
<v t="ekr.20260812053055.109"><vh>fn visit</vh></v>
</v>
<v t="ekr.20260812053055.79"><vh>fn position_contains_derived</vh></v>
<v t="ekr.20260812053055.80"><vh>fn subtree_contains</vh></v>
<v t="ekr.20260812053055.81"><vh>fn split_position</vh></v>
<v t="ekr.20260812053055.82"><vh>fn join_position</vh></v>
<v t="ekr.20260812053055.83"><vh>fn children_mut</vh></v>
<v t="ekr.20260812053055.84"><vh>fn remove_position</vh></v>
<v t="ekr.20260812053055.85"><vh>fn select_position</vh></v>
<v t="ekr.20260812053055.86"><vh>fn fresh_node_id</vh></v>
<v t="ekr.20260812053055.87"><vh>fn draw</vh></v>
<v t="ekr.20260812053055.88"><vh>fn controls</vh></v>
<v t="ekr.20260812053055.89"><vh>fn body_text</vh></v>
<v t="ekr.20260812053055.90"><vh>struct LoadReport</vh></v>
<v t="ekr.20260812053055.91"><vh>struct DerivedJob</vh></v>
<v t="ekr.20260812053055.92"><vh>struct SourceLocation</vh></v>
<v t="ekr.20260812053055.93"><vh>fn load_derived_files</vh></v>
<v t="ekr.20260812053055.94"><vh>fn open_selected</vh></v>
<v t="ekr.20260812053055.95"><vh>fn suspend_and_open</vh></v>
<v t="ekr.20260812053055.96"><vh>fn run_editor</vh></v>
<v t="ekr.20260812053055.97"><vh>fn derived_jobs</vh>
<v t="ekr.20260812053055.110"><vh>fn visit</vh></v>
</v>
<v t="ekr.20260812053055.98"><vh>fn thin_filename</vh></v>
<v t="ekr.20260812053055.99"><vh>fn path_directive</vh></v>
<v t="ekr.20260812053055.100"><vh>fn strip_path_cruft</vh></v>
<v t="ekr.20260812053055.101"><vh>fn clone_count</vh>
<v t="ekr.20260812053055.111"><vh>fn count</vh></v>
</v>
<v t="ekr.20260812053055.102"><vh>mod tests</vh>
<v t="ekr.20260812053055.112"><vh>fn editing_app</vh></v>
<v t="ekr.20260812053055.113"><vh>fn recognizes_only_sentinelled_file_headlines</vh></v>
<v t="ekr.20260812053055.114"><vh>fn extracts_leo_path_directives</vh></v>
<v t="ekr.20260812053055.115"><vh>fn moves_nodes_with_leo_control_arrow_semantics</vh></v>
<v t="ekr.20260812053055.116"><vh>fn restores_external_children_before_serializing</vh></v>
</v>
</v>
<v t="ekr.20260812053055.118"><vh>@clean src/xml.rs</vh>
<v t="ekr.20260812053055.119"><vh>unnamed use</vh></v>
<v t="ekr.20260812053055.120"><vh>enum LeoXmlError</vh></v>
<v t="ekr.20260812053055.121"><vh>struct LeoDocument</vh></v>
<v t="ekr.20260812053055.122"><vh>impl LeoDocument</vh>
<v t="ekr.20260812053055.129"><vh>fn open</vh></v>
<v t="ekr.20260812053055.130"><vh>fn parse</vh></v>
<v t="ekr.20260812053055.131"><vh>fn to_xml</vh></v>
<v t="ekr.20260812053055.132"><vh>fn save</vh></v>
</v>
<v t="ekr.20260812053055.123"><vh>fn attributes</vh></v>
<v t="ekr.20260812053055.124"><vh>fn attach</vh></v>
<v t="ekr.20260812053055.125"><vh>fn replace_section</vh></v>
<v t="ekr.20260812053055.126"><vh>fn esc</vh></v>
<v t="ekr.20260812053055.127"><vh>fn render_vnodes</vh>
<v t="ekr.20260812053055.133"><vh>fn position</vh></v>
</v>
<v t="ekr.20260812053055.128"><vh>fn render_tnodes</vh></v>
</v>
</v>
</vnodes>
<tnodes>
<t tx="ekr.20260812052828.2"></t>
<t tx="ekr.20260812052904.1">"""Recursively import all python files in a directory and clean the result."""
@tabwidth - 4  # For a better match.
g.cls()

dir_ = r'C:\Repos\leo-cub\src'  # &lt;---- Change path to suit.

c.recursiveImport(
    dir_=dir_,
    kind= '@clean',
    recursive=True,
    safe_at_file=False,
    theTypes=['.rs',],
    verbose=True
)
if 1:
    last = c.lastTopLevel()
    last.expand()
    if last.hasChildren():
        last.firstChild().expand()
    c.redraw(last)
print('Done')

@language python
</t>
<t tx="ekr.20260812053055.10">fn leading_width(line: &amp;str) -&gt; usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

</t>
<t tx="ekr.20260812053055.100">fn strip_path_cruft(path: &amp;str) -&gt; &amp;str {
    let path = path.trim();
    if path.len() &gt; 2 {
        let pair = (path.as_bytes()[0], path.as_bytes()[path.len() - 1]);
        if matches!(pair, (b'&lt;', b'&gt;') | (b'"', b'"') | (b'\'', b'\'')) {
            return path[1..path.len() - 1].trim();
        }
    }
    path
}

</t>
<t tx="ekr.20260812053055.101">fn clone_count(outline: &amp;Outline, id: &amp;NodeId) -&gt; usize {
    @others
    count(&amp;outline.roots, id)
}

</t>
<t tx="ekr.20260812053055.102">#[cfg(test)]
mod tests {
    @others
}
</t>
<t tx="ekr.20260812053055.103">fn new(
    document: LeoDocument,
    path: PathBuf,
    status: String,
    source_locations: HashMap&lt;PositionId, SourceLocation&gt;,
    source_nodes: HashMap&lt;NodeId, SourceLocation&gt;,
    derived_nodes: HashSet&lt;NodeId&gt;,
    derived_roots: HashMap&lt;NodeId, Vec&lt;Position&gt;&gt;,
) -&gt; Self {
    let expanded = document
        .outline
        .roots
        .iter()
        .enumerate()
        .map(|(index, _)| PositionId(index.to_string()))
        .collect();
    Self {
        document,
        path,
        expanded,
        selected: 0,
        status,
        input: None,
        dirty: false,
        quit_armed: false,
        source_locations,
        source_nodes,
        derived_nodes,
        derived_roots,
        #[cfg(feature = "syntax")]
        syntax: crate::syntax::SyntaxHighlighter::new(),
        #[cfg(feature = "syntax")]
        syntax_enabled: true,
        #[cfg(feature = "syntax")]
        highlight_cache: HashMap::new(),
    }
}

</t>
<t tx="ekr.20260812053055.104">fn rows(&amp;self) -&gt; Vec&lt;Row&gt; {
    @others
    let mut rows = Vec::new();
    visit(
        &amp;self.document.outline.roots,
        "",
        0,
        &amp;self.expanded,
        &amp;mut rows,
    );
    rows
}

</t>
<t tx="ekr.20260812053055.105">fn move_selection(&amp;mut self, delta: isize) {
    let len = self.rows().len();
    if len == 0 {
        self.selected = 0;
        return;
    }
    self.selected = self.selected.saturating_add_signed(delta).min(len - 1);
}

</t>
<t tx="ekr.20260812053055.106">fn toggle(&amp;mut self, expand: bool) {
    let rows = self.rows();
    let Some(row) = rows.get(self.selected) else {
        return;
    };
    if expand &amp;&amp; row.has_children {
        self.expanded.insert(row.position.clone());
    }
    if !expand {
        self.expanded.remove(&amp;row.position);
    }
}

</t>
<t tx="ekr.20260812053055.107">fn selected_row(&amp;self) -&gt; Option&lt;Row&gt; {
    self.rows().get(self.selected).cloned()
}

</t>
<t tx="ekr.20260812053055.108">fn editable(&amp;mut self, row: &amp;Row) -&gt; bool {
    if self.derived_nodes.contains(&amp;row.node) {
        self.status = "derived descendants are read-only; press o to edit the source".into();
        false
    } else {
        true
    }
}
</t>
<t tx="ekr.20260812053055.109">let mut result = HashSet::new();
fn visit(positions: &amp;[Position], result: &amp;mut HashSet&lt;NodeId&gt;) {
    for position in positions {
        result.insert(position.node.clone());
        visit(&amp;position.children, result);
    }
}
</t>
<t tx="ekr.20260812053055.11">fn strip_indent(line: &amp;str, indent: usize) -&gt; &amp;str {
    if indent &gt; 0
        &amp;&amp; line.len() &gt; indent
        &amp;&amp; line.as_bytes()[..indent]
            .iter()
            .all(u8::is_ascii_whitespace)
    {
        &amp;line[indent..]
    } else {
        line
    }
}

</t>
<t tx="ekr.20260812053055.110">fn visit(
    outline: &amp;Outline,
    positions: &amp;[Position],
    parent_id: &amp;str,
    base: &amp;Path,
    inherited_paths: &amp;[String],
    jobs: &amp;mut Vec&lt;DerivedJob&gt;,
) {
    for (index, position) in positions.iter().enumerate() {
        let position_id = if parent_id.is_empty() {
            index.to_string()
        } else {
            format!("{parent_id}/{index}")
        };
        let node = &amp;outline.nodes[&amp;position.node];
        let mut paths = inherited_paths.to_vec();
        if let Some(path) =
            path_directive(&amp;node.headline).or_else(|| path_directive(&amp;node.body))
        {
            paths.push(path);
        }
        if let Some(filename) = thin_filename(&amp;node.headline) {
            let mut path = base.to_path_buf();
            for component in inherited_paths {
                path.push(component);
            }
            path.push(filename);
            jobs.push(DerivedJob {
                position: PositionId(position_id.clone()),
                path,
            });
        }
        visit(
            outline,
            &amp;position.children,
            &amp;position_id,
            base,
            &amp;paths,
            jobs,
        );
    }
}
</t>
<t tx="ekr.20260812053055.111">fn count(items: &amp;[Position], id: &amp;NodeId) -&gt; usize {
    items
        .iter()
        .map(|p| usize::from(&amp;p.node == id) + count(&amp;p.children, id))
        .sum()
}
</t>
<t tx="ekr.20260812053055.112">use super::*;

fn editing_app() -&gt; App {
    let document = LeoDocument::parse(
        r#"&lt;leo_file&gt;&lt;vnodes&gt;&lt;v t="a"&gt;&lt;vh&gt;A&lt;/vh&gt;&lt;v t="b"&gt;&lt;vh&gt;B&lt;/vh&gt;&lt;/v&gt;&lt;v t="c"&gt;&lt;vh&gt;C&lt;/vh&gt;&lt;/v&gt;&lt;/v&gt;&lt;/vnodes&gt;&lt;tnodes&gt;&lt;t tx="a"&gt;&lt;/t&gt;&lt;t tx="b"&gt;&lt;/t&gt;&lt;t tx="c"&gt;&lt;/t&gt;&lt;/tnodes&gt;&lt;/leo_file&gt;"#,
    )
    .unwrap();
    App::new(
        document,
        PathBuf::from("test.leo"),
        String::new(),
        HashMap::new(),
        HashMap::new(),
        HashSet::new(),
        HashMap::new(),
    )
}

</t>
<t tx="ekr.20260812053055.113">#[test]
fn recognizes_only_sentinelled_file_headlines() {
    assert_eq!(thin_filename("@file src/main.rs"), Some("src/main.rs"));
    assert_eq!(thin_filename("@file \"src/main.rs\""), Some("src/main.rs"));
    assert_eq!(thin_filename("@clean src/main.rs"), None);
    assert_eq!(thin_filename("ordinary"), None);
}

</t>
<t tx="ekr.20260812053055.114">#[test]
fn extracts_leo_path_directives() {
    assert_eq!(
        path_directive("@language rust\n@path &lt;src/core&gt;\n"),
        Some("src/core".into())
    );
}

</t>
<t tx="ekr.20260812053055.115">#[test]
fn moves_nodes_with_leo_control_arrow_semantics() {
    let mut app = editing_app();
    select_position(&amp;mut app, &amp;PositionId("0/1".into()));
    move_selected(&amp;mut app, MoveDirection::Up);
    assert_eq!(
        app.document.outline.roots[0].children[0].node,
        NodeId::from("c")
    );

    move_selected(&amp;mut app, MoveDirection::Down);
    move_selected(&amp;mut app, MoveDirection::Right);
    assert_eq!(
        app.document.outline.roots[0].children[0].children[0].node,
        NodeId::from("c")
    );

    move_selected(&amp;mut app, MoveDirection::Left);
    assert_eq!(
        app.document.outline.roots[0].children[1].node,
        NodeId::from("c")
    );
}

</t>
<t tx="ekr.20260812053055.116">#[test]
fn restores_external_children_before_serializing() {
    let mut roots = vec![Position {
        node: NodeId::from("file"),
        children: vec![Position {
            node: NodeId::from("derived"),
            children: vec![],
        }],
    }];
    let originals = HashMap::from([(NodeId::from("file"), Vec::new())]);
    restore_derived_children(&amp;mut roots, &amp;originals);
    assert!(roots[0].children.is_empty());
}
</t>
<t tx="ekr.20260812053055.117">fn visit(
    positions: &amp;[Position],
    parent: &amp;str,
    depth: usize,
    expanded: &amp;HashSet&lt;PositionId&gt;,
    rows: &amp;mut Vec&lt;Row&gt;,
) {
    for (index, position) in positions.iter().enumerate() {
        let path = if parent.is_empty() {
            index.to_string()
        } else {
            format!("{parent}/{index}")
        };
        let id = PositionId(path.clone());
        rows.push(Row {
            position: id.clone(),
            node: position.node.clone(),
            depth,
            has_children: !position.children.is_empty(),
        });
        if expanded.contains(&amp;id) {
            visit(&amp;position.children, &amp;path, depth + 1, expanded, rows);
        }
    }
}
</t>
<t tx="ekr.20260812053055.118">@others
@language rust
@tabwidth -4
</t>
<t tx="ekr.20260812053055.119">use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

</t>
<t tx="ekr.20260812053055.12">fn append_body(outline: &amp;mut Outline, id: Option&lt;&amp;NodeId&gt;, text: &amp;str) {
    if let Some(node) = id.and_then(|id| outline.nodes.get_mut(id)) {
        node.body.push_str(text);
    }
}

</t>
<t tx="ekr.20260812053055.120">use quick_xml::{Reader, events::Event};
use thiserror::Error;

use crate::{Node, NodeId, Outline, Position};

#[derive(Debug, Error)]
pub enum LeoXmlError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("invalid Leo file: {0}")]
    Invalid(String),
}

</t>
<t tx="ekr.20260812053055.121">/// A parsed outline plus the original envelope. Saving replaces only `&lt;vnodes&gt;`
/// and `&lt;tnodes&gt;`, retaining globals, preferences, namespaces and extensions.
#[derive(Clone, Debug)]
pub struct LeoDocument {
    pub outline: Outline,
    original: String,
}

</t>
<t tx="ekr.20260812053055.122">impl LeoDocument {
    @others
}

</t>
<t tx="ekr.20260812053055.123">fn attributes(
    e: &amp;quick_xml::events::BytesStart&lt;'_&gt;,
) -&gt; Result&lt;HashMap&lt;String, String&gt;, LeoXmlError&gt; {
    let mut map = HashMap::new();
    for a in e.attributes() {
        let a = a.map_err(|e| LeoXmlError::Invalid(e.to_string()))?;
        #[allow(deprecated)]
        let value = a
            .unescape_value()
            .map_err(|e| LeoXmlError::Invalid(e.to_string()))?
            .into_owned();
        map.insert(String::from_utf8_lossy(a.key.as_ref()).into_owned(), value);
    }
    Ok(map)
}

</t>
<t tx="ekr.20260812053055.124">fn attach(p: Position, stack: &amp;mut [Position], roots: &amp;mut Vec&lt;Position&gt;) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(p)
    } else {
        roots.push(p)
    }
}

</t>
<t tx="ekr.20260812053055.125">fn replace_section(source: &amp;str, tag: &amp;str, replacement: &amp;str) -&gt; Result&lt;String, LeoXmlError&gt; {
    let start_tag = format!("&lt;{tag}&gt;");
    let end_tag = format!("&lt;/{tag}&gt;");
    let start = source
        .find(&amp;start_tag)
        .ok_or_else(|| LeoXmlError::Invalid(format!("missing {start_tag}")))?;
    let end = source[start..]
        .find(&amp;end_tag)
        .map(|n| start + n + end_tag.len())
        .ok_or_else(|| LeoXmlError::Invalid(format!("missing {end_tag}")))?;
    Ok(format!(
        "{}{}{}",
        &amp;source[..start],
        replacement,
        &amp;source[end..]
    ))
}

</t>
<t tx="ekr.20260812053055.126">fn esc(s: &amp;str) -&gt; String {
    s.replace('&amp;', "&amp;amp;")
        .replace('&lt;', "&amp;lt;")
        .replace('&gt;', "&amp;gt;")
        .replace('"', "&amp;quot;")
}

</t>
<t tx="ekr.20260812053055.127">fn render_vnodes(outline: &amp;Outline) -&gt; String {
    @others
    let mut out = "&lt;vnodes&gt;\n".to_owned();
    let mut seen = HashSet::new();
    for p in &amp;outline.roots {
        position(p, outline, &amp;mut seen, &amp;mut out, 1);
    }
    out.push_str("&lt;/vnodes&gt;");
    out
}

</t>
<t tx="ekr.20260812053055.128">fn render_tnodes(outline: &amp;Outline) -&gt; String {
    let mut ids: Vec&lt;_&gt; = outline.nodes.keys().collect();
    ids.sort();
    let mut out = "&lt;tnodes&gt;\n".to_owned();
    for id in ids {
        let n = &amp;outline.nodes[id];
        let attrs: String = n
            .tnode_attributes
            .iter()
            .map(|(k, v)| format!(" {}=\"{}\"", k, esc(v)))
            .collect();
        out.push_str(&amp;format!(
            "  &lt;t tx=\"{}\"{attrs}&gt;{}&lt;/t&gt;\n",
            esc(&amp;id.0),
            esc(&amp;n.body)
        ));
    }
    out.push_str("&lt;/tnodes&gt;");
    out
}
</t>
<t tx="ekr.20260812053055.129">pub fn open(path: impl AsRef&lt;Path&gt;) -&gt; Result&lt;Self, LeoXmlError&gt; {
    Self::parse(&amp;fs::read_to_string(path)?)
}

</t>
<t tx="ekr.20260812053055.13">fn restore_expansion(
    stack: &amp;mut Vec&lt;(NodeId, usize)&gt;,
    line: usize,
) -&gt; Result&lt;(Option&lt;NodeId&gt;, usize), SentinelError&gt; {
    stack
        .pop()
        .map(|(id, indent)| (Some(id), indent))
        .ok_or(SentinelError::UnmatchedExpansion { line })
}

</t>
<t tx="ekr.20260812053055.130">pub fn parse(source: &amp;str) -&gt; Result&lt;Self, LeoXmlError&gt; {
    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(false);
    let mut outline = Outline::default();
    let mut stack: Vec&lt;Position&gt; = vec![];
    let mut first_seen = HashSet::new();
    let mut in_vnodes = false;
    let mut in_tnodes = false;
    let mut current_vh: Option&lt;NodeId&gt; = None;
    let mut current_t: Option&lt;NodeId&gt; = None;
    loop {
        match reader.read_event()? {
            Event::Start(e) if e.name().as_ref() == b"vnodes" =&gt; in_vnodes = true,
            Event::End(e) if e.name().as_ref() == b"vnodes" =&gt; in_vnodes = false,
            Event::Start(e) if e.name().as_ref() == b"tnodes" =&gt; in_tnodes = true,
            Event::End(e) if e.name().as_ref() == b"tnodes" =&gt; in_tnodes = false,
            Event::Start(e) if in_vnodes &amp;&amp; e.name().as_ref() == b"v" =&gt; {
                let attrs = attributes(&amp;e)?;
                let id = NodeId(
                    attrs
                        .get("t")
                        .cloned()
                        .ok_or_else(|| LeoXmlError::Invalid("&lt;v&gt; without t".into()))?,
                );
                let vnode_attributes = attrs.into_iter().filter(|(k, _)| k != "t").collect();
                if first_seen.insert(id.clone()) {
                    outline.nodes.insert(
                        id.clone(),
                        Node {
                            id: id.clone(),
                            headline: String::new(),
                            body: String::new(),
                            vnode_attributes,
                            tnode_attributes: HashMap::new(),
                        },
                    );
                }
                stack.push(Position {
                    node: id,
                    children: vec![],
                });
            }
            Event::Empty(e) if in_vnodes &amp;&amp; e.name().as_ref() == b"v" =&gt; {
                let id = attributes(&amp;e)?;
                let id = NodeId(
                    id.get("t")
                        .cloned()
                        .ok_or_else(|| LeoXmlError::Invalid("&lt;v&gt; without t".into()))?,
                );
                attach(
                    Position {
                        node: id,
                        children: vec![],
                    },
                    &amp;mut stack,
                    &amp;mut outline.roots,
                );
            }
            Event::End(e) if in_vnodes &amp;&amp; e.name().as_ref() == b"v" =&gt; {
                let p = stack
                    .pop()
                    .ok_or_else(|| LeoXmlError::Invalid("unbalanced &lt;v&gt;".into()))?;
                attach(p, &amp;mut stack, &amp;mut outline.roots);
            }
            Event::Start(e) if in_vnodes &amp;&amp; e.name().as_ref() == b"vh" =&gt; {
                current_vh = stack.last().map(|p| p.node.clone())
            }
            Event::End(e) if e.name().as_ref() == b"vh" =&gt; current_vh = None,
            Event::Start(e) if in_tnodes &amp;&amp; e.name().as_ref() == b"t" =&gt; {
                let attrs = attributes(&amp;e)?;
                let id = NodeId(
                    attrs
                        .get("tx")
                        .cloned()
                        .ok_or_else(|| LeoXmlError::Invalid("&lt;t&gt; without tx".into()))?,
                );
                let node = outline.nodes.entry(id.clone()).or_insert(Node {
                    id: id.clone(),
                    headline: String::new(),
                    body: String::new(),
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                });
                node.tnode_attributes = attrs.into_iter().filter(|(k, _)| k != "tx").collect();
                current_t = Some(id);
            }
            Event::End(e) if e.name().as_ref() == b"t" =&gt; current_t = None,
            Event::Text(e) =&gt; {
                let text = e
                    .decode()
                    .map_err(|e| LeoXmlError::Invalid(e.to_string()))?
                    .into_owned();
                if let Some(id) = &amp;current_vh
                    &amp;&amp; let Some(n) = outline.nodes.get_mut(id)
                {
                    n.headline.push_str(&amp;text);
                }
                if let Some(id) = &amp;current_t
                    &amp;&amp; let Some(n) = outline.nodes.get_mut(id)
                {
                    n.body.push_str(&amp;text);
                }
            }
            Event::GeneralRef(e) =&gt; {
                let name = e
                    .decode()
                    .map_err(|e| LeoXmlError::Invalid(e.to_string()))?;
                let text = match name.as_ref() {
                    "amp" =&gt; "&amp;".to_owned(),
                    "lt" =&gt; "&lt;".to_owned(),
                    "gt" =&gt; "&gt;".to_owned(),
                    "quot" =&gt; "\"".to_owned(),
                    "apos" =&gt; "'".to_owned(),
                    value if value.starts_with("#x") =&gt; u32::from_str_radix(&amp;value[2..], 16)
                        .ok()
                        .and_then(char::from_u32)
                        .map(String::from)
                        .ok_or_else(|| {
                            LeoXmlError::Invalid(format!("invalid entity &amp;{value};"))
                        })?,
                    value if value.starts_with('#') =&gt; value[1..]
                        .parse::&lt;u32&gt;()
                        .ok()
                        .and_then(char::from_u32)
                        .map(String::from)
                        .ok_or_else(|| {
                            LeoXmlError::Invalid(format!("invalid entity &amp;{value};"))
                        })?,
                    value =&gt; {
                        return Err(LeoXmlError::Invalid(format!("unknown entity &amp;{value};")));
                    }
                };
                if let Some(id) = &amp;current_vh
                    &amp;&amp; let Some(n) = outline.nodes.get_mut(id)
                {
                    n.headline.push_str(&amp;text);
                }
                if let Some(id) = &amp;current_t
                    &amp;&amp; let Some(n) = outline.nodes.get_mut(id)
                {
                    n.body.push_str(&amp;text);
                }
            }
            Event::Eof =&gt; break,
            _ =&gt; {}
        }
    }
    if !stack.is_empty() {
        return Err(LeoXmlError::Invalid("unbalanced vnodes".into()));
    }
    Ok(Self {
        outline,
        original: source.to_owned(),
    })
}

</t>
<t tx="ekr.20260812053055.131">pub fn to_xml(&amp;self) -&gt; Result&lt;String, LeoXmlError&gt; {
    let vnodes = render_vnodes(&amp;self.outline);
    let tnodes = render_tnodes(&amp;self.outline);
    let result = replace_section(&amp;self.original, "vnodes", &amp;vnodes)?;
    replace_section(&amp;result, "tnodes", &amp;tnodes)
}

</t>
<t tx="ekr.20260812053055.132">pub fn save(&amp;self, path: impl AsRef&lt;Path&gt;) -&gt; Result&lt;(), LeoXmlError&gt; {
    let path = path.as_ref();
    let tmp = path.with_extension("leo.tmp");
    fs::write(&amp;tmp, self.to_xml()?)?;
    fs::rename(tmp, path)?;
    Ok(())
}
</t>
<t tx="ekr.20260812053055.133">fn position(
    p: &amp;Position,
    outline: &amp;Outline,
    seen: &amp;mut HashSet&lt;NodeId&gt;,
    out: &amp;mut String,
    depth: usize,
) {
    let pad = "  ".repeat(depth);
    if !seen.insert(p.node.clone()) {
        out.push_str(&amp;format!("{pad}&lt;v t=\"{}\"&gt;&lt;/v&gt;\n", esc(&amp;p.node.0)));
        return;
    }
    let n = &amp;outline.nodes[&amp;p.node];
    let attrs: String = n
        .vnode_attributes
        .iter()
        .map(|(k, v)| format!(" {}=\"{}\"", k, esc(v)))
        .collect();
    out.push_str(&amp;format!(
        "{pad}&lt;v t=\"{}\"{attrs}&gt;&lt;vh&gt;{}&lt;/vh&gt;",
        esc(&amp;p.node.0),
        esc(&amp;n.headline)
    ));
    if p.children.is_empty() {
        out.push_str("&lt;/v&gt;\n");
    } else {
        out.push('\n');
        for c in &amp;p.children {
            position(c, outline, seen, out, depth + 1);
        }
        out.push_str(&amp;format!("{pad}&lt;/v&gt;\n"));
    }
}
</t>
<t tx="ekr.20260812053055.14">fn children_at&lt;'a&gt;(roots: &amp;'a mut [Position], path: &amp;[usize]) -&gt; Option&lt;&amp;'a mut Vec&lt;Position&gt;&gt; {
    let mut position = roots.get_mut(*path.first()?)?;
    for index in &amp;path[1..] {
        position = position.children.get_mut(*index)?;
    }
    Some(&amp;mut position.children)
}

</t>
<t tx="ekr.20260812053055.15">fn referenced_nodes(positions: &amp;[Position]) -&gt; HashSet&lt;NodeId&gt; {
    @others
    let mut result = HashSet::new();
    visit(positions, &amp;mut result);
    result
}
</t>
<t tx="ekr.20260812053055.16">/// Parse a Leo 5 thin derived file and reconstruct its logical outline.
pub fn parse(source: &amp;str) -&gt; Result&lt;Self, SentinelError&gt; {
    let lines: Vec&lt;&amp;str&gt; = source.split_inclusive('\n').collect();
    let (header_index, start_delimiter, end_delimiter) = scan_header(&amp;lines)?;
    let sentinel_prefix = format!("{start_delimiter}@");
    let node_pattern = Regex::new(&amp;format!(
        r"^(\s*){}@\+node:([^:]+): (.*){}\r?\n?$",
        regex::escape(&amp;start_delimiter),
        regex::escape(&amp;end_delimiter)
    ))
    .expect("generated node pattern");

    let mut outline = Outline::default();
    let mut root = None;
    let mut level_paths: Vec&lt;Vec&lt;usize&gt;&gt; = Vec::new();
    let mut locations = HashMap::new();
    let mut current: Option&lt;NodeId&gt; = None;
    let mut indent = 0usize;
    let mut expansions: Vec&lt;(NodeId, usize)&gt; = Vec::new();
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
            append_body(&amp;mut outline, current.as_ref(), strip_indent(line, indent));
            verbatim_next = false;
            continue;
        }
        if after_ref {
            if let Some(id) = current.as_ref()
                &amp;&amp; let Some(node) = outline.nodes.get_mut(id)
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
            let descriptor = &amp;captures[3];
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
                    node.headline.clone_from(&amp;headline);
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
                let children = children_at(&amp;mut outline.roots, &amp;parent_path).ok_or(
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
                    .collect::&lt;Vec&lt;_&gt;&gt;()
                    .join("/"),
            );
            locations.insert(position_id, line_number);
            current = Some(gnx);
            indent = captures[1].len();
            continue;
        }

        if !trimmed.starts_with(&amp;sentinel_prefix) {
            append_body(&amp;mut outline, current.as_ref(), strip_indent(line, indent));
            continue;
        }
        let inner = sentinel_content(trimmed, &amp;start_delimiter, &amp;end_delimiter);
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
                .filter(|_| first_index &lt; header_index)
            {
                append_body(&amp;mut outline, current.as_ref(), &amp;format!("@first {first}"));
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
            append_body(
                &amp;mut outline,
                Some(&amp;id),
                &amp;format!("{}@others{}\n", &amp;line[..leading], tail),
            );
            expansions.push((id, indent));
            indent += leading;
            continue;
        }
        if inner.starts_with("@-others") {
            (current, indent) = restore_expansion(&amp;mut expansions, line_number)?;
            continue;
        }
        if let Some(tail) = inner.strip_prefix("@+all") {
            let id = current.clone().ok_or(SentinelError::MissingRoot)?;
            let leading = leading_width(line);
            append_body(
                &amp;mut outline,
                Some(&amp;id),
                &amp;format!("{}@all{}\n", &amp;line[..leading], tail),
            );
            expansions.push((id, indent));
            indent += leading;
            continue;
        }
        if inner.starts_with("@-all") {
            (current, indent) = restore_expansion(&amp;mut expansions, line_number)?;
            continue;
        }
        if let Some(section) = inner.strip_prefix("@+&lt;&lt;") {
            let id = current.clone().ok_or(SentinelError::MissingRoot)?;
            let section = section.strip_suffix("&gt;&gt;").unwrap_or(section);
            let leading = leading_width(line);
            append_body(
                &amp;mut outline,
                Some(&amp;id),
                &amp;format!("{}&lt;&lt;{}&gt;&gt;\n", &amp;line[..leading], section),
            );
            expansions.push((id, indent));
            indent += leading;
            continue;
        }
        if inner.starts_with("@-&lt;&lt;") {
            (current, indent) = restore_expansion(&amp;mut expansions, line_number)?;
            continue;
        }
        if let Some(directive) = inner.strip_prefix("@@") {
            append_body(&amp;mut outline, current.as_ref(), &amp;format!("@{directive}\n"));
        }
    }
    if verbatim_next {
        return Err(SentinelError::UnterminatedVerbatim);
    }
    if !expansions.is_empty() {
        return Err(SentinelError::UnterminatedExpansion);
    }
    let root = root.ok_or(SentinelError::MissingRoot)?;
    if last_count &gt; 0 {
        for line in lines.iter().skip(tail_start).take(last_count) {
            append_body(
                &amp;mut outline,
                current.as_ref(),
                &amp;format!("@last {}\n", line.trim_end()),
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

</t>
<t tx="ekr.20260812053055.17">/// Transactionally replace the descendants and bodies represented by a
/// derived file. The `@file` root headline remains owned by the `.leo` file.
pub fn merge_into(
    &amp;self,
    outline: &amp;mut Outline,
    target: &amp;PositionId,
) -&gt; Result&lt;(), SentinelError&gt; {
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
    let derived_root = &amp;self.outline.roots[0];
    let root_body = self.outline.nodes[&amp;self.root].body.clone();
    next.nodes
        .get_mut(&amp;self.root)
        .expect("validated target")
        .body = root_body;
    for (id, node) in &amp;self.outline.nodes {
        if id != &amp;self.root {
            next.nodes.insert(id.clone(), node.clone());
        }
    }
    next.children_mut(Some(target))
        .expect("validated target")
        .clone_from(&amp;derived_root.children);
    let referenced = referenced_nodes(&amp;next.roots);
    next.nodes.retain(|id, _| referenced.contains(id));
    *outline = next;
    Ok(())
}
</t>
<t tx="ekr.20260812053055.18">fn visit(items: &amp;[Position], result: &amp;mut HashSet&lt;NodeId&gt;) {
    for position in items {
        result.insert(position.node.clone());
        visit(&amp;position.children, result);
    }
}
</t>
<t tx="ekr.20260812053055.19">//! Clone-aware, automation-safe manipulation of Leo XML outlines.

mod derived;
mod model;
mod operation;
mod xml;

pub use derived::{DerivedFile, SentinelError};
pub use model::{Node, NodeId, Outline, Position, PositionId, ValidationError};
pub use operation::{ApplyError, ApplyReport, Operation, OperationBatch, Target};
pub use xml::{LeoDocument, LeoXmlError};
@language rust
@tabwidth -4
</t>
<t tx="ekr.20260812053055.2"></t>
<t tx="ekr.20260812053055.20">use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use leo::{DerivedFile, LeoDocument, OperationBatch, PositionId};
use std::{fs, path::PathBuf};

@others
@language rust
@tabwidth -4
</t>
<t tx="ekr.20260812053055.21">#[cfg(all(feature = "tui", feature = "syntax"))]
mod syntax;
#[cfg(feature = "tui")]
mod tui;

#[derive(Parser)]
#[command(name = "cub", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

</t>
<t tx="ekr.20260812053055.22">#[derive(Subcommand)]
enum Command {
    /// Browse an outline interactively (read-only).
    #[cfg(feature = "tui")]
    Tui {
        file: PathBuf,
        /// Show only the hierarchy stored directly in the .leo XML.
        #[arg(long)]
        no_derived: bool,
    },
    Inspect {
        file: PathBuf,
    },
    Validate {
        file: PathBuf,
    },
    Apply {
        file: PathBuf,
        operations: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Reconstruct an @file subtree from a thin derived file.
    RefreshDerived {
        file: PathBuf,
        position: String,
        derived: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Parse a thin derived file and print its reconstructed outline.
    InspectDerived {
        derived: PathBuf,
        #[arg(long)]
        summary: bool,
    },
    Diff {
        before: PathBuf,
        after: PathBuf,
    },
}

</t>
<t tx="ekr.20260812053055.23">fn main() -&gt; Result&lt;()&gt; {
    let cli = Cli::parse();
    match cli.command {
        #[cfg(feature = "tui")]
        Command::Tui { file, no_derived } =&gt; tui::run(file, !no_derived)?,
        Command::Inspect { file } =&gt; println!(
            "{}",
            serde_json::to_string_pretty(&amp;LeoDocument::open(file)?.outline)?
        ),
        Command::Validate { file } =&gt; {
            let errors = LeoDocument::open(file)?.outline.validate();
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &amp;errors.iter().map(ToString::to_string).collect::&lt;Vec&lt;_&gt;&gt;()
                )?
            );
            if !errors.is_empty() {
                bail!("outline is invalid")
            }
        }
        Command::Apply {
            file,
            operations,
            dry_run,
        } =&gt; {
            let mut doc = LeoDocument::open(&amp;file)?;
            let batch: OperationBatch = serde_json::from_str(
                &amp;fs::read_to_string(operations).context("read operations file")?,
            )?;
            let report = doc.outline.apply(&amp;batch)?;
            if !dry_run {
                doc.save(file)?;
            }
            println!("{}", serde_json::to_string_pretty(&amp;report)?);
        }
        Command::RefreshDerived {
            file,
            position,
            derived,
            dry_run,
        } =&gt; {
            let mut document = LeoDocument::open(&amp;file)?;
            let source = fs::read_to_string(&amp;derived).context("read derived file")?;
            let parsed = DerivedFile::parse(&amp;source)?;
            parsed.merge_into(&amp;mut document.outline, &amp;PositionId(position))?;
            if !dry_run {
                document.save(file)?;
            }
            println!(
                "{}",
                serde_json::json!({
                    "root": parsed.root,
                    "nodes": parsed.outline.nodes.len(),
                    "dry_run": dry_run
                })
            );
        }
        Command::InspectDerived { derived, summary } =&gt; {
            let source = fs::read_to_string(derived).context("read derived file")?;
            let parsed = DerivedFile::parse(&amp;source)?;
            if summary {
                println!(
                    "{}",
                    serde_json::json!({
                        "root": parsed.root,
                        "nodes": parsed.outline.nodes.len(),
                        "positions": count_positions(&amp;parsed.outline.roots),
                        "start_delimiter": parsed.start_delimiter,
                        "end_delimiter": parsed.end_delimiter
                    })
                );
            } else {
                println!("{}", serde_json::to_string_pretty(&amp;parsed.outline)?);
            }
        }
        Command::Diff { before, after } =&gt; {
            let a = LeoDocument::open(before)?.outline;
            let b = LeoDocument::open(after)?.outline;
            println!(
                "{}",
                serde_json::json!({"equal": a == b, "before": a, "after": b})
            );
        }
    }
    Ok(())
}

</t>
<t tx="ekr.20260812053055.24">fn count_positions(positions: &amp;[leo::Position]) -&gt; usize {
    positions
        .iter()
        .map(|position| 1 + count_positions(&amp;position.children))
        .sum()
}
</t>
<t tx="ekr.20260812053055.25">use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

@others
@language rust
@tabwidth -4
</t>
<t tx="ekr.20260812053055.26">#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub String);

impl From&lt;&amp;str&gt; for NodeId {
    @others
}

</t>
<t tx="ekr.20260812053055.27">/// Identifies one occurrence in the outline, including cloned occurrences.
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
    pub vnode_attributes: HashMap&lt;String, String&gt;,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tnode_attributes: HashMap&lt;String, String&gt;,
}

</t>
<t tx="ekr.20260812053055.28">#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub node: NodeId,
    #[serde(default)]
    pub children: Vec&lt;Position&gt;,
}

</t>
<t tx="ekr.20260812053055.29">#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Outline {
    pub nodes: HashMap&lt;NodeId, Node&gt;,
    pub roots: Vec&lt;Position&gt;,
}

</t>
<t tx="ekr.20260812053055.3">@others
@language rust
@tabwidth -4
</t>
<t tx="ekr.20260812053055.30">#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    #[error("position {position} references missing node {node}")]
    MissingNode { position: String, node: String },
    #[error("node table key {key} does not match node id {node}")]
    MismatchedNodeId { key: String, node: String },
    #[error("node {0} is not referenced by any position")]
    OrphanNode(String),
}

</t>
<t tx="ekr.20260812053055.31">impl Outline {
    @others
}

</t>
<t tx="ekr.20260812053055.32">fn parse_path(id: &amp;PositionId) -&gt; Option&lt;Vec&lt;usize&gt;&gt; {
    if id.0.is_empty() {
        return None;
    }
    id.0.split('/').map(|s| s.parse().ok()).collect()
}
</t>
<t tx="ekr.20260812053055.33">fn from(value: &amp;str) -&gt; Self {
    Self(value.to_owned())
}
</t>
<t tx="ekr.20260812053055.34">pub fn validate(&amp;self) -&gt; Vec&lt;ValidationError&gt; {
    @others
    for (i, root) in self.roots.iter().enumerate() {
        walk(
            root,
            i.to_string(),
            &amp;self.nodes,
            &amp;mut referenced,
            &amp;mut errors,
        );
    }
    for id in self.nodes.keys() {
        if !referenced.contains(id) {
            errors.push(ValidationError::OrphanNode(id.0.clone()));
        }
    }
    errors
}

</t>
<t tx="ekr.20260812053055.35">pub fn position(&amp;self, id: &amp;PositionId) -&gt; Option&lt;&amp;Position&gt; {
    let indices = parse_path(id)?;
    let mut p = self.roots.get(*indices.first()?)?;
    for index in &amp;indices[1..] {
        p = p.children.get(*index)?;
    }
    Some(p)
}

</t>
<t tx="ekr.20260812053055.36">pub(crate) fn children_mut(
    &amp;mut self,
    parent: Option&lt;&amp;PositionId&gt;,
) -&gt; Option&lt;&amp;mut Vec&lt;Position&gt;&gt; {
    let Some(parent) = parent else {
        return Some(&amp;mut self.roots);
    };
    let indices = parse_path(parent)?;
    let mut p = self.roots.get_mut(*indices.first()?)?;
    for index in &amp;indices[1..] {
        p = p.children.get_mut(*index)?;
    }
    Some(&amp;mut p.children)
}
</t>
<t tx="ekr.20260812053055.37">let mut errors = Vec::new();
for (key, node) in &amp;self.nodes {
    if key != &amp;node.id {
        errors.push(ValidationError::MismatchedNodeId {
            key: key.0.clone(),
            node: node.id.0.clone(),
        });
    }
}
let mut referenced = HashSet::new();
fn walk(
    p: &amp;Position,
    path: String,
    nodes: &amp;HashMap&lt;NodeId, Node&gt;,
    seen: &amp;mut HashSet&lt;NodeId&gt;,
    errors: &amp;mut Vec&lt;ValidationError&gt;,
) {
    if !nodes.contains_key(&amp;p.node) {
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
</t>
<t tx="ekr.20260812053055.38">use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Node, NodeId, Outline, Position, PositionId};

@others
@language rust
@tabwidth -4
</t>
<t tx="ekr.20260812053055.39">#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Target {
    Position { position: PositionId },
    Node { node: NodeId },
}

</t>
<t tx="ekr.20260812053055.4">//! Parser for Leo 5 thin derived files.

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
    pub locations: HashMap&lt;PositionId, usize&gt;,
}

</t>
<t tx="ekr.20260812053055.40">#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum Operation {
    Insert {
        parent: Option&lt;PositionId&gt;,
        index: Option&lt;usize&gt;,
        node: Node,
    },
    Clone {
        parent: Option&lt;PositionId&gt;,
        index: Option&lt;usize&gt;,
        node: NodeId,
    },
    SetHeadline {
        node: NodeId,
        headline: String,
        expected: Option&lt;String&gt;,
    },
    SetBody {
        node: NodeId,
        body: String,
        expected: Option&lt;String&gt;,
    },
    Remove {
        position: PositionId,
    },
}

</t>
<t tx="ekr.20260812053055.41">#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OperationBatch {
    pub operations: Vec&lt;Operation&gt;,
}

</t>
<t tx="ekr.20260812053055.42">#[derive(Clone, Debug, Serialize)]
pub struct ApplyReport {
    pub applied: usize,
}

</t>
<t tx="ekr.20260812053055.43">#[derive(Debug, Error)]
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
    Precondition { node: String, field: &amp;'static str },
    #[error("batch would create an invalid outline: {0}")]
    Invalid(String),
}

</t>
<t tx="ekr.20260812053055.44">impl Outline {
    @others
}

</t>
<t tx="ekr.20260812053055.45">fn set_text(
    node: Option&lt;&amp;mut Node&gt;,
    id: &amp;NodeId,
    expected: &amp;Option&lt;String&gt;,
    value: &amp;str,
    headline: bool,
) -&gt; Result&lt;(), ApplyError&gt; {
    let node = node.ok_or_else(|| ApplyError::NodeNotFound(id.0.clone()))?;
    let current = if headline {
        &amp;mut node.headline
    } else {
        &amp;mut node.body
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

</t>
<t tx="ekr.20260812053055.46">fn positions(items: &amp;[Position]) -&gt; Box&lt;dyn Iterator&lt;Item = &amp;Position&gt; + '_&gt; {
    Box::new(
        items
            .iter()
            .flat_map(|p| std::iter::once(p).chain(positions(&amp;p.children))),
    )
}
</t>
<t tx="ekr.20260812053055.47">/// Applies all operations to a copy and commits only if every operation succeeds.
pub fn apply(&amp;mut self, batch: &amp;OperationBatch) -&gt; Result&lt;ApplyReport, ApplyError&gt; {
    let mut next = self.clone();
    for op in &amp;batch.operations {
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

</t>
<t tx="ekr.20260812053055.48">fn apply_one(&amp;mut self, op: &amp;Operation) -&gt; Result&lt;(), ApplyError&gt; {
    match op {
        Operation::Insert {
            parent,
            index,
            node,
        } =&gt; {
            if self.nodes.contains_key(&amp;node.id) {
                return Err(ApplyError::DuplicateNode(node.id.0.clone()));
            }
            let id = node.id.clone();
            let children = self.children_mut(parent.as_ref()).ok_or_else(|| {
                ApplyError::PositionNotFound(parent.as_ref().unwrap().0.clone())
            })?;
            let at = index.unwrap_or(children.len());
            if at &gt; children.len() {
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
        } =&gt; {
            if !self.nodes.contains_key(node) {
                return Err(ApplyError::NodeNotFound(node.0.clone()));
            }
            let children = self.children_mut(parent.as_ref()).ok_or_else(|| {
                ApplyError::PositionNotFound(parent.as_ref().unwrap().0.clone())
            })?;
            let at = index.unwrap_or(children.len());
            if at &gt; children.len() {
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
        } =&gt; set_text(self.nodes.get_mut(node), node, expected, headline, true)?,
        Operation::SetBody {
            node,
            body,
            expected,
        } =&gt; set_text(self.nodes.get_mut(node), node, expected, body, false)?,
        Operation::Remove { position } =&gt; {
            let mut path: Vec&lt;usize&gt; = position
                .0
                .split('/')
                .map(str::parse)
                .collect::&lt;Result&lt;_, _&gt;&gt;()
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
                        .collect::&lt;Vec&lt;_&gt;&gt;()
                        .join("/"),
                ))
            };
            let children = self
                .children_mut(parent.as_ref())
                .ok_or_else(|| ApplyError::PositionNotFound(position.0.clone()))?;
            if index &gt;= children.len() {
                return Err(ApplyError::PositionNotFound(position.0.clone()));
            }
            children.remove(index);
            let used: std::collections::HashSet&lt;_&gt; =
                positions(&amp;self.roots).map(|p| p.node.clone()).collect();
            self.nodes.retain(|id, _| used.contains(id));
        }
    }
    Ok(())
}
</t>
<t tx="ekr.20260812053055.49">use std::path::Path;

use ratatui::{
@others
@language rust
@tabwidth -4
</t>
<t tx="ekr.20260812053055.5">#[derive(Debug, Error, PartialEq)]
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

</t>
<t tx="ekr.20260812053055.50">    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
</t>
<t tx="ekr.20260812053055.51">use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};

</t>
<t tx="ekr.20260812053055.52">pub struct SyntaxHighlighter {
    syntaxes: SyntaxSet,
    themes: ThemeSet,
}

</t>
<t tx="ekr.20260812053055.53">impl SyntaxHighlighter {
    @others
}

</t>
<t tx="ekr.20260812053055.54">fn language_directive(body: &amp;str) -&gt; Option&lt;&amp;str&gt; {
    body.lines().find_map(|line| {
        line.trim_start()
            .strip_prefix("@language")
            .and_then(|rest| rest.split_whitespace().next())
    })
}

</t>
<t tx="ekr.20260812053055.55">#[cfg(test)]
mod tests {
    @others
}
</t>
<t tx="ekr.20260812053055.56">pub fn new() -&gt; Self {
    Self {
        syntaxes: SyntaxSet::load_defaults_newlines(),
        themes: ThemeSet::load_defaults(),
    }
}

</t>
<t tx="ekr.20260812053055.57">pub fn highlight(&amp;self, body: &amp;str, source_path: Option&lt;&amp;Path&gt;) -&gt; Text&lt;'static&gt; {
    let syntax = self.syntax_for(body, source_path);
    let theme = &amp;self.themes.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut lines = Vec::new();
    for source_line in body.split_inclusive('\n') {
        let spans = highlighter
            .highlight_line(source_line, &amp;self.syntaxes)
            .map(|ranges| {
                ranges
                    .into_iter()
                    .filter_map(|(style, text)| {
                        let text = text.strip_suffix('\n').unwrap_or(text);
                        (!text.is_empty()).then(|| {
                            let mut ratatui_style = Style::default().fg(Color::Rgb(
                                style.foreground.r,
                                style.foreground.g,
                                style.foreground.b,
                            ));
                            if style.font_style.contains(FontStyle::BOLD) {
                                ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                            }
                            if style.font_style.contains(FontStyle::ITALIC) {
                                ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                            }
                            if style.font_style.contains(FontStyle::UNDERLINE) {
                                ratatui_style =
                                    ratatui_style.add_modifier(Modifier::UNDERLINED);
                            }
                            Span::styled(text.to_owned(), ratatui_style)
                        })
                    })
                    .collect()
            })
            .unwrap_or_else(|_| vec![Span::raw(source_line.trim_end_matches('\n').to_owned())]);
        lines.push(Line::from(spans));
    }
    Text::from(lines)
}

</t>
<t tx="ekr.20260812053055.58">fn syntax_for&lt;'a&gt;(&amp;'a self, body: &amp;str, source_path: Option&lt;&amp;Path&gt;) -&gt; &amp;'a SyntaxReference {
    language_directive(body)
        .and_then(|language| self.syntaxes.find_syntax_by_token(language))
        .or_else(|| {
            source_path
                .and_then(Path::extension)
                .and_then(|extension| extension.to_str())
                .and_then(|extension| self.syntaxes.find_syntax_by_extension(extension))
        })
        .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text())
}
</t>
<t tx="ekr.20260812053055.59">use super::*;

#[test]
fn finds_leo_language_directives() {
    assert_eq!(
        language_directive("@tabwidth -4\n@language rust\n"),
        Some("rust")
    );
    assert_eq!(language_directive("let language = rust;"), None);
}

</t>
<t tx="ekr.20260812053055.6">impl DerivedFile {
    @others
}

</t>
<t tx="ekr.20260812053055.60">#[test]
fn highlighting_preserves_line_structure_and_whitespace() {
    let text = SyntaxHighlighter::new()
        .highlight("fn main() {\n    true\n}\n", Some(Path::new("main.rs")));
    assert_eq!(text.lines.len(), 3);
    assert_eq!(text.lines[1].width(), 8);
}
</t>
<t tx="ekr.20260812053055.61">@others
@language rust
@tabwidth -4
</t>
<t tx="ekr.20260812053055.62">use std::{
    collections::{HashMap, HashSet},
    env, fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

</t>
<t tx="ekr.20260812053055.63">use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
</t>
<t tx="ekr.20260812053055.64">use leo::{DerivedFile, LeoDocument, NodeId, Outline, Position, PositionId};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

</t>
<t tx="ekr.20260812053055.65">#[derive(Clone)]
struct Row {
    position: PositionId,
    node: NodeId,
    depth: usize,
    has_children: bool,
}

</t>
<t tx="ekr.20260812053055.66">struct App {
    document: LeoDocument,
    path: PathBuf,
    expanded: HashSet&lt;PositionId&gt;,
    selected: usize,
    status: String,
    input: Option&lt;HeadlineInput&gt;,
    dirty: bool,
    quit_armed: bool,
    source_locations: HashMap&lt;PositionId, SourceLocation&gt;,
    source_nodes: HashMap&lt;NodeId, SourceLocation&gt;,
    derived_nodes: HashSet&lt;NodeId&gt;,
    derived_roots: HashMap&lt;NodeId, Vec&lt;Position&gt;&gt;,
    #[cfg(feature = "syntax")]
    syntax: crate::syntax::SyntaxHighlighter,
    #[cfg(feature = "syntax")]
    syntax_enabled: bool,
    #[cfg(feature = "syntax")]
    highlight_cache: HashMap&lt;PositionId, Text&lt;'static&gt;&gt;,
}

</t>
<t tx="ekr.20260812053055.67">impl App {
    @others
}

</t>
<t tx="ekr.20260812053055.68">struct HeadlineInput {
    node: NodeId,
    value: String,
    original: String,
    inserted_position: Option&lt;PositionId&gt;,
}

</t>
<t tx="ekr.20260812053055.69">#[derive(Clone, Copy)]
enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
}

</t>
<t tx="ekr.20260812053055.7">fn scan_header(lines: &amp;[&amp;str]) -&gt; Result&lt;(usize, String, String), SentinelError&gt; {
    let pattern =
        Regex::new(r"^(.+)@\+leo(?:-ver=\d+)?(?:-thin)?(?:-encoding=.*?,\.)?(.*)\r?\n?$").unwrap();
    for (index, line) in lines.iter().enumerate() {
        if let Some(captures) = pattern.captures(line) {
            return Ok((index, captures[1].to_owned(), captures[2].to_owned()));
        }
    }
    Err(SentinelError::MissingHeader)
}

</t>
<t tx="ekr.20260812053055.70">pub fn run(path: PathBuf, load_derived: bool) -&gt; Result&lt;()&gt; {
    let mut document = LeoDocument::open(&amp;path)?;
    let (status, source_locations, source_nodes, derived_nodes, derived_roots) = if load_derived {
        let report = load_derived_files(&amp;mut document.outline, &amp;path);
        let status = if report.errors.is_empty() {
            format!("loaded {} derived file(s)", report.loaded)
        } else {
            format!(
                "loaded {}; {} error(s): {}",
                report.loaded,
                report.errors.len(),
                report.errors.join(" | ")
            )
        };
        (
            status,
            report.locations,
            report.node_locations,
            report.derived_nodes,
            report.original_children,
        )
    } else {
        (
            "derived files disabled".to_owned(),
            HashMap::new(),
            HashMap::new(),
            HashSet::new(),
            HashMap::new(),
        )
    };
    let mut app = App::new(
        document,
        path,
        status,
        source_locations,
        source_nodes,
        derived_nodes,
        derived_roots,
    );
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&amp;mut terminal, &amp;mut app);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

</t>
<t tx="ekr.20260812053055.71">fn event_loop(terminal: &amp;mut Terminal&lt;CrosstermBackend&lt;io::Stdout&gt;&gt;, app: &amp;mut App) -&gt; Result&lt;()&gt; {
    loop {
        terminal.draw(|frame| draw(frame, app))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if app.input.is_some() {
            handle_headline_input(app, key);
            continue;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') =&gt; save(app),
                KeyCode::Char('i') | KeyCode::Tab =&gt; insert_headline(app),
                KeyCode::Char('h') | KeyCode::Backspace =&gt; edit_headline(app),
                KeyCode::Up =&gt; move_selected(app, MoveDirection::Up),
                KeyCode::Down =&gt; move_selected(app, MoveDirection::Down),
                KeyCode::Left =&gt; move_selected(app, MoveDirection::Left),
                KeyCode::Right =&gt; move_selected(app, MoveDirection::Right),
                _ =&gt; {}
            }
            continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc =&gt; {
                if !app.dirty || app.quit_armed {
                    return Ok(());
                }
                app.quit_armed = true;
                app.status = "unsaved changes; press q again to discard, or Ctrl-S to save".into();
            }
            KeyCode::Char('o') =&gt; open_selected(terminal, app),
            KeyCode::Tab =&gt; insert_headline(app),
            KeyCode::Backspace =&gt; edit_headline(app),
            #[cfg(feature = "syntax")]
            KeyCode::Char('y') =&gt; {
                app.syntax_enabled = !app.syntax_enabled;
                app.status = format!(
                    "syntax highlighting {}",
                    if app.syntax_enabled { "on" } else { "off" }
                );
            }
            KeyCode::Down | KeyCode::Char('j') =&gt; app.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') =&gt; app.move_selection(-1),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter =&gt; app.toggle(true),
            KeyCode::Left | KeyCode::Char('h') =&gt; app.toggle(false),
            KeyCode::Home =&gt; app.selected = 0,
            KeyCode::End =&gt; app.selected = app.rows().len().saturating_sub(1),
            _ =&gt; {}
        }
    }
}

</t>
<t tx="ekr.20260812053055.72">fn handle_headline_input(app: &amp;mut App, key: KeyEvent) {
    let Some(input) = app.input.as_mut() else {
        return;
    };
    match key.code {
        KeyCode::Enter =&gt; {
            let headline = input.value.trim().to_owned();
            if headline.is_empty() {
                app.status = "headline may not be empty".into();
                return;
            }
            app.document
                .outline
                .nodes
                .get_mut(&amp;input.node)
                .expect("edited node exists")
                .headline = headline;
            app.input = None;
            app.dirty = true;
            app.quit_armed = false;
            app.status = "headline changed (Ctrl-S to save)".into();
        }
        KeyCode::Esc =&gt; {
            let input = app.input.take().expect("input exists");
            if let Some(position) = input.inserted_position {
                remove_position(&amp;mut app.document.outline, &amp;position);
                app.document.outline.nodes.remove(&amp;input.node);
            } else {
                app.document
                    .outline
                    .nodes
                    .get_mut(&amp;input.node)
                    .expect("node exists")
                    .headline = input.original;
            }
            app.status = "headline edit cancelled".into();
        }
        KeyCode::Backspace =&gt; {
            input.value.pop();
        }
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =&gt;
        {
            input.value.push(character);
        }
        _ =&gt; {}
    }
}

</t>
<t tx="ekr.20260812053055.73">fn edit_headline(app: &amp;mut App) {
    let Some(row) = app.selected_row() else {
        return;
    };
    if !app.editable(&amp;row) {
        return;
    }
    let original = app.document.outline.nodes[&amp;row.node].headline.clone();
    app.input = Some(HeadlineInput {
        node: row.node,
        value: original.clone(),
        original,
        inserted_position: None,
    });
    app.status = "editing headline: Enter accepts, Esc cancels".into();
}

</t>
<t tx="ekr.20260812053055.74">fn insert_headline(app: &amp;mut App) {
    let Some(row) = app.selected_row() else {
        return;
    };
    if !app.editable(&amp;row) || position_contains_derived(app, &amp;row.position) {
        app.status = "cannot insert beside an external derived subtree".into();
        return;
    }
    let Some((parent, index)) = split_position(&amp;row.position) else {
        return;
    };
    let id = fresh_node_id();
    let position = Position {
        node: id.clone(),
        children: Vec::new(),
    };
    let Some(siblings) = children_mut(&amp;mut app.document.outline, parent.as_ref()) else {
        return;
    };
    siblings.insert(index + 1, position);
    let inserted = join_position(parent.as_ref(), index + 1);
    app.document.outline.nodes.insert(
        id.clone(),
        leo::Node {
            id: id.clone(),
            headline: "New Headline".into(),
            body: String::new(),
            vnode_attributes: HashMap::new(),
            tnode_attributes: HashMap::new(),
        },
    );
    select_position(app, &amp;inserted);
    app.input = Some(HeadlineInput {
        node: id,
        value: String::new(),
        original: String::new(),
        inserted_position: Some(inserted),
    });
    app.status = "new headline: type a name, Enter accepts, Esc cancels".into();
}

</t>
<t tx="ekr.20260812053055.75">fn move_selected(app: &amp;mut App, direction: MoveDirection) {
    let Some(row) = app.selected_row() else {
        return;
    };
    if !app.editable(&amp;row) || position_contains_derived(app, &amp;row.position) {
        app.status = "external derived subtrees cannot be moved".into();
        return;
    }
    let Some((parent, index)) = split_position(&amp;row.position) else {
        return;
    };
    let target = match direction {
        MoveDirection::Up | MoveDirection::Down =&gt; {
            let Some(siblings) = children_mut(&amp;mut app.document.outline, parent.as_ref()) else {
                return;
            };
            let other = if matches!(direction, MoveDirection::Up) {
                index.checked_sub(1)
            } else if index + 1 &lt; siblings.len() {
                Some(index + 1)
            } else {
                None
            };
            let Some(other) = other else {
                app.status = "node is already at the edge".into();
                return;
            };
            siblings.swap(index, other);
            join_position(parent.as_ref(), other)
        }
        MoveDirection::Right =&gt; {
            let Some(previous) = index.checked_sub(1) else {
                app.status = "no previous sibling to become parent".into();
                return;
            };
            let Some(siblings) = children_mut(&amp;mut app.document.outline, parent.as_ref()) else {
                return;
            };
            let position = siblings.remove(index);
            let child_index = siblings[previous].children.len();
            siblings[previous].children.push(position);
            let previous_path = join_position(parent.as_ref(), previous);
            app.expanded.insert(previous_path.clone());
            join_position(Some(&amp;previous_path), child_index)
        }
        MoveDirection::Left =&gt; {
            let Some(parent_id) = parent else {
                app.status = "top-level nodes cannot be promoted".into();
                return;
            };
            let Some((grandparent, parent_index)) = split_position(&amp;parent_id) else {
                return;
            };
            let position = {
                let Some(siblings) = children_mut(&amp;mut app.document.outline, Some(&amp;parent_id))
                else {
                    return;
                };
                siblings.remove(index)
            };
            let Some(grand_siblings) =
                children_mut(&amp;mut app.document.outline, grandparent.as_ref())
            else {
                return;
            };
            grand_siblings.insert(parent_index + 1, position);
            join_position(grandparent.as_ref(), parent_index + 1)
        }
    };
    app.dirty = true;
    app.quit_armed = false;
    select_position(app, &amp;target);
    app.status = "node moved (Ctrl-S to save)".into();
}

</t>
<t tx="ekr.20260812053055.76">fn save(app: &amp;mut App) {
    let mut persisted = app.document.clone();
    restore_derived_children(&amp;mut persisted.outline.roots, &amp;app.derived_roots);
    let referenced = referenced_nodes(&amp;persisted.outline.roots);
    persisted
        .outline
        .nodes
        .retain(|id, _| referenced.contains(id));
    match persisted.save(&amp;app.path) {
        Ok(()) =&gt; {
            app.dirty = false;
            app.quit_armed = false;
            app.status = format!("saved {}", app.path.display());
        }
        Err(error) =&gt; app.status = format!("save failed: {error}"),
    }
}

</t>
<t tx="ekr.20260812053055.77">fn restore_derived_children(
    positions: &amp;mut [Position],
    originals: &amp;HashMap&lt;NodeId, Vec&lt;Position&gt;&gt;,
) {
    for position in positions {
        if let Some(children) = originals.get(&amp;position.node) {
            position.children.clone_from(children);
        } else {
            restore_derived_children(&amp;mut position.children, originals);
        }
    }
}

</t>
<t tx="ekr.20260812053055.78">fn referenced_nodes(positions: &amp;[Position]) -&gt; HashSet&lt;NodeId&gt; {
    @others
    visit(positions, &amp;mut result);
    result
}

</t>
<t tx="ekr.20260812053055.79">fn position_contains_derived(app: &amp;App, id: &amp;PositionId) -&gt; bool {
    app.document
        .outline
        .position(id)
        .is_some_and(|position| subtree_contains(&amp;position.children, &amp;app.derived_nodes))
}

</t>
<t tx="ekr.20260812053055.8">fn parse_level(token: &amp;str) -&gt; Option&lt;usize&gt; {
    let inner = token.strip_prefix('*')?;
    if inner.is_empty() {
        return Some(1);
    }
    if inner.bytes().all(|byte| byte == b'*') {
        return Some(1 + inner.len());
    }
    inner.strip_suffix('*')?.parse().ok()
}

</t>
<t tx="ekr.20260812053055.80">fn subtree_contains(positions: &amp;[Position], nodes: &amp;HashSet&lt;NodeId&gt;) -&gt; bool {
    positions.iter().any(|position| {
        nodes.contains(&amp;position.node) || subtree_contains(&amp;position.children, nodes)
    })
}

</t>
<t tx="ekr.20260812053055.81">fn split_position(id: &amp;PositionId) -&gt; Option&lt;(Option&lt;PositionId&gt;, usize)&gt; {
    let (parent, index) =
        id.0.rsplit_once('/')
            .map_or((None, id.0.as_str()), |(p, i)| {
                (Some(PositionId(p.to_owned())), i)
            });
    Some((parent, index.parse().ok()?))
}

</t>
<t tx="ekr.20260812053055.82">fn join_position(parent: Option&lt;&amp;PositionId&gt;, index: usize) -&gt; PositionId {
    PositionId(parent.map_or_else(|| index.to_string(), |p| format!("{}/{index}", p.0)))
}

</t>
<t tx="ekr.20260812053055.83">fn children_mut&lt;'a&gt;(
    outline: &amp;'a mut Outline,
    parent: Option&lt;&amp;PositionId&gt;,
) -&gt; Option&lt;&amp;'a mut Vec&lt;Position&gt;&gt; {
    let Some(parent) = parent else {
        return Some(&amp;mut outline.roots);
    };
    let mut indices = parent.0.split('/').map(str::parse::&lt;usize&gt;);
    let mut position = outline.roots.get_mut(indices.next()?.ok()?)?;
    for index in indices {
        position = position.children.get_mut(index.ok()?)?;
    }
    Some(&amp;mut position.children)
}

</t>
<t tx="ekr.20260812053055.84">fn remove_position(outline: &amp;mut Outline, id: &amp;PositionId) -&gt; Option&lt;Position&gt; {
    let (parent, index) = split_position(id)?;
    let siblings = children_mut(outline, parent.as_ref())?;
    (index &lt; siblings.len()).then(|| siblings.remove(index))
}

</t>
<t tx="ekr.20260812053055.85">fn select_position(app: &amp;mut App, id: &amp;PositionId) {
    if let Some(index) = app.rows().iter().position(|row| &amp;row.position == id) {
        app.selected = index;
    }
}

</t>
<t tx="ekr.20260812053055.86">fn fresh_node_id() -&gt; NodeId {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    NodeId(format!(
        "cub.{}.{}",
        duration.as_secs(),
        duration.subsec_nanos()
    ))
}

</t>
<t tx="ekr.20260812053055.87">fn draw(frame: &amp;mut ratatui::Frame&lt;'_&gt;, app: &amp;mut App) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(1)])
        .split(frame.area());
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(areas[0]);
    let rows = app.rows();
    let items: Vec&lt;_&gt; = rows
        .iter()
        .map(|row| {
            let node = &amp;app.document.outline.nodes[&amp;row.node];
            let marker = if row.has_children {
                if app.expanded.contains(&amp;row.position) {
                    "▾ "
                } else {
                    "▸ "
                }
            } else {
                "  "
            };
            let clone_count = clone_count(&amp;app.document.outline, &amp;row.node);
            let clone = if clone_count &gt; 1 {
                format!(" ×{clone_count}")
            } else {
                String::new()
            };
            ListItem::new(Line::from(vec![
                Span::raw("  ".repeat(row.depth)),
                Span::raw(marker),
                Span::raw(
                    app.input
                        .as_ref()
                        .filter(|input| input.node == row.node)
                        .map_or(node.headline.as_str(), |input| input.value.as_str()),
                ),
                Span::raw(
                    app.input
                        .as_ref()
                        .filter(|input| input.node == row.node)
                        .map_or("", |_| "▏"),
                ),
                Span::styled(clone, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();
    let mut state = ListState::default().with_selected((!rows.is_empty()).then_some(app.selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().title(" Outline ").borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        columns[0],
        &amp;mut state,
    );

    let details = rows
        .get(app.selected)
        .map(|row| {
            let node = &amp;app.document.outline.nodes[&amp;row.node];
            let headline = node.headline.clone();
            let gnx = node.id.0.clone();
            let mut text = Text::from(vec![
                Line::styled(headline, Style::default().add_modifier(Modifier::BOLD)),
                Line::from(format!("GNX: {gnx}")),
                Line::from(format!("Position: {}", row.position.0)),
                Line::from(""),
            ]);
            text.extend(body_text(app, row));
            text
        })
        .unwrap_or_default();
    frame.render_widget(
        Paragraph::new(details).block(Block::default().title(" Node ").borders(Borders::ALL)),
        columns[1],
    );
    frame.render_widget(
        Paragraph::new(format!("{}   [{}]", controls(), app.status))
            .style(Style::default().fg(Color::DarkGray)),
        areas[1],
    );
}

</t>
<t tx="ekr.20260812053055.88">fn controls() -&gt; &amp;'static str {
    #[cfg(feature = "syntax")]
    return "Ctrl-I new  Ctrl-H rename  Ctrl-↑↓←→ move  Ctrl-S save  o open  y syntax  q quit";
    #[cfg(not(feature = "syntax"))]
    "Ctrl-I new  Ctrl-H rename  Ctrl-↑↓←→ move  Ctrl-S save  o open  q quit"
}

</t>
<t tx="ekr.20260812053055.89">fn body_text(app: &amp;mut App, row: &amp;Row) -&gt; Text&lt;'static&gt; {
    let body = app.document.outline.nodes[&amp;row.node].body.clone();
    #[cfg(feature = "syntax")]
    if app.syntax_enabled {
        if let Some(cached) = app.highlight_cache.get(&amp;row.position) {
            return cached.clone();
        }
        let source_path = app
            .source_locations
            .get(&amp;row.position)
            .map(|location| location.path.as_path());
        let highlighted = app.syntax.highlight(&amp;body, source_path);
        app.highlight_cache
            .insert(row.position.clone(), highlighted.clone());
        return highlighted;
    }
    Text::from(body)
}

</t>
<t tx="ekr.20260812053055.9">fn sentinel_content&lt;'a&gt;(line: &amp;'a str, start: &amp;str, end: &amp;str) -&gt; &amp;'a str {
    line.strip_prefix(start)
        .and_then(|s| s.strip_suffix(end))
        .unwrap_or(line)
}

</t>
<t tx="ekr.20260812053055.90">#[derive(Default)]
struct LoadReport {
    loaded: usize,
    errors: Vec&lt;String&gt;,
    locations: HashMap&lt;PositionId, SourceLocation&gt;,
    node_locations: HashMap&lt;NodeId, SourceLocation&gt;,
    derived_nodes: HashSet&lt;NodeId&gt;,
    original_children: HashMap&lt;NodeId, Vec&lt;Position&gt;&gt;,
}

</t>
<t tx="ekr.20260812053055.91">struct DerivedJob {
    position: PositionId,
    path: PathBuf,
}

</t>
<t tx="ekr.20260812053055.92">#[derive(Clone)]
struct SourceLocation {
    path: PathBuf,
    line: usize,
}

</t>
<t tx="ekr.20260812053055.93">fn load_derived_files(outline: &amp;mut Outline, outline_path: &amp;Path) -&gt; LoadReport {
    let jobs = derived_jobs(outline, outline_path);
    let mut report = LoadReport::default();
    for job in jobs {
        let label = job.path.display().to_string();
        let result = fs::read_to_string(&amp;job.path)
            .map_err(|error| error.to_string())
            .and_then(|source| DerivedFile::parse(&amp;source).map_err(|error| error.to_string()))
            .and_then(|derived| {
                let root_node = outline
                    .position(&amp;job.position)
                    .map(|position| position.node.clone())
                    .ok_or_else(|| "derived root position disappeared".to_owned())?;
                let original_children = outline
                    .position(&amp;job.position)
                    .map(|position| position.children.clone())
                    .unwrap_or_default();
                derived
                    .merge_into(outline, &amp;job.position)
                    .map_err(|error| error.to_string())?;
                for (derived_position, line) in &amp;derived.locations {
                    let suffix = derived_position
                        .0
                        .strip_prefix("0")
                        .unwrap_or(&amp;derived_position.0);
                    let position = PositionId(format!("{}{}", job.position.0, suffix));
                    report.locations.insert(
                        position,
                        SourceLocation {
                            path: job.path.clone(),
                            line: *line,
                        },
                    );
                    if let Some(position) = derived.outline.position(derived_position) {
                        report
                            .node_locations
                            .entry(position.node.clone())
                            .or_insert(SourceLocation {
                                path: job.path.clone(),
                                line: *line,
                            });
                    }
                }
                report
                    .original_children
                    .entry(root_node)
                    .or_insert(original_children);
                report.derived_nodes.extend(
                    derived
                        .outline
                        .nodes
                        .keys()
                        .filter(|id| **id != derived.root)
                        .cloned(),
                );
                Ok(())
            });
        match result {
            Ok(()) =&gt; report.loaded += 1,
            Err(error) =&gt; report.errors.push(format!("{label}: {error}")),
        }
    }
    report
}

</t>
<t tx="ekr.20260812053055.94">fn open_selected(terminal: &amp;mut Terminal&lt;CrosstermBackend&lt;io::Stdout&gt;&gt;, app: &amp;mut App) {
    let Some(row) = app.rows().get(app.selected).cloned() else {
        return;
    };
    let Some(location) = app
        .source_locations
        .get(&amp;row.position)
        .or_else(|| app.source_nodes.get(&amp;row.node))
        .cloned()
    else {
        app.status = "selected node has no external source location".to_owned();
        return;
    };
    if let Err(error) = suspend_and_open(terminal, &amp;location) {
        app.status = format!("open failed: {error}");
    } else {
        app.status = format!("opened {}:{}", location.path.display(), location.line);
    }
}

</t>
<t tx="ekr.20260812053055.95">fn suspend_and_open(
    terminal: &amp;mut Terminal&lt;CrosstermBackend&lt;io::Stdout&gt;&gt;,
    location: &amp;SourceLocation,
) -&gt; Result&lt;()&gt; {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    let editor_result = run_editor(location);
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    terminal.clear()?;
    editor_result
}

</t>
<t tx="ekr.20260812053055.96">fn run_editor(location: &amp;SourceLocation) -&gt; Result&lt;()&gt; {
    let editor = env::var("VISUAL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| env::var("EDITOR").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| "vi".to_owned());
    let mut parts = editor.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty editor command"))?;
    let mut command = Command::new(program);
    command.args(parts);
    let name = Path::new(program)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(program);
    let path = location.path.as_os_str();
    match name {
        "vim" | "nvim" | "vi" | "view" | "nano" =&gt; {
            command.arg(format!("+{}", location.line)).arg(path);
        }
        "emacs" | "emacsclient" =&gt; {
            command.arg(format!("+{}:1", location.line)).arg(path);
        }
        "code" | "code-insiders" | "codium" =&gt; {
            command
                .arg("--goto")
                .arg(format!("{}:{}:1", location.path.display(), location.line));
        }
        "hx" | "helix" | "kak" =&gt; {
            command.arg(format!("{}:{}:1", location.path.display(), location.line));
        }
        _ =&gt; {
            command.arg(path);
        }
    }
    let status = command.status()?;
    if !status.success() {
        anyhow::bail!("editor exited with {status}");
    }
    Ok(())
}

</t>
<t tx="ekr.20260812053055.97">fn derived_jobs(outline: &amp;Outline, outline_path: &amp;Path) -&gt; Vec&lt;DerivedJob&gt; {
    @others
    let base = outline_path.parent().unwrap_or_else(|| Path::new("."));
    let mut jobs = Vec::new();
    visit(outline, &amp;outline.roots, "", base, &amp;[], &amp;mut jobs);
    jobs
}

</t>
<t tx="ekr.20260812053055.98">fn thin_filename(headline: &amp;str) -&gt; Option&lt;&amp;str&gt; {
    let (directive, filename) = headline.trim().split_once(char::is_whitespace)?;
    matches!(directive, "@file" | "@thin" | "@file-thin")
        .then(|| strip_path_cruft(filename))
        .filter(|filename| !filename.is_empty())
}

</t>
<t tx="ekr.20260812053055.99">fn path_directive(text: &amp;str) -&gt; Option&lt;String&gt; {
    text.lines().find_map(|line| {
        line.strip_prefix("@path")
            .and_then(|rest| rest.starts_with(char::is_whitespace).then_some(rest))
            .map(strip_path_cruft)
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
    })
}

</t>
</tnodes>
</leo_file>
