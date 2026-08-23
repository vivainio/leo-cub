# Scripting with Rhai

`cub` embeds [Rhai](https://rhai.rs), a small scripting language for
automating an outline. Scripts can run from the command line or from the
TUI, and they all use the same [`Doc` and `Node` API](../reference/rhai-api.md).

## Choose how to run a script

| Use | Best for | How the script gets the outline |
| --- | --- | --- |
| `cub run SCRIPT.rhai` | Repeatable tasks, checks, and CI | Call `open(path)` in the script. |
| `@action name` | A short command stored in an outline | `doc` and the selected node are predefined. |
| `@import path/to/script.rhai` | Commands shared between outlines or kept in source control | Export functions that receive the open document and selected node. |

Start with `cub run` when experimenting outside the editor. Use an `@action`
when the behavior belongs in one outline, and `@import` when it belongs in a
regular source file.

`cub SCRIPT.rhai` (no `run`) works too -- the same shorthand as `cub
notes.leo` opening the TUI, dispatched by the file's `.rhai` extension.

## Run your first script

Create `notes.rhai`:

```rhai
let doc = open("notes.leo");

let task = doc.ensure("Project/Tasks/Write docs");
task.b = "Cover the Rhai API.";

assert_eq(task.h, "Write docs");
doc.save();

print("wrote " + doc.count() + " nodes");
```

Create the outline if it does not exist, then run the script:

```sh
cub new notes.leo
cub run notes.rhai
```

`open()` reads the outline into a `Doc`. `ensure()` creates any missing
segments of a slash-separated headline path and returns the final node.
Changing `task.b` updates that node's body in memory. The file is not written
until `doc.save()` or `doc.save_as(path)` is called.

If parsing fails, a method throws, or an assertion fails, `cub run` exits
non-zero. This makes the same kind of script useful both interactively and in
CI.

## Work with nodes

A `Node` is the usual way to read, edit, and traverse an outline:

```rhai
let doc = open("notes.leo");
let tasks = doc.ensure("Project/Tasks");

doc.ensure("Project/Tasks/Write onboarding docs");
doc.ensure("Project/Tasks/Ship v2");

for child in tasks.children() {
    print(child.h);
}

for todo in doc.find_h("^TODO") {
    let h = todo.h;
    h.replace("TODO", "DONE");
    todo.h = h;
}

doc.save();
```

The properties and methods used most often are:

| API | Purpose |
| --- | --- |
| `node.h`, `node.b` | Read or change the headline and body. |
| `node.parent()` | Get the parent node. |
| `node.children()` | Get the direct children in outline order. |
| `node.subtree()` | Get this node and all its descendants. |
| `node.path()` | Get its slash-separated headline path. |
| `node.gnx` | Get its stable Leo node id. |
| `doc.ensure(path)` | Find or create a headline path. |
| `doc.find_h(regex)`, `doc.find_b(regex)` | Search headlines or bodies. |

See the [Rhai API reference](../reference/rhai-api.md) for the complete API,
including GNX-based operations, clones, filesystem access, and subprocesses.

## Create an action in the TUI

An `@action` node is a command stored directly in an outline:

1. Create a node with the headline `@action mark done`.
2. Put this Rhai code in its body:

    ```rhai
    p.h = p.h + " ✓";
    p.b = p.b + "\nDone.";
    print("marked " + p.h);
    ```

3. Select the node you want to change.
4. Press `Shift-A`, choose **mark done**, and press `Enter`.

An action body gets these values automatically:

| Name | Value |
| --- | --- |
| `doc` | The `Doc` currently open in the TUI. |
| `p` | A `Node` for the exact occurrence selected when the action was invoked. |
| `target` | The selected node's GNX string. Prefer `p` unless an API specifically needs a GNX. |

There is no `open()` call because the action operates directly on the TUI's
in-memory outline. Mutations mark the outline as changed, but do not write it
to disk until the usual `Ctrl-S` save. `print()` and `debug()` output appears
in the body pane until the selection moves.

## Share commands with `@import`

For commands you want to edit as normal source files or reuse across outlines,
put the Rhai code in a file. For example, `scripts/tasks.rhai` could contain:

```rhai
const COMMANDS = ["mark_done", "list_todos"];

fn mark_done(doc, target) {
    target.h = target.h + " ✓";
}

fn list_todos(doc, target) {
    for node in doc.find_h("^TODO") {
        print(node.path());
    }
}
```

Then add a node to the outline with this headline and an empty body:

```text
@import scripts/tasks.rhai
```

The path is relative to the `.leo` file. Each function named in `COMMANDS`
appears in the `Shift-A` action palette. It must accept exactly `(doc,
target)`, where `target` is a positioned `Node` for the current selection.
Other functions may use any signature, but do not appear in the palette.

The repository's [`scripts/github.rhai`](https://github.com/vivainio/leo-cub/blob/main/scripts/github.rhai)
is a larger example with both palette commands and private helper functions.

## Configure a script with an `@variables` tree

`@variables` isn't a directive `cub` treats specially -- it's a plain
headline convention for settings a script should read instead of
hard-coding. Put one node headlined `@variables` in the outline, with one
child per setting: the child's headline is the name, its body is the value.

```text
@variables
  repo
    leo-editor/leo-editor
```

A script reads it with `doc.ensure()` and `children()`, same as any other
lookup:

```rhai
fn get_variables(doc) {
    let vars = #{};
    for child in doc.ensure("@variables").children() {
        let value = child.b;
        value.trim();
        vars[child.h] = value;
    }
    vars
}
```

`doc.ensure("@variables")` returns an empty node (no children) when the tree
is missing, so `get_variables()` just yields `#{}` rather than throwing --
callers decide what an absent key should default to.

`scripts/github.rhai` uses exactly this pattern for a `repo` variable: when
`@variables/repo` is set to `owner/name`, every `gh` command it runs adds
` --repo owner/name`, targeting that repository instead of the one implied by
the current directory. Leave `@variables` (or the `repo` child) absent and
`gh` falls back to its own default.

`@variables` suits settings that belong to one outline. For a value that
changes per invocation instead -- which outline to open, a one-off flag --
pass it on the command line and read it from `ARGS`, an array of the
strings after the script path:

```sh
cub run rename.rhai notes.leo "A/B/C" "new headline"
```

```rhai
let doc = open(ARGS[0]);
doc.set_headline(doc.gnx(ARGS[1]), ARGS[2]);
doc.save();
```

`ARGS` is `[]` when no extra arguments were given, so a script that only
sometimes needs one can check `ARGS.len()` first.

## Assertions, output, and errors

```rhai
assert(doc.count() > 0, "outline should not be empty");
assert_eq(doc.validate().len, 0);
print("check passed");
debug(doc.render());
```

- `assert(condition)` and `assert(condition, message)` stop on a false value.
- `assert_eq(actual, expected)` stops when the values differ.
- `print(...)` writes normal output; `debug(...)` includes its source position.
- A parsing error, thrown API error, or failed assertion stops the script.

For a command-line script, failure produces a non-zero exit status. In an
action, the error is displayed in the TUI.

## Run external tools and read files

Use `doc.sh()` when a command should run relative to the open outline:

```rhai
let result = doc.sh("git rev-parse --short HEAD");
assert(result.code == 0, "git failed: " + result.stderr);
let hash = result.stdout;
hash.trim();
print(hash);
```

The result contains `stdout`, `stderr`, and `code`. A non-zero command does not
throw, so check `code` yourself. Rhai's `trim()` mutates its target in place and
returns nothing, so it needs a variable to act on -- `result.stdout.trim()`
discards the trimmed copy; see [gotchas](../reference/rhai-api.md#gotchas) in
the API reference. The global `sh(command)` form instead uses
`cub`'s current working directory and also accepts `#{ cwd: "path" }` options.

For direct filesystem access, Rhai's file functions are available too:

```rhai
let log = doc.dir() + "/script.log";
let file = open_file(log, "a");
file.write("script ran\n");
```

The [API reference](../reference/rhai-api.md#subprocesses-and-files) lists the
available path and file operations.

## Apply a JSON operation batch

Direct `Node` edits are clearest for most scripts. Use `doc.apply()` when you
already have tree-shaped JSON or need several operations to be transactional:

```rhai
let doc = open("notes.leo");
let report = doc.apply(`{
  "operations": [
    {"op": "insert-tree", "parent-headline": "Imports/PRs",
     "tree": {"PR #142": {"_body": "Ready for review"}}}
  ]
}`);

print(report);
doc.save();
```

See [Automation and AI tools](automation.md) for operation formats such as
`insert-tree`, `merge-tree`, and `replace-tree`.

## Clones and destructive edits

A GNX identifies a node, while a position identifies one occurrence of it.
This only matters when a node is cloned into more than one place:

- Nodes reached through `p`, `node_at()`, `children()`, or `subtree()` retain
  their exact position.
- Nodes reached only by GNX may fall back to the first occurrence.
- `node.remove()` removes the exact occurrence when the node has a position.

For destructive work involving clones, start from a positioned node. The
[API reference](../reference/rhai-api.md#nodes-clones-and-positions) explains
the detailed behavior.

## If you know Leo's Python scripting

The names are different, but the everyday shape should feel familiar:

| Leo | cub Rhai |
| --- | --- |
| `c` | `doc` |
| `p` | `p` in an action, or another `Node` |
| `p.h`, `p.b` | `node.h`, `node.b` |
| `p.children()` | `node.children()` |
| `g.es(...)` | `print(...)` |

`cub` deliberately exposes a smaller outline-focused API. A `Node` is stable
by GNX and may also carry a position to distinguish clones; it is not a live
cursor over the full Leo application.

## Keep scripted changes safe

- Keep important outlines under version control or make a backup first.
- Remember that `cub run` writes only when the script calls `save()`.
- Remember that TUI actions change memory immediately but use the normal save
  flow unless they explicitly call `save()`.
- Assert assumptions before destructive edits.
- Call `doc.validate()` after a substantial batch of changes.
- Use `cub apply --dry-run` when developing external JSON operation batches.
