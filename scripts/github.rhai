//@+leo-ver=cub-1-thin
//@0 [cub.1786725356.346248974.5] @f ../scripts/github.rhai
// GitHub CLI (`gh`) wrappers for cub's rhai Doc API.
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
// in the palette -- everything else here (shq, gh_repo_flag, gh_parse_url,
// gh_view_url, and the functions needing more than `doc`/`target`) is a
// library helper, callable from other scripts or the REPL but not listed
// for direct invocation. Every listed function takes exactly `(doc,
// target)` -- `target` is a `Node` for whatever was selected when the
// command was run (the current position), with `.h`/`.b`/`.gnx`/etc.
// straight away, no `doc.node(...)` needed first. Most below don't happen
// to need it, but the palette always passes it regardless.
//
// `gh_import_url` isn't listed here -- it only makes sense when the
// selected node's body actually is a GitHub issue/PR URL, so it's offered
// by `available_commands` below instead, which the palette calls with the
// live selection and unions with COMMANDS. `gh_import_url` itself is the
// one exception among the always-listed ones too: it reads and overwrites
// `target` rather than treating it as a fixed drop point like the others.
const COMMANDS = #{
    gh_pr_list: "List open PRs into the output panel.",
    gh_issue_list: "List open issues into the output panel.",
    import_prs_and_issues: "Import open PRs and issues as outline nodes under Imports/.",
};

/// Wraps `s` in single quotes for safe interpolation into a `sh -c`
/// string, escaping any embedded single quotes. Use this any time a
/// value that isn't a trusted literal (a title, a body, ...) is built
/// into a `gh` command line. Rhai's `replace()` mutates in place and
/// returns `()` -- see the API reference's gotchas section -- so it runs
/// as a statement on `escaped`, not chained as an expression.
//@+others
//@> fn shq
fn shq(s) {
    let escaped = s.to_string();
    escaped.replace("'", "'\\''");
    "'" + escaped + "'"
}

