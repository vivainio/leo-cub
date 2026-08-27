// Verifies github.rhai's `jira` command against real `zaira` tickets.
//
// Run with `cub run scripts/verify-jira.rhai [KEY...]` (defaults to
// PROJ-123, which has paragraph fields and a link but no comments, when
// no keys are given -- pass a ticket with comments, e.g. PROJ-456, to
// also exercise the Comments-splitting path).
//
// Builds a throwaway outline via `cub new`, imports github.rhai, runs
// `jira` on each key, and asserts the resulting node shape: `target.b`
// holds the rendered markdown body (not empty, no stray frontmatter),
// `target`'s only children are `Meta` and (when the ticket has
// comments) `Comments` -- no leftover `Description` child -- and each
// `Comments` child is individually labeled and bodied.

import "scripts/github.rhai" as gh;

let keys = if ARGS.len() > 0 { ARGS } else { ["PROJ-123"] };

let scratch = "/tmp/verify-jira-scratch.leo";
sh("rm -f " + scratch);
let new_result = sh("cub new " + scratch);
if new_result.code != 0 {
    throw "`cub new` failed: " + new_result.stderr;
}

let doc = open(scratch);

fn check(cond, msg) {
    if !cond {
        throw "FAIL: " + msg;
    }
    print("  ok: " + msg);
}

for key in keys {
    print("=== " + key + " ===");
    let target = doc.ensure("@jira " + key);
    gh::jira(doc, target);

    check(target.h.starts_with("@jira " + key + ": "), "headline is `@jira " + key + ": <summary>`");
    check(target.b.starts_with("@language markdown\n"), "body starts with @language markdown");
    let content = target.b.sub_string(20);
    check(content.len() > 0, "body has content past the @language marker");
    check(!content.contains("---\nkey:"), "body doesn't still carry raw frontmatter");

    let child_names = [];
    for c in target.children() {
        child_names.push(c.h);
    }
    for name in child_names {
        check(name == "Meta" || name == "Comments", "child `" + name + "` is Meta or Comments, not a leftover Description");
    }
    check(child_names.contains("Meta"), "has a Meta child");

    let meta = doc.ensure(target.path() + "/Meta");
    check(meta.b.contains("key: " + key), "Meta contains `key: " + key + "`");
    check(meta.b.contains("url: https://"), "Meta contains a url line");

    if child_names.contains("Comments") {
        let comments = doc.ensure(target.path() + "/Comments");
        let n = comments.children().len();
        check(n > 0, "Comments has " + n.to_string() + " child(ren)");
        for c in comments.children() {
            check(regex_is_match("^#\\d+ .+ \\(\\d{4}-\\d{2}-\\d{2} \\d{2}:\\d{2}\\)$", c.h), "comment headline `" + c.h + "` matches `#N author (date)`");
            check(c.b.starts_with("@language markdown\n"), "comment `" + c.h + "` body starts with @language markdown");
        }
    } else {
        print("  (no comments on this ticket)");
    }
}

print("all checks passed");
