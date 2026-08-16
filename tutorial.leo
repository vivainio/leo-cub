<?xml version="1.0" encoding="utf-8"?>
<!-- Created for the leo-cub documentation tutorial. -->
<leo_file xmlns:leo="https://leo-editor.github.io/leo-editor/namespaces/leo-python-editor/1.1">
<leo_header file_format="2"/>
<globals/>
<preferences/>
<find_panel_settings/>
<vnodes>
  <v t="tutorial.root"><vh>Tutorial project</vh>
    <v t="tutorial.project"><vh>Project</vh>
      <v t="tutorial.source"><vh>Source</vh>
        <v t="tutorial.main"><vh>main.rs</vh></v>
        <v t="tutorial.lib"><vh>lib.rs</vh></v>
        <v t="tutorial.tests"><vh>tests.rs</vh></v>
      </v>
      <v t="tutorial.docs"><vh>Documentation</vh>
        <v t="tutorial.page"><vh>Tutorial</vh></v>
        <v t="tutorial.concepts"><vh>Concepts</vh></v>
        <v t="tutorial.commands"><vh>Command reference</vh></v>
      </v>
      <v t="tutorial.tasks"><vh>Tasks</vh>
        <v t="tutorial.first-task"><vh>Write the first draft</vh></v>
        <v t="tutorial.review"><vh>Review the examples</vh></v>
        <v t="tutorial.publish"><vh>Publish the guide</vh></v>
      </v>
    </v>
    <v t="tutorial.reference"><vh>Reference</vh>
      <v t="tutorial.tasks"></v>
      <v t="tutorial.concepts"></v>
      <v t="tutorial.commands"></v>
    </v>
  </v>
</vnodes>
<tnodes>
  <t tx="tutorial.root">A small outline used throughout the leo-cub tutorial.</t>
  <t tx="tutorial.project">The project tree groups source, documentation, and work items.</t>
  <t tx="tutorial.source">Code belongs here.</t>
  <t tx="tutorial.main">fn main() {
    println!("hello from the outline");
}</t>
  <t tx="tutorial.lib">pub fn greet() -&gt; &amp;'static str {
    "hello"
}</t>
  <t tx="tutorial.tests">#[test]
fn greeting_is_stable() {}</t>
  <t tx="tutorial.docs">Written material belongs here.</t>
  <t tx="tutorial.page">This page is assembled from the outline and rendered by Zensical.</t>
  <t tx="tutorial.concepts">Explain positions, GNXs, and clones.</t>
  <t tx="tutorial.commands">Document the commands used in each step.</t>
  <t tx="tutorial.tasks">Open work items belong here.</t>
  <t tx="tutorial.first-task">Explain how positions and GNXs differ.</t>
  <t tx="tutorial.review">Check that every command has a visible result.</t>
  <t tx="tutorial.publish">Build the site and publish the guide.</t>
  <t tx="tutorial.reference">The same Tasks node is cloned here to demonstrate shared identity.</t>
</tnodes>
</leo_file>
