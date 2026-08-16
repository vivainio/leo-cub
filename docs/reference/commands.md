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
| `cub tui FILE` | Browse an outline interactively. |
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

`render` emits a nested Markdown list containing headlines only. Repeated vnode
occurrences are marked with `↪ clone`; clone descendants are not repeated. Use
`--current POSITION` to highlight an occurrence and its ancestors. Use
`--collapsed` to render branches as native HTML `<details>` elements, and
repeat `--expand POSITION` to open additional branches.
