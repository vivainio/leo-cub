<?xml version="1.0" encoding="utf-8"?>
<!-- Created by cub: https://github.com/vivainio/leo-cub -->
<leo_file>
<leo_header file_format="2"/>
<globals/>
<preferences/>
<find_panel_settings/>
<vnodes>
  <v t="cub.1786725356.346248974.1"><vh>@auto demo.cs</vh></v>
  <v t="cub.1786725356.346248974.2"><vh>@auto demo.js</vh></v>
  <v t="cub.1786725356.346248974.3"><vh>@auto demo.ts</vh></v>
  <v t="cub.1786725356.346248974.4"><vh>@auto demo.go</vh></v>
  <v t="cub.1786725356.346248974.5"><vh>@f ../scripts/github.rhai</vh>
    <v t="cub.1786725356.346248974.5::f-anon:33"><vh>fn shq</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:40"><vh>fn sh_in</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:49"><vh>fn get_variables</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:65"><vh>fn gh_repo_flag</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:79"><vh>fn gh_pr_list</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:84"><vh>fn gh_issue_list</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:89"><vh>fn gh_pr_view</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:94"><vh>fn gh_issue_view</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:99"><vh>fn gh_parse_url</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:118"><vh>fn gh_view_url</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:140"><vh>fn available_commands</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:159"><vh>fn gh_import_url</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:164"><vh>fn gh_pr_create</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:169"><vh>fn gh_issue_create</vh></v>
    <v t="cub.1786725356.346248974.5::f-anon:195"><vh>fn import_prs_and_issues</vh></v>
  </v>
</vnodes>
<tnodes>
  <t tx="cub.1786725356.346248974.5">// GitHub CLI (`gh`) wrappers for cub's rhai Doc API.
//
// An outline node with headline `@import scripts/github.rhai` (path
// resolved relative to the open .leo file's own directory) makes every
// name in COMMANDS below runnable from the action palette (Shift-A).
// `gh` commands run through `sh_in(doc, ...)`, which `cd`s into the open
// .leo file's directory first, so these assume that's inside (or
// dominated by) a git repo with a GitHub remote and an authenticated `gh`
// (`gh auth status`).
//
// COMMANDS maps the functions meant to be exposed as runnable commands to
// a one-line description of what running them does, shown alongside each
// in the palette -- everything else here (shq, get_variables,
// gh_repo_flag, gh_parse_url, gh_view_url, and the functions needing more
// than `doc`/`target`) is a library helper, callable from other scripts or
// the REPL but not listed for direct invocation. Every listed function
// takes exactly `(doc, target)` -- `target` is a `Node` for whatever was
// selected when the command was run (the current position), with
// `.h`/`.b`/`.gnx`/etc. straight away, no `doc.node(...)` needed first.
// Most below don't happen to need it, but the palette always passes it
// regardless.
//
// `gh_import_url` isn't listed here -- it only makes sense when the
// selected node's body actually is a GitHub issue/PR URL, so it's offered
// by `available_commands` below instead, which the palette calls with the
// live selection and unions with COMMANDS. `gh_import_url` itself is the
// one exception among the always-listed ones too: it reads and overwrites
// `target` rather than treating it as a fixed drop point like the others.
const COMMANDS = #{
    gh_pr_list: &quot;List open PRs into the output panel.&quot;,
    gh_issue_list: &quot;List open issues into the output panel.&quot;,
    import_prs_and_issues: &quot;Import open PRs and issues as outline nodes under Imports/.&quot;,
};

/// Wraps `s` in single quotes for safe interpolation into a `sh -c`
/// string, escaping any embedded single quotes. Use this any time a
/// value that isn't a trusted literal (a title, a body, ...) is built
/// into a `gh` command line. Rhai's `replace()` mutates in place and
/// returns `()` -- see the API reference's gotchas section -- so it runs
/// as a statement on `escaped`, not chained as an expression.
@others
@language rhai
@tabwidth -4
</t>
  <t tx="cub.1786725356.346248974.5::f-anon:118">/// Fetches an issue or PR's full details -- including its description --
/// straight from its GitHub URL, via `gh_parse_url`. The `repo` it parses
/// out of the URL overrides `doc`'s `@variables` repo, so this works
/// against a link into any repo regardless of what `sh_in`'s directory
/// defaults to. Returns `()` (after printing a warning) when `url` isn't a
/// GitHub issue/PR link; otherwise the returned map also carries `kind`
/// (`&quot;issue&quot;` or `&quot;pr&quot;`) alongside the usual `number`/`title`/`author`/
/// `state`/`url`/`body` fields `gh` returns.
fn gh_view_url(doc, url) {
    let parsed = gh_parse_url(url);
    if parsed == () {
        print(&quot;gh_view_url: not a GitHub issue/PR URL: &quot; + url);
        return ();
    }
    let repo_flag = &quot; --repo &quot; + shq(parsed.repo);
    let sub = if parsed.kind == &quot;pr&quot; { &quot;pr&quot; } else { &quot;issue&quot; };
    let item = parse_json(sh_in(doc.dir(), &quot;gh &quot; + sub + &quot; view &quot; + parsed.number.to_string() + &quot; --json number,title,author,state,url,body&quot; + repo_flag).stdout);
    item.kind = parsed.kind;
    item
}

