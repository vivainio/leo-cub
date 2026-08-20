<?xml version="1.0" encoding="utf-8"?>
<!-- Created by cub: https://github.com/vivainio/leo-cub -->
<leo_file xmlns:leo="https://leo-editor.github.io/leo-editor/namespaces/leo-python-editor/1.1">
<leo_header file_format="2"/>
<globals/>
<preferences/>
<find_panel_settings/>
<vnodes>
  <v t="cub.1787246563.223586211"><vh>GitHub CLI fetch demo</vh>
    <v t="demo.1787246563.226321852.1"><vh>@action bash + jq</vh></v>
    <v t="demo.1787246563.226321852.2"><vh>@action nushell</vh></v>
    <v t="demo.1787246563.226321852.3"><vh>@action python</vh></v>
  </v>
</vnodes>
<tnodes>
  <t tx="demo.1787246563.226321852.1">@language bash
@apply
set -euo pipefail

prs=$(gh pr list --json number,title,author,state,url)
issues=$(gh issue list --json number,title,author,state,url)

jq -n --argjson prs &quot;$prs&quot; --argjson issues &quot;$issues&quot; '
  def entry:
    { (&quot;#\(.number): \(.title)&quot;): { &quot;_body&quot;: &quot;\(.state) · \(.author.login) · \(.url)&quot; } };
  {
    &quot;gnx-prefix&quot;: &quot;gh&quot;,
    &quot;operations&quot;: [
      { &quot;op&quot;: &quot;merge-tree&quot;, &quot;parent-headline&quot;: &quot;Imports/PRs&quot;,
        &quot;tree&quot;: ([$prs[] | entry] | add // {}) },
      { &quot;op&quot;: &quot;merge-tree&quot;, &quot;parent-headline&quot;: &quot;Imports/Issues&quot;,
        &quot;tree&quot;: ([$issues[] | entry] | add // {}) }
    ]
  }
'
</t>
  <t tx="demo.1787246563.226321852.2">@language nu
@apply
def to-tree [items] {
    $items
    | reduce -f {} {|item, acc|
        let key = $&quot;#($item.number): ($item.title)&quot;
        let body = $&quot;($item.state) · ($item.author.login) · ($item.url)&quot;
        $acc | insert $key { _body: $body }
    }
}

let prs = (gh pr list --json number,title,author,state,url | from json)
let issues = (gh issue list --json number,title,author,state,url | from json)

{
    &quot;gnx-prefix&quot;: &quot;gh&quot;,
    operations: [
        { op: &quot;merge-tree&quot;, &quot;parent-headline&quot;: &quot;Imports/PRs&quot;, tree: (to-tree $prs) },
        { op: &quot;merge-tree&quot;, &quot;parent-headline&quot;: &quot;Imports/Issues&quot;, tree: (to-tree $issues) }
    ]
} | to json
</t>
  <t tx="demo.1787246563.226321852.3">@language python
@apply
import json
import subprocess


def gh_json(*args: str) -&gt; list[dict]:
    result = subprocess.run(
        [&quot;gh&quot;, *args, &quot;--json&quot;, &quot;number,title,author,state,url&quot;],
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(result.stdout)


def tree(items: list[dict]) -&gt; dict:
    return {
        f&quot;#{item['number']}: {item['title']}&quot;: {
            &quot;_body&quot;: f&quot;{item['state']} · {item['author']['login']} · {item['url']}&quot;
        }
        for item in items
    }


batch = {
    &quot;gnx-prefix&quot;: &quot;gh&quot;,
    &quot;operations&quot;: [
        {
            &quot;op&quot;: &quot;merge-tree&quot;,
            &quot;parent-headline&quot;: &quot;Imports/PRs&quot;,
            &quot;tree&quot;: tree(gh_json(&quot;pr&quot;, &quot;list&quot;)),
        },
        {
            &quot;op&quot;: &quot;merge-tree&quot;,
            &quot;parent-headline&quot;: &quot;Imports/Issues&quot;,
            &quot;tree&quot;: tree(gh_json(&quot;issue&quot;, &quot;list&quot;)),
        },
    ],
}

print(json.dumps(batch))
</t>
</tnodes>
</leo_file>
