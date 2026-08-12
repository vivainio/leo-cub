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
cub inspect outline.leo src/main.rs
cub inspect outline.leo --gnx ekr.20260811210000.1
cub inspect outline.leo --position 0/2/1
cub inspect outline.leo --search 'render_(compact|json)'
cub inspect outline.leo --search TODO --search FIXME
cub validate outline.leo
cub inspect-derived path/to/derived.py --summary
cub diff before.leo after.leo
```

Use `inspect` directly when reading an outline. Give it an external filename,
GNX, or position path to emit only matching subtrees; bodies are included and
GNX lookup returns every clone occurrence. Use `--format json` only when a
script needs structured output. Use repeatable `--search REGEX` options to find
headline or body matches (using OR semantics) as short, line-numbered excerpts.
Search and GNX lookup include lazily reconstructed thin external files.
`validate` exits unsuccessfully
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
