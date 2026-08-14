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
    <v t="None.20260812210606.3"><vh>@clean src/sync.rs</vh></v>
    <v t="None.20260812205242.3"><vh>@clean src/clean.rs</vh></v>
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
  <v t="cub.1786724143.313584131.1"><vh>@auto README.md</vh></v>
</vnodes>
<tnodes>
  <t tx="None.20260812205242.3" _mod_time="4741da9f2cb110dc042e">//! Leo's Mulder/Ream algorithm for propagating public-file edits through
//! sentinelized private text.

use std::ops::Range;

/// Reinsert the sentinels from `old_private` into the edited public text.
///
/// `start_delimiter` and `end_delimiter` describe the sentinel comment. For a
/// line comment such as `#`, pass an empty end delimiter. For a block comment,
/// pass both delimiters, such as `/*` and `*/`.
pub fn propagate_clean_changes(
    new_public: &amp;str,
    old_private: &amp;str,
    start_delimiter: &amp;str,
    end_delimiter: &amp;str,
) -&gt; String {
    let marker = Marker {
        start: start_delimiter,
        end: end_delimiter,
    };
    let (old_public, mut sentinels, trailing) = split_private(old_private, marker);
    let old_public = normalized_lines(&amp;old_public.concat());
    let new_public = normalized_lines(new_public);
    let matches = lcs_matches(&amp;old_public, &amp;new_public);
    let mut result = Vec::new();

    // Leo puts leading sentinels first, then prevents them being emitted twice.
    put_sentinels(&amp;mut result, &amp;sentinels, 0);
    if let Some(first) = sentinels.first_mut() {
        first.clear();
    }

    let mut old_at = 0;
    let mut new_at = 0;
    for (old_match, new_match) in matches
        .into_iter()
        .chain(std::iter::once((old_public.len(), new_public.len())))
    {
        replace_region(
            &amp;mut result,
            &amp;sentinels,
            &amp;new_public,
            old_at..old_match,
            new_at..new_match,
            marker,
        );
        if old_match &lt; old_public.len() {
            put_sentinels(&amp;mut result, &amp;sentinels, old_match);
            put_plain_line(&amp;mut result, &amp;old_public[old_match], marker);
        }
        old_at = old_match + 1;
        new_at = new_match + 1;
    }
    result.extend(trailing);
    result.concat()
}

#[derive(Clone, Copy)]
struct Marker&lt;'a&gt; {
    start: &amp;'a str,
    end: &amp;'a str,
}

impl Marker&lt;'_&gt; {
    fn is_sentinel(self, line: &amp;str) -&gt; bool {
        let line = line.trim();
        line.starts_with(&amp;format!(&quot;{}@&quot;, self.start))
            &amp;&amp; (self.end.is_empty() || line.ends_with(self.end))
    }

    fn is_verbatim(self, line: &amp;str) -&gt; bool {
        let line = line.trim();
        line.starts_with(&amp;format!(&quot;{}@verbatim&quot;, self.start))
            &amp;&amp; (self.end.is_empty() || line.ends_with(self.end))
    }
}

fn normalized_lines(text: &amp;str) -&gt; Vec&lt;String&gt; {
    text.lines().map(|line| format!(&quot;{line}\n&quot;)).collect()
}

fn original_lines(text: &amp;str) -&gt; Vec&lt;String&gt; {
    if text.is_empty() {
        return Vec::new();
    }
    text.split_inclusive('\n').map(str::to_owned).collect()
}

fn split_private(
    old_private: &amp;str,
    marker: Marker&lt;'_&gt;,
) -&gt; (Vec&lt;String&gt;, Vec&lt;Vec&lt;String&gt;&gt;, Vec&lt;String&gt;) {
    let lines = original_lines(old_private);
    let mut public = Vec::new();
    let mut sentinels = Vec::new();
    let mut pending = Vec::new();
    let mut index = 0;
    while index &lt; lines.len() {
        let line = &amp;lines[index];
        index += 1;
        if marker.is_verbatim(line) {
            if let Some(line) = lines.get(index) {
                sentinels.push(std::mem::take(&amp;mut pending));
                public.push(line.clone());
                index += 1;
            }
        } else if marker.is_sentinel(line) {
            pending.push(line.clone());
        } else {
            sentinels.push(std::mem::take(&amp;mut pending));
            public.push(line.clone());
        }
    }
    (public, sentinels, pending)
}

fn replace_region(
    result: &amp;mut Vec&lt;String&gt;,
    sentinels: &amp;[Vec&lt;String&gt;],
    new_lines: &amp;[String],
    old: Range&lt;usize&gt;,
    new: Range&lt;usize&gt;,
    marker: Marker&lt;'_&gt;,
) {
    let mut replacements = new_lines[new].iter();
    for old_index in old {
        put_sentinels(result, sentinels, old_index);
        if let Some(line) = replacements.next() {
            put_plain_line(result, line, marker);
        }
    }
    for line in replacements {
        put_plain_line(result, line, marker);
    }
}

fn put_sentinels(result: &amp;mut Vec&lt;String&gt;, sentinels: &amp;[Vec&lt;String&gt;], index: usize) {
    if let Some(lines) = sentinels.get(index) {
        result.extend(lines.iter().cloned());
    }
}

fn put_plain_line(result: &amp;mut Vec&lt;String&gt;, line: &amp;str, marker: Marker&lt;'_&gt;) {
    if marker.is_sentinel(line) {
        result.push(format!(&quot;{}@verbatim{}\n&quot;, marker.start, marker.end));
    }
    result.push(line.to_owned());
}

