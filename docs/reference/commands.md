# Command reference

| Command | Purpose |
| --- | --- |
| `cub new FILE` | Create a new outline. |
| `cub add FILE PATH...` | Add slash-separated headline paths. |
| `cub inspect FILE` | Print an outline or selected subtree. |
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
```
