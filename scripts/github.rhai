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
// COMMANDS names the functions meant to be exposed as runnable commands --
// everything else here (shq, get_variables, gh_repo_flag, and the
// functions needing more than `doc`/`target`) is a library helper,
// callable from other scripts or the REPL but not listed for direct
// invocation. Every listed function takes exactly `(doc, target)` --
// `target` is a `Node` for whatever was selected when the command was run
// (the current position), with `.h`/`.b`/`.gnx`/etc. straight away, no
// `doc.node(...)` needed first. None of the three below happen to need
// it, but the palette always passes it, so the parameter has to be there
// regardless.
const COMMANDS = ["gh_pr_list", "gh_issue_list", "import_prs_and_issues"];

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

//@ fn get_variables
/// Reads the `@variables` node's children into a map, keyed by headline
/// with each body (trimmed) as the value -- e.g. a child headlined `repo`
/// with body `leo-editor/leo-editor` becomes `get_variables(doc).repo`. Missing
/// `@variables` or an empty body just means the map is missing that key;
/// callers decide what "unset" means for them.
fn get_variables(doc) {
    let vars = #{};
    for child in doc.ensure("@variables").children() {
        let value = child.b;
        value.trim();
        vars[child.h] = value;
    }
    vars
}

//@ fn gh_repo_flag
/// The `gh --repo` flag to append to a command, driven by the `repo`
/// `@variables` entry (e.g. `leo-editor/leo-editor`). Empty when unset, so
/// `gh` falls back to its default: the remote of the directory `sh_in`
/// runs it in.
fn gh_repo_flag(doc) {
    let vars = get_variables(doc);
    if vars.contains("repo") && vars.repo != "" {
        " --repo " + shq(vars.repo)
    } else {
        ""
    }
}

//@ fn gh_pr_list
fn gh_pr_list(doc, target) {
    parse_json(sh_in(doc.dir(), "gh pr list --json number,title,author,state,url" + gh_repo_flag(doc)).stdout)
}

//@ fn gh_issue_list
fn gh_issue_list(doc, target) {
    parse_json(sh_in(doc.dir(), "gh issue list --json number,title,author,state,url" + gh_repo_flag(doc)).stdout)
}

//@ fn gh_pr_view
fn gh_pr_view(doc, number) {
    parse_json(sh_in(doc.dir(), "gh pr view " + number.to_string() + " --json number,title,author,state,url,body" + gh_repo_flag(doc)).stdout)
}

//@ fn gh_issue_view
fn gh_issue_view(doc, number) {
    parse_json(sh_in(doc.dir(), "gh issue view " + number.to_string() + " --json number,title,author,state,url,body" + gh_repo_flag(doc)).stdout)
}

//@ fn gh_pr_create
fn gh_pr_create(doc, title, body) {
    sh_in(doc.dir(), "gh pr create --title " + shq(title) + " --body " + shq(body) + gh_repo_flag(doc))
}

//@ fn gh_issue_create
fn gh_issue_create(doc, title, body) {
    sh_in(doc.dir(), "gh issue create --title " + shq(title) + " --body " + shq(body) + gh_repo_flag(doc))
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
