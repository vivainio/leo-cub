---
name: leo-cub
description: Inspect, validate, compare, edit, and synchronize Leo Editor (.leo) outlines with the `cub` CLI.
---

# leo-cub

Use `cub` to work with Leo Editor outlines without manipulating their XML
directly. Run `cub --help` or `cub <command> --help` for complete options.

## Inspect and validate

```bash
cub inspect outline.leo
cub validate outline.leo
cub inspect-derived path/to/derived.py --summary
cub diff before.leo after.leo
```

`inspect` emits the logical outline as JSON. `validate` exits unsuccessfully
when it finds structural errors. `inspect-derived` reconstructs an outline from
a Leo thin derived file.

## Synchronize external files

```bash
cub sync outline.leo --dry-run
cub sync outline.leo
cub sync outline.leo src/main.rs
cub sync outline.leo --gnx ekr.20260811210000.1
```

Use `--dry-run` first for important outlines. With no selector, `sync` updates
all external nodes; a filename or `--gnx` limits the operation.

## Apply edits

Run `cub apply --help` before constructing an operation file; its help text is
the source of truth for the JSON format and supported operations. Then use
`cub apply outline.leo operations.json --dry-run` to apply the batch without
writing it. Remove `--dry-run` only after checking the report. Prefer this
command over editing `.leo` XML because Leo clone identity and outline positions
are distinct concepts.

## Browse interactively

```bash
cub tui outline.leo
```

The TUI supports outline browsing and editing. Press `?` for its keybindings.