//@ fn sh_in
/// Runs `cmd` with `dir` as its working directory -- `sh()` itself always
/// runs relative to `cub`'s own working directory, so this is how a script
/// reaches a directory-relative command (here, `gh`, which resolves its
/// target repo from the git remote of the directory it runs in).
fn sh_in(dir, cmd) {
    sh(cmd, #{ cwd: dir })
}

//@ fn gh_repo_flag
/// The `gh --repo` flag to append to a command, driven by the `repo`
/// `@variables` setting nearest `target` (`cub::variable` walks `target`
/// and its ancestors, so different subtrees in the same outline can each
/// target a different repo -- see docs/workflows/scripting.md). Empty when
/// unset anywhere in reach, so `gh` falls back to its own default: the
/// remote of the directory `sh_in` runs it in.
fn gh_repo_flag(doc, target) {
    let repo = cub::variable(target, "repo");
    if repo == () || repo == "" {
        ""
    } else {
        " --repo " + shq(repo)
    }
}

//@ fn gh_pr_list
/// Also returns the parsed list, for `import_prs_and_issues` (or another
/// script) to reuse -- but a command's return value never reaches the
/// output panel (only `print()` output does), so this prints one line per
/// PR itself, unlike a pure library helper, to actually show something
/// when run directly from the palette.
fn gh_pr_list(doc, target) {
    let prs = parse_json(sh_in(doc.dir(), "gh pr list --json number,title,author,state,url" + gh_repo_flag(doc, target)).stdout);
    for pr in prs {
        print("#" + pr.number + " " + pr.title + " -- " + pr.state + " · " + pr.author.login + " · " + pr.url);
    }
    prs
}

//@ fn gh_issue_list
/// See `gh_pr_list` -- same reasoning, same shape, for issues instead.
fn gh_issue_list(doc, target) {
    let issues = parse_json(sh_in(doc.dir(), "gh issue list --json number,title,author,state,url" + gh_repo_flag(doc, target)).stdout);
    for issue in issues {
        print("#" + issue.number + " " + issue.title + " -- " + issue.state + " · " + issue.author.login + " · " + issue.url);
    }
    issues
}

//@ fn gh_pr_view
fn gh_pr_view(doc, target, number) {
    parse_json(sh_in(doc.dir(), "gh pr view " + number.to_string() + " --json number,title,author,state,url,body" + gh_repo_flag(doc, target)).stdout)
}

//@ fn gh_issue_view
fn gh_issue_view(doc, target, number) {
    parse_json(sh_in(doc.dir(), "gh issue view " + number.to_string() + " --json number,title,author,state,url,body" + gh_repo_flag(doc, target)).stdout)
}

//@ fn gh_parse_url
/// Parses a GitHub issue or PR URL -- e.g.
/// `https://github.com/owner/repo/issues/123` or
/// `https://github.com/owner/repo/pull/123`, including one with a trailing
/// path segment, query string, or `#anchor` (`.../pull/123/files`,
/// `.../issues/123#issuecomment-1`) -- into `#{ repo: "owner/repo", kind:
/// "issue" | "pr", number: 123 }`. Returns `()` when `url` doesn't look
/// like a GitHub issue/PR link.
fn gh_parse_url(url) {
    let u = url.to_string();
    u.trim();
    let caps = regex_captures("^https?://github\\.com/([^/]+)/([^/]+)/(issues|pull)/(\\d+)", u);
    if caps == () {
        return ();
    }
    let kind = if caps[3] == "pull" { "pr" } else { "issue" };
    #{ repo: caps[1] + "/" + caps[2], kind: kind, number: parse_int(caps[4]) }
}

//@ fn gh_view_url
/// Fetches an issue or PR's full details -- including its description --
/// straight from its GitHub URL, via `gh_parse_url`. The `repo` it parses
/// out of the URL overrides `doc`'s `@variables` repo, so this works
/// against a link into any repo regardless of what `sh_in`'s directory
/// defaults to. Returns `()` (after printing a warning) when `url` isn't a
/// GitHub issue/PR link; otherwise the returned map also carries `kind`
/// (`"issue"` or `"pr"`) alongside the usual `number`/`title`/`author`/
/// `state`/`url`/`body` fields `gh` returns.
fn gh_view_url(doc, url) {
    let parsed = gh_parse_url(url);
    if parsed == () {
        print("gh_view_url: not a GitHub issue/PR URL: " + url);
        return ();
    }
    let repo_flag = " --repo " + shq(parsed.repo);
    let sub = if parsed.kind == "pr" { "pr" } else { "issue" };
    let item = parse_json(sh_in(doc.dir(), "gh " + sub + " view " + parsed.number.to_string() + " --json number,title,author,state,url,body" + repo_flag).stdout);
    item.kind = parsed.kind;
    item
}

//@ fn available_commands
/// The palette's dynamic counterpart to `COMMANDS` -- called with the live
/// selection to decide which of this script's *conditional* commands
/// apply right now. Offers `gh_import_url` only when `target`'s body is
/// already a GitHub issue/PR URL (checked via `gh_parse_url`, the same
/// parser `gh_import_url` itself uses), since running it on anything else
/// would just print a warning and do nothing.
fn available_commands(doc, target) {
    if gh_parse_url(target.b) != () {
        #{ gh_import_url: "Fetch the GitHub issue/PR URL in the selected node's body and replace it with that issue/PR's content." }
    } else {
        #{}
    }
}

//@ fn gh_import_url
/// Command: fetches the issue/PR whose URL is `target`'s body and
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
    let label = if item.kind == "pr" { "PR" } else { "Issue" };
    target.h = label + " #" + item.number + ": " + item.title;
    target.b = item.state + " · " + item.author.login + " · " + item.url + "\n\n" + item.body;
}

//@ fn gh_pr_create
fn gh_pr_create(doc, target, title, body) {
    sh_in(doc.dir(), "gh pr create --title " + shq(title) + " --body " + shq(body) + gh_repo_flag(doc, target))
}

//@ fn gh_issue_create
fn gh_issue_create(doc, target, title, body) {
    sh_in(doc.dir(), "gh issue create --title " + shq(title) + " --body " + shq(body) + gh_repo_flag(doc, target))
}

//@ fn import_prs_and_issues
/// Imports open PRs and issues into `Imports/PRs` and `Imports/Issues`,
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
        let n = doc.ensure("Imports/PRs/#" + pr.number + ": " + pr.title);
        n.b = pr.state + " · " + pr.author.login + " · " + pr.url;
    }
    for issue in issues {
        let n = doc.ensure("Imports/Issues/#" + issue.number + ": " + issue.title);
        n.b = issue.state + " · " + issue.author.login + " · " + issue.url;
    }
    print("imported " + prs.len() + " PR(s) and " + issues.len() + " issue(s)");
}
//@-others
//@@language rhai
//@@tabwidth -4
//@-leo
