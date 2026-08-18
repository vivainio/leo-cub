<?xml version="1.0" encoding="utf-8"?>
<!-- A working outline you open and read through, adapted from leo-editor's
     quickstart.leo but trimmed to what leo-cub itself supports. See
     getting-started/quickstart-outline.md for the accompanying walkthrough. -->
<leo_file xmlns:leo="https://leo-editor.github.io/leo-editor/namespaces/leo-python-editor/1.1">
<leo_header file_format="2"/>
<globals/>
<preferences/>
<find_panel_settings/>
<vnodes>
  <v t="quickstart.welcome"><vh>Welcome</vh></v>
  <v t="quickstart.move"><vh>Move around</vh>
    <v t="quickstart.move.first"><vh>First stop</vh></v>
    <v t="quickstart.move.second"><vh>Second stop</vh></v>
    <v t="quickstart.move.third"><vh>Third stop</vh></v>
  </v>
  <v t="quickstart.edit"><vh>Edit the tree</vh>
    <v t="quickstart.edit.scratch"><vh>Scratch</vh></v>
  </v>
  <v t="quickstart.find"><vh>Find and search</vh>
    <v t="quickstart.find.quiet"><vh>A quiet node</vh></v>
  </v>
  <v t="quickstart.external"><vh>External files: @clean, @auto, @file</vh></v>
  <v t="quickstart.actions"><vh>Run an action</vh>
    <v t="quickstart.actions.hello"><vh>@action Say hello</vh></v>
    <v t="quickstart.actions.date"><vh>@action Show today's date</vh></v>
  </v>
  <v t="quickstart.automation"><vh>Automate from outside</vh></v>
  <v t="quickstart.next"><vh>Where to go next</vh></v>
</vnodes>
<tnodes>
  <t tx="quickstart.welcome">This is a working Leo outline, not just documentation about one. Open
it in the TUI and read through it node by node:

    cub docs/quickstart.leo

Move with the arrow keys, expand a node with the right arrow or Enter,
collapse it with the left arrow. Press ? at any time for the full key
list, and q to quit - nothing here is saved unless you press Ctrl-S.

Each of the following top-level nodes is a short, self-contained
lesson. Skip around freely; nothing depends on doing them in order.</t>
  <t tx="quickstart.move">Select this node, then press the right arrow to expand it and see
three children below. Move down to each one with the down arrow, then
collapse this node again with the left arrow.

That is the whole vocabulary for browsing: up/down moves the
selection, left/right collapses and expands, Home/End jump to the
first and last visible node.</t>
  <t tx="quickstart.move.first"></t>
  <t tx="quickstart.move.second"></t>
  <t tx="quickstart.move.third">You made it.</t>
  <t tx="quickstart.edit">i inserts a new sibling after the selection; h renames the selected
headline. Ctrl-Right/Left demote and promote a node one level;
Ctrl-Up/Down reorder it among its siblings.

Try it on "Scratch" below: select it, press i, type something, Enter.
Then press h on your new node to rename it to something else. Nothing
is written to disk until Ctrl-S, so experiment freely.

For the complete walkthrough - insert, rename, indent, reorder, copy,
cut, paste, and clone - see "Interactive editing" in the docs.</t>
  <t tx="quickstart.edit.scratch">Practice on this node.</t>
  <t tx="quickstart.find">Ctrl-P finds a headline: press it, type "second", and Enter jumps
straight to "Second stop" above. Escape cancels and returns you to
wherever you started.

/ searches headlines and body text together - try it on the word
"needle", which appears once, hidden in the body of the node below.</t>
  <t tx="quickstart.find.quiet">Nothing to see here, except the word needle.</t>
  <t tx="quickstart.external">Leo can treat a node's body as a mirror of a file on disk instead of
freestanding text. leo-cub understands three flavors:

  @clean &lt;path&gt;   The body is the file, exactly. No sentinel
                        comments, no markers - what you see is what
                        gets written.
  @auto  &lt;path&gt;   The body is reconstructed from the file's own
                        structure (functions, classes, ...), so
                        editing the file externally stays safe.
  @file  &lt;path&gt;   The classic thin format: sentinel comments in
                        the file mark where each node's text begins
                        and ends.

`cub import` builds these from real files or directories; `cub sync`
brings an outline back in step with files that changed outside Leo.
See "First outline" in the docs for a hands-on `cub import` example.</t>
  <t tx="quickstart.actions">Any node headlined "@action &lt;name&gt;" is runnable: press a to open
the action palette, type to filter by name, and Enter runs the
selected node's body as a script. The palette lists every @action node
in the outline, wherever it lives.

Try it now: press a, type "hello", Enter. The body pane below switches
to the command's output until you select a different node.

The interpreter is picked from an @language directive in the body (sh
by default; python, node, ruby, and bash are also recognized).</t>
  <t tx="quickstart.actions.hello">echo "Hello from an action!"
echo "This node's body just ran as a shell script."</t>
  <t tx="quickstart.actions.date">@language python
import datetime
print(datetime.date.today())</t>
  <t tx="quickstart.automation">Everything above works by hand, inside the TUI. The same outline can
be edited from the outside with the same guarantees: `cub inspect`
gives a compact machine-readable view, and `cub apply` performs a
transactional batch of edits described as JSON - all-or-nothing, with
optional preconditions so a script can't silently clobber a change
made since it last read the file.

This is the intended way for agents and CI to edit a .leo file. See
"Automation and AI tools" in the docs for the full JSON schema and
examples.</t>
  <t tx="quickstart.next">- Installation                        getting-started/installation.md
- First outline                       getting-started/first-outline.md
- Interactive editing                 getting-started/interactive-editing.md
- Concepts (GNXs, positions, clones)  reference/concepts.md
- Command reference                   reference/commands.md
- Automation and AI tools             workflows/automation.md</t>
</tnodes>
</leo_file>
