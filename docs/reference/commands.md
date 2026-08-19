# Command reference

| Command | Purpose |
| --- | --- |
| `cub new FILE` | Create a new outline. |
| `cub add FILE PATH...` | Add slash-separated headline paths. |
| `cub inspect FILE` | Print an outline or selected subtree. |
| `cub render FILE` | Render a selected outline hierarchy as Markdown. |
| `cub validate FILE` | Validate the outline and exit non-zero on errors. |
| `cub import FILE INPUT...` | Import files or directories as external or editable nodes. |
| `cub sync FILE` | Synchronize supported external nodes. |
| `cub apply FILE OPS.json` | Apply a transactional JSON operation batch. |
| `cub diff BEFORE AFTER` | Compare two outline files. |
| `cub inspect-derived FILE` | Inspect a thin derived file. |
| `cub FILE` | Browse an outline interactively (shorthand for `cub tui FILE`). |
| `cub tui FILE` | Browse an outline interactively using the explicit subcommand. |
| `cub install-skills` | Install the bundled local agent skill. |

Most commands accept `--help` for their complete options:

```sh
cub inspect --help
cub import --help
cub apply --help
```

For a machine-readable inspection, select JSON output:

```sh
cub inspect project.leo --format json
cub inspect project.leo --search 'TODO|FIXME' --format json
cub render project.leo --position 0/2
```

`--format json-tree` reshapes the output so nodes are addressable by
headline path instead of by GNX or position index — handy from tools like
[nu](https://www.nushell.sh) that chain `get`:

```sh
cub inspect project.leo --format json-tree | nu -c 'from json | get "Run an action" | get "Say hello"'
```

Each node is a record with reserved `_gnx`/`_body` keys plus one key per
child headline. Because every `get` step must return exactly one node,
`json-tree` fails outright (no output at all) if any two siblings share a
headline, or if a headline collides with `_gnx`/`_body` — it does not
silently fall back to an array or an error placeholder. It is also
incompatible with `--search`, which returns match excerpts rather than a
tree. Point `--position`/`--gnx` at a narrower subtree to work around a
collision elsewhere in a large outline.

`cub apply FILE OPS.json` also accepts `-` in place of `OPS.json` to read the
operation batch from stdin, so a script that computes edits can pipe them
straight in without an intermediate file:

```sh
cub inspect project.leo --format json-tree \
  | nu -c 'from json | ...compute an {operations: [...]} batch...' \
  | cub apply project.leo -
```

For scripts that build structure rather than edit existing nodes, the
`insert-tree` operation adds a whole subtree — or several sibling
subtrees — in one call, instead of one `insert` per node with manual
parent-id chaining. It takes the same shape `--format json-tree` prints:
a map from headline to a node with reserved `_gnx`/`_body` keys plus one
key per child headline. Both are optional — a node without `_body`
gets `""`, and a node without `_gnx` gets a fresh id generated from the
batch's top-level `gnx-prefix` (default `"cub"`), so quick scripts don't
need to invent ids themselves:

```json
{
  "gnx-prefix": "acme",
  "operations": [
    {
      "op": "insert-tree",
      "parent": "ekr.1",
      "tree": {
        "Project Plan": {
          "_body": "Top-level notes for the plan.",
          "Milestones": {
            "Kickoff": { "_body": "Draft agenda and invite list." },
            "Beta release": { "_gnx": "ekr.42", "_body": "Target date TBD." }
          }
        }
      }
    }
  ]
}
```

Because the tree is a JSON object, sibling nodes come out in headline
order (`Beta release` before `Kickoff` above), not the order they were
written in — the same ordering `json-tree` output has. Give siblings
headlines that already sort the way you want, or add one `insert`/index
move afterward, if exact order matters.

`insert-tree` and `merge-tree` also accept `"parent-headline"` instead of
`"parent"`: a slash-separated headline path, resolved the same way as
`cub add`'s paths — but unlike `replace-tree`'s `"headline"`, a missing
path is *created* rather than treated as an error, reusing whatever
prefix already exists. This saves a `cub add` (or a GNX lookup) before a
script's first `apply` when the destination section may not exist yet:

```json
{"op": "insert-tree", "parent-headline": "Imports/PRs", "tree": {"...": {}}}
```

Give at most one of `"parent"`/`"parent-headline"`; omitting both means
the outline root.

Headline paths — `"parent-headline"` here, `replace-tree`'s `"headline"`,
and `cub add`'s arguments — treat `/` as a separator, so a headline that
contains one itself (a branch-name-style PR title, say) needs escaping:
write `\/` for a literal slash and `\\` for a literal backslash within one
path component; any other backslash is kept as-is.

To regenerate a section wholesale rather than append to it, `replace-tree`
removes an existing node's defining occurrence and its whole subtree, then
inserts a fresh `insert-tree`-shaped `tree` at that same parent/index. Point
it at either `"node"` (a GNX) or `"headline"` (a slash-separated path,
resolved the same way as `cub add`'s paths) — a script that only knows a
stable headline like `"Docs/Changelog"` doesn't need to look up its GNX
first:

```json
{
  "operations": [
    {
      "op": "replace-tree",
      "headline": "Docs/Changelog",
      "tree": {
        "Changelog": {
          "_body": "Regenerated from the latest release notes.",
          "0.4.0": { "_body": "..." }
        }
      }
    }
  ]
}
```

The removed node's GNX is discarded, not reused — the new tree's nodes get
fresh ids from `gnx-prefix` just like `insert-tree`, unless they set their
own `_gnx`. If the target is a cloned vnode, only its defining occurrence is
replaced; other occurrences keep referencing the original node.

When you want to update or extend a subtree without discarding anything it
already contains, `merge-tree` merges `tree` into `parent`'s children
(again, one `insert-tree`-shaped tree), matching entries to existing
children by headline instead of wiping them out:

```json
{
  "operations": [
    {
      "op": "merge-tree",
      "parent": "ekr.1",
      "tree": {
        "Milestones": {
          "Kickoff": { "_body": "Draft agenda — updated." },
          "Launch": { "_body": "New milestone." }
        }
      }
    }
  ]
}
```

A headline that already exists as a child gets its body updated (only if
`_body` is given — omit it to leave the existing body alone) and its own
children merged the same way, recursively. A headline with no existing
match is inserted fresh, exactly like `insert-tree`. `merge-tree` never
removes a node, and fails the batch if a headline matches more than one
sibling.

`render` emits a nested Markdown list containing headlines only. Repeated vnode
occurrences are marked with `↪ clone`; clone descendants are not repeated. Use
`--current POSITION` to highlight an occurrence and its ancestors. Use
`--collapsed` to render branches as native HTML `<details>` elements, and
repeat `--expand POSITION` to open additional branches.
