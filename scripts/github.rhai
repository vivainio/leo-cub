// GitHub CLI (`gh`) wrappers for cub's rhai Doc API.
//
// An outline node with headline `@import scripts/github.rhai` (path
// resolved relative to the open .leo file's own directory) makes every
// name in COMMANDS below runnable from the action palette (Shift-A).
// `doc.sh(...)` also runs relative to the open .leo file's directory, so
// these assume that's inside (or dominated by) a git repo with a GitHub
// remote and an authenticated `gh` (`gh auth status`).
//
// COMMANDS names the functions meant to be exposed as runnable commands --
// everything else here (shq, and the functions needing more than
// `doc`/`target`) is a library helper, callable from other scripts or the
// REPL but not listed for direct invocation. Every listed function takes
// exactly `(doc, target)` -- `target` is a `Node` for whatever was
// selected when the command was run (the current position), with
// `.h`/`.b`/`.gnx`/etc. straight away, no `doc.node(...)` needed first.
// None of the three below happen to need it, but the palette always
// passes it, so the parameter has to be there regardless.
const COMMANDS = ["gh_pr_list", "gh_issue_list", "import_prs_and_issues"];

/// Wraps `s` in single quotes for safe interpolation into a `sh -c`
/// string, escaping any embedded single quotes. Use this any time a
/// value that isn't a trusted literal (a title, a body, ...) is built
/// into a `gh` command line.
fn shq(s) {
    "'" + s.to_string().replace("'", "'\\''") + "'"
}

fn gh_pr_list(doc, target) {
    parse_json(doc.sh("gh pr list --json number,title,author,state,url").stdout)
}

fn gh_issue_list(doc, target) {
    parse_json(doc.sh("gh issue list --json number,title,author,state,url").stdout)
}

fn gh_pr_view(doc, number) {
    parse_json(doc.sh("gh pr view " + number.to_string() + " --json number,title,author,state,url,body").stdout)
}

fn gh_issue_view(doc, number) {
    parse_json(doc.sh("gh issue view " + number.to_string() + " --json number,title,author,state,url,body").stdout)
}

fn gh_pr_create(doc, title, body) {
    doc.sh("gh pr create --title " + shq(title) + " --body " + shq(body))
}

fn gh_issue_create(doc, title, body) {
    doc.sh("gh issue create --title " + shq(title) + " --body " + shq(body))
}

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
