# Files and paths

An outline node can be a plain headline and body, or it can stand in for a
real file on disk. This chapter is about the second case: which directives
`leo-cub` recognizes, how they resolve to an on-disk path, which ones are
read-only versus writable, and how a node's real path is looked up again
later by `cub inspect FILE`, `cub sync FILE`, and the TUI's `o` key.

## The directive families

| Directive | Read/write | What it holds |
| --- | --- | --- |
| `@path <dir>` | n/a | Not a file itself: a directory segment every external-file descendant's path is resolved through. |
| `@auto <file>` | Read-only | A structural, Tree-sitter-parsed view of the file, regenerated on every load. |
| `@auto-md <file>` / `@auto-markdown <file>` | Read-only | Same, for Markdown, honoring Leo's `leo-noheader` marker. |
| `@auto-dir <dir-or-glob>` | Read-only | One `@auto` child per file matching a directory or glob, re-enumerated on every load. |
| `@file <file>` / `@thin <file>` / `@file-thin <file>` | Writable | Leo 5 thin sentinels: absolute depth, every node keeps its GNX. |
| `@f <file>` | Writable | leo-cub's own lighter cub-1-thin sentinels: depth relative to the preceding node, GNX omitted except for the root, clones, and UA-bearing nodes. |
| `@clean <file>` | Writable | Plain file content with no visible sentinels; structure is reconciled against a hidden private copy with the Mulder/Ream algorithm. |
| `@edit <file>` | Writable | A real Leo directive: the whole file is the node's flat body, no children allowed. |

`@edit` is structure-free: the file is read into the node's body on load and
written back on save, same as real Leo's `readOneAtEditNode`/
`writeOneAtEditNode`. Unlike real Leo, which silently deletes any children on
read, `leo-cub` refuses to load or save an `@edit` node that has children,
and the TUI won't let you nest one under it in the first place.

The read-only/writable split matters for the TUI: it permits structural and
headline edits inside `@file`/`@thin`/`@file-thin`/`@f` trees and writes
changed thin files on `Ctrl-S`, and reconciles `@clean` files the same way on
save, but `@auto`-family descendants stay read-only, because writing them
back would require a language-specific exporter, not just a parser. Press
`o` on one to edit its real source file directly instead. `@edit` has no
descendants to worry about, so `Ctrl-S` there is just an ordinary body save.

## How a path resolves

An external-file node's on-disk path is built by walking from the outline's
root down to that node, joining:

1. The `.leo` file's own directory.
2. Every ancestor `@path <dir>` directive along the way, outermost first —
   a `@path` line can appear in a node's headline or its body.
3. The node's own directive argument.

So this outline:

```
@path widget-refresh
  @auto requirements.md
  @auto-dir ./specs/**
```

resolves `requirements.md` to
`<outline-dir>/widget-refresh/requirements.md`, and expands the
`@auto-dir` node by walking
`<outline-dir>/widget-refresh/specs/**`.

Nest `@path` nodes to mirror deeper directory trees; each one only
contributes its own name; the accumulation is what supplies the rest.

## `@auto-dir`: read-only, recursive, and self-organizing

`@auto-dir <dir-or-glob>` is a directive you write directly into a headline,
not a mode of `cub import`: instead of creating one node per file once and
leaving it to go stale, it expands live, on every load, to one `@auto` child
per matching file. Add a file that matches and it appears next time the
outline is opened — no outline edit required.

- `@auto-dir src` lists the immediate files in `src`, non-recursively —
  matching `cub import`'s own one-level default.
- `@auto-dir src/*.rs` glob-filters that same one-level listing.
- `@auto-dir src/**/*.rs` opts into a recursive walk.

When a recursive walk matches files in more than one subdirectory, those
matches are grouped under synthetic `@path <name>` nodes mirroring that
subdirectory structure — the same shape `cub import --recursive --paths`
builds by hand — instead of being dumped as one flat list of siblings. Given

```
specs/
  01-resend-flow-ui-interaction/
    functional-spec.md
    tasks.md
```

`@auto-dir ./specs/**` expands to:

```
@auto-dir ./specs/**
  @path 01-resend-flow-ui-interaction
    @auto functional-spec.md
    @auto tasks.md
```

These synthetic `@path` nodes exist purely for readability; they are rebuilt
from scratch on every load and never written to the `.leo` file. One
consequence worth knowing: `@auto-dir`'s own resolved directory (`specs`
above) isn't itself represented as a `@path` node anywhere in the tree, so a
query can't be anchored above the `@auto-dir` boundary — see the caveat
below.

## Selecting a file (`cub inspect FILE`)

```sh
cub inspect outline.leo src/main.rs
cub inspect outline.leo main.rs
```

`cub inspect FILE` matches the given argument against every external-file
node's resolved path by exact match or path suffix, so a bare filename
matches any node ending in it — useful when you don't remember (or don't
want to type) the full directory. If more than one node matches, every
occurrence is returned.

Because matching walks the same ancestor-`@path` accumulation described
above, a query can reach as far up as a real `@path` ancestor, but not past
an `@auto-dir` node's own directory argument — that segment isn't backed by
a `@path` node, so matching re-anchors at the `@auto-dir` boundary instead of
guessing. For the tree above, `functional-spec.md` and
`01-resend-flow-ui-interaction/functional-spec.md` both match; a query that
also tries to include `specs/` does not.

## Jump-to-source in the TUI (`o`)

Pressing `o` on a derived node opens its real source file in `$VISUAL` or
`$EDITOR`, at the sentinel's line. For most external-file nodes this is
resolved the same way as file selection above: ancestor `@path`
accumulation plus the node's own directive argument. For a node produced by
`@auto-dir`, the exact path is looked up directly instead — it was already
recorded when the directory was expanded — so the same `@auto-dir`-boundary
gap that limits `cub inspect FILE` queries never affects opening the actual
file.

`@auto-dir`'s own node has no single file behind it (its argument is a
directory or glob, not a path), so `o` on it is a no-op, the same as any
other node with no recorded source.

## See also

- [Command reference](commands.md) for `cub import`'s `--recursive`,
  `--paths`, `--mode`, and `--parent` flags.
- [Concepts](concepts.md) for how GNXs and positions relate to clones.
