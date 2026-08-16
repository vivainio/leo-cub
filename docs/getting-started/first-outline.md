# First outline

Create a new outline without overwriting an existing file:

```sh
cub new project.leo --headline "Project"
```

Add a small hierarchy using slash-separated headline paths:

```sh
cub add project.leo \
  "Project/Source" \
  "Project/Documentation" \
  "Project/Tasks/First task"
```

Inspect and validate the result:

```sh
cub inspect project.leo
cub validate project.leo
```

`inspect` prints a compact, human-readable representation. `validate` prints
an empty JSON list on success and exits non-zero when the outline is invalid.

## Import source files

Import a directory recursively while preserving its directory layout:

```sh
cub import project.leo src \
  --recursive --mode auto --paths \
  --parent "Project/Source"
```

Use `--dry-run` to check an import before writing the outline. Use
`--mode edit` when the text should live in the outline rather than reference
an external file.

## Browse interactively

```sh
cub tui project.leo
```

Use the arrow keys to navigate, `Enter` to expand a node, `o` to open source,
and `?` to see the complete keybinding list. Press `q` to quit.
