//@+leo-ver=cub-1-thin
//@0 [cub.1786725356.346248974.5] @f ../scripts/github.rhai
// GitHub CLI (`gh`) and Jira CLI (`zaira`) wrappers for cub's rhai Doc API.
//
// An outline node with headline `@import scripts/github.rhai` (path
// resolved relative to the open .leo file's own directory) makes every
// name in COMMANDS below runnable from the action palette (Shift-A).
// `gh` commands run through `sh_in(doc, ...)`, which `cd`s into the open
// .leo file's directory first, so these assume that's inside (or
// dominated by) a git repo with a GitHub remote and an authenticated `gh`
// (`gh auth status`). `zaira` commands run the same way, and assume
// `zaira init` has already set up Jira credentials -- see
// https://github.com/vivainio/zaira.
//
// COMMANDS maps the functions meant to be exposed as runnable commands to
// a one-line description of what running them does, shown alongside each
// in the palette -- everything else here (shq, gh_repo_flag, gh_parse_url,
// gh_view_url, jira_parse_headline, zaira_run_json, zaira_get_segments, and the functions needing
// more than `doc`/`target`) is a library helper, callable from other
// scripts or the REPL but not listed for direct invocation. Every listed
// function takes exactly `(doc, target)` -- `target` is a `Node` for
// whatever was selected when the command was run (the current position),
// with `.h`/`.b`/`.gnx`/etc. straight away, no `doc.node(...)` needed
// first. Most below don't happen to need it, but the palette always
// passes it regardless.
//
// `gh_import_url` isn't listed here -- it only makes sense when the
// selected node's body actually is a GitHub issue/PR URL, so it's offered
// by `available_commands` below instead, which the palette calls with the
// live selection and unions with COMMANDS. `gh_import_url` itself is the
// one exception among the always-listed ones too: it reads and overwrites
// `target` rather than treating it as a fixed drop point like the others.
// `jira` works the same way for the Jira side, offered instead when the
// selected node's headline is one of three `@jira` forms (see
// `jira_parse_headline`): `@jira KEY` -- a Jira issue key like `FOO-123`,
// optionally followed by free text (`@jira FOO-123 investigate flaky
// test`) -- replaces the node with that ticket's content; `@jira my` and
// `@jira search TEXT` instead refresh the node's children with your open
// tickets or a text search's results, one `@jira KEY: summary` child per
// match.
const COMMANDS = #{
    gh_pr_list: "List open PRs into the output panel.",
    gh_issue_list: "List open issues into the output panel.",
    import_prs_and_issues: "Import open PRs and issues as outline nodes under Imports/.",
    zaira_my_list: "List your open Jira tickets into the output panel.",
    import_my_tickets: "Import your open Jira tickets as outline nodes under Imports/Jira.",
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
/// would just print a warning and do nothing. Offers `jira` the same
/// way, keyed off `target`'s headline instead via `jira_parse_headline`.
fn available_commands(doc, target) {
    let cmds = #{};
    if gh_parse_url(target.b) != () {
        cmds.gh_import_url = "Fetch the GitHub issue/PR URL in the selected node's body and replace it with that issue/PR's content.";
    }
    let jira = jira_parse_headline(target.h);
    if jira != () {
        cmds.jira = if jira.kind == "ticket" {
            "Fetch the Jira ticket named in the selected node's headline (`@jira KEY ...`) and replace it with that ticket's content."
        } else if jira.kind == "my" {
            "Refresh this node's children with your open Jira tickets (`@jira my`)."
        } else {
            "Refresh this node's children with Jira search results for \"" + jira.query + "\" (`@jira search ...`)."
        };
    }
    cmds
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

//@ fn jira_parse_headline
/// Parses a `@jira ...` node headline into what it names -- one of three
/// forms:
/// - `@jira KEY ...` (e.g. `@jira FOO-123 investigate flaky test`) -- a
///   single ticket -- into `#{ kind: "ticket", key: "FOO-123" }`.
/// - `@jira my` (exactly, nothing after it) -- your open tickets -- into
///   `#{ kind: "my" }`.
/// - `@jira search TEXT` -- a free-text search -- into `#{ kind:
///   "search", query: "TEXT" }`.
/// Returns `()` when `h` doesn't match any of these.
fn jira_parse_headline(h) {
    let s = h.to_string();
    s.trim();
    if s == "@jira my" {
        return #{ kind: "my" };
    }
    let search_caps = regex_captures("^@jira\\s+search\\s+(.+)$", s);
    if search_caps != () {
        return #{ kind: "search", query: search_caps[1] };
    }
    let ticket_caps = regex_captures("^@jira\\s+([A-Z][A-Z0-9]*-\\d+)", s);
    if ticket_caps == () {
        return ();
    }
    #{ kind: "ticket", key: ticket_caps[1] }
}

//@ fn zaira_run_json
/// Runs a `zaira ... --format json` command and parses its stdout --
/// checking `code` first so a nonzero exit throws a clear error carrying
/// `zaira`'s own stderr, instead of leaving `parse_json` to fail on
/// whatever (often empty) stdout a crash left behind. Worth doing
/// specifically for `zaira`, unlike `gh` above: `zaira search` has been
/// observed to crash outright (a Python `AttributeError`) on tickets from
/// a project where the Priority field is disabled, since it accesses that
/// field unconditionally rather than checking for it first.
fn zaira_run_json(doc, cmd) {
    let result = sh_in(doc.dir(), cmd);
    if result.code != 0 {
        throw "`" + cmd + "` failed (exit " + result.code.to_string() + "): " + result.stderr;
    }
    parse_json(result.stdout)
}

//@ fn zaira_get_segments
/// Fetches a Jira ticket via `zaira get KEY --format ndjson` -- one
/// self-describing JSON object per line (`type`: `"meta"`,
/// `"description"`, `"paragraph"`, `"links"`, or `"comment"`), already
/// split apart by `zaira` itself. `jira` folds these into `target.b`
/// and its `Meta`/`Comments` children without having to pick a rendered
/// markdown/frontmatter blob back apart by hand. `zaira` resolves Jira
/// credentials and the ticket's project from its own config (`zaira
/// init`), so unlike `gh_repo_flag` there's no per-outline `--repo`-style
/// override here.
fn zaira_get_segments(doc, key) {
    let cmd = "zaira get " + shq(key) + " --format ndjson";
    let result = sh_in(doc.dir(), cmd);
    if result.code != 0 {
        throw "`" + cmd + "` failed (exit " + result.code.to_string() + "): " + result.stderr;
    }
    let segments = [];
    for line in result.stdout.split("\n") {
        if line != "" {
            segments.push(parse_json(line));
        }
    }
    segments
}

//@ fn jira_join
/// Joins an array of stringifiable values with `sep` -- Rhai's `Array`
/// has no built-in `join`. "none" for an empty array, so `Meta`'s
/// `components`/`labels` lines are never left blank.
fn jira_join(arr, sep) {
    if arr.len() == 0 {
        return "none";
    }
    let s = arr[0].to_string();
    for i in 1..arr.len() {
        s += sep + arr[i].to_string();
    }
    s
}

//@ fn jira_populate_children
/// Ensures one child under `parent_path` per ticket in `tickets` --
/// headlined `@jira KEY: summary` so each is individually recognized by
/// `jira_parse_headline` (and refreshable into full content with `jira`).
/// Shared by `import_my_tickets` and `jira`'s `@jira my` / `@jira
/// search ...` handling.
fn jira_populate_children(doc, parent_path, tickets) {
    for t in tickets {
        let n = doc.ensure(parent_path + "/@jira " + t.key + ": " + t.summary);
        n.b = t.status + " · " + t.assignee;
    }
}

//@ fn zaira_search_text
/// Free-text ticket search via `zaira search TEXT --format json` -- same
/// shape as `zaira_my_list` (prints one line per match as well as
/// returning the parsed list), for `jira`'s `@jira search ...` handling
/// or another script to reuse.
fn zaira_search_text(doc, text) {
    let tickets = zaira_run_json(doc, "zaira search " + shq(text) + " --format json");
    for t in tickets {
        print(t.key + " " + t.summary + " -- " + t.status + " · " + t.assignee);
    }
    tickets
}

//@ fn jira
/// Command, dispatching on `jira_parse_headline(target.h)`:
/// - `@jira KEY ...` -- overwrites `target`'s headline with the ticket's
///   key/summary (the `@jira KEY` prefix is kept so the node stays
///   recognized by this script after import), folds the `description`/
///   `paragraph`/`links` segments from `zaira_get_segments` into
///   `target.b` as description text followed by one `## ` section per
///   paragraph field and a trailing `## Links` section, and splits the
///   rest into two children: `Meta` (key/type/status/priority/assignee/
///   reporter/dates/components/labels/parent/custom fields/url) and
///   `Comments` (one `#N author (date)` child per `comment` segment,
///   oldest first, numbered so same-author-same-day comments don't
///   collide as identical headlines).
/// - `@jira my` / `@jira search TEXT` -- leaves `target` itself alone and
///   instead refreshes its children (via `jira_populate_children`) with
///   one `@jira KEY: summary` node per ticket assigned to you, or
///   matching the search.
/// Leaves `target` untouched (past a printed warning) when its headline
/// doesn't match any `@jira` form.
fn jira(doc, target) {
    let parsed = jira_parse_headline(target.h);
    if parsed == () {
        print("jira: headline isn't '@jira KEY', '@jira my', or '@jira search ...': " + target.h);
        return;
    }
    if parsed.kind == "ticket" {
        let segments = zaira_get_segments(doc, parsed.key);
        let meta = #{};
        let body = "";
        let comments = [];
        for seg in segments {
            if seg.type == "meta" {
                meta = seg;
            } else if seg.type == "description" {
                body += seg.markdown;
            } else if seg.type == "paragraph" {
                body += "\n\n## " + seg.name + "\n\n" + seg.markdown;
            } else if seg.type == "links" {
                body += "\n\n## Links\n\n" + seg.markdown;
            } else if seg.type == "comment" {
                comments.push(seg);
            }
        }
        target.h = "@jira " + meta.key + ": " + meta.summary;
        target.b = "@language markdown\n" + body;
        let path = target.path();
        let meta_node = doc.ensure(path + "/Meta");
        let meta_b = "key: " + meta.key + "\n"
            + "summary: " + meta.summary + "\n"
            + "type: " + meta.issuetype + "\n"
            + "status: " + meta.status + "\n"
            + "priority: " + meta.priority + "\n"
            + "assignee: " + meta.assignee + "\n"
            + "reporter: " + meta.reporter + "\n"
            + "created: " + meta.created + "\n"
            + "updated: " + meta.updated + "\n"
            + "components: " + jira_join(meta.components, ", ") + "\n"
            + "labels: " + jira_join(meta.labels, ", ") + "\n";
        if meta.parent != () {
            meta_b += "parent: " + meta.parent.to_string() + "\n";
        }
        for key in meta.custom_fields.keys() {
            meta_b += key + ": " + meta.custom_fields[key] + "\n";
        }
        meta_b += "url: " + meta.url;
        meta_node.b = meta_b;
        for i in 0..comments.len() {
            let c = comments[i];
            let label = "#" + (i + 1).to_string() + " " + c.author + " (" + c.created + ")";
            let n = doc.ensure(path + "/Comments/" + label);
            n.b = "@language markdown\n" + c.markdown;
        }
        return;
    }
    let tickets = if parsed.kind == "my" {
        zaira_my_list(doc, target)
    } else {
        zaira_search_text(doc, parsed.query)
    };
    jira_populate_children(doc, target.path(), tickets);
    print("jira: refreshed " + tickets.len() + " ticket(s) under \"" + target.h + "\"");
}

//@ fn zaira_my_list
/// Also returns the parsed list, for `import_my_tickets` (or another
/// script) to reuse -- but a command's return value never reaches the
/// output panel (only `print()` output does), so this prints one line per
/// ticket itself, unlike a pure library helper, to actually show
/// something when run directly from the palette. Uses the same JQL
/// `zaira my` itself uses for tickets assigned to you.
fn zaira_my_list(doc, target) {
    let jql = "assignee = currentUser() AND status NOT IN (Done, Closed, Resolved, Disposal, Rejected) ORDER BY updated DESC";
    let tickets = zaira_run_json(doc, "zaira search --jql " + shq(jql) + " --format json");
    for t in tickets {
        print(t.key + " " + t.summary + " -- " + t.status + " · " + t.assignee);
    }
    tickets
}

//@ fn import_my_tickets
/// Imports your open Jira tickets into `Imports/Jira`, one node per
/// ticket, headlined so it's immediately recognized by
/// `jira_parse_headline` (and refreshable later with `jira`) -- same
/// shape as `import_prs_and_issues`. `target` isn't used -- tickets
/// always land under the same fixed path regardless of what was selected
/// -- but every COMMANDS function still takes it (see the file header).
fn import_my_tickets(doc, target) {
    let tickets = zaira_my_list(doc, target);
    jira_populate_children(doc, "Imports/Jira", tickets);
    print("imported " + tickets.len() + " Jira ticket(s)");
}
//@-others
//@@language rhai
//@@tabwidth -4
//@-leo
