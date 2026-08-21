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

            let gnx = doc.add("Root/Tasks/First task");
            doc.set_body(gnx, "hello from rhai");

            assert_eq(doc.headline(gnx), "First task");
            assert_eq(doc.body(gnx), "hello from rhai");
            assert(doc.count() == 3, "expected 3 nodes after add");

            let errors = doc.validate();
            assert_eq(errors.len(), 0);

            doc.set_headline(gnx, "First task (renamed)");
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

            let tasks = doc.add("Root/Tasks");
            let first = doc.add("Root/Tasks/First task");
            let second = doc.add("Root/Tasks/Second task");

            assert_eq(doc.parent(first), tasks);
            assert_eq(doc.parent(tasks), root);

            let kids = doc.children(tasks);
            assert_eq(kids.len(), 2);
            assert_eq(kids[0], first);
            assert_eq(kids[1], second);
            assert_eq(doc.children(first).len(), 0);

            assert_eq(doc.path(first), "Root/Tasks/First task");
            assert_eq(doc.gnx(doc.path(first)), first);
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
fn cub_run_exits_nonzero_and_reports_the_failed_assertion() {
    let script_path = temp_path("rhai_run_failure.rhai");
    fs::write(&script_path, "assert_eq(1, 2);").unwrap();

    let output = run_cub(&["run", script_path.to_str().unwrap()]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("assertion failed: 1 != 2"));
}
