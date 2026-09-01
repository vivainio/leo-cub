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
    <v t="cub.1787939083.451027836.1"><vh>@auto-dir src/*.rs</vh></v>
  </v>
  <v t="cub.1788202452.440999926"><vh>@auto-dir docs/**</vh></v>
  <v t="cub.1786724143.313584131.1"><vh>@auto README.md</vh></v>
  <v t="cub.1787941913.781965478"><vh>extra notes</vh></v>
</vnodes>
<tnodes>
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
</tnodes>
</leo_file>
