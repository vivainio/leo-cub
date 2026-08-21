# Scripting with Rhai

`cub` embeds [Rhai](https://rhai.rs), a small scripting language, in two
places:

- **`cub run SCRIPT.rhai`**, a headless command that drives a `.leo` file
  outside the TUI — the tool to reach for when you want to script or test a
  sequence of edits without a terminal.
- **`@action` bodies** with an `@language rhai` directive, run in-process
  from inside the TUI when you trigger the action (`Shift-A`).

Both give a script the exact same `Doc` API — a script that works under
`cub run` works, unchanged, as an `@action` body, and vice versa. The only
difference is how the script gets its `Doc`: `cub run` scripts call
`open(path)` themselves, while an `@action` script gets `doc` and `target`
predefined, already bound to the outline the editor has open. Neither is a
cut-down version of the other.

## Why `cub run`

Earlier versions of `cub` had a jsonl script format for driving the TUI
(pressing keys, asserting on the rendered screen) that doubled as a rough
integration-test mechanism. It needed a real terminal, so it was never
actually runnable in CI. `cub run` replaces that: it opens an outline, calls
methods on it, and asserts — no terminal, no keypresses, just the same
library code the other `cub` subcommands use. That makes it something you
can drop straight into a CI step:

```sh
cub run tests/smoke.rhai
```

The process exits non-zero if the script fails to parse, throws, or fails an
`assert`/`assert_eq` — the same signal a test runner gives you anywhere
else.

## A first script

```rhai
let doc = open("notes.leo");

let gnx = doc.add("Project/Tasks/Write docs");
doc.set_body(gnx, "Cover the Rhai API.");

assert_eq(doc.headline(gnx), "Write docs");
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
script calls `doc.save()` or `doc.save_as(path)`.

## The `Doc` API

`open(path)` returns a `Doc`, whose methods mutate or read the in-memory
outline. This is the complete API — the same one an `@action` rhai body
gets through its predefined `doc`:

| Method | Does |
| --- | --- |
| `open(path)` | Global function (not a `Doc` method): reads a `.leo` file from disk and returns a new `Doc`. Available to `@action` scripts too, for reading a *different* file than the one the editor has open — the action's own outline arrives already bound as `doc`, not through this call. |
| `doc.add(path)` | Ensures a slash-separated headline path exists (creating missing segments, reusing existing ones — same rules as `cub add`); returns the leaf's gnx. |
| `doc.gnx(path)` | Resolves a headline path to a gnx without creating anything; fails if it's missing or ambiguous. |
| `doc.roots()` | An array of the gnxs of the outline's top-level nodes, in outline order. |
| `doc.children(gnx)` | An array of `gnx`'s children's gnxs, in outline order (empty for a leaf); fails if `gnx` isn't in the outline. |
| `doc.subtree(gnx)` | An array of `gnx` and every gnx under it, depth-first in outline order (`gnx` itself first); fails if `gnx` isn't in the outline. |
| `doc.all()` | An array of every gnx in the outline, depth-first in outline order. |
| `doc.parent(gnx)` | `gnx`'s parent's gnx, or `""` if `gnx` is a root; fails if `gnx` isn't in the outline. |
| `doc.path(gnx)` | The slash-separated headline path from the root down to `gnx` — the inverse of `doc.gnx(path)`/`doc.add(path)`, so `doc.gnx(doc.path(gnx)) == gnx`. |
| `doc.headline(gnx)` / `doc.set_headline(gnx, text)` | Read or write a node's headline. |
| `doc.body(gnx)` / `doc.set_body(gnx, text)` | Read or write a node's body. |
| `doc.render()` | The whole outline as `cub render`'s compact Markdown. |
| `doc.count()` | Number of nodes in the outline. |
| `doc.validate()` | An array of validation error strings; empty means valid. |
| `doc.apply(json)` | Applies a `cub apply`-style [operation batch](../workflows/automation.md) given as a JSON string; returns the report as a JSON string. |
| `doc.save()` | Writes back to the path the `Doc` was opened or bound with. |
| `doc.save_as(path)` | Writes to a different path and retargets future `doc.save()` calls there. |

`doc.children`/`doc.subtree`/`doc.all`/`doc.parent`/`doc.path` walk the
tree structurally (parent/child links), independent of headlines, so they
work even when headlines aren't unique. `doc.subtree`/`doc.all` are the
ones to reach for when a script wants to visit a whole (sub)tree — they
return the full flattened list up front rather than making the script
manage its own traversal:

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

`doc.apply` is the escape hatch for anything the other methods don't cover
directly — `insert-tree`, `merge-tree`, `replace-tree`, and the rest of the
[operation batch](../workflows/automation.md) format all work from a script
the same way they do from `cub apply`:

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

### Building structure with a loop

`doc.add` is usually the more direct way to build a tree from a script than
`doc.apply` with `insert-tree` — since it creates missing headline segments
as it goes, a script can just loop over the structure it wants:

```rhai
let doc = open("notes.leo");

let teams = ["Team A", "Team B", "Team C"];
for team in teams {
    let tasks = doc.add(team + "/Tasks");
    doc.set_body(tasks, "Backlog for " + team);
}
doc.add("Team A/Tasks/Write onboarding docs");

doc.save();
```

Reach for `doc.apply` with `insert-tree`/`merge-tree` instead when the shape
is already data (parsed from JSON, say) rather than something the script is
building up step by step.

### Cloning a node

Cloning — adding another occurrence of an existing node, rather than a new
node — has no dedicated `Doc` method; go through `doc.apply` with a `clone`
operation, the same as `cub apply` would:

```rhai
let doc = open("notes.leo");

let source = doc.gnx("Team A/Tasks");
doc.apply(`{
  "operations": [
    {"op": "clone", "parent-headline": "Shared/Cross-team", "index": 0, "node": "` + source + `"}
  ]
}`);

// The clone is the same node, not a copy, so both headline paths resolve
// to the same gnx and share the same children.
assert_eq(doc.gnx("Shared/Cross-team/Tasks"), source);

doc.save();
```

Like `insert-tree`, `clone` takes `"parent"` (a gnx) or `"parent-headline"`
(created if missing), never both. See
[the `clone` operation](../workflows/automation.md#cloning-a-node) for the
full JSON shape.

Every node reference in this API is a **gnx** (a Leo global node id, the
same string `cub inspect`/`cub apply` use) — not a stateful position or
cursor object. `doc.gnx(path)` and `doc.add(path)` are how a script turns a
readable slash-separated headline path into the gnx the rest of the API
takes.

## Assertions and output

- `assert(cond)` / `assert(cond, "message")` — fails the script if `cond` is
  false.
- `assert_eq(a, b)` — fails the script if `a` and `b` differ; compares
  numbers, booleans, and strings.
- `print(...)` writes to stdout; `debug(...)` writes to stderr with its
  source position. Ordinary Rhai — string concatenation with `+`, `if`,
  `for`, functions, and so on — all works too.

A failed assertion aborts the script and `cub run` exits non-zero:

```sh
$ cub run broken.rhai
Error: run broken.rhai

Caused by:
    Runtime error: assertion failed: 1 != 2 (line 3, position 1)
$ echo $?
1
```

## `@action` bodies

An `@action` node's headline marks it as runnable from the action palette
(`Shift-A`); its body is the script. A body that starts with an
`@language rhai` directive runs in-process, with two symbols predefined —
no `open()` call needed:

| Symbol | Is |
| --- | --- |
| `doc` | A `Doc` already bound to the outline this editor session has open — the same object the TUI itself is showing, not a fresh read from disk. Every `Doc` method above works on it, and mutations are visible in the editor as soon as the action finishes. |
| `target` | The gnx of the node the user had selected when they invoked the action — not the `@action` node itself, which may live anywhere in the tree. Combine it with `doc.headline`/`doc.body`/`doc.set_headline`/`doc.set_body` to read or change the node the action was run against. |

```rhai
@language rhai
doc.set_headline(target, doc.headline(target) + " ✓");
doc.set_body(target, doc.body(target) + "\nDone: " + doc.count() + " nodes total.");
print("marked " + doc.headline(target));
```

`print`/`debug` output becomes the action's displayed output (shown in the
body pane until the selection moves), exactly like a subprocess action's
captured stdout/stderr — `@apply` (below) also still works for a rhai body,
since it only looks at that same output text.

Running a script that calls a `doc` method which mutates the outline
(`add`, `set_headline`, `set_body`, `apply`) marks the outline dirty and
refreshes the editor's caches, the same as any other edit; a script that
only reads (`doc.headline(target)`, `doc.render()`, …) or that throws before
mutating anything leaves the outline exactly as it was.

### `@action` bodies in other languages

Without `@language rhai`, an `@action` body runs as a subprocess (`sh` by
default; `@language python`/`js`/`ruby`/`bash`/`nu` select an interpreter).
A subprocess has no in-process access to `doc`/`target`, so it gets the same
information through environment variables instead:

| Env var | Is |
| --- | --- |
| `CUB_GNX` | The target's gnx — the subprocess equivalent of `target`. |
| `CUB_PARENT_GNX` | The target's parent's gnx; unset if the target is a root. Rhai equivalent: `doc.parent(target)` (`""` for a root instead of unset). |
| `CUB_HEADLINE` | The target's headline. Rhai equivalent: `doc.headline(target)`. |
| `CUB_POSITION` | The target's position id (e.g. `0/1`) — identifies the specific clone *occurrence* invoked, which gnx-based `target` cannot. |
| `CUB_PATH` | The target's slash-separated headline path from the root. Rhai equivalent: `doc.path(target)`. |
| `CUB_DOC` | The open `.leo` file's absolute path. |

A subprocess can't mutate the outline directly — it can only shell out to
`cub` itself (`cub apply "$CUB_DOC" ...`), or write a JSON operation batch
to stdout and add a bare `@apply` directive line to the body, which tells
`cub` to parse that stdout as a batch and apply it once the process exits
successfully:

```
@language python
@apply
import json, os
print(json.dumps({
    "operations": [{"op": "set-body", "node": os.environ["CUB_GNX"], "body": "done"}]
}))
```

A rhai body needs none of this — `doc` already *is* the outline, so
`doc.set_body(target, "done")` does the same thing directly, in-process,
with no serialization round trip.

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
| `p` | `target` | Leo's `p` is a stateful **position** object: it can walk the tree (`p.next()`, `p.parent()`, `p.children()`) and becomes invalid if the outline changes under it. cub's `target` is a plain gnx **string** — stable across edits, but with no traversal methods of its own; reach a different node with `doc.gnx(path)`/`doc.add(path)` instead of walking from `target`. |
| `p.h` / `p.b` | `doc.headline(gnx)` / `doc.body(gnx)` | Leo exposes headline/body as properties on the position; cub exposes them as `Doc` methods taking an explicit gnx, since cub has no position object to hang a property off of. |
| `v.gnx` | *(the gnx string itself)* | Every node handle in cub's API already *is* its gnx — there's no separate node/vnode object to unwrap it from. |
| `g.es(...)` | `print(...)` | Both write to a log a human is expected to read; cub's is plain Rhai `print`, not a Leo-specific function. |
| `p.children()` / `p.parent()` | `doc.children(gnx)` / `doc.parent(gnx)` | Same idea, but methods on `doc` taking/returning gnx strings rather than generators yielding position objects. |
| `p.self_and_subtree()` | `doc.subtree(gnx)` | Leo's is a lazy generator you `for p in ...`; cub's returns the whole flattened array up front — outlines are small enough that this is simpler than adding lazy iterators to the embedding. |
| `c.all_positions()` | `doc.all()` | Same trade-off as `subtree`: one eager array of every gnx in outline order, rather than a generator. |
| `c.undoer` | *(none)* | cub scripts aren't undoable the way a Leo `@button` command is; treat a script's edits like any other unreviewed change and keep the file under version control. |

The practical upshot: a cub script tends to look less like "walk the tree
and touch what you find" and more like "resolve the headline path or gnx
you want, then read or write it directly" — closer to `cub apply`'s
operation-batch style than to Leo's position-generator style.
