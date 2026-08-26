// Imports Claude Code session history into the open outline: one node per
// session, under Sessions/<project>/<short-id> <title>, with when it
// started, where, how many messages, and the first prompt.
//
// An outline node with headline `@import scripts/claude-history.rhai`
// (path resolved relative to the open .leo file's own directory) makes
// `import_claude_history` runnable from the action palette (Shift-A). See
// demos/scripting.leo for a working example node.
//
// Reads $HOME/.claude/projects (via `env_var`) with rhai-fs's own file
// functions (open_dir/open_file/read_string) instead of shelling out to
// `find`/`cat`; see the API reference's "Paths and directories", "Files",
// and "Subprocesses and files" sections.
//
// Each project directory holds one `*.jsonl` transcript per session, one
// JSON object per line. Rather than `parse_json` and branch on every
// line's `type` -- deeply-nested branching inside one function trips
// Rhai's max-expression-depth guard, and isn't needed here anyway -- this
// finds the one `ai-title` line and one `user` line it actually wants
// with a plain substring search, and counts the rest by substring too.
const COMMANDS = #{
    import_claude_history: "Import Claude Code session history as outline nodes under Sessions/.",
};

/// The first line in `lines` containing `needle`, or "".
fn find_line(lines, needle) {
    for line in lines {
        if line.contains(needle) {
            return line;
        }
    }
    ""
}

/// A `user` message's `content` is either a plain string or an array of
/// typed blocks (tool results, images, ...) -- either way, its first
/// plain-text block, or "" when there isn't one.
fn first_text(content) {
    if type_of(content) == "string" {
        return content;
    }
    for block in content {
        if type_of(block) == "map" && block.contains("type") && block.type == "text" {
            return block.text;
        }
    }
    ""
}

/// A prompt, trimmed to one line and `max_len` characters, for a headline
/// or body summary.
fn excerpt(text, max_len) {
    let single_line = text;
    single_line.replace("\n", " ");
    single_line.trim();
    if single_line.len() > max_len {
        single_line = single_line.sub_string(0, max_len) + "…";
    }
    single_line
}

//@ fn import_session
/// Ensures the Sessions/<project>/<short-id> <title> node for one
/// `*.jsonl` transcript and fills in its body. `session_id` is the
/// filename without ".jsonl".
fn import_session(doc, project_name, session_path, session_id) {
    let lines = open_file(session_path).read_string().split("\n");

    let title_line = find_line(lines, "\"type\":\"ai-title\"");
    let title = if title_line == "" { "" } else { parse_json(title_line).aiTitle };

    let user_line = find_line(lines, "\"type\":\"user\"");
    if user_line == "" {
        return; // no real user turn -- an aborted or tool-only session
    }
    let first_message = parse_json(user_line);
    let first_prompt = first_text(first_message.message.content);

    let user_count = lines.filter(|line| line.contains("\"type\":\"user\"")).len();
    let assistant_count = lines.filter(|line| line.contains("\"type\":\"assistant\"")).len();

    let short_id = session_id.sub_string(0, 8);
    let label = if title == "" { excerpt(first_prompt, 60) } else { title };
    let leaf = short_id + " " + label;
    leaf.replace("/", "-");

    let node = doc.ensure("Sessions/" + project_name + "/" + leaf);
    node.b = "Started: " + first_message.timestamp + "\n" +
        "Directory: " + first_message.cwd + "\n" +
        "Messages: " + user_count + " user / " + assistant_count + " assistant\n" +
        "Transcript: " + session_path + "\n\n" +
        "First prompt:\n" + excerpt(first_prompt, 400);
}

//@ fn import_claude_history
/// `target` isn't used -- sessions always land under the same fixed
/// `Sessions/` path regardless of what was selected -- but every
/// `COMMANDS` function still takes it (see the file header).
fn import_claude_history(doc, target) {
    let projects_dir = env_var("HOME") + "/.claude/projects";
    let imported = 0;

    for project_entry in open_dir(projects_dir) {
        if !project_entry.is_dir {
            continue;
        }
        let project_path = project_entry.to_string();
        let project_name = project_path.split("/")[-1];

        for session_entry in open_dir(project_path) {
            let session_path = session_entry.to_string();
            if !session_entry.is_file || !session_path.ends_with(".jsonl") {
                continue;
            }
            let session_id = session_path.split("/")[-1];
            session_id = session_id.sub_string(0, session_id.len() - ".jsonl".len());
            import_session(doc, project_name, session_path, session_id);
            imported += 1;
        }
    }

    print("scanned " + imported + " session(s) under " + projects_dir);
}