</t>
  <t tx="cub.1786725356.346248974.5::f-anon:140">/// The palette's dynamic counterpart to `COMMANDS` -- called with the live
/// selection to decide which of this script's *conditional* commands
/// apply right now. Offers `gh_import_url` only when `target`'s body is
/// already a GitHub issue/PR URL (checked via `gh_parse_url`, the same
/// parser `gh_import_url` itself uses), since running it on anything else
/// would just print a warning and do nothing.
fn available_commands(doc, target) {
    if gh_parse_url(target.b) != () {
        #{ gh_import_url: &quot;Fetch the GitHub issue/PR URL in the selected node's body and replace it with that issue/PR's content.&quot; }
    } else {
        #{}
    }
}

</t>
  <t tx="cub.1786725356.346248974.5::f-anon:159">/// Command: fetches the issue/PR whose URL is `target`'s body and
/// overwrites `target` with its title (headline) and state/author/url plus
/// full description (body) -- point a node's body at a GitHub link, run
/// this from the action palette, and it turns into that issue/PR's
/// content. Leaves `target` untouched (past a printed warning) when its
/// body isn't a GitHub issue/PR URL.
fn gh_import_url(doc, target) {
    let url = target.b;
    url.trim();
    let item = gh_view_url(doc, url);
    if item == () {
        return;
    }
    let label = if item.kind == &quot;pr&quot; { &quot;PR&quot; } else { &quot;Issue&quot; };
    target.h = label + &quot; #&quot; + item.number + &quot;: &quot; + item.title;
    target.b = item.state + &quot; · &quot; + item.author.login + &quot; · &quot; + item.url + &quot;\n\n&quot; + item.body;
}

</t>
  <t tx="cub.1786725356.346248974.5::f-anon:164">fn gh_pr_create(doc, title, body) {
    sh_in(doc.dir(), &quot;gh pr create --title &quot; + shq(title) + &quot; --body &quot; + shq(body) + gh_repo_flag(doc))
}

</t>
  <t tx="cub.1786725356.346248974.5::f-anon:169">fn gh_issue_create(doc, title, body) {
    sh_in(doc.dir(), &quot;gh issue create --title &quot; + shq(title) + &quot; --body &quot; + shq(body) + gh_repo_flag(doc))
}

</t>
  <t tx="cub.1786725356.346248974.5::f-anon:195">/// Imports open PRs and issues into `Imports/PRs` and `Imports/Issues`,
