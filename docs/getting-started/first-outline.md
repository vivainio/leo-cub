# First outline

Create a new outline without overwriting an existing file:

```sh
cub new project.leo --headline "Project"
```

There's no TUI action for this first step — it needs a file to open before
the TUI has anything to browse. Everything after it happens inside the TUI:

```sh
cub project.leo
```

## Build a small hierarchy

With `Project` selected, insert a sibling and demote it into place —
`i` always inserts a new sibling right after the current selection, so
building a child means insert-then-demote:

1. Press `i`. A new node opens for editing. Type `Source`, `Enter`.
2. Press `Ctrl-→` to demote `Source` under `Project`.
3. With `Source` still selected, press `i`, type `Documentation`, `Enter`.
4. Press `i` again, type `Tasks`, `Enter`.
5. Select `Tasks`, press `i`, type `First task`, `Enter`, then `Ctrl-→` to
   demote it under `Tasks`.

Press `Ctrl-S` to save. The result:

```text
- Project
  - Source
  - Documentation
  - Tasks
    - First task
```

This is the same shape [Interactive editing](interactive-editing.md) builds
in more depth, with renaming, reordering, copy/cut/paste, and clones — worth
reading next if this is your first time in the TUI.

From the shell, confirm the saved file matches, and check it's valid:

```sh
cub inspect project.leo
cub validate project.leo
```

`inspect` prints a compact, human-readable representation. `validate` prints
an empty JSON list on success and exits non-zero when the outline is invalid
— both are read-only checks, useful in CI as well as here.

## Import source files

Rename `Source` to a real `@path` node — select it, press `h`, change it to
`@path src` (assuming a `src/` directory sits next to `project.leo`), then
`Enter`.

With `@path src` still selected, press `a` to open the command palette, type
`import` to filter down to **Import new files into @path**, and press
`Enter`. It scans `src/` and adds one read-only `@auto <name>` node per file
not already represented — no script involved, this is a built-in TUI
command. A subdirectory with no matching `@path` child of its own gets one
created automatically, so selecting that new node and running the same
command again descends one level further, instead of requiring every
`@path` node along the way to exist first.

The TUI import command always creates `@auto` nodes. From the shell, `cub
import` covers the cases it doesn't: `--mode edit` for text that should live
in the outline instead of referencing the file, `--mode clean` for a synced
file with no visible sentinels, `--recursive --paths` to import a whole
directory tree with its layout preserved in one shot, and `--dry-run` to
check first. See [Files and paths](../reference/files-and-paths.md) for how
these modes differ.
