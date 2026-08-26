<?xml version="1.0" encoding="utf-8"?>
<!-- Created by cub: https://github.com/vivainio/leo-cub -->
<leo_file xmlns:leo="https://leo-editor.github.io/leo-editor/namespaces/leo-python-editor/1.1">
<leo_header file_format="2"/>
<globals/>
<preferences/>
<find_panel_settings/>
<vnodes>
  <v t="cub.1787246563.223586211"><vh>GitHub CLI fetch demo</vh>
    <v t="demo.1787246563.226321852.1"><vh>@action Import PRs and issues</vh></v>
    <v t="cub.1787761438.595926913.1"><vh>@import ../scripts/github.rhai</vh></v>
  </v>
  <v t="cub.1787246563.998877665"><vh>Insert child at selection demo</vh>
    <v t="demo.1787246563.998877665.1"><vh>@action new child</vh></v>
  </v>
  <v t="cub.1787507388.876784675.1"><vh>@variables</vh>
    <v t="cub.1787507388.876784675.2"><vh>repo</vh></v>
  </v>
  <v t="cub.1787689114.613937740.1"><vh>Claude session history import demo</vh>
    <v t="cub.1787689114.613937740.2"><vh>@import ../scripts/claude-history.rhai</vh></v>
  </v>
</vnodes>
<tnodes>
  <t tx="cub.1787507388.876784675.2">leo-editor/leo-editor</t>
  <t tx="cub.1787689114.613937740.1">Imports Claude Code session transcripts into Sessions/&lt;project&gt;/&lt;session&gt;
nodes below, via the import_claude_history command scripts/claude-history.rhai
lists in its COMMANDS map. Run it from the action palette (Shift-A).</t>
  <t tx="demo.1787246563.226321852.1">let vars = #{};
for child in doc.ensure(&quot;@variables&quot;).children() {
    let value = child.b;
    value.trim();
    vars[child.h] = value;
}
let repo_flag = if vars.contains(&quot;repo&quot;) &amp;&amp; vars.repo != &quot;&quot; { &quot; --repo '&quot; + vars.repo + &quot;'&quot; } else { &quot;&quot; };

let opts = #{ cwd: doc.dir() };
let prs = parse_json(sh(&quot;gh pr list --json number,title,author,state,url&quot; + repo_flag, opts).stdout);
let issues = parse_json(sh(&quot;gh issue list --json number,title,author,state,url&quot; + repo_flag, opts).stdout);

for pr in prs {
    let n = doc.ensure(&quot;Imports/PRs/#&quot; + pr.number + &quot;: &quot; + pr.title);
    n.b = pr.state + &quot; · &quot; + pr.author.login + &quot; · &quot; + pr.url;
}
for issue in issues {
    let n = doc.ensure(&quot;Imports/Issues/#&quot; + issue.number + &quot;: &quot; + issue.title);
    n.b = issue.state + &quot; · &quot; + issue.author.login + &quot; · &quot; + issue.url;
}
print(&quot;imported &quot; + prs.len() + &quot; PR(s) and &quot; + issues.len() + &quot; issue(s) from &quot; + (if vars.contains(&quot;repo&quot;) &amp;&amp; vars.repo != &quot;&quot; { vars.repo } else { &quot;the current repo&quot; }));
</t>
  <t tx="demo.1787246563.998877665.1">let t = doc.node(target);
let batch = #{
    operations: [
        #{ op: &quot;insert-tree&quot;, parent: target,
           tree: #{ &quot;New child&quot;: #{ _body: &quot;child of &quot; + t.h } } }
    ]
};
doc.apply(batch.to_json());
print(&quot;added child under &quot; + t.h);
</t>
</tnodes>
</leo_file>