/// Return one increasing set of equal-line pairs. This is the set of anchors
/// needed by the propagation rules; regions between anchors are replacements.
fn lcs_matches(old: &amp;[String], new: &amp;[String]) -&gt; Vec&lt;(usize, usize)&gt; {
    let mut lengths = vec![vec![0usize; new.len() + 1]; old.len() + 1];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            lengths[i][j] = if old[i] == new[j] {
                lengths[i + 1][j + 1] + 1
            } else {
                lengths[i + 1][j].max(lengths[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut matches = Vec::new();
    while i &lt; old.len() &amp;&amp; j &lt; new.len() {
        if old[i] == new[j] {
            matches.push((i, j));
            i += 1;
            j += 1;
        } else if lengths[i + 1][j] &gt;= lengths[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIVATE: &amp;str = &quot;#@+leo-ver=5-thin\n#@+node:root: * root\nroot 1\n#@+node:child: ** child\nchild 1\nchild 2\n#@-leo\n&quot;;

    #[test]
    fn propagates_insert_delete_and_replace_without_moving_sentinels() {
        let public = &quot;root changed\ninserted\nchild 2\n&quot;;
        let expected = &quot;#@+leo-ver=5-thin\n#@+node:root: * root\nroot changed\n#@+node:child: ** child\ninserted\nchild 2\n#@-leo\n&quot;;
        assert_eq!(propagate_clean_changes(public, PRIVATE, &quot;#&quot;, &quot;&quot;), expected);
    }

    #[test]
    fn protects_new_lines_that_resemble_sentinels() {
        let public = &quot;root 1\n#@not-a-sentinel-to-the-user\nchild 1\nchild 2\n&quot;;
        let result = propagate_clean_changes(public, PRIVATE, &quot;#&quot;, &quot;&quot;);
        assert!(result.contains(&quot;#@verbatim\n#@not-a-sentinel-to-the-user\n&quot;));
    }

    #[test]
    fn supports_block_comment_sentinels() {
        let private = &quot;/*@+leo-ver=5-thin*/\n/*@+node:r: * r*/\none\n/*@-leo*/\n&quot;;
        let expected = &quot;/*@+leo-ver=5-thin*/\n/*@+node:r: * r*/\ntwo\n/*@-leo*/\n&quot;;
        assert_eq!(
            propagate_clean_changes(&quot;two\n&quot;, private, &quot;/*&quot;, &quot;*/&quot;),
            expected
        );
    }
}
</t>
  <t tx="None.20260812210606.3" _mod_time="4741da9f2f223bfe512e">//! Synchronize external Leo file nodes into an outline.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use thiserror::Error;

use crate::{
    DerivedFile, LeoDocument, NodeId, Outline, Position, PositionId, propagate_clean_changes,
};

#[derive(Debug, Error)]
pub enum SyncError {
    #[error(&quot;I/O error for {path}: {source}&quot;)]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(&quot;unsupported @clean expansion in node {gnx}: {directive}&quot;)]
    UnsupportedExpansion { gnx: String, directive: String },
    #[error(&quot;no external node matches {0:?}&quot;)]
    NoMatch(String),
    #[error(&quot;no external node has GNX {0}&quot;)]
    NoGnx(String),
    #[error(&quot;cloned external node {gnx} resolves to multiple files: {paths:?}&quot;)]
    AmbiguousClone { gnx: String, paths: Vec&lt;PathBuf&gt; },
    #[error(transparent)]
    Sentinel(#[from] crate::SentinelError),
    #[error(&quot;sync produced an invalid outline: {0:?}&quot;)]
    InvalidOutline(Vec&lt;String&gt;),
}

#[derive(Clone, Debug, Serialize)]
pub struct SyncItem {
    pub gnx: NodeId,
    pub path: PathBuf,
    pub directive: String,
    pub changed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SyncReport {
    pub examined: usize,
    pub changed: usize,
    pub dry_run: bool,
    pub items: Vec&lt;SyncItem&gt;,
}

#[derive(Clone)]
struct Job {
    position: PositionId,
    gnx: NodeId,
    path: PathBuf,
    directive: String,
}

/// Sync selected external files into `document`, without writing it.
pub fn sync_document(
    document: &amp;mut LeoDocument,
    outline_path: &amp;Path,
    filename: Option&lt;&amp;str&gt;,
    gnx: Option&lt;&amp;str&gt;,
    dry_run: bool,
) -&gt; Result&lt;SyncReport, SyncError&gt; {
    let jobs = select_jobs(
        external_jobs(&amp;document.outline, outline_path),
        filename,
        gnx,
    )?;
    let mut next = document.clone();
    let mut items = Vec::new();
    for job in jobs {
        let before = next.outline.clone();
        let source = fs::read_to_string(&amp;job.path).map_err(|source| SyncError::Io {
            path: job.path.clone(),
            source,
        })?;
        if job.directive == &quot;@clean&quot; {
            let (start, end) = comment_delimiters(&amp;job.path);
            let private = render_private(&amp;next.outline, &amp;job.position, start, end)?;
            let updated = propagate_clean_changes(&amp;source, &amp;private, start, end);
            let parsed = DerivedFile::parse(&amp;updated)?;
            parsed.merge_into(&amp;mut next.outline, &amp;job.position)?;
        } else {
            // Leo reconstructs @file trees in memory, but their content remains
            // exclusively in the thin derived file. Validate the derived file and
            // its root GNX without materializing that transient tree in the .leo file.
            let parsed = DerivedFile::parse(&amp;source)?;
            let target_node = next
                .outline
                .position(&amp;job.position)
                .map(|target| target.node.clone())
                .ok_or_else(|| crate::SentinelError::PositionNotFound(job.position.0.clone()))?;
            if target_node != parsed.root {
                return Err(crate::SentinelError::RootMismatch {
                    outline: target_node.0.clone(),
                    derived: parsed.root.0,
                }
                .into());
            }
            if let Some(node) = next.outline.nodes.get_mut(&amp;target_node) {
                node.vnode_attributes.remove(&quot;expanded&quot;);
            }
        }
        items.push(SyncItem {
            gnx: job.gnx,
            path: job.path,
            directive: job.directive,
            changed: before != next.outline,
        });
    }
    // Leo treats `_mod_time` as a session-only cache and discards values
    // serialized by older releases whenever an outline is refreshed and saved.
    for node in next.outline.nodes.values_mut() {
        node.tnode_attributes.remove(&quot;_mod_time&quot;);
    }
    let errors = next
        .outline
        .validate()
        .into_iter()
        .map(|error| error.to_string())
        .collect::&lt;Vec&lt;_&gt;&gt;();
    if !errors.is_empty() {
        return Err(SyncError::InvalidOutline(errors));
    }
    let changed = items.iter().filter(|item| item.changed).count();
    if !dry_run {
        *document = next;
    }
    Ok(SyncReport {
        examined: items.len(),
        changed,
        dry_run,
        items,
    })
}

fn select_jobs(
    jobs: Vec&lt;Job&gt;,
    filename: Option&lt;&amp;str&gt;,
    gnx: Option&lt;&amp;str&gt;,
) -&gt; Result&lt;Vec&lt;Job&gt;, SyncError&gt; {
    let mut by_gnx: HashMap&lt;NodeId, Vec&lt;Job&gt;&gt; = HashMap::new();
    for job in jobs {
        by_gnx.entry(job.gnx.clone()).or_default().push(job);
    }
    let mut unique = Vec::new();
    for (id, mut clones) in by_gnx {
        clones.sort_by(|a, b| a.path.cmp(&amp;b.path));
        clones.dedup_by(|a, b| a.path == b.path);
        if clones.len() &gt; 1 {
            return Err(SyncError::AmbiguousClone {
                gnx: id.0,
                paths: clones.into_iter().map(|job| job.path).collect(),
            });
        }
        unique.push(clones.remove(0));
    }
    unique.sort_by(|a, b| a.path.cmp(&amp;b.path));
    if let Some(gnx) = gnx {
        let selected = unique
            .into_iter()
            .filter(|job| job.gnx.0 == gnx)
            .collect::&lt;Vec&lt;_&gt;&gt;();
        return (!selected.is_empty())
            .then_some(selected)
            .ok_or_else(|| SyncError::NoGnx(gnx.to_owned()));
    }
    if let Some(filename) = filename {
        let wanted = Path::new(filename);
        let selected = unique
            .into_iter()
            .filter(|job| job.path == wanted || job.path.ends_with(wanted))
            .collect::&lt;Vec&lt;_&gt;&gt;();
        return (!selected.is_empty())
            .then_some(selected)
            .ok_or_else(|| SyncError::NoMatch(filename.to_owned()));
    }
    Ok(unique)
}

fn external_jobs(outline: &amp;Outline, outline_path: &amp;Path) -&gt; Vec&lt;Job&gt; {
    fn visit(
        outline: &amp;Outline,
        positions: &amp;[Position],
        parent: &amp;str,
        base: &amp;Path,
        inherited_paths: &amp;[String],
        jobs: &amp;mut Vec&lt;Job&gt;,
    ) {
        for (index, position) in positions.iter().enumerate() {
            let id = if parent.is_empty() {
                index.to_string()
            } else {
                format!(&quot;{parent}/{index}&quot;)
            };
            let node = &amp;outline.nodes[&amp;position.node];
            let mut paths = inherited_paths.to_vec();
            if let Some(path) =
                path_directive(&amp;node.headline).or_else(|| path_directive(&amp;node.body))
            {
                paths.push(path);
            }
            if let Some((directive, filename)) = external_filename(&amp;node.headline) {
                let mut path = base.to_path_buf();
                for component in inherited_paths {
                    path.push(component);
                }
                path.push(filename);
                jobs.push(Job {
                    position: PositionId(id.clone()),
                    gnx: position.node.clone(),
                    path,
                    directive: directive.to_owned(),
                });
            }
            visit(outline, &amp;position.children, &amp;id, base, &amp;paths, jobs);
        }
    }
    let mut jobs = Vec::new();
    visit(
        outline,
        &amp;outline.roots,
        &quot;&quot;,
        outline_path.parent().unwrap_or_else(|| Path::new(&quot;.&quot;)),
        &amp;[],
        &amp;mut jobs,
    );
    jobs
}

fn external_filename(headline: &amp;str) -&gt; Option&lt;(&amp;str, &amp;str)&gt; {
    let (directive, filename) = headline.trim().split_once(char::is_whitespace)?;
    matches!(directive, &quot;@file&quot; | &quot;@thin&quot; | &quot;@file-thin&quot; | &quot;@clean&quot;)
        .then(|| (directive, strip_path_cruft(filename)))
        .filter(|(_, filename)| !filename.is_empty())
}

fn path_directive(text: &amp;str) -&gt; Option&lt;String&gt; {
    text.lines().find_map(|line| {
        line.strip_prefix(&quot;@path&quot;)
            .and_then(|rest| rest.starts_with(char::is_whitespace).then_some(rest))
            .map(strip_path_cruft)
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
    })
}

fn strip_path_cruft(path: &amp;str) -&gt; &amp;str {
    let path = path.trim();
    if path.len() &gt; 2 {
        let pair = (path.as_bytes()[0], path.as_bytes()[path.len() - 1]);
        if matches!(pair, (b'&lt;', b'&gt;') | (b'&quot;', b'&quot;') | (b'\'', b'\'')) {
            return path[1..path.len() - 1].trim();
        }
    }
    path
}

fn comment_delimiters(path: &amp;Path) -&gt; (&amp;'static str, &amp;'static str) {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        &quot;py&quot; | &quot;pyw&quot; | &quot;sh&quot; | &quot;bash&quot; | &quot;zsh&quot; | &quot;fish&quot; | &quot;rb&quot; | &quot;pl&quot; | &quot;pm&quot; | &quot;r&quot; | &quot;toml&quot;
        | &quot;yaml&quot; | &quot;yml&quot; =&gt; (&quot;#&quot;, &quot;&quot;),
        &quot;rs&quot; | &quot;c&quot; | &quot;h&quot; | &quot;cc&quot; | &quot;cpp&quot; | &quot;cxx&quot; | &quot;hpp&quot; | &quot;java&quot; | &quot;js&quot; | &quot;jsx&quot; | &quot;ts&quot; | &quot;tsx&quot;
        | &quot;go&quot; | &quot;swift&quot; | &quot;kt&quot; | &quot;kts&quot; | &quot;cs&quot; =&gt; (&quot;//&quot;, &quot;&quot;),
        &quot;html&quot; | &quot;htm&quot; | &quot;xml&quot; | &quot;xhtml&quot; | &quot;svg&quot; =&gt; (&quot;&lt;!--&quot;, &quot;--&gt;&quot;),
        &quot;css&quot; | &quot;scss&quot; | &quot;less&quot; =&gt; (&quot;/*&quot;, &quot;*/&quot;),
        &quot;sql&quot; | &quot;lua&quot; =&gt; (&quot;--&quot;, &quot;&quot;),
        &quot;ini&quot; | &quot;cfg&quot; =&gt; (&quot;#&quot;, &quot;&quot;),
        // The private sentinel stream is an internal merge representation and
        // is never written to the external @clean file. A line-comment fallback
        // therefore supports plain-text and otherwise unknown extensions safely.
        _ =&gt; (&quot;#&quot;, &quot;&quot;),
    }
}

fn render_private(
    outline: &amp;Outline,
    target: &amp;PositionId,
    start: &amp;str,
    end: &amp;str,
) -&gt; Result&lt;String, SyncError&gt; {
    let root = outline
        .position(target)
        .ok_or_else(|| crate::SentinelError::PositionNotFound(target.0.clone()))?;
    ensure_supported_clean_tree(outline, root)?;
    let mut result = format!(&quot;{start}@+leo-ver=5-thin{end}\n&quot;);
    render_position(outline, root, 1, &quot;&quot;, start, end, &amp;mut result);
    result.push_str(&amp;format!(&quot;{start}@-leo{end}\n&quot;));
    Ok(result)
}

fn ensure_supported_clean_tree(outline: &amp;Outline, position: &amp;Position) -&gt; Result&lt;(), SyncError&gt; {
    let node = &amp;outline.nodes[&amp;position.node];
    for line in node.body.lines() {
        let line = line.trim();
        if line == &quot;@all&quot; || (line.starts_with(&quot;&lt;&lt;&quot;) &amp;&amp; line.ends_with(&quot;&gt;&gt;&quot;)) {
            return Err(SyncError::UnsupportedExpansion {
                gnx: node.id.0.clone(),
                directive: line.to_owned(),
            });
        }
    }
    for child in &amp;position.children {
        ensure_supported_clean_tree(outline, child)?;
    }
    Ok(())
}

fn render_position(
    outline: &amp;Outline,
    position: &amp;Position,
    level: usize,
    indent: &amp;str,
    start: &amp;str,
    end: &amp;str,
    result: &amp;mut String,
) {
    let node = &amp;outline.nodes[&amp;position.node];
    let stars = if level &lt;= 5 {
        &quot;*&quot;.repeat(level)
    } else {
        format!(&quot;*{level}*&quot;)
    };
    result.push_str(&amp;format!(
        &quot;{indent}{start}@+node:{}: {stars} {}{end}\n&quot;,
        node.id.0, node.headline
    ));
    let mut expanded = false;
    for line in node.body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.trim_end() == &quot;@others&quot; {
            expanded = true;
            let leading = &amp;line[..line.len() - trimmed.len()];
            let child_indent = format!(&quot;{indent}{leading}&quot;);
            result.push_str(&amp;format!(&quot;{indent}{leading}{start}@+others{end}\n&quot;));
            for child in &amp;position.children {
                render_position(outline, child, level + 1, &amp;child_indent, start, end, result);
            }
            result.push_str(&amp;format!(&quot;{indent}{leading}{start}@-others{end}\n&quot;));
        } else if let Some(directive) = trimmed.strip_prefix('@') {
            result.push_str(&amp;format!(
                &quot;{indent}{start}@@{}{end}\n&quot;,
                directive.trim_end_matches(['\r', '\n'])
            ));
        } else {
            let rendered = format!(&quot;{indent}{line}&quot;);
            if is_sentinel_like(&amp;rendered, start, end) {
                result.push_str(&amp;format!(&quot;{indent}{start}@verbatim{end}\n&quot;));
            }
            result.push_str(&amp;rendered);
            if !line.ends_with('\n') {
                result.push('\n');
            }
        }
    }
    if !expanded &amp;&amp; !position.children.is_empty() {
        result.push_str(&amp;format!(&quot;{indent}{start}@+others{end}\n&quot;));
        for child in &amp;position.children {
            render_position(outline, child, level + 1, indent, start, end, result);
        }
        result.push_str(&amp;format!(&quot;{indent}{start}@-others{end}\n&quot;));
    }
}

fn is_sentinel_like(line: &amp;str, start: &amp;str, end: &amp;str) -&gt; bool {
    let line = line.trim();
    line.starts_with(&amp;format!(&quot;{start}@&quot;)) &amp;&amp; (end.is_empty() || line.ends_with(end))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -&gt; LeoDocument {
        let source = concat!(
@others
            r#&quot;&lt;leo_file&gt;&lt;vnodes&gt;&lt;v t=&quot;r&quot;&gt;&lt;vh&gt;@clean test.py&lt;/vh&gt;&lt;v t=&quot;c&quot;&gt;&lt;vh&gt;child&lt;/vh&gt;&lt;/v&gt;&lt;/v&gt;&lt;/vnodes&gt;&lt;tnodes&gt;&lt;t tx=&quot;r&quot;&gt;root&quot;#,
            &quot;\n@&quot;,
            &quot;others\ntail\n&quot;,
            r#&quot;&lt;/t&gt;&lt;t tx=&quot;c&quot;&gt;child
&lt;/t&gt;&lt;/tnodes&gt;&lt;/leo_file&gt;&quot;#,
        );
        LeoDocument::parse(source).unwrap()
    }

    #[test]
    fn renders_clean_tree_as_parseable_private_text() {
        let doc = document();
        let private = render_private(&amp;doc.outline, &amp;PositionId(&quot;0&quot;.into()), &quot;#&quot;, &quot;&quot;).unwrap();
        let parsed = DerivedFile::parse(&amp;private).unwrap();
        assert_eq!(
            parsed.outline.nodes[&amp;NodeId::from(&quot;r&quot;)].body,
            &quot;root\n@others\ntail\n&quot;
        );
        assert_eq!(parsed.outline.nodes[&amp;NodeId::from(&quot;c&quot;)].body, &quot;child\n&quot;);
    }

    #[test]
    fn dry_run_does_not_mutate_document() {
        let mut doc = document();
        let before = doc.outline.clone();
        let dir = std::env::temp_dir().join(format!(&quot;leo-cub-sync-test-{}&quot;, std::process::id()));
        fs::create_dir_all(&amp;dir).unwrap();
        let outline_path = dir.join(&quot;outline.leo&quot;);
        fs::write(dir.join(&quot;test.py&quot;), &quot;changed\nchild\ntail\n&quot;).unwrap();
        let report = sync_document(&amp;mut doc, &amp;outline_path, None, None, true).unwrap();
        assert_eq!(report.changed, 1);
        assert_eq!(doc.outline, before);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn file_sync_validates_without_materializing_the_derived_tree() {
        let source = concat!(
            r#&quot;&lt;leo_file&gt;&lt;vnodes&gt;&lt;v t=&quot;r&quot;&gt;&lt;vh&gt;@file test.py&lt;/vh&gt;&lt;/v&gt;&lt;/vnodes&gt;&quot;#,
            r#&quot;&lt;tnodes&gt;&lt;t tx=&quot;r&quot;&gt;&lt;/t&gt;&lt;/tnodes&gt;&lt;/leo_file&gt;&quot;#,
        );
        let mut doc = LeoDocument::parse(source).unwrap();
        let before = doc.outline.clone();
        let dir =
            std::env::temp_dir().join(format!(&quot;leo-cub-file-sync-test-{}&quot;, std::process::id()));
        fs::create_dir_all(&amp;dir).unwrap();
        let outline_path = dir.join(&quot;outline.leo&quot;);
        fs::write(
            dir.join(&quot;test.py&quot;),
            &quot;#@+leo-ver=5-thin\n#@+node:r: * @file test.py\n#@+others\n#@+node:c: ** child\nbody\n#@-others\n#@-leo\n&quot;,
        )
        .unwrap();
        let report = sync_document(&amp;mut doc, &amp;outline_path, None, None, false).unwrap();
        assert_eq!(report.changed, 0);
        assert_eq!(doc.outline, before);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn clean_sync_uses_fallback_delimiters_for_plain_text() {
        let source = concat!(
            r#&quot;&lt;leo_file&gt;&lt;vnodes&gt;&lt;v t=&quot;r&quot;&gt;&lt;vh&gt;@clean test.txt&lt;/vh&gt;&lt;/v&gt;&lt;/vnodes&gt;&quot;#,
            r#&quot;&lt;tnodes&gt;&lt;t tx=&quot;r&quot;&gt;old\n&lt;/t&gt;&lt;/tnodes&gt;&lt;/leo_file&gt;&quot;#,
        );
        let mut doc = LeoDocument::parse(source).unwrap();
        let dir =
            std::env::temp_dir().join(format!(&quot;leo-cub-clean-sync-test-{}&quot;, std::process::id()));
        fs::create_dir_all(&amp;dir).unwrap();
        let outline_path = dir.join(&quot;outline.leo&quot;);
        fs::write(dir.join(&quot;test.txt&quot;), &quot;new\n&quot;).unwrap();
        let report = sync_document(&amp;mut doc, &amp;outline_path, None, None, false).unwrap();
        assert_eq!(report.changed, 1);
        assert_eq!(doc.outline.nodes[&amp;NodeId::from(&quot;r&quot;)].body, &quot;new\n&quot;);
        let _ = fs::remove_dir_all(dir);
    }
}
</t>
  <t tx="cub.1786724143.313584131.1"></t>
  <t tx="ekr.20260812052828.2"></t>
  <t tx="ekr.20260812052904.1">&quot;&quot;&quot;Recursively import all python files in a directory and clean the result.&quot;&quot;&quot;
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
        if matches!(pair, (b'&lt;', b'&gt;') | (b'&quot;', b'&quot;') | (b'\'', b'\'')) {
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
        body_scroll: 0,
        body_page_size: 1,
        body_scroll_max: 0,
        help: false,
        status,
        input: None,
        find: None,
        dirty: false,
        quit_armed: false,
        source_locations,
        source_nodes,
        derived_nodes,
        derived_roots,
        #[cfg(feature = &quot;syntax&quot;)]
        syntax: crate::syntax::SyntaxHighlighter::new(),
        #[cfg(feature = &quot;syntax&quot;)]
        syntax_enabled: true,
        #[cfg(feature = &quot;syntax&quot;)]
        highlight_cache: HashMap::new(),
    }
}

</t>
  <t tx="ekr.20260812053055.104">fn rows(&amp;self) -&gt; Vec&lt;Row&gt; {
    @others
    let mut rows = Vec::new();
    visit(
        &amp;self.document.outline.roots,
        &quot;&quot;,
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
    let selected = self.selected.saturating_add_signed(delta).min(len - 1);
    if selected != self.selected {
        self.selected = selected;
        self.body_scroll = 0;
    }
}

fn scroll_body(&amp;mut self, pages: isize) {
    let amount = self.body_page_size.max(1);
    self.body_scroll = self
        .body_scroll
        .saturating_add_signed(pages.saturating_mul(amount as isize))
        .min(self.body_scroll_max);
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
        self.status = &quot;derived descendants are read-only; press o to edit the source&quot;.into();
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
            format!(&quot;{parent_id}/{index}&quot;)
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
        r#&quot;&lt;leo_file&gt;&lt;vnodes&gt;&lt;v t=&quot;a&quot;&gt;&lt;vh&gt;A&lt;/vh&gt;&lt;v t=&quot;b&quot;&gt;&lt;vh&gt;B&lt;/vh&gt;&lt;/v&gt;&lt;v t=&quot;c&quot;&gt;&lt;vh&gt;C&lt;/vh&gt;&lt;/v&gt;&lt;/v&gt;&lt;/vnodes&gt;&lt;tnodes&gt;&lt;t tx=&quot;a&quot;&gt;&lt;/t&gt;&lt;t tx=&quot;b&quot;&gt;&lt;/t&gt;&lt;t tx=&quot;c&quot;&gt;&lt;/t&gt;&lt;/tnodes&gt;&lt;/leo_file&gt;&quot;#,
    )
    .unwrap();
    App::new(
        document,
        PathBuf::from(&quot;test.leo&quot;),
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
    assert_eq!(thin_filename(&quot;@file src/main.rs&quot;), Some(&quot;src/main.rs&quot;));
    assert_eq!(thin_filename(&quot;@file \&quot;src/main.rs\&quot;&quot;), Some(&quot;src/main.rs&quot;));
    assert_eq!(thin_filename(&quot;@clean src/main.rs&quot;), None);
    assert_eq!(thin_filename(&quot;ordinary&quot;), None);
}

</t>
  <t tx="ekr.20260812053055.114">#[test]
fn recognizes_clean_files_for_syntax_context() {
    assert_eq!(external_filename(&quot;@clean src/main.rs&quot;), Some(&quot;src/main.rs&quot;));
    assert_eq!(
        external_filename(&quot;@clean \&quot;src/main.rs\&quot;&quot;),
        Some(&quot;src/main.rs&quot;)
    );
}

#[test]
fn inherits_syntax_context_from_clean_file_ancestor() {
    let document = LeoDocument::parse(
        r#&quot;&lt;leo_file&gt;&lt;vnodes&gt;&lt;v t=&quot;a&quot;&gt;&lt;vh&gt;@clean src/main.rs&lt;/vh&gt;&lt;v t=&quot;b&quot;&gt;&lt;vh&gt;child&lt;/vh&gt;&lt;/v&gt;&lt;/v&gt;&lt;/vnodes&gt;&lt;tnodes&gt;&lt;t tx=&quot;a&quot;&gt;@language rust&lt;/t&gt;&lt;t tx=&quot;b&quot;&gt;fn child() {}&lt;/t&gt;&lt;/tnodes&gt;&lt;/leo_file&gt;&quot;#,
    )
    .unwrap();
    let (language, path) = syntax_context(&amp;document.outline, &amp;PositionId(&quot;0/0&quot;.into()));
    assert_eq!(language.as_deref(), Some(&quot;rust&quot;));
    assert_eq!(path.as_deref(), Some(Path::new(&quot;src/main.rs&quot;)));
}

#[test]
fn extracts_leo_path_directives() {
    assert_eq!(
        path_directive(&quot;@language rust\n@path &lt;src/core&gt;\n&quot;),
        Some(&quot;src/core&quot;.into())
    );
}

</t>
  <t tx="ekr.20260812053055.115">#[test]
fn moves_nodes_with_leo_control_arrow_semantics() {
    let mut app = editing_app();
    select_position(&amp;mut app, &amp;PositionId(&quot;0/1&quot;.into()));
    move_selected(&amp;mut app, MoveDirection::Up);
    assert_eq!(
        app.document.outline.roots[0].children[0].node,
        NodeId::from(&quot;c&quot;)
    );

    move_selected(&amp;mut app, MoveDirection::Down);
    move_selected(&amp;mut app, MoveDirection::Right);
    assert_eq!(
        app.document.outline.roots[0].children[0].children[0].node,
        NodeId::from(&quot;c&quot;)
    );

    move_selected(&amp;mut app, MoveDirection::Left);
    assert_eq!(
        app.document.outline.roots[0].children[1].node,
        NodeId::from(&quot;c&quot;)
    );
}

#[test]
fn finds_headlines_incrementally_and_reveals_collapsed_matches() {
    let mut app = editing_app();
    app.expanded.clear();
    start_find(&amp;mut app);
    handle_find_input(
        &amp;mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
    );

    assert_eq!(
        app.selected_row().unwrap().position,
        PositionId(&quot;0/1&quot;.into())
    );
    assert!(app.expanded.contains(&amp;PositionId(&quot;0&quot;.into())));
    assert_eq!(app.find.as_ref().unwrap().matches.len(), 1);
}

#[test]
fn scrolls_body_by_a_page_and_stays_in_bounds() {
    let mut app = editing_app();
    app.body_page_size = 10;
    app.body_scroll_max = 24;

    app.scroll_body(1);
    assert_eq!(app.body_scroll, 10);
    app.scroll_body(2);
    assert_eq!(app.body_scroll, 24);
    app.scroll_body(-1);
    assert_eq!(app.body_scroll, 14);
    app.scroll_body(-2);
    assert_eq!(app.body_scroll, 0);
}

#[test]
fn changing_selection_resets_body_scroll() {
    let mut app = editing_app();
    app.body_scroll = 12;

    app.move_selection(1);

    assert_eq!(app.selected, 1);
    assert_eq!(app.body_scroll, 0);
}

#[test]
fn find_cycles_matches_and_escape_restores_selection() {
    let mut app = editing_app();
    app.document
        .outline
        .nodes
        .get_mut(&amp;NodeId::from(&quot;a&quot;))
        .unwrap()
        .headline = &quot;Alpha&quot;.into();
    app.document
        .outline
        .nodes
        .get_mut(&amp;NodeId::from(&quot;b&quot;))
        .unwrap()
        .headline = &quot;Beta&quot;.into();
    start_find(&amp;mut app);
    handle_find_input(
        &amp;mut app,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::SHIFT),
    );
    assert_eq!(app.find.as_ref().unwrap().matches.len(), 2);
    handle_find_input(&amp;mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        app.selected_row().unwrap().position,
        PositionId(&quot;0/0&quot;.into())
    );

    handle_find_input(&amp;mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.find.is_none());
    assert_eq!(app.selected_row().unwrap().position, PositionId(&quot;0&quot;.into()));
}

</t>
  <t tx="ekr.20260812053055.116">#[test]
fn restores_external_children_before_serializing() {
    let mut roots = vec![Position {
        node: NodeId::from(&quot;file&quot;),
        children: vec![Position {
            node: NodeId::from(&quot;derived&quot;),
            children: vec![],
        }],
    }];
    let originals = HashMap::from([(NodeId::from(&quot;file&quot;), Vec::new())]);
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
            format!(&quot;{parent}/{index}&quot;)
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
  <t tx="ekr.20260812053055.118" _mod_time="4741da9edef0e8a5152e">@others
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
    #[error(&quot;I/O error: {0}&quot;)]
    Io(#[from] std::io::Error),
    #[error(&quot;XML error: {0}&quot;)]
    Xml(#[from] quick_xml::Error),
    #[error(&quot;invalid Leo file: {0}&quot;)]
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
    let start_tag = format!(&quot;&lt;{tag}&gt;&quot;);
    let end_tag = format!(&quot;&lt;/{tag}&gt;&quot;);
    let start = source
        .find(&amp;start_tag)
        .ok_or_else(|| LeoXmlError::Invalid(format!(&quot;missing {start_tag}&quot;)))?;
    let end = source[start..]
        .find(&amp;end_tag)
        .map(|n| start + n + end_tag.len())
        .ok_or_else(|| LeoXmlError::Invalid(format!(&quot;missing {end_tag}&quot;)))?;
    Ok(format!(
        &quot;{}{}{}&quot;,
        &amp;source[..start],
        replacement,
        &amp;source[end..]
    ))
}

</t>
  <t tx="ekr.20260812053055.126">fn esc(s: &amp;str) -&gt; String {
    s.replace('&amp;', &quot;&amp;amp;&quot;)
        .replace('&lt;', &quot;&amp;lt;&quot;)
        .replace('&gt;', &quot;&amp;gt;&quot;)
        .replace('&quot;', &quot;&amp;quot;&quot;)
}

</t>
  <t tx="ekr.20260812053055.127">fn render_vnodes(outline: &amp;Outline) -&gt; String {
    @others
    let mut out = &quot;&lt;vnodes&gt;\n&quot;.to_owned();
    let mut seen = HashSet::new();
    for p in &amp;outline.roots {
        position(p, outline, &amp;mut seen, &amp;mut out, 1);
    }
    out.push_str(&quot;&lt;/vnodes&gt;&quot;);
    out
}

</t>
  <t tx="ekr.20260812053055.128">fn render_tnodes(outline: &amp;Outline) -&gt; String {
    let mut ids: Vec&lt;_&gt; = outline.nodes.keys().collect();
    ids.sort();
    let mut out = &quot;&lt;tnodes&gt;\n&quot;.to_owned();
    for id in ids {
        let n = &amp;outline.nodes[id];
        let attrs: String = n
            .tnode_attributes
            .iter()
            .map(|(k, v)| format!(&quot; {}=\&quot;{}\&quot;&quot;, k, esc(v)))
            .collect();
        out.push_str(&amp;format!(
            &quot;  &lt;t tx=\&quot;{}\&quot;{attrs}&gt;{}&lt;/t&gt;\n&quot;,
            esc(&amp;id.0),
            esc(&amp;n.body)
        ));
    }
    out.push_str(&quot;&lt;/tnodes&gt;&quot;);
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
            Event::Start(e) if e.name().as_ref() == b&quot;vnodes&quot; =&gt; in_vnodes = true,
            Event::End(e) if e.name().as_ref() == b&quot;vnodes&quot; =&gt; in_vnodes = false,
            Event::Start(e) if e.name().as_ref() == b&quot;tnodes&quot; =&gt; in_tnodes = true,
            Event::End(e) if e.name().as_ref() == b&quot;tnodes&quot; =&gt; in_tnodes = false,
            Event::Start(e) if in_vnodes &amp;&amp; e.name().as_ref() == b&quot;v&quot; =&gt; {
                let attrs = attributes(&amp;e)?;
                let id = NodeId(
                    attrs
                        .get(&quot;t&quot;)
                        .cloned()
                        .ok_or_else(|| LeoXmlError::Invalid(&quot;&lt;v&gt; without t&quot;.into()))?,
                );
                let vnode_attributes = attrs.into_iter().filter(|(k, _)| k != &quot;t&quot;).collect();
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
            Event::Empty(e) if in_vnodes &amp;&amp; e.name().as_ref() == b&quot;v&quot; =&gt; {
                let id = attributes(&amp;e)?;
                let id = NodeId(
                    id.get(&quot;t&quot;)
                        .cloned()
                        .ok_or_else(|| LeoXmlError::Invalid(&quot;&lt;v&gt; without t&quot;.into()))?,
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
            Event::End(e) if in_vnodes &amp;&amp; e.name().as_ref() == b&quot;v&quot; =&gt; {
                let p = stack
                    .pop()
                    .ok_or_else(|| LeoXmlError::Invalid(&quot;unbalanced &lt;v&gt;&quot;.into()))?;
                attach(p, &amp;mut stack, &amp;mut outline.roots);
            }
            Event::Start(e) if in_vnodes &amp;&amp; e.name().as_ref() == b&quot;vh&quot; =&gt; {
                current_vh = stack.last().map(|p| p.node.clone())
            }
            Event::End(e) if e.name().as_ref() == b&quot;vh&quot; =&gt; current_vh = None,
            Event::Start(e) if in_tnodes &amp;&amp; e.name().as_ref() == b&quot;t&quot; =&gt; {
                let attrs = attributes(&amp;e)?;
                let id = NodeId(
                    attrs
                        .get(&quot;tx&quot;)
                        .cloned()
                        .ok_or_else(|| LeoXmlError::Invalid(&quot;&lt;t&gt; without tx&quot;.into()))?,
                );
                let node = outline.nodes.entry(id.clone()).or_insert(Node {
                    id: id.clone(),
                    headline: String::new(),
                    body: String::new(),
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                });
                node.tnode_attributes = attrs.into_iter().filter(|(k, _)| k != &quot;tx&quot;).collect();
                current_t = Some(id);
            }
            Event::End(e) if e.name().as_ref() == b&quot;t&quot; =&gt; current_t = None,
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
                    &quot;amp&quot; =&gt; &quot;&amp;&quot;.to_owned(),
                    &quot;lt&quot; =&gt; &quot;&lt;&quot;.to_owned(),
                    &quot;gt&quot; =&gt; &quot;&gt;&quot;.to_owned(),
                    &quot;quot&quot; =&gt; &quot;\&quot;&quot;.to_owned(),
                    &quot;apos&quot; =&gt; &quot;'&quot;.to_owned(),
                    value if value.starts_with(&quot;#x&quot;) =&gt; u32::from_str_radix(&amp;value[2..], 16)
                        .ok()
                        .and_then(char::from_u32)
                        .map(String::from)
                        .ok_or_else(|| {
                            LeoXmlError::Invalid(format!(&quot;invalid entity &amp;{value};&quot;))
                        })?,
                    value if value.starts_with('#') =&gt; value[1..]
                        .parse::&lt;u32&gt;()
                        .ok()
                        .and_then(char::from_u32)
                        .map(String::from)
                        .ok_or_else(|| {
                            LeoXmlError::Invalid(format!(&quot;invalid entity &amp;{value};&quot;))
                        })?,
                    value =&gt; {
                        return Err(LeoXmlError::Invalid(format!(&quot;unknown entity &amp;{value};&quot;)));
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
        return Err(LeoXmlError::Invalid(&quot;unbalanced vnodes&quot;.into()));
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
    let result = replace_section(&amp;self.original, &quot;vnodes&quot;, &amp;vnodes)?;
    replace_section(&amp;result, &quot;tnodes&quot;, &amp;tnodes)
}

</t>
  <t tx="ekr.20260812053055.132">pub fn save(&amp;self, path: impl AsRef&lt;Path&gt;) -&gt; Result&lt;(), LeoXmlError&gt; {
    let path = path.as_ref();
    let tmp = path.with_extension(&quot;leo.tmp&quot;);
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
    let pad = &quot;  &quot;.repeat(depth);
    if !seen.insert(p.node.clone()) {
        out.push_str(&amp;format!(&quot;{pad}&lt;v t=\&quot;{}\&quot;&gt;&lt;/v&gt;\n&quot;, esc(&amp;p.node.0)));
        return;
    }
    let n = &amp;outline.nodes[&amp;p.node];
    let attrs: String = n
        .vnode_attributes
        .iter()
        .map(|(k, v)| format!(&quot; {}=\&quot;{}\&quot;&quot;, k, esc(v)))
        .collect();
    out.push_str(&amp;format!(
        &quot;{pad}&lt;v t=\&quot;{}\&quot;{attrs}&gt;&lt;vh&gt;{}&lt;/vh&gt;&quot;,
        esc(&amp;p.node.0),
        esc(&amp;n.headline)
    ));
    if p.children.is_empty() {
        out.push_str(&quot;&lt;/v&gt;\n&quot;);
    } else {
        out.push('\n');
        for c in &amp;p.children {
            position(c, outline, seen, out, depth + 1);
        }
        out.push_str(&amp;format!(&quot;{pad}&lt;/v&gt;\n&quot;));
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
    let sentinel_prefix = format!(&quot;{start_delimiter}@&quot;);
    let node_pattern = Regex::new(&amp;format!(
        r&quot;^(\s*){}@\+node:([^:]+): (.*){}\r?\n?$&quot;,
        regex::escape(&amp;start_delimiter),
        regex::escape(&amp;end_delimiter)
    ))
    .expect(&quot;generated node pattern&quot;);

    let mut outline = Outline::default();
    let mut root = None;
    let mut level_paths: Vec&lt;Vec&lt;usize&gt;&gt; = Vec::new();
    let mut locations = HashMap::new();
    let mut current: Option&lt;NodeId&gt; = None;
    let mut indent = 0usize;
    let mut expansions: Vec&lt;(NodeId, usize)&gt; = Vec::new();
    let verbatim = format!(&quot;{start_delimiter}@verbatim{end_delimiter}&quot;);
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
                let joined = format!(&quot;{}{}&quot;, node.body.trim_end(), line);
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
                    .expect(&quot;just pushed&quot;)
                    .iter()
                    .map(usize::to_string)
                    .collect::&lt;Vec&lt;_&gt;&gt;()
                    .join(&quot;/&quot;),
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
        if inner == &quot;@-leo&quot; {
            tail_start = offset + 1;
            break;
        }
        if inner == &quot;@afterref&quot; {
            after_ref = true;
            continue;
        }
        if inner == &quot;@@first&quot; {
            if let Some(first) = lines
                .get(first_index)
                .filter(|_| first_index &lt; header_index)
            {
                append_body(&amp;mut outline, current.as_ref(), &amp;format!(&quot;@first {first}&quot;));
                first_index += 1;
            }
            continue;
        }
        if inner == &quot;@@last&quot; {
            last_count += 1;
            continue;
        }

        if let Some(tail) = inner.strip_prefix(&quot;@+others&quot;) {
            let id = current.clone().ok_or(SentinelError::MissingRoot)?;
            let leading = leading_width(line);
            let local = leading.saturating_sub(indent);
            append_body(
                &amp;mut outline,
                Some(&amp;id),
                &amp;format!(&quot;{}@others{}\n&quot;, &amp;line[indent..indent + local], tail),
            );
            expansions.push((id, indent));
            indent = leading;
            continue;
        }
        if inner.starts_with(&quot;@-others&quot;) {
            (current, indent) = restore_expansion(&amp;mut expansions, line_number)?;
            continue;
        }
        if let Some(tail) = inner.strip_prefix(&quot;@+all&quot;) {
            let id = current.clone().ok_or(SentinelError::MissingRoot)?;
            let leading = leading_width(line);
            let local = leading.saturating_sub(indent);
            append_body(
                &amp;mut outline,
                Some(&amp;id),
                &amp;format!(&quot;{}@all{}\n&quot;, &amp;line[indent..indent + local], tail),
            );
            expansions.push((id, indent));
            indent = leading;
            continue;
        }
        if inner.starts_with(&quot;@-all&quot;) {
            (current, indent) = restore_expansion(&amp;mut expansions, line_number)?;
            continue;
        }
        if let Some(section) = inner.strip_prefix(&quot;@+&lt;&lt;&quot;) {
            let id = current.clone().ok_or(SentinelError::MissingRoot)?;
            let section = section.strip_suffix(&quot;&gt;&gt;&quot;).unwrap_or(section);
            let leading = leading_width(line);
            let local = leading.saturating_sub(indent);
            append_body(
                &amp;mut outline,
                Some(&amp;id),
                &amp;format!(&quot;{}&lt;&lt;{}&gt;&gt;\n&quot;, &amp;line[indent..indent + local], section),
            );
            expansions.push((id, indent));
            indent = leading;
            continue;
        }
        if inner.starts_with(&quot;@-&lt;&lt;&quot;) {
            (current, indent) = restore_expansion(&amp;mut expansions, line_number)?;
            continue;
        }
        if let Some(directive) = inner.strip_prefix(&quot;@@&quot;) {
            append_body(&amp;mut outline, current.as_ref(), &amp;format!(&quot;@{directive}\n&quot;));
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
                &amp;format!(&quot;@last {}\n&quot;, line.trim_end()),
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
        .expect(&quot;validated target&quot;)
        .body = root_body;
    for (id, node) in &amp;self.outline.nodes {
        if id != &amp;self.root {
            if let Some(existing) = next.nodes.get_mut(id) {
                existing.headline.clone_from(&amp;node.headline);
                existing.body.clone_from(&amp;node.body);
            } else {
                next.nodes.insert(id.clone(), node.clone());
            }
        }
    }
    next.children_mut(Some(target))
        .expect(&quot;validated target&quot;)
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
  <t tx="ekr.20260812053055.19" _mod_time="4741da9f2d4ca63d8f2e">//! Clone-aware, automation-safe manipulation of Leo XML outlines.

mod clean;
mod derived;
mod model;
mod operation;
mod sync;
mod xml;

pub use clean::propagate_clean_changes;
pub use derived::{DerivedFile, SentinelError};
pub use model::{Node, NodeId, Outline, Position, PositionId, ValidationError};
pub use operation::{ApplyError, ApplyReport, Operation, OperationBatch, Target};
pub use sync::{SyncError, SyncItem, SyncReport, sync_document};
pub use xml::{LeoDocument, LeoXmlError};
@language rust
@tabwidth -4
</t>
  <t tx="ekr.20260812053055.2"></t>
  <t tx="ekr.20260812053055.20" _mod_time="4741da9f2f584fb3352e">use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use leo::{DerivedFile, LeoDocument, OperationBatch, sync_document};
use std::{fs, path::PathBuf};

@others
@language rust
@tabwidth -4
</t>
  <t tx="ekr.20260812053055.21">#[cfg(all(feature = &quot;tui&quot;, feature = &quot;syntax&quot;))]
mod syntax;
#[cfg(feature = &quot;tui&quot;)]
mod tui;

#[derive(Parser)]
#[command(name = &quot;cub&quot;, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

</t>
  <t tx="ekr.20260812053055.22">#[derive(Subcommand)]
enum Command {
    /// Browse an outline interactively (read-only).
    #[cfg(feature = &quot;tui&quot;)]
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
    /// Synchronize external @file and @clean nodes into an outline.
    Sync {
        file: PathBuf,
        /// External filename to sync. Omit to sync all external nodes.
        external: Option&lt;String&gt;,
        /// Sync the external node with this GNX.
        #[arg(long, conflicts_with = &quot;external&quot;)]
        gnx: Option&lt;String&gt;,
        /// Validate and report changes without writing the outline.
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
        #[cfg(feature = &quot;tui&quot;)]
        Command::Tui { file, no_derived } =&gt; tui::run(file, !no_derived)?,
        Command::Inspect { file } =&gt; println!(
            &quot;{}&quot;,
            serde_json::to_string_pretty(&amp;LeoDocument::open(file)?.outline)?
        ),
        Command::Validate { file } =&gt; {
            let errors = LeoDocument::open(file)?.outline.validate();
            println!(
                &quot;{}&quot;,
                serde_json::to_string_pretty(
                    &amp;errors.iter().map(ToString::to_string).collect::&lt;Vec&lt;_&gt;&gt;()
                )?
            );
            if !errors.is_empty() {
                bail!(&quot;outline is invalid&quot;)
            }
        }
        Command::Apply {
            file,
            operations,
            dry_run,
        } =&gt; {
            let mut doc = LeoDocument::open(&amp;file)?;
            let batch: OperationBatch = serde_json::from_str(
                &amp;fs::read_to_string(operations).context(&quot;read operations file&quot;)?,
            )?;
            let report = doc.outline.apply(&amp;batch)?;
            if !dry_run {
                doc.save(file)?;
            }
            println!(&quot;{}&quot;, serde_json::to_string_pretty(&amp;report)?);
        }
        Command::Sync {
            file,
            external,
            gnx,
            dry_run,
        } =&gt; {
            let mut document = LeoDocument::open(&amp;file)?;
            let report = sync_document(
                &amp;mut document,
                &amp;file,
                external.as_deref(),
                gnx.as_deref(),
                dry_run,
            )?;
            if !dry_run &amp;&amp; report.changed &gt; 0 {
                document.save(&amp;file)?;
            }
            println!(&quot;{}&quot;, serde_json::to_string_pretty(&amp;report)?);
        }
        Command::InspectDerived { derived, summary } =&gt; {
            let source = fs::read_to_string(derived).context(&quot;read derived file&quot;)?;
            let parsed = DerivedFile::parse(&amp;source)?;
            if summary {
                println!(
                    &quot;{}&quot;,
                    serde_json::json!({
                        &quot;root&quot;: parsed.root,
                        &quot;nodes&quot;: parsed.outline.nodes.len(),
                        &quot;positions&quot;: count_positions(&amp;parsed.outline.roots),
                        &quot;start_delimiter&quot;: parsed.start_delimiter,
                        &quot;end_delimiter&quot;: parsed.end_delimiter
                    })
                );
            } else {
                println!(&quot;{}&quot;, serde_json::to_string_pretty(&amp;parsed.outline)?);
            }
        }
        Command::Diff { before, after } =&gt; {
            let a = LeoDocument::open(before)?.outline;
            let b = LeoDocument::open(after)?.outline;
            println!(
                &quot;{}&quot;,
                serde_json::json!({&quot;equal&quot;: a == b, &quot;before&quot;: a, &quot;after&quot;: b})
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
  <t tx="ekr.20260812053055.25" _mod_time="4741da9eded7ef6c852e">use std::collections::{HashMap, HashSet};

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
    #[serde(default, skip_serializing_if = &quot;HashMap::is_empty&quot;)]
    pub vnode_attributes: HashMap&lt;String, String&gt;,
    #[serde(default, skip_serializing_if = &quot;HashMap::is_empty&quot;)]
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
  <t tx="ekr.20260812053055.3" _mod_time="4741da9f2d794b1fc42e">@others
@language rust
@tabwidth -4
</t>
  <t tx="ekr.20260812053055.30">#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    #[error(&quot;position {position} references missing node {node}&quot;)]
    MissingNode { position: String, node: String },
    #[error(&quot;node table key {key} does not match node id {node}&quot;)]
    MismatchedNodeId { key: String, node: String },
    #[error(&quot;node {0} is not referenced by any position&quot;)]
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
        walk(child, format!(&quot;{path}/{i}&quot;), nodes, seen, errors);
    }
}
</t>
  <t tx="ekr.20260812053055.38" _mod_time="4741da9eded7ef6c852e">use serde::{Deserialize, Serialize};
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
#[serde(tag = &quot;op&quot;, rename_all = &quot;kebab-case&quot;)]
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
    #[error(&quot;node already exists: {0}&quot;)]
    DuplicateNode(String),
    #[error(&quot;node not found: {0}&quot;)]
    NodeNotFound(String),
    #[error(&quot;position not found: {0}&quot;)]
    PositionNotFound(String),
    #[error(&quot;index {index} is out of range (length {len})&quot;)]
    BadIndex { index: usize, len: usize },
    #[error(&quot;precondition failed for {field} of node {node}&quot;)]
    Precondition { node: String, field: &amp;'static str },
    #[error(&quot;batch would create an invalid outline: {0}&quot;)]
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
            field: if headline { &quot;headline&quot; } else { &quot;body&quot; },
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
                        .join(&quot;/&quot;),
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
  <t tx="ekr.20260812053055.49" _mod_time="4741da9f1f160d8c102e">use std::path::Path;

use ratatui::{
@others
@language rust
@tabwidth -4
</t>
  <t tx="ekr.20260812053055.5">#[derive(Debug, Error, PartialEq)]
pub enum SentinelError {
    #[error(&quot;no @+leo header sentinel found&quot;)]
    MissingHeader,
    #[error(&quot;derived file contains no @+node sentinel&quot;)]
    MissingRoot,
    #[error(&quot;line {line}: malformed node sentinel: {text}&quot;)]
    MalformedNode { line: usize, text: String },
    #[error(&quot;line {line}: node level {level} has no parent&quot;)]
    MissingParent { line: usize, level: usize },
    #[error(&quot;line {line}: unmatched closing expansion sentinel&quot;)]
    UnmatchedExpansion { line: usize },
    #[error(&quot;unterminated expansion sentinel&quot;)]
    UnterminatedExpansion,
    #[error(&quot;unterminated @verbatim sentinel&quot;)]
    UnterminatedVerbatim,
    #[error(&quot;root GNX mismatch: outline has {outline}, derived file has {derived}&quot;)]
    RootMismatch { outline: String, derived: String },
    #[error(&quot;position not found: {0}&quot;)]
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
  <t tx="ekr.20260812053055.54">pub(crate) fn language_directive(body: &amp;str) -&gt; Option&lt;&amp;str&gt; {
    body.lines().find_map(|line| {
        line.trim_start()
            .strip_prefix(&quot;@language&quot;)
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
  <t tx="ekr.20260812053055.57">pub fn highlight_with_language(
    &amp;self,
    body: &amp;str,
    source_path: Option&lt;&amp;Path&gt;,
    inherited_language: Option&lt;&amp;str&gt;,
) -&gt; Text&lt;'static&gt; {
    let syntax = self.syntax_for(body, source_path, inherited_language);
    let theme = &amp;self.themes.themes[&quot;base16-ocean.dark&quot;];
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
  <t tx="ekr.20260812053055.58">fn syntax_for&lt;'a&gt;(
    &amp;'a self,
    body: &amp;str,
    source_path: Option&lt;&amp;Path&gt;,
    inherited_language: Option&lt;&amp;str&gt;,
) -&gt; &amp;'a SyntaxReference {
    language_directive(body)
        .or(inherited_language)
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
        language_directive(&quot;@tabwidth -4\n@language rust\n&quot;),
        Some(&quot;rust&quot;)
    );
    assert_eq!(language_directive(&quot;let language = rust;&quot;), None);
}

</t>
  <t tx="ekr.20260812053055.6">impl DerivedFile {
    @others
}

</t>
  <t tx="ekr.20260812053055.60">#[test]
fn highlighting_preserves_line_structure_and_whitespace() {
    let text = SyntaxHighlighter::new().highlight_with_language(
        &quot;fn main() {\n    true\n}\n&quot;,
        Some(Path::new(&quot;main.rs&quot;)),
        None,
    );
    assert_eq!(text.lines.len(), 3);
    assert_eq!(text.lines[1].width(), 8);
}
</t>
  <t tx="ekr.20260812053055.61" _mod_time="4741da9f2c5cabaf662e">@others
@language rust
@tabwidth -4
</t>
  <t tx="ekr.20260812053055.62">use std::{
    collections::{HashMap, HashSet},
    env, fs,
    fs::OpenOptions,
    io,
    io::Write,
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
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
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
    body_scroll: usize,
    body_page_size: usize,
    body_scroll_max: usize,
    help: bool,
    status: String,
    input: Option&lt;HeadlineInput&gt;,
    find: Option&lt;FindInput&gt;,
    dirty: bool,
    quit_armed: bool,
    source_locations: HashMap&lt;PositionId, SourceLocation&gt;,
    source_nodes: HashMap&lt;NodeId, SourceLocation&gt;,
    derived_nodes: HashSet&lt;NodeId&gt;,
    derived_roots: HashMap&lt;NodeId, Vec&lt;Position&gt;&gt;,
    #[cfg(feature = &quot;syntax&quot;)]
    syntax: crate::syntax::SyntaxHighlighter,
    #[cfg(feature = &quot;syntax&quot;)]
    syntax_enabled: bool,
    #[cfg(feature = &quot;syntax&quot;)]
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

struct FindInput {
    query: String,
    matches: Vec&lt;PositionId&gt;,
    active: usize,
    original: Option&lt;PositionId&gt;,
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
        Regex::new(r&quot;^(.+)@\+leo(?:-ver=\d+)?(?:-thin)?(?:-encoding=.*?,\.)?(.*)\r?\n?$&quot;).unwrap();
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
            format!(&quot;loaded {} derived file(s)&quot;, report.loaded)
        } else {
            format!(
                &quot;loaded {}; {} error(s): {}&quot;,
                report.loaded,
                report.errors.len(),
                report.errors.join(&quot; | &quot;)
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
            &quot;derived files disabled&quot;.to_owned(),
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
        if app.find.is_some() {
            handle_find_input(app, key);
            continue;
        }
        if app.help {
            if matches!(
                key.code,
                KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Esc
            ) {
                app.help = false;
            }
            continue;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('p') =&gt; start_find(app),
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
            KeyCode::Char('?') =&gt; app.help = true,
            KeyCode::Char('q') | KeyCode::Esc =&gt; {
                if !app.dirty || app.quit_armed {
                    return Ok(());
                }
                app.quit_armed = true;
                app.status = &quot;unsaved changes; press q again to discard, or Ctrl-S to save&quot;.into();
            }
            KeyCode::Char('o') =&gt; open_selected(terminal, app),
            KeyCode::Tab =&gt; insert_headline(app),
            KeyCode::Backspace =&gt; edit_headline(app),
            #[cfg(feature = &quot;syntax&quot;)]
            KeyCode::Char('y') =&gt; {
                app.syntax_enabled = !app.syntax_enabled;
                app.status = format!(
                    &quot;syntax highlighting {}&quot;,
                    if app.syntax_enabled { &quot;on&quot; } else { &quot;off&quot; }
                );
            }
            KeyCode::Down | KeyCode::Char('j') =&gt; app.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') =&gt; app.move_selection(-1),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter =&gt; app.toggle(true),
            KeyCode::Left | KeyCode::Char('h') =&gt; app.toggle(false),
            KeyCode::Home =&gt; {
                app.selected = 0;
                app.body_scroll = 0;
            }
            KeyCode::End =&gt; {
                app.selected = app.rows().len().saturating_sub(1);
                app.body_scroll = 0;
            }
            KeyCode::PageUp =&gt; app.scroll_body(-1),
            KeyCode::PageDown =&gt; app.scroll_body(1),
            _ =&gt; {}
        }
    }
}

fn start_find(app: &amp;mut App) {
    let original = app.selected_row().map(|row| row.position);
    app.find = Some(FindInput {
        query: String::new(),
        matches: Vec::new(),
        active: 0,
        original,
    });
    app.status = &quot;find headline: type to search, Enter accepts, Esc cancels&quot;.into();
}

fn handle_find_input(app: &amp;mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter =&gt; {
            app.find = None;
            app.status = &quot;headline selected&quot;.into();
        }
        KeyCode::Esc =&gt; {
            let original = app.find.take().and_then(|find| find.original);
            if let Some(position) = original {
                reveal_and_select(app, &amp;position);
            }
            app.status = &quot;headline find cancelled&quot;.into();
        }
        KeyCode::Backspace =&gt; {
            app.find.as_mut().expect(&quot;find input exists&quot;).query.pop();
            update_find_matches(app, 0);
        }
        KeyCode::Down =&gt; cycle_find_match(app, 1),
        KeyCode::Up =&gt; cycle_find_match(app, -1),
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =&gt;
        {
            app.find
                .as_mut()
                .expect(&quot;find input exists&quot;)
                .query
                .push(character);
            update_find_matches(app, 0);
        }
        _ =&gt; {}
    }
}

fn update_find_matches(app: &amp;mut App, active: usize) {
    let query = app
        .find
        .as_ref()
        .expect(&quot;find input exists&quot;)
        .query
        .to_lowercase();
    let matches = if query.is_empty() {
        Vec::new()
    } else {
        all_rows(&amp;app.document.outline)
            .into_iter()
            .filter(|row| {
                app.document.outline.nodes[&amp;row.node]
                    .headline
                    .to_lowercase()
                    .contains(&amp;query)
            })
            .map(|row| row.position)
            .collect::&lt;Vec&lt;_&gt;&gt;()
    };
    let active = active.min(matches.len().saturating_sub(1));
    let selected = matches.get(active).cloned();
    let find = app.find.as_mut().expect(&quot;find input exists&quot;);
    find.matches = matches;
    find.active = active;
    if let Some(position) = selected {
        reveal_and_select(app, &amp;position);
    }
}

fn cycle_find_match(app: &amp;mut App, delta: isize) {
    let find = app.find.as_ref().expect(&quot;find input exists&quot;);
    if find.matches.is_empty() {
        return;
    }
    let len = find.matches.len() as isize;
    let active = (find.active as isize + delta).rem_euclid(len) as usize;
    let position = find.matches[active].clone();
    app.find.as_mut().expect(&quot;find input exists&quot;).active = active;
    reveal_and_select(app, &amp;position);
}

fn all_rows(outline: &amp;Outline) -&gt; Vec&lt;Row&gt; {
    fn collect(position: &amp;Position, path: String, depth: usize, rows: &amp;mut Vec&lt;Row&gt;) {
        rows.push(Row {
            position: PositionId(path.clone()),
            node: position.node.clone(),
            depth,
            has_children: !position.children.is_empty(),
        });
        for (index, child) in position.children.iter().enumerate() {
            collect(child, format!(&quot;{path}/{index}&quot;), depth + 1, rows);
        }
    }
    let mut rows = Vec::new();
    for (index, root) in outline.roots.iter().enumerate() {
        collect(root, index.to_string(), 0, &amp;mut rows);
    }
    rows
}

fn reveal_and_select(app: &amp;mut App, position: &amp;PositionId) {
    let components = position.0.split('/').collect::&lt;Vec&lt;_&gt;&gt;();
    for end in 1..components.len() {
        app.expanded.insert(PositionId(components[..end].join(&quot;/&quot;)));
    }
    select_position(app, position);
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
                app.status = &quot;headline may not be empty&quot;.into();
                return;
            }
            app.document
                .outline
                .nodes
                .get_mut(&amp;input.node)
                .expect(&quot;edited node exists&quot;)
                .headline = headline;
            app.input = None;
            app.dirty = true;
            app.quit_armed = false;
            app.status = &quot;headline changed (Ctrl-S to save)&quot;.into();
        }
        KeyCode::Esc =&gt; {
            let input = app.input.take().expect(&quot;input exists&quot;);
            if let Some(position) = input.inserted_position {
                remove_position(&amp;mut app.document.outline, &amp;position);
                app.document.outline.nodes.remove(&amp;input.node);
            } else {
                app.document
                    .outline
                    .nodes
                    .get_mut(&amp;input.node)
                    .expect(&quot;node exists&quot;)
                    .headline = input.original;
            }
            app.status = &quot;headline edit cancelled&quot;.into();
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
    app.status = &quot;editing headline: Enter accepts, Esc cancels&quot;.into();
}

</t>
  <t tx="ekr.20260812053055.74">fn insert_headline(app: &amp;mut App) {
    let Some(row) = app.selected_row() else {
        return;
    };
    if !app.editable(&amp;row) || position_contains_derived(app, &amp;row.position) {
        app.status = &quot;cannot insert beside an external derived subtree&quot;.into();
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
            headline: &quot;New Headline&quot;.into(),
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
    app.status = &quot;new headline: type a name, Enter accepts, Esc cancels&quot;.into();
}

</t>
  <t tx="ekr.20260812053055.75">fn move_selected(app: &amp;mut App, direction: MoveDirection) {
    let Some(row) = app.selected_row() else {
        return;
    };
    if !app.editable(&amp;row) || position_contains_derived(app, &amp;row.position) {
        app.status = &quot;external derived subtrees cannot be moved&quot;.into();
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
                app.status = &quot;node is already at the edge&quot;.into();
                return;
            };
            siblings.swap(index, other);
            join_position(parent.as_ref(), other)
        }
        MoveDirection::Right =&gt; {
            let Some(previous) = index.checked_sub(1) else {
                app.status = &quot;no previous sibling to become parent&quot;.into();
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
                app.status = &quot;top-level nodes cannot be promoted&quot;.into();
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
    app.status = &quot;node moved (Ctrl-S to save)&quot;.into();
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
            app.status = format!(&quot;saved {}&quot;, app.path.display());
        }
        Err(error) =&gt; app.status = format!(&quot;save failed: {error}&quot;),
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
    PositionId(parent.map_or_else(|| index.to_string(), |p| format!(&quot;{}/{index}&quot;, p.0)))
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
        if index != app.selected {
            app.body_scroll = 0;
        }
        app.selected = index;
    }
}

</t>
  <t tx="ekr.20260812053055.86">fn fresh_node_id() -&gt; NodeId {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    NodeId(format!(
        &quot;cub.{}.{}&quot;,
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
                    &quot;▾ &quot;
                } else {
                    &quot;▸ &quot;
                }
            } else {
                &quot;  &quot;
            };
            let clone_count = clone_count(&amp;app.document.outline, &amp;row.node);
            let clone = if clone_count &gt; 1 {
                format!(&quot; ×{clone_count}&quot;)
            } else {
                String::new()
            };
            ListItem::new(Line::from(vec![
                Span::raw(&quot;  &quot;.repeat(row.depth)),
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
                        .map_or(&quot;&quot;, |_| &quot;▏&quot;),
                ),
                Span::styled(clone, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();
    let mut state = ListState::default().with_selected((!rows.is_empty()).then_some(app.selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().title(&quot; Outline &quot;).borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        columns[0],
        &amp;mut state,
    );

    let node_block = Block::default().title(&quot; Node &quot;).borders(Borders::ALL);
    let node_area = node_block.inner(columns[1]);
    frame.render_widget(node_block, columns[1]);
    if let Some(row) = rows.get(app.selected) {
        let node = &amp;app.document.outline.nodes[&amp;row.node];
        let metadata = Text::from(vec![
            Line::styled(
                node.headline.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Line::from(format!(&quot;GNX: {}&quot;, node.id.0)),
            Line::from(format!(&quot;Position: {}&quot;, row.position.0)),
        ]);
        let node_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(0)])
            .split(node_area);
        frame.render_widget(Paragraph::new(metadata), node_rows[0]);

        let body = body_text(app, row);
        app.body_page_size = usize::from(node_rows[1].height).max(1);
        app.body_scroll_max = body.lines.len().saturating_sub(app.body_page_size);
        app.body_scroll = app.body_scroll.min(app.body_scroll_max);
        frame.render_widget(
            Paragraph::new(body).scroll((app.body_scroll.min(u16::MAX as usize) as u16, 0)),
            node_rows[1],
        );
    }
    frame.render_widget(
        Paragraph::new(format!(&quot;{}   [{}]&quot;, controls(), app.status))
            .style(Style::default().fg(Color::DarkGray)),
        areas[1],
    );
    if let Some(find) = &amp;app.find {
        let width = frame.area().width.saturating_sub(4).min(72);
        let shown = find.matches.len().min(5);
        let height = (3 + shown as u16).min(frame.area().height.saturating_sub(2));
        let area = Rect::new(
            frame.area().x + (frame.area().width.saturating_sub(width)) / 2,
            frame.area().y + 1,
            width,
            height,
        );
        let count = if find.query.is_empty() {
            String::new()
        } else if find.matches.is_empty() {
            &quot;no matches&quot;.into()
        } else {
            format!(&quot;{} of {}&quot;, find.active + 1, find.matches.len())
        };
        let rows = all_rows(&amp;app.document.outline);
        let first = find.active.saturating_sub(4);
        let mut lines = vec![Line::from(format!(&quot;&gt; {}▏&quot;, find.query))];
        lines.extend(find.matches.iter().enumerate().skip(first).take(5).map(
            |(index, position)| {
                let row = rows
                    .iter()
                    .find(|row| &amp;row.position == position)
                    .expect(&quot;matched position exists&quot;);
                let marker = if index == find.active { &quot;› &quot; } else { &quot;  &quot; };
                Line::from(format!(
                    &quot;{marker}{}&quot;,
                    app.document.outline.nodes[&amp;row.node].headline
                ))
            },
        ));
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(lines).block(
                Block::default()
                    .title(format!(&quot; Find headline  {count} &quot;))
                    .borders(Borders::ALL),
            ),
            area,
        );
    }
    if app.help {
        draw_help(frame);
    }
}

</t>
  <t tx="ekr.20260812053055.88">fn controls() -&gt; &amp;'static str {
    #[cfg(feature = &quot;syntax&quot;)]
    return &quot;? help  PgUp/PgDn body  o open/edit  Ctrl-P find  Ctrl-I new  Ctrl-H rename  Ctrl-↑↓←→ move  Ctrl-S save  y syntax  q quit&quot;;
    #[cfg(not(feature = &quot;syntax&quot;))]
    &quot;? help  PgUp/PgDn body  o open/edit  Ctrl-P find  Ctrl-I new  Ctrl-H rename  Ctrl-↑↓←→ move  Ctrl-S save  q quit&quot;
}

fn draw_help(frame: &amp;mut ratatui::Frame&lt;'_&gt;) {
    let width = frame.area().width.saturating_sub(4).min(72);
    let height = frame.area().height.saturating_sub(2).min(20);
    let area = Rect::new(
        frame.area().x + (frame.area().width.saturating_sub(width)) / 2,
        frame.area().y + (frame.area().height.saturating_sub(height)) / 2,
        width,
        height,
    );
    let lines = vec![
        Line::from(&quot;↑/↓ or k/j       Select previous/next node&quot;),
        Line::from(&quot;←/→ or h/l       Collapse/expand node&quot;),
        Line::from(&quot;Enter            Expand node&quot;),
        Line::from(&quot;Home/End         Select first/last visible node&quot;),
        Line::from(&quot;PageUp/PageDown  Scroll the body pane&quot;),
        Line::from(&quot;Ctrl-P           Find a headline&quot;),
        Line::from(&quot;Ctrl-I or Tab    Insert a sibling&quot;),
        Line::from(&quot;Ctrl-H/Backspace Rename the headline&quot;),
        Line::from(&quot;Ctrl-↑↓←→        Move/promote/demote node&quot;),
        Line::from(&quot;Ctrl-S           Save&quot;),
        Line::from(&quot;o                Edit body, or open derived source&quot;),
        #[cfg(feature = &quot;syntax&quot;)]
        Line::from(&quot;y                Toggle syntax highlighting&quot;),
        Line::from(&quot;q or Esc         Quit&quot;),
        Line::from(&quot;&quot;),
        Line::styled(
            &quot;Press ?, q, or Esc to close&quot;,
            Style::default().fg(Color::DarkGray),
        ),
    ];
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(&quot; Command help &quot;)
                .borders(Borders::ALL),
        ),
        area,
    );
}

</t>
  <t tx="ekr.20260812053055.89">fn body_text(app: &amp;mut App, row: &amp;Row) -&gt; Text&lt;'static&gt; {
    let body = app.document.outline.nodes[&amp;row.node].body.clone();
    #[cfg(feature = &quot;syntax&quot;)]
    if app.syntax_enabled {
        if let Some(cached) = app.highlight_cache.get(&amp;row.position) {
            return cached.clone();
        }
        let (inherited_language, external_path) =
            syntax_context(&amp;app.document.outline, &amp;row.position);
        let source_path = app
            .source_locations
            .get(&amp;row.position)
            .map(|location| location.path.as_path())
            .or(external_path.as_deref());
        let highlighted =
            app.syntax
                .highlight_with_language(&amp;body, source_path, inherited_language.as_deref());
        app.highlight_cache
            .insert(row.position.clone(), highlighted.clone());
        return highlighted;
    }
    Text::from(body)
}

#[cfg(feature = &quot;syntax&quot;)]
fn syntax_context(outline: &amp;Outline, position: &amp;PositionId) -&gt; (Option&lt;String&gt;, Option&lt;PathBuf&gt;) {
    let mut language = None;
    let mut source_path = None;
    let mut prefix = String::new();

    for component in position.0.split('/') {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(component);
        let Some(position) = outline.position(&amp;PositionId(prefix.clone())) else {
            break;
        };
        let node = &amp;outline.nodes[&amp;position.node];
        if let Some(value) = crate::syntax::language_directive(&amp;node.body) {
            language = Some(value.to_owned());
        }
        if let Some(filename) = external_filename(&amp;node.headline) {
            source_path = Some(PathBuf::from(filename));
        }
    }

    (language, source_path)
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
                    .ok_or_else(|| &quot;derived root position disappeared&quot;.to_owned())?;
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
                        .strip_prefix(&quot;0&quot;)
                        .unwrap_or(&amp;derived_position.0);
                    let position = PositionId(format!(&quot;{}{}&quot;, job.position.0, suffix));
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
            Err(error) =&gt; report.errors.push(format!(&quot;{label}: {error}&quot;)),
        }
    }
    report
}

</t>
  <t tx="ekr.20260812053055.94">fn open_selected(terminal: &amp;mut Terminal&lt;CrosstermBackend&lt;io::Stdout&gt;&gt;, app: &amp;mut App) {
    let Some(row) = app.rows().get(app.selected).cloned() else {
        return;
    };
    if let Some(location) = app
        .source_locations
        .get(&amp;row.position)
        .or_else(|| app.source_nodes.get(&amp;row.node))
        .cloned()
    {
        if let Err(error) = suspend_and_open(terminal, &amp;location) {
            app.status = format!(&quot;open failed: {error}&quot;);
        } else {
            app.status = format!(&quot;opened {}:{}&quot;, location.path.display(), location.line);
        }
        return;
    }

    if !app.editable(&amp;row) {
        return;
    }
    match edit_body_in_temp_file(terminal, &amp;app.document.outline.nodes[&amp;row.node].body) {
        Ok(Some(body)) =&gt; {
            app.document
                .outline
                .nodes
                .get_mut(&amp;row.node)
                .expect(&quot;edited node exists&quot;)
                .body = body;
            #[cfg(feature = &quot;syntax&quot;)]
            app.highlight_cache.clear();
            app.body_scroll = 0;
            app.dirty = true;
            app.quit_armed = false;
            app.status = &quot;body changed (Ctrl-S to save)&quot;.into();
        }
        Ok(None) =&gt; app.status = &quot;body unchanged&quot;.into(),
        Err(error) =&gt; app.status = format!(&quot;body edit failed: {error}&quot;),
    }
}

fn edit_body_in_temp_file(
    terminal: &amp;mut Terminal&lt;CrosstermBackend&lt;io::Stdout&gt;&gt;,
    original: &amp;str,
) -&gt; Result&lt;Option&lt;String&gt;&gt; {
    let path = unique_body_temp_path();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&amp;path)?;
    file.write_all(original.as_bytes())?;
    drop(file);

    let location = SourceLocation {
        path: path.clone(),
        line: 1,
    };
    let editor_result = suspend_and_open(terminal, &amp;location);
    let result = editor_result.and_then(|()| {
        let edited = fs::read_to_string(&amp;path)?;
        Ok((edited != original).then_some(edited))
    });
    let _ = fs::remove_file(path);
    result
}

fn unique_body_temp_path() -&gt; PathBuf {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    env::temp_dir().join(format!(
        &quot;leo-cub-body-{}-{}-{}.txt&quot;,
        std::process::id(),
        duration.as_secs(),
        duration.subsec_nanos()
    ))
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
    let editor = env::var(&quot;VISUAL&quot;)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| env::var(&quot;EDITOR&quot;).ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| &quot;vi&quot;.to_owned());
    let mut parts = editor.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!(&quot;empty editor command&quot;))?;
    let mut command = Command::new(program);
    command.args(parts);
    let name = Path::new(program)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(program);
    let path = location.path.as_os_str();
    match name {
        &quot;vim&quot; | &quot;nvim&quot; | &quot;vi&quot; | &quot;view&quot; | &quot;nano&quot; =&gt; {
            command.arg(format!(&quot;+{}&quot;, location.line)).arg(path);
        }
        &quot;emacs&quot; | &quot;emacsclient&quot; =&gt; {
            command.arg(format!(&quot;+{}:1&quot;, location.line)).arg(path);
        }
        &quot;code&quot; | &quot;code-insiders&quot; | &quot;codium&quot; =&gt; {
            command
                .arg(&quot;--goto&quot;)
                .arg(format!(&quot;{}:{}:1&quot;, location.path.display(), location.line));
        }
        &quot;edit&quot; | &quot;msedit&quot; | &quot;hx&quot; | &quot;helix&quot; | &quot;kak&quot; =&gt; {
            command.arg(format!(&quot;{}:{}:1&quot;, location.path.display(), location.line));
        }
        _ =&gt; {
            command.arg(path);
        }
    }
    let status = command.status()?;
    if !status.success() {
        anyhow::bail!(&quot;editor exited with {status}&quot;);
    }
    Ok(())
}

</t>
  <t tx="ekr.20260812053055.97">fn derived_jobs(outline: &amp;Outline, outline_path: &amp;Path) -&gt; Vec&lt;DerivedJob&gt; {
    @others
    let base = outline_path.parent().unwrap_or_else(|| Path::new(&quot;.&quot;));
    let mut jobs = Vec::new();
    visit(outline, &amp;outline.roots, &quot;&quot;, base, &amp;[], &amp;mut jobs);
    jobs
}

</t>
  <t tx="ekr.20260812053055.98">fn thin_filename(headline: &amp;str) -&gt; Option&lt;&amp;str&gt; {
    let (directive, filename) = headline.trim().split_once(char::is_whitespace)?;
    matches!(directive, &quot;@file&quot; | &quot;@thin&quot; | &quot;@file-thin&quot;)
        .then(|| strip_path_cruft(filename))
        .filter(|filename| !filename.is_empty())
}

fn external_filename(headline: &amp;str) -&gt; Option&lt;&amp;str&gt; {
    let (directive, filename) = headline.trim().split_once(char::is_whitespace)?;
    matches!(directive, &quot;@file&quot; | &quot;@thin&quot; | &quot;@file-thin&quot; | &quot;@clean&quot;)
        .then(|| strip_path_cruft(filename))
        .filter(|filename| !filename.is_empty())
}

</t>
  <t tx="ekr.20260812053055.99">fn path_directive(text: &amp;str) -&gt; Option&lt;String&gt; {
    text.lines().find_map(|line| {
        line.strip_prefix(&quot;@path&quot;)
            .and_then(|rest| rest.starts_with(char::is_whitespace).then_some(rest))
            .map(strip_path_cruft)
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
    })
}

</t>
</tnodes>
</leo_file>
