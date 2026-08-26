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

            let n = doc.ensure("Root/Tasks/First task");
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

            let tasks = doc.ensure("Root/Tasks").gnx;
            let first = doc.ensure("Root/Tasks/First task").gnx;
            let second = doc.ensure("Root/Tasks/Second task").gnx;

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
            let tasks = doc.ensure("Root/Tasks").gnx;
            let first = doc.ensure("Root/Tasks/First task").gnx;
            doc.ensure("Root/Tasks/Second task");

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
            let a = doc.ensure("Root/Alpha task");
            let b = doc.ensure("Root/Beta task");
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
            let tasks = doc.ensure("Team A/Tasks").gnx;
            doc.ensure("Team A/Tasks/Write tests");
            let team_b = doc.ensure("Team B").gnx;

            // clone_node takes an existing parent by gnx -- no headline
            // path to resolve, nothing gets auto-created. Appends by
            // default.
            doc.clone_node(tasks, team_b);
            assert_eq(doc.children(team_b)[0], tasks);
            assert_eq(doc.children(tasks).len(), 1);

            // clone_node's 3-arg overload takes an explicit index.
            let extra = doc.ensure("Extra").gnx;
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
fn cub_run_disambiguates_clone_occurrences_by_position() {
    let leo_path = temp_path("rhai_run_position.leo");
    let _ = fs::remove_file(&leo_path);
    LeoDocument::new("Root").save_new(&leo_path).unwrap();

    let script_path = temp_path("rhai_run_position.rhai");
    let escaped_path = leo_path.display().to_string().replace('\\', "\\\\");
    fs::write(
        &script_path,
        format!(
            r#"
            let doc = open("{escaped_path}");
            let shared_gnx = doc.ensure("Root/Shared").gnx;
            let team_b = doc.ensure("Root/Team B").gnx;
            doc.clone_node(shared_gnx, team_b);

            // A bare gnx handle only ever knows the first occurrence.
            let bare = doc.node(shared_gnx);
            assert_eq(bare.position, "");
            assert_eq(bare.path(), "Root/Shared");
            assert_eq(bare.parent().h, "Root");

            // `doc.node_at` anchors to the exact occurrence named by an
            // index path, so the same gnx can be told apart by position.
            let root = doc.node_at("0");
            let first = root.children()[0];
            let second_root_child = root.children()[1];
            let cloned = second_root_child.children()[0];

            assert_eq(first.gnx, shared_gnx);
            assert_eq(cloned.gnx, shared_gnx);
            assert(first.position != cloned.position, "clone occurrences must have distinct positions");

            assert_eq(first.path(), "Root/Shared");
            assert_eq(cloned.path(), "Root/Team B/Shared");
            assert_eq(cloned.parent().h, "Team B");
            assert_eq(cloned.parent().gnx, team_b);

            // A position-anchored subtree carries positions through too.
            let sub = second_root_child.subtree();
            assert_eq(sub.len(), 2);
            assert_eq(sub[1].position, cloned.position);

            // An unresolvable position fails cleanly rather than silently
            // falling back to something else.
            try {{
                doc.node_at("99/99");
                assert(false, "expected node_at to fail");
            }} catch (e) {{
                assert(e.to_string().contains("position not found"));
            }}

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
fn cub_run_node_remove_targets_the_exact_occurrence_not_the_defining_one() {
    let leo_path = temp_path("rhai_run_node_remove.leo");
    let _ = fs::remove_file(&leo_path);
    LeoDocument::new("Root").save_new(&leo_path).unwrap();

    let script_path = temp_path("rhai_run_node_remove.rhai");
    let escaped_path = leo_path.display().to_string().replace('\\', "\\\\");
    fs::write(
        &script_path,
        format!(
            r#"
            let doc = open("{escaped_path}");
            let shared_gnx = doc.ensure("Root/Shared").gnx;
            let team_b = doc.ensure("Root/Team B").gnx;
            doc.clone_node(shared_gnx, team_b);

            // The clone under Team B, anchored to that exact occurrence --
            // not the defining one under Root.
            let root = doc.node_at("0");
            let team_b_node = root.children()[1];
            let cloned = team_b_node.children()[0];
            assert_eq(cloned.gnx, shared_gnx);
            assert_eq(cloned.path(), "Root/Team B/Shared");

            // Removing it deletes only that occurrence: the defining one
            // under Root survives and stays the defining occurrence.
            cloned.remove();
            assert_eq(team_b_node.children().len(), 0);
            assert_eq(doc.gnx("Root/Shared"), shared_gnx);
            assert_eq(doc.parent(shared_gnx), doc.gnx("Root"));

            // A bare-gnx handle has no position, so it falls back to
            // Doc::remove's defining-occurrence behavior.
            let bare = doc.node(shared_gnx);
            assert_eq(bare.position, "");
            bare.remove();
            assert(doc.find_h("^Shared$").len() == 0, "defining occurrence should be gone");

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
fn cub_run_promotes_an_auto_node_to_at_f_by_renaming_and_saving() {
    // `open` now runs the same derived-file load the TUI does: an `@auto`
    // node's functions are already real, live outline nodes by the time the
    // script sees them (not bare, unexpanded headline-only nodes the way a
    // plain `LeoDocument::open` would leave them). Renaming the root to
    // `@f <path>` and saving should render those already-merged nodes out
    // as real cub-1-thin sentinels -- promoting the plain script file in
    // place -- exactly like doing the same rename+save in the TUI does.
    let dir = temp_path("rhai_run_promote_dir");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("script.rhai"),
        "fn greet(name) {\n    \"hi \" + name\n}\n",
    )
    .unwrap();
    let leo_path = dir.join("outline.leo");
    fs::write(
        &leo_path,
        r#"<leo_file><vnodes><v t="r"><vh>@auto script.rhai</vh></v></vnodes><tnodes><t tx="r"></t></tnodes></leo_file>"#,
    )
    .unwrap();

    let script_path = dir.join("promote.rhai");
    let escaped_path = leo_path.display().to_string().replace('\\', "\\\\");
    fs::write(
        &script_path,
        format!(
            r#"
            let doc = open("{escaped_path}");
            assert(doc.count() > 1, "the @auto file's functions should already be merged in by open()");
            doc.set_headline("r", "@f script.rhai");
            doc.save();
            print("promoted");
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("promoted"));

    let rewritten = fs::read_to_string(dir.join("script.rhai")).unwrap();
    assert!(
        rewritten.starts_with("//@+leo-ver=cub-1-thin\n"),
        "{rewritten}"
    );
    assert!(rewritten.contains("fn greet(name) {"), "{rewritten}");

    let reparsed = leo::RelativeFile::parse(&rewritten).unwrap();
    assert_eq!(reparsed.outline.roots[0].children.len(), 1);

    // The .leo file itself stays a lightweight `@f` pointer -- the real
    // content lives in script.rhai, not baked into the outline XML.
    let saved = LeoDocument::open(&leo_path).unwrap();
    assert_eq!(
        saved.outline.nodes[&leo::NodeId::from("r")].headline,
        "@f script.rhai"
    );
    assert!(saved.outline.roots[0].children.is_empty());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cub_run_warns_instead_of_silently_swallowing_a_derived_load_failure() {
    // Two different .leo outlines can legitimately share a gnx for the
    // same external file -- leo-editor's own LeoPyRef.leo does this for
    // hundreds of files also owned by their own dedicated .leo outlines.
    // If a script converts/saves both in one run, the first save rewrites
    // the shared file to a new sentinel format; the second outline's own
    // node still expects the old format at *its* open() time, so the load
    // fails -- and used to fail *silently* (Doc::open discarded
    // load_derived_files's report.errors entirely), leaving that node's
    // body empty with no indication anything went wrong. A later save of
    // that second outline would then happily render and write the empty
    // state, destroying the first outline's correct content. open() must
    // now print something so this isn't invisible.
    let dir = temp_path("rhai_run_load_conflict_dir");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("shared.py"),
        "#@+leo-ver=5-thin\n#@+node:shared: * @file shared.py\nprint(\"hi\")\n#@-leo\n",
    )
    .unwrap();
    let leo_xml = r#"<leo_file><vnodes><v t="shared"><vh>@file shared.py</vh></v></vnodes><tnodes><t tx="shared"></t></tnodes></leo_file>"#;
    fs::write(dir.join("a.leo"), leo_xml).unwrap();
    fs::write(dir.join("b.leo"), leo_xml).unwrap();

    let script_path = dir.join("conflict.rhai");
    let escaped_dir = dir.display().to_string().replace('\\', "\\\\");
    fs::write(
        &script_path,
        format!(
            r#"
            let a = open("{escaped_dir}/a.leo");
            a.set_headline("shared", "@f shared.py");
            a.save();

            let b = open("{escaped_dir}/b.leo");
            print("opened b");
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("opened b"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no @+node sentinel"),
        "expected the second open() to warn about the load failure it hit, not swallow it silently:\n{stderr}"
    );

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cub_run_renaming_an_already_writable_thin_node_to_at_f_switches_its_sentinel_format() {
    // Unlike the plain-`@auto` promotion above, a `@thin` node is already
    // tracked as writable at `open()` time. The rename must still swap its
    // `ExternalFormat` from `Thin` (5-thin sentinels) to `Relative`
    // (cub-1-thin) -- `track_external_rename`'s `and_modify` branch used to
    // only refresh the path/delimiters and leave the old format in place,
    // so the file kept 5-thin sentinels under a headline that said `@f`.
    let dir = temp_path("rhai_run_rename_thin_to_f_dir");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let leo_path = dir.join("outline.leo");
    fs::write(
        &leo_path,
        r#"<leo_file><vnodes><v t="r"><vh>@thin bar.py</vh></v></vnodes><tnodes><t tx="r">print("bye")
</t></tnodes></leo_file>"#,
    )
    .unwrap();

    let script_path = dir.join("rename.rhai");
    let escaped_path = leo_path.display().to_string().replace('\\', "\\\\");
    fs::write(
        &script_path,
        format!(
            r#"
            let doc = open("{escaped_path}");
            doc.set_headline("r", "@f bar.py");
            doc.save();
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

    let rewritten = fs::read_to_string(dir.join("bar.py")).unwrap();
    assert!(
        rewritten.starts_with("#@+leo-ver=cub-1-thin\n"),
        "expected cub-1-thin sentinels after renaming to @f, got:\n{rewritten}"
    );

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cub_run_set_headline_resolves_an_ancestor_at_path_directive() {
    // `set_headline` used to place a renamed external node's file flat
    // under the open `.leo` file's own directory (`base.join(filename)`),
    // ignoring every ancestor `@path` -- unlike `file_path`/
    // `external_file_path`, which already walked them correctly. A node
    // several `@path` levels deep (exactly the shape leo-editor's own
    // leo/scripts/scripts.leo -> "Windows-only scripts" (@path win) ->
    // "@file elevate.py" has) would land next to the .leo file instead of
    // in its real directory on the very next rename+save.
    let dir = temp_path("rhai_run_set_headline_at_path_dir");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(dir.join("win")).unwrap();
    let leo_path = dir.join("outline.leo");
    fs::write(
        &leo_path,
        concat!(
            r#"<leo_file><vnodes><v t="w"><vh>Windows-only scripts</vh>"#,
            r#"<v t="e"><vh>elevate</vh></v></v></vnodes>"#,
            r#"<tnodes><t tx="w">@path win</t><t tx="e">print("hi")</t></tnodes></leo_file>"#,
        ),
    )
    .unwrap();

    let script_path = dir.join("rename.rhai");
    let escaped_path = leo_path.display().to_string().replace('\\', "\\\\");
    fs::write(
        &script_path,
        format!(
            r#"
            let doc = open("{escaped_path}");
            doc.set_headline("e", "@f elevate.py");
            doc.save();
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

    assert!(
        !dir.join("elevate.py").exists(),
        "must not write the renamed file flat next to the .leo file, ignoring the @path win ancestor"
    );
    let written = fs::read_to_string(dir.join("win").join("elevate.py")).unwrap_or_else(|error| {
        panic!("expected dir/win/elevate.py (the @path win ancestor's directory): {error}")
    });
    assert!(written.contains("print(\"hi\")"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cub_run_sh_defaults_to_cubs_cwd_and_honors_an_explicit_cwd_option() {
    // The global `sh(cmd)` needs no open `Doc` -- unlike `Doc::sh`, which
    // this replaced, it runs relative to `cub`'s own working directory
    // unless `#{cwd: path}` overrides it.
    let dir = temp_path("rhai_run_sh_dir");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let script_path = dir.join("sh.rhai");
    fs::write(
        &script_path,
        r#"
        let default_result = sh("pwd");
        let default_pwd = default_result.stdout;
        default_pwd.trim();
        print("default: " + default_pwd);

        let scoped_result = sh("pwd", #{ cwd: "CWD_DIR" });
        let scoped_pwd = scoped_result.stdout;
        scoped_pwd.trim();
        print("scoped: " + scoped_pwd);
        "#
        .replace("CWD_DIR", dir.display().to_string().as_str()),
    )
    .unwrap();

    let output = run_cub(&["run", script_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "cub run failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let cub_cwd = std::env::current_dir().unwrap();
    assert!(
        stdout.contains(&format!("default: {}", cub_cwd.display())),
        "{stdout}"
    );
    let expected_scoped = fs::canonicalize(&dir).unwrap();
    assert!(
        stdout.contains(&format!("scoped: {}", expected_scoped.display())),
        "{stdout}"
    );

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cub_run_regex_functions_match_find_capture_and_replace() {
    // Like `sh`, these are global -- no open `Doc` needed.
    let script_path = temp_path("rhai_run_regex.rhai");
    fs::write(
        &script_path,
        r#"
        assert_eq(regex_is_match("^\\d+$", "123"), true);
        assert_eq(regex_is_match("^\\d+$", "abc"), false);

        assert_eq(regex_find("\\d+", "abc123def456"), "123");
        assert(regex_find("\\d+", "abc") == ());

        let all = regex_find_all("\\d+", "abc123def456");
        assert_eq(all.len(), 2);
        assert_eq(all[0], "123");
        assert_eq(all[1], "456");

        let caps = regex_captures("(\\w+)@(\\w+)", "user@host");
        assert_eq(caps.len(), 3);
        assert_eq(caps[0], "user@host");
        assert_eq(caps[1], "user");
        assert_eq(caps[2], "host");
        assert(regex_captures("\\d+", "abc") == ());

        assert_eq(regex_replace("\\d+", "abc123def456", "X"), "abcXdef456");
        assert_eq(regex_replace_all("\\d+", "abc123def456", "X"), "abcXdefX");

        print("regex functions ok");
        "#,
    )
    .unwrap();

    let output = run_cub(&["run", script_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "cub run failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("regex functions ok"));
}

#[test]
fn cub_run_regex_functions_fail_on_an_invalid_pattern() {
    let script_path = temp_path("rhai_run_regex_invalid.rhai");
    fs::write(&script_path, r#"regex_is_match("(", "abc");"#).unwrap();

    let output = run_cub(&["run", script_path.to_str().unwrap()]);

    assert!(!output.status.success());
}

// The bare `cub foo.rhai` shorthand (no `run` subcommand) is dispatched
// from the same positional argument the TUI shorthand (`cub foo.leo`) uses,
// which only exists when the `tui` feature is compiled in (see `Cli::file`
// in src/main.rs) -- so this needs `tui`, unlike every other test here.
#[test]
#[cfg(feature = "tui")]
fn cub_dispatches_a_bare_rhai_file_as_run_without_the_subcommand() {
    let script_path = temp_path("rhai_shorthand.rhai");
    fs::write(
        &script_path,
        r#"print("ran via shorthand, ARGS=" + ARGS.len());"#,
    )
    .unwrap();

    let output = run_cub(&[script_path.to_str().unwrap(), "one", "two"]);
    assert!(
        output.status.success(),
        "cub <script.rhai> failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // The shorthand passes trailing arguments through as ARGS too.
    assert!(String::from_utf8_lossy(&output.stdout).contains("ran via shorthand, ARGS=2"));
}

#[test]
fn cub_run_exposes_trailing_arguments_as_args() {
    let script_path = temp_path("rhai_run_args.rhai");
    fs::write(
        &script_path,
        r#"
        assert_eq(ARGS.len(), 2);
        assert_eq(ARGS[0], "notes.leo");
        assert_eq(ARGS[1], "with a space");
        print("saw " + ARGS.len() + " args");
        "#,
    )
    .unwrap();

    let output = run_cub(&[
        "run",
        script_path.to_str().unwrap(),
        "notes.leo",
        "with a space",
    ]);
    assert!(
        output.status.success(),
        "cub run failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("saw 2 args"));
}

#[test]
fn cub_run_args_is_empty_when_none_are_given() {
    let script_path = temp_path("rhai_run_no_args.rhai");
    fs::write(&script_path, "assert_eq(ARGS.len(), 0);").unwrap();

    let output = run_cub(&["run", script_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "cub run failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cub_run_exits_nonzero_and_reports_the_failed_assertion() {
    let script_path = temp_path("rhai_run_failure.rhai");
    fs::write(&script_path, "assert_eq(1, 2);").unwrap();

    let output = run_cub(&["run", script_path.to_str().unwrap()]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("assertion failed: 1 != 2"));
}
