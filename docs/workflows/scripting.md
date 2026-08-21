# Scripting with Rhai

`cub` embeds [Rhai](https://rhai.rs), a small scripting language, in two
places:

- **`cub run SCRIPT.rhai`**, a headless command that drives a `.leo` file
  outside the TUI — the tool to reach for when you want to script or test a
  sequence of edits without a terminal.
- **`@action` bodies**, run in-process from inside the TUI when you
  trigger the action (`Shift-A`).

Both give a script the exact same API — a script that works under `cub run`
works, unchanged, as an `@action` body, and vice versa. The only difference
is how the script gets its `Doc`: `cub run` scripts call `open(path)`
themselves, while an `@action` script gets `doc` and `target` predefined,
already bound to the outline the editor has open. Neither is a cut-down
version of the other.

This same `Doc`/`Node` API is meant to be `cub`'s main path for
customization and extension going forward — the way to teach `cub` a new
behavior is generally to script it against this API, rather than to wait
on a new CLI flag or subcommand.

## Why `cub run`

`cub run` opens an outline, calls methods on it, and asserts — no terminal,
no keypresses, just the same library code the other `cub` subcommands use.
That makes it something you can drop straight into a CI step:

```sh
cub run tests/smoke.rhai
```

The process exits non-zero if the script fails to parse, throws, or fails an
`assert`/`assert_eq` — the same signal a test runner gives you anywhere
else.

## A first script

```rhai
let doc = open("notes.leo");

let task = doc.ensure("Project/Tasks/Write docs");
task.b = "Cover the Rhai API.";

assert_eq(task.h, "Write docs");
assert(doc.count() > 0, "outline should not be empty");

doc.save();
print("wrote " + doc.count() + " nodes");
```

Run it with:

```sh
cub run notes.rhai
```

`open` reads an existing `.leo` file — create one first with `cub new` if
you're starting from nothing. Nothing is written back to disk until the
script calls `doc.save()` or `doc.save_as(path)`. `doc.ensure(path)` returns
a `Node` — a handle with `.h`/`.b` properties, the primary way scripts work
with a node; see [Node handles](#node-handles) below for the full
reference.

## Node handles

`doc.node(gnx)` wraps a gnx as a `Node` — a handle with `.h`/`.b`
properties and `.parent()`/`.children()`/`.subtree()` traversal, so a
script that reads, writes, or walks from the same node doesn't have to
keep re-passing its gnx. This is the API to reach for first; a lower-level,
all-gnx `Doc` API (below) exists underneath it for structural operations
(`clone_node`, `remove`) and batches that don't have a `Node` form.

| Signature | Does |
| --- | --- |
| `doc.node(gnx: string) -> Node` | Wraps an existing `gnx` as a `Node`. Fails if `gnx` isn't a node in the outline. |
| `doc.ensure(path: string) -> Node` | Ensures a slash-separated headline path exists (creating missing segments, reusing existing ones — same rules as `cub add`), and returns the leaf as a `Node`. |
| `node.h: string` (get/set) | The node's headline. |
| `node.b: string` (get/set) | The node's body. |
| `node.gnx: string` (read-only) | The plain gnx string this `Node` wraps — for passing to a `Doc` method that doesn't have a `Node` form, like `doc.clone_node`. |
| `node.parent() -> Node` | The parent `Node` (wraps `""` if this node is a root, same as `doc.parent`). |
| `node.children() -> array` | This node's children, in outline order, as `Node`s. |
| `node.subtree() -> array` | This node and everything under it, depth-first (itself first), as `Node`s — the deeper counterpart to `.children()`. |
| `node.path() -> string` | The slash-separated headline path from the root down to this node — same as `doc.path(gnx)`. |
| `node.file_path() -> string` | The on-disk path this node's `@file`/`@thin`/`@file-thin`/`@clean`/`@f` body syncs to — the outline's own directory plus every ancestor `@path` directive plus the filename in the headline, resolved the same way `cub sync` finds it. `""` if this node isn't itself an external-file node — an ancestor's `@path` names a directory for such descendants, not a path for itself. Same as `doc.file_path(gnx)`. |
| `doc.find_h(pattern: string) -> array` | Nodes whose headline matches `pattern` (a regex — same syntax as `cub inspect --search`), as `Node`s in outline order. Fails if `pattern` isn't a valid regex. |
| `doc.find_b(pattern: string) -> array` | Same as `doc.find_h`, but matching a node's body instead of its headline. |

```rhai
let doc = open("notes.leo");
let tasks = doc.ensure("Project/Tasks");
tasks.b = "Backlog for the quarter.";

doc.ensure("Project/Tasks/Write onboarding docs");
doc.ensure("Project/Tasks/Ship v2");

for child in tasks.children() {
    print(child.h);
}

doc.save();
```

A `Node` is just a gnx plus a handle back onto the `Doc` it came from —
`tasks.h = "..."` and the low-level `doc.headline(gnx)` read and write the
exact same data, so the two styles mix freely in one script. A node matched
or visited more than once because of a clone occurrence still yields one
`Node` (its first occurrence), the same rule `doc.children`/`.parent` use.

`doc.find_h`/`doc.find_b` return `Node`s too, so a search result chains
straight into `.h`/`.b`/`.children()` without an extra `doc.node()` call:

```rhai
for n in doc.find_h("^TODO") {
    print(n.path() + ": " + n.b);
}
```

### Building structure with a loop

Since `doc.ensure` creates missing headline segments as it goes, a script can
just loop over the structure it wants instead of assembling a tree upfront:

```rhai
let doc = open("notes.leo");

let teams = ["Team A", "Team B", "Team C"];
for team in teams {
    let tasks = doc.ensure(team + "/Tasks");
    tasks.b = "Backlog for " + team;
}
doc.ensure("Team A/Tasks/Write onboarding docs");

doc.save();
```

Reach for `doc.apply` with `insert-tree`/`merge-tree` (below) instead when
the shape is already data (parsed from JSON, say) rather than something the
script is building up step by step.

## The low-level `Doc` API

`Node` is built on top of a lower-level API directly on `Doc`, where every
node reference is a plain **gnx** string (a Leo global node id, the same
one `cub inspect`/`cub apply` use) rather than a handle. Reach for it
directly for structural operations that don't have a `Node` form yet
(`clone_node`, `remove`), JSON batches (`apply`), or a one-off read/write
that doesn't need a handle. This is the same API an `@action` rhai body
gets through its predefined `doc`, grouped below by what each group is for.

### Opening and saving

| Signature | Does |
| --- | --- |
| `open(path: string) -> Doc` | Global function (not a `Doc` method): reads a `.leo` file from disk and returns a new `Doc`. Available to `@action` scripts too, for reading a *different* file than the one the editor has open — the action's own outline arrives already bound as `doc`, not through this call. |
| `doc.save()` | Writes back to the path the `Doc` was opened or bound with. |
| `doc.save_as(path: string)` | Writes to a different path and retargets future `doc.save()` calls there. |

### Resolving and creating nodes

| Signature | Does |
| --- | --- |
| `doc.gnx(path: string) -> string` | Resolves a headline path to a gnx without creating anything; fails if it's missing or ambiguous. |
| `doc.ensure(path: string) -> Node` | Same [`doc.ensure`](#node-handles) as above — creating and reaching for a node is normally what you want a handle for. Use `.gnx` on the result for the plain string, e.g. to feed into `doc.clone_node`. |
| `doc.path(gnx: string) -> string` | The slash-separated headline path from the root down to `gnx` — the inverse of `doc.gnx(path)`, so `doc.gnx(doc.path(gnx)) == gnx`. |
| `doc.file_path(gnx: string) -> string` | See [`node.file_path()`](#node-handles) above — the same lookup, taking a gnx instead of a `Node`. |

### Traversing structure

| Signature | Does |
| --- | --- |
| `doc.roots() -> array` | The gnxs of the outline's top-level nodes, in outline order. |
| `doc.children(gnx: string) -> array` | `gnx`'s children's gnxs, in outline order (empty for a leaf); fails if `gnx` isn't in the outline. |
| `doc.subtree(gnx: string) -> array` | `gnx` and every gnx under it, depth-first in outline order (`gnx` itself first); fails if `gnx` isn't in the outline. |
| `doc.all() -> array` | Every gnx in the outline, depth-first in outline order. |
| `doc.parent(gnx: string) -> string` | `gnx`'s parent's gnx, or `""` if `gnx` is a root; fails if `gnx` isn't in the outline. |

`doc.children`/`doc.subtree`/`doc.all`/`doc.parent`/`doc.path` walk the
tree structurally (parent/child links), independent of headlines, so they
work even when headlines aren't unique. `doc.subtree`/`doc.all` are the
ones to reach for when a script wants to visit a whole (sub)tree — they
return the full flattened list up front rather than making the script
manage its own traversal (`node.subtree()` above returns the same thing as
`Node`s, which is usually the more convenient form):

```rhai
let doc = open("notes.leo");
for gnx in doc.subtree(doc.gnx("Project")) {
    print(doc.headline(gnx));
}
```

Reach for `doc.children` directly, one level at a time, only when a walk
needs to stop early or skip a branch based on what it finds — `doc.subtree`
always visits everything under the node.

Clones (the same node appearing at more than one position) don't have a
separate identity in this API — a gnx is a node's identity, not one
occurrence's, so `doc.children`/`doc.parent`/`doc.path` always answer for
the node's first position in the outline, which is what you want almost
all of the time since a clone's children are shared across every
occurrence by definition. `doc.all` is the exception: it walks every
*position*, the same way Leo's `c.all_positions()` does, so a node cloned
to three places yields its gnx three times. The one thing this API can't
tell you is *which* occurrence of a multiply-cloned node a particular
action targets — `target` in an `@action` body identifies the node, not
the specific occurrence the user had selected.

### Reading and writing content

| Signature | Does |
| --- | --- |
| `doc.headline(gnx: string) -> string` / `doc.set_headline(gnx: string, text: string)` | Read or write a node's headline — the low-level equivalent of `node.h`. |
| `doc.body(gnx: string) -> string` / `doc.set_body(gnx: string, text: string)` | Read or write a node's body — the low-level equivalent of `node.b`. |

### Clone and remove

| Signature | Does |
| --- | --- |
| `doc.clone_node(gnx: string, parent_gnx: string) -> string` | Inserts a new occurrence of `gnx` as `parent_gnx`'s last child. Both `gnx` and `parent_gnx` must already be nodes in the outline — nothing is created; resolve a headline path to a gnx first with `doc.gnx(path)`/`doc.ensure(path).gnx` if that's what a script has. Returns `gnx`. |
| `doc.clone_node(gnx: string, parent_gnx: string, index: int) -> string` | Same, inserting at `index` among `parent_gnx`'s existing children instead of appending. |
| `doc.remove(gnx: string)` | Removes `gnx`'s defining occurrence and its subtree. Other clone occurrences of `gnx`, if any, are left in place. |

### Batch operations

| Signature | Does |
| --- | --- |
| `doc.apply(json: string) -> string` | Applies a `cub apply`-style [operation batch](../workflows/automation.md) given as a JSON string; returns the report as a JSON string. |

`insert-tree`/`merge-tree`/`replace-tree` predate the rhai API and are
still the right tool when a script already has a tree's worth of *data* to
drop in — say, JSON pulled from an import or generated report — but for
structure the script is building up itself, `doc.ensure` is usually more
direct than assembling this JSON:

```rhai
let doc = open("notes.leo");
let report = doc.apply(`{
  "operations": [
    {"op": "insert-tree", "parent-headline": "Imports/PRs",
     "tree": {"PR #142: Fix flaky retry": {"_body": "..."}}}
  ]
}`);
print(report);
doc.save();
```

(Rhai's `` `...` `` backtick strings span multiple lines, which is handy for
inline JSON like this.)

### Inspecting the outline

| Signature | Does |
| --- | --- |
| `doc.render() -> string` | The whole outline as `cub render`'s compact Markdown. |
| `doc.count() -> int` | Number of nodes in the outline. |
| `doc.validate() -> array` | Validation error strings; empty means valid. |

### Cloning and removing a node

`doc.clone_node` adds another occurrence of an existing node — not a copy,
the same node appearing at a second position — as a specific parent's last
child:

```rhai
let doc = open("notes.leo");

let source = doc.ensure("Team A/Tasks").gnx;
let team_b = doc.ensure("Team B").gnx;
doc.clone_node(source, team_b);

// The clone is the same node, not a copy, so both parents' children
// resolve to the same gnx.
assert_eq(doc.children(team_b)[0], source);

doc.save();
```

Give a third argument to insert at a specific index instead of appending:
`doc.clone_node(source, team_b, 0)`.

Like every other low-level `Doc` method, `clone_node` takes gnxs, not
headline paths — if a script only has a path for the parent, resolve it
once with `doc.gnx(path)` (or `doc.ensure(path).gnx` if it also needs
creating) rather than passing the path around; that's the only place in
the whole API a path ever needs resolving, and it means `clone_node` fails
cleanly on a parent that doesn't exist yet instead of quietly creating one
that doesn't match what the script meant.

`doc.remove(gnx)` is the reverse: it deletes `gnx`'s defining occurrence and
its whole subtree. If `gnx` is cloned elsewhere, those other occurrences are
left alone — the next one in outline order becomes the new defining
occurrence, transparently, so nothing else in the script needs to change:

```rhai
// Removes "Team A/Tasks" (source's defining occurrence). The clone under
// team_b survives and becomes source's new defining occurrence.
doc.remove(source);
assert_eq(doc.parent(source), team_b);
```

Reach for `doc.apply` directly instead when a script needs to batch a
clone or removal together with other operations atomically. See
[the `clone` operation](../workflows/automation.md#cloning-a-node) for the
full JSON shape.

## Assertions and output

| Signature | Does |
| --- | --- |
| `assert(cond: bool)` | Fails the script if `cond` is false. |
| `assert(cond: bool, msg: string)` | Same, with a message included in the failure. |
| `assert_eq(a: any, b: any)` | Fails the script if `a` and `b` differ; compares numbers, booleans, and strings. |
| `print(...)` | Writes to stdout. |
| `debug(...)` | Writes to stderr, with its source position. |

Ordinary Rhai — string concatenation with `+`, `if`, `for`, functions, and
so on — all works too.

A failed assertion aborts the script and `cub run` exits non-zero:

```sh
$ cub run broken.rhai
Error: run broken.rhai

Caused by:
    Runtime error: assertion failed: 1 != 2 (line 3, position 1)
$ echo $?
1
```

## Running a subprocess

Every script — `cub run` or a bound `@action` alike — runs in-process
against the `Doc` API above. For the rare case where a script still needs
to shell out (a build step, `git`, some other CLI tool), use `sh`:

| Signature | Does |
| --- | --- |
| `sh(cmd: string) -> #{stdout, stderr, code}` | Runs `cmd` through `sh -c` and returns a map of what it produced. `code` is the exit status (`-1` if the process was killed by a signal). |
| `sh(cmd: string, opts: #{cwd?: string}) -> #{stdout, stderr, code}` | Same, with `opts.cwd` as the subprocess's working directory. Without it, `cmd` runs relative to `cub`'s own working directory, not the open `.leo` file's. |

```rhai
let r = sh("git rev-parse --short HEAD");
assert_eq(r.code, 0, "git failed: " + r.stderr);
print("HEAD is " + r.stdout.trim());

let r2 = sh("cat notes.txt", #{ cwd: "quickstart-files" });
```

`sh` never throws for a nonzero exit — that's for the script to check via
`r.code`, same as `cub apply`'s `$?` in a shell script. It only fails
(throwing, like the rest of the `Doc` API) if `sh` itself can't be
launched.

## `@action` bodies

An `@action` node's headline marks it as runnable from the action palette
(`Shift-A`); its body is a rhai script, run in-process with two symbols
predefined — no `open()` call needed:

| Symbol | Is |
| --- | --- |
| `doc` | A `Doc` already bound to the outline this editor session has open — the same object the TUI itself is showing, not a fresh read from disk. Every method above works on it, and mutations are visible in the editor as soon as the action finishes. |
| `target` | The gnx of the node the user had selected when they invoked the action — not the `@action` node itself, which may live anywhere in the tree. Wrap it with `doc.node(target)` for a `Node` handle, or use it directly with the low-level `doc.headline`/`doc.body`/`doc.set_headline`/`doc.set_body`. |

```rhai
let n = doc.node(target);
n.h = n.h + " ✓";
n.b = n.b + "\nDone: " + doc.count() + " nodes total.";
print("marked " + n.h);
```

`print`/`debug` output becomes the action's displayed output, shown in the
body pane until the selection moves.

Running a script that calls a method which mutates the outline (`doc.ensure`,
`node.h =`/`node.b =`, `doc.set_headline`, `doc.set_body`, `doc.apply`, …)
marks the outline dirty and refreshes the editor's caches, the same as any
other edit; a script that only reads (`n.h`, `doc.render()`, …) or that
throws before mutating anything leaves the outline exactly as it was.

A body may still start with a leftover `@language rhai` directive from
before every `@action` body ran as rhai unconditionally — it's stripped
before the script runs, so it's harmless, but no longer needed.

## Practical guardrails

- Keep the original `.leo` file under version control before running a
  script that saves, or before running `@action` bodies that mutate `doc`.
- Prefer scripts that `assert` their way through a scenario over ones that
  print output for a human to eyeball — that's what makes `cub run` usable
  as a CI check.
- `doc.validate()` after a batch of edits catches structural mistakes
  (dangling references, orphaned nodes) before they reach disk.
- An `@action` script's mutations land in the editor's in-memory outline
  immediately, but disk is untouched until the usual save (or an explicit
  `doc.save()` in the script) — `Ctrl-Z`/quit-without-saving still works as
  an escape hatch.

## Compared to Leo's Python scripting

Classic [Leo](https://leo-editor.github.io/leo-editor/) predefines three
symbols for a script run from a node: **`c`** (the commander — the whole
outline, plus all of Leo's own code), **`g`** (`leo.core.leoGlobals`, a grab
bag of utilities), and **`p`** (the currently selected **position** — a
cursor-like object with `.h`/`.b` properties and traversal methods like
`p.parent()`, `p.next()`, `p.children()`).

`cub`'s rhai API plays the same role with a much smaller surface, and maps
onto it loosely:

| Leo | cub | Difference |
| --- | --- | --- |
| `c` | `doc` | `c` is a live, mutable handle onto Leo's entire running commander (undo stack, GUI frame, all subsystems). `doc` is just the outline plus save/apply/validate — no undo stack, no GUI. |
| `p` | `doc.node(target)` | Leo's `p` is a stateful **position** object: it can walk the tree (`p.next()`, `p.parent()`, `p.children()`) and becomes invalid if the outline changes under it. `doc.node(gnx)` returns a `Node` handle with the same shape of API, keyed by stable gnx instead of a position that can go stale. `target` itself is the plain gnx **string**, for scripts that don't need a handle. |
| `p.h` / `p.b` | `node.h` / `node.b` | Same property syntax; cub's is backed by gnx identity rather than a live tree position, so it stays valid even if the outline changes around it. The low-level `doc.headline(gnx)`/`doc.body(gnx)` read and write the same data for a one-off call. |
| `v.gnx` | `node.gnx`, or the gnx string itself | Every node handle in cub's API already *is*, or wraps, its gnx — there's no separate node/vnode object to unwrap it from. |
| `g.es(...)` | `print(...)` | Both write to a log a human is expected to read; cub's is plain Rhai `print`, not a Leo-specific function. |
| `p.children()` / `p.parent()` | `node.children()` / `node.parent()` | Same idea, returning further `Node`s so a walk can keep chaining `.h`/`.b`/`.children()`. The low-level `doc.children(gnx)`/`doc.parent(gnx)` return plain gnx strings instead. |
| `p.self_and_subtree()` | `node.subtree()` | Leo's is a lazy generator you `for p in ...`; cub's returns the whole flattened array of `Node`s up front — outlines are small enough that this is simpler than adding lazy iterators to the embedding. |
| `c.all_positions()` | `doc.all()` | Same trade-off as `subtree`, but document-wide rather than node-scoped: one eager array of every gnx in outline order, rather than a generator. No `Node` form, since it isn't rooted at one node. |
| `c.undoer` | *(none)* | cub scripts aren't undoable the way a Leo `@button` command is; treat a script's edits like any other unreviewed change and keep the file under version control. |

The practical upshot: a cub script built on `Node` reads a lot like Leo's
`p`-based scripts — walk from a node, read or write `.h`/`.b`, recurse into
`.children()` — just keyed by a stable gnx instead of a position that can
invalidate itself. The low-level `Doc` API underneath is closer to `cub
apply`'s operation-batch style: resolve the gnx you want, then read or
write it directly, without a handle. Reach for it for structural edits
(`clone_node`, `remove`) and JSON batches that don't have a `Node` form.
