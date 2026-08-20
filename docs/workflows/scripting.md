# Scripting with Rhai

`cub` embeds [Rhai](https://rhai.rs), a small scripting language, in two
places:

- **`@action` bodies** with an `@language rhai` directive, run in-process
  from inside the TUI.
- **`cub run SCRIPT.rhai`**, a headless command that drives a `.leo` file
  outside the TUI — the tool to reach for when you want to script or test a
  sequence of edits without a terminal.

This page focuses on `cub run`, since that's the one meant for scripts, CI,
and agents.

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
outline:

| Method | Does |
| --- | --- |
| `doc.add(path)` | Ensures a slash-separated headline path exists (creating missing segments, reusing existing ones — same rules as `cub add`); returns the leaf's gnx. |
| `doc.gnx(path)` | Resolves a headline path to a gnx without creating anything; fails if it's missing or ambiguous. |
| `doc.headline(gnx)` / `doc.set_headline(gnx, text)` | Read or write a node's headline. |
| `doc.body(gnx)` / `doc.set_body(gnx, text)` | Read or write a node's body. |
| `doc.render()` | The whole outline as `cub render`'s compact Markdown. |
| `doc.count()` | Number of nodes in the outline. |
| `doc.validate()` | An array of validation error strings; empty means valid. |
| `doc.apply(json)` | Applies a `cub apply`-style [operation batch](../workflows/automation.md) given as a JSON string; returns the report as a JSON string. |
| `doc.save()` | Writes back to the path `open` used. |
| `doc.save_as(path)` | Writes to a different path and retargets future `doc.save()` calls there. |

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

## `@action` bodies, for comparison

An `@action` node whose body starts with an `@language rhai` directive runs
the rest of the body as a script when triggered from the action palette in
the TUI, in-process rather than as a subprocess. It only has `print`/`debug`
available — no outline access, since it's meant for quick in-editor
scripting rather than driving edits programmatically. Reach for `cub run`
instead when a script needs to read or change the outline it's acting on.

## Practical guardrails

- Keep the original `.leo` file under version control before running a
  script that saves.
- Prefer scripts that `assert` their way through a scenario over ones that
  print output for a human to eyeball — that's what makes `cub run` usable
  as a CI check.
- `doc.validate()` after a batch of edits catches structural mistakes
  (dangling references, orphaned nodes) before they reach disk.
