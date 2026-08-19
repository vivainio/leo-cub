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

`render` emits a nested Markdown list containing headlines only. Repeated vnode
occurrences are marked with `↪ clone`; clone descendants are not repeated. Use
`--current POSITION` to highlight an occurrence and its ancestors. Use
`--collapsed` to render branches as native HTML `<details>` elements, and
repeat `--expand POSITION` to open additional branches.
