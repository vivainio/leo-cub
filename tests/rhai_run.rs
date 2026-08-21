//! End-to-end coverage for `cub run <script.rhai>`: spawns the real `cub`
//! binary against a temp `.leo` file the same way a CI step would, instead
//! of calling the engine in-process. This is the headless replacement for
//! the old jsonl/TUI `--script` integration tests, which needed a real
//! terminal and so were never actually exercised in CI.
#![cfg(feature = "rhai")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use leo::LeoDocument;

fn temp_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name)
}

fn run_cub(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cub"))
        .args(args)
        .output()
        .expect("spawn cub")
}

#[test]
fn cub_run_drives_a_temp_outline_through_the_rhai_api() {
    let leo_path = temp_path("rhai_run_smoke.leo");
    let _ = fs::remove_file(&leo_path);
    LeoDocument::new("Root").save_new(&leo_path).unwrap();

    let script_path = temp_path("rhai_run_smoke.rhai");
    let escaped_path = leo_path.display().to_string().replace('\\', "\\\\");
    fs::write(
        &script_path,
        format!(
            r#"
            let doc = open("{escaped_path}");
            assert_eq(doc.count(), 1);

            let n = doc.add("Root/Tasks/First task");
            n.b = "hello from rhai";

            assert_eq(n.h, "First task");
            assert_eq(n.b, "hello from rhai");
            assert(doc.count() == 3, "expected 3 nodes after add");

            let errors = doc.validate();
            assert_eq(errors.len(), 0);

            n.h = "First task (renamed)";
            doc.save();
            print("wrote " + doc.count() + " nodes");
            "#
        ),
    )
    .unwrap();

    let output = run_cub(&["run", script_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "cub run failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("wrote 3 nodes"));

    let saved = LeoDocument::open(&leo_path).unwrap();
    assert_eq!(saved.outline.nodes.len(), 3);
    let gnx = saved
        .outline
        .resolve_headline_path("Root/Tasks/First task (renamed)")
        .unwrap();
    assert_eq!(saved.outline.nodes[&gnx].body, "hello from rhai");
}

#[test]
fn cub_run_walks_the_tree_with_roots_children_parent_and_path() {
    let leo_path = temp_path("rhai_run_tree.leo");
    let _ = fs::remove_file(&leo_path);
    LeoDocument::new("Root").save_new(&leo_path).unwrap();

    let script_path = temp_path("rhai_run_tree.rhai");
    let escaped_path = leo_path.display().to_string().replace('\\', "\\\\");
    fs::write(
        &script_path,
        format!(
            r#"
            let doc = open("{escaped_path}");
            let root = doc.gnx("Root");
            assert_eq(doc.roots().len(), 1);
            assert_eq(doc.roots()[0], root);
            assert_eq(doc.parent(root), "");

            let tasks = doc.add("Root/Tasks").gnx;
            let first = doc.add("Root/Tasks/First task").gnx;
            let second = doc.add("Root/Tasks/Second task").gnx;

            assert_eq(doc.parent(first), tasks);
            assert_eq(doc.parent(tasks), root);

            let kids = doc.children(tasks);
            assert_eq(kids.len(), 2);
            assert_eq(kids[0], first);
            assert_eq(kids[1], second);
            assert_eq(doc.children(first).len(), 0);

            assert_eq(doc.path(first), "Root/Tasks/First task");
            assert_eq(doc.gnx(doc.path(first)), first);

            assert_eq(doc.subtree(tasks), [tasks, first, second]);
            assert_eq(doc.subtree(first), [first]);
            assert_eq(doc.all(), [root, tasks, first, second]);
            print("ok");
            "#
        ),
    )
    .unwrap();

    let output = run_cub(&["run", script_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "cub run failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("ok"));
}

#[test]
fn cub_run_reads_and_writes_nodes_through_the_node_wrapper() {
    let leo_path = temp_path("rhai_run_node.leo");
    let _ = fs::remove_file(&leo_path);
    LeoDocument::new("Root").save_new(&leo_path).unwrap();

    let script_path = temp_path("rhai_run_node.rhai");
    let escaped_path = leo_path.display().to_string().replace('\\', "\\\\");
    fs::write(
        &script_path,
        format!(
            r#"
            let doc = open("{escaped_path}");
            let tasks = doc.add("Root/Tasks").gnx;
            let first = doc.add("Root/Tasks/First task").gnx;
            doc.add("Root/Tasks/Second task");

            let n = doc.node(first);
            n.h = "First task (renamed)";
            n.b = "hello from a Node";
            assert_eq(n.h, "First task (renamed)");
            assert_eq(n.b, "hello from a Node");

            // Mutations through the wrapper are visible through `doc` too --
            // they share the same underlying document.
            assert_eq(doc.headline(first), "First task (renamed)");
            assert_eq(doc.body(first), "hello from a Node");

            let p = n.parent();
            assert_eq(p.gnx, tasks);
            assert_eq(p.h, "Tasks");

            let kids = p.children();
            assert_eq(kids.len(), 2);
            assert_eq(kids[0].gnx, first);
            assert_eq(kids[0].h, "First task (renamed)");
            assert_eq(kids[1].h, "Second task");

            assert_eq(n.path(), "Root/Tasks/First task (renamed)");

            let sub = p.subtree();
            assert_eq(sub.len(), 3);
            assert_eq(sub[0].gnx, tasks);
            assert_eq(sub[1].h, "First task (renamed)");
            assert_eq(sub[2].h, "Second task");

            print("ok");
            "#
        ),
    )
    .unwrap();

    let output = run_cub(&["run", script_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "cub run failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("ok"));
}

#[test]
fn cub_run_finds_nodes_by_headline_and_body_pattern() {
    let leo_path = temp_path("rhai_run_find.leo");
    let _ = fs::remove_file(&leo_path);
    LeoDocument::new("Root").save_new(&leo_path).unwrap();

    let script_path = temp_path("rhai_run_find.rhai");
    let escaped_path = leo_path.display().to_string().replace('\\', "\\\\");
    fs::write(
        &script_path,
        format!(
            r#"
            let doc = open("{escaped_path}");
            let a = doc.add("Root/Alpha task");
            let b = doc.add("Root/Beta task");
            a.b = "TODO: write tests";
            b.b = "already done";

            let by_headline = doc.find_h("^Alpha");
            assert_eq(by_headline.len(), 1);
            assert_eq(by_headline[0].gnx, a.gnx);

            let by_body = doc.find_b("TODO");
            assert_eq(by_body.len(), 1);
            assert_eq(by_body[0].gnx, a.gnx);

            assert_eq(doc.find_h("task").len(), 2);
            assert_eq(doc.find_b("nonexistent").len(), 0);
            print("ok");
            "#
        ),
    )
    .unwrap();

    let output = run_cub(&["run", script_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "cub run failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("ok"));
}

#[test]
fn cub_run_clones_and_removes_nodes_directly_without_apply() {
    let leo_path = temp_path("rhai_run_clone_remove.leo");
    let _ = fs::remove_file(&leo_path);
    LeoDocument::new("Root").save_new(&leo_path).unwrap();

    let script_path = temp_path("rhai_run_clone_remove.rhai");
    let escaped_path = leo_path.display().to_string().replace('\\', "\\\\");
    fs::write(
        &script_path,
        format!(
            r#"
            let doc = open("{escaped_path}");
            let tasks = doc.add("Team A/Tasks").gnx;
            doc.add("Team A/Tasks/Write tests");
            let team_b = doc.add("Team B").gnx;

            // clone_node takes an existing parent by gnx -- no headline
            // path to resolve, nothing gets auto-created. Appends by
            // default.
            doc.clone_node(tasks, team_b);
            assert_eq(doc.children(team_b)[0], tasks);
            assert_eq(doc.children(tasks).len(), 1);

            // clone_node's 3-arg overload takes an explicit index.
            let extra = doc.add("Extra").gnx;
            doc.clone_node(extra, team_b, 0);
            assert_eq(doc.children(team_b)[0], extra);
            assert_eq(doc.children(team_b)[1], tasks);

            // Fails cleanly on a parent gnx that doesn't exist, rather than
            // silently creating anything.
            try {{
                doc.clone_node(tasks, "not-a-real-gnx");
                assert(false, "expected clone_node to fail");
            }} catch (e) {{
                assert(e.to_string().contains("not-a-real-gnx"));
            }}

            // remove deletes a node's defining occurrence and its subtree;
            // the clone under Team B is untouched.
            doc.remove(doc.gnx("Team A/Tasks/Write tests"));
            assert_eq(doc.children(tasks).len(), 0);
            assert_eq(doc.children(team_b)[1], tasks);

            assert_eq(doc.validate().len(), 0);
            doc.save();
            print("ok");
            "#
        ),
    )
    .unwrap();

    let output = run_cub(&["run", script_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "cub run failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("ok"));

    let saved = LeoDocument::open(&leo_path).unwrap();
    assert!(saved.outline.validate().is_empty());
}

#[test]
fn cub_run_exits_nonzero_and_reports_the_failed_assertion() {
    let script_path = temp_path("rhai_run_failure.rhai");
    fs::write(&script_path, "assert_eq(1, 2);").unwrap();

    let output = run_cub(&["run", script_path.to_str().unwrap()]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("assertion failed: 1 != 2"));
}