/// one node per item -- the same shape as the `@action Import PRs and
/// issues` demo in demos/scripting.leo, factored out so other scripts can
/// call it instead of duplicating the fetch/ensure loop. `target` isn't
/// used -- PRs/issues always land under the same fixed path regardless of
/// what was selected -- but every COMMANDS function still takes it (see
/// the file header).
fn import_prs_and_issues(doc, target) {
    let prs = gh_pr_list(doc, target);
    let issues = gh_issue_list(doc, target);
    for pr in prs {
        let n = doc.ensure(&quot;Imports/PRs/#&quot; + pr.number + &quot;: &quot; + pr.title);
        n.b = pr.state + &quot; · &quot; + pr.author.login + &quot; · &quot; + pr.url;
    }
    for issue in issues {
        let n = doc.ensure(&quot;Imports/Issues/#&quot; + issue.number + &quot;: &quot; + issue.title);
        n.b = issue.state + &quot; · &quot; + issue.author.login + &quot; · &quot; + issue.url;
    }
    print(&quot;imported &quot; + prs.len() + &quot; PR(s) and &quot; + issues.len() + &quot; issue(s)&quot;);
}
</t>
  <t tx="cub.1786725356.346248974.5::f-anon:33">fn shq(s) {
    let escaped = s.to_string();
    escaped.replace(&quot;'&quot;, &quot;'\\''&quot;);
    &quot;'&quot; + escaped + &quot;'&quot;
}

</t>
  <t tx="cub.1786725356.346248974.5::f-anon:40">/// Runs `cmd` with `dir` as its working directory -- `sh()` itself always
/// runs relative to `cub`'s own working directory, so this is how a script
/// reaches a directory-relative command (here, `gh`, which resolves its
/// target repo from the git remote of the directory it runs in).
fn sh_in(dir, cmd) {
    sh(cmd, #{ cwd: dir })
}

</t>
  <t tx="cub.1786725356.346248974.5::f-anon:49">/// Reads the `@variables` node's children into a map, keyed by headline
/// with each body (trimmed) as the value -- e.g. a child headlined `repo`
/// with body `leo-editor/leo-editor` becomes `get_variables(doc).repo`. Missing
/// `@variables` or an empty body just means the map is missing that key;
/// callers decide what &quot;unset&quot; means for them.
fn get_variables(doc) {
    let vars = #{};
    for child in doc.ensure(&quot;@variables&quot;).children() {
        let value = child.b;
        value.trim();
        vars[child.h] = value;
    }
    vars
}

</t>
  <t tx="cub.1786725356.346248974.5::f-anon:65">/// The `gh --repo` flag to append to a command, driven by the `repo`
/// `@variables` entry (e.g. `leo-editor/leo-editor`). Empty when unset, so
/// `gh` falls back to its default: the remote of the directory `sh_in`
/// runs it in.
fn gh_repo_flag(doc) {
    let vars = get_variables(doc);
    if vars.contains(&quot;repo&quot;) &amp;&amp; vars.repo != &quot;&quot; {
        &quot; --repo &quot; + shq(vars.repo)
    } else {
        &quot;&quot;
    }
}

</t>
  <t tx="cub.1786725356.346248974.5::f-anon:79">/// Also returns the parsed list, for `import_prs_and_issues` (or another
/// script) to reuse -- but a command's return value never reaches the
/// output panel (only `print()` output does), so this prints one line per
/// PR itself, unlike a pure library helper, to actually show something
/// when run directly from the palette.
fn gh_pr_list(doc, target) {
    let prs = parse_json(sh_in(doc.dir(), &quot;gh pr list --json number,title,author,state,url&quot; + gh_repo_flag(doc)).stdout);
    for pr in prs {
        print(&quot;#&quot; + pr.number + &quot; &quot; + pr.title + &quot; -- &quot; + pr.state + &quot; · &quot; + pr.author.login + &quot; · &quot; + pr.url);
    }
    prs
}

</t>
  <t tx="cub.1786725356.346248974.5::f-anon:84">/// See `gh_pr_list` -- same reasoning, same shape, for issues instead.
fn gh_issue_list(doc, target) {
    let issues = parse_json(sh_in(doc.dir(), &quot;gh issue list --json number,title,author,state,url&quot; + gh_repo_flag(doc)).stdout);
    for issue in issues {
        print(&quot;#&quot; + issue.number + &quot; &quot; + issue.title + &quot; -- &quot; + issue.state + &quot; · &quot; + issue.author.login + &quot; · &quot; + issue.url);
    }
    issues
}

</t>
  <t tx="cub.1786725356.346248974.5::f-anon:89">fn gh_pr_view(doc, number) {
    parse_json(sh_in(doc.dir(), &quot;gh pr view &quot; + number.to_string() + &quot; --json number,title,author,state,url,body&quot; + gh_repo_flag(doc)).stdout)
}

</t>
  <t tx="cub.1786725356.346248974.5::f-anon:94">fn gh_issue_view(doc, number) {
    parse_json(sh_in(doc.dir(), &quot;gh issue view &quot; + number.to_string() + &quot; --json number,title,author,state,url,body&quot; + gh_repo_flag(doc)).stdout)
}

</t>
  <t tx="cub.1786725356.346248974.5::f-anon:99">/// Parses a GitHub issue or PR URL -- e.g.
/// `https://github.com/owner/repo/issues/123` or
/// `https://github.com/owner/repo/pull/123`, including one with a trailing
/// path segment, query string, or `#anchor` (`.../pull/123/files`,
/// `.../issues/123#issuecomment-1`) -- into `#{ repo: &quot;owner/repo&quot;, kind:
/// &quot;issue&quot; | &quot;pr&quot;, number: 123 }`. Returns `()` when `url` doesn't look
/// like a GitHub issue/PR link.
fn gh_parse_url(url) {
    let u = url.to_string();
    u.trim();
    let caps = regex_captures(&quot;^https?://github\\.com/([^/]+)/([^/]+)/(issues|pull)/(\\d+)&quot;, u);
    if caps == () {
        return ();
    }
    let kind = if caps[3] == &quot;pull&quot; { &quot;pr&quot; } else { &quot;issue&quot; };
    #{ repo: caps[1] + &quot;/&quot; + caps[2], kind: kind, number: parse_int(caps[4]) }
}

</t>
</tnodes>
</leo_file>
