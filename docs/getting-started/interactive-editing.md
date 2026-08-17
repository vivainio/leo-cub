# Interactive editing

The commands in [First outline](first-outline.md) build a tree from the shell.
This chapter builds the same shape of tree from inside the TUI, so you can
see how insertion, renaming, indentation, reordering, copying, and cloning
feel as keystrokes rather than flags. Each stage below is checked with
`cub render`, so you can compare your outline against what it should look
like as you go.

Nothing here is destructive: nothing is written to disk until you press
`Ctrl-S`, and quitting with unsaved changes asks for confirmation first.

## Start a session

```sh
cub new project.leo --headline "Project"
cub project.leo
```

The outline opens with a single root, `Project`, selected. The explicit
`cub tui project.leo` command does the same thing.

## Keys used in this chapter

| Key | Action |
| --- | --- |
| `↑` / `↓` | Move the selection |
| `→` / `Enter`, `←` | Expand / collapse the selected node |
| `i` | Insert a new sibling after the selection |
| `h` | Rename the selected headline |
| `Ctrl-→` / `Ctrl-←` | Demote / promote (indent / outdent) |
| `Ctrl-↑` / `Ctrl-↓` | Reorder among siblings |
| `c` / `x` | Copy / cut the selected tree(s) |
| `v` / `Shift-V` | Paste a copy / paste a clone |
| `o` | Edit the body in `$EDITOR` |
| `Ctrl-P` | Find a headline |
| `Ctrl-S` | Save |
| `q` | Quit |

The full list is always available inside the TUI by pressing `?`.

## Insert and rename

`i` always inserts a new sibling immediately after the current selection, so
building a child means insert-then-demote rather than insert-as-child.

With `Project` selected:

1. Press `i`. A new node, `New Headline`, appears as a second root and opens
   for editing. Type `Source`, then `Enter`.
2. Press `Ctrl-→` to demote `Source` under `Project`.
3. With `Source` still selected, press `i`, type `Documentation`, `Enter`.
4. Press `i` again, type `Tasks`, `Enter`.

Renaming works the same way in reverse: select a node and press `h` instead
of `i`. Try it — select `Documentation`, press `h`, change it to `Docs`, then
`h` again to change it back.

Quit to the shell with `q` without saving, or open another terminal, and
check the shape so far:

```sh
cub render project.leo
```

```text
- Project
  - Source
  - Documentation
  - Tasks
```

## Add a grandchild

Back in `cub project.leo`, select `Source` and repeat the
insert-then-demote pattern one level deeper:

1. Press `i`, type `main.rs`, `Enter`. It lands as a sibling of `Source`,
   still under `Project`.
2. Press `Ctrl-→` to demote `main.rs` under `Source`.

```text
- Project
  - Source
    - main.rs
  - Documentation
  - Tasks
```

## Reorder siblings

Select `Tasks` and press `Ctrl-↑` twice. Each press swaps it with the sibling
above:

```text
- Project
  - Source
    - main.rs
  - Tasks
  - Documentation
```

Press `Ctrl-↓` twice to put `Tasks` back where it was — `Ctrl-↑`/`Ctrl-↓` only
reorder among siblings that share a parent; they never change the parent.

## Copy, cut, and paste

Select `Source` (with `main.rs` still nested under it) and press `c`. The
status line confirms the tree was copied. Select `Tasks`, then press `v`:

```text
- Project
  - Source
    - main.rs
  - Documentation
  - Tasks
  - Source
    - main.rs
```

The pasted `Source` is a second, independent tree — its nodes have fresh
identities, even though the headlines are identical and `render` has no way
to tell them apart. Select it and press `x` to cut it again, which restores
the tree from the previous step. `x` copies before removing, so the
clipboard still holds what you just cut.

## Paste a clone

A clone is different: it is the *same* node at a second position, not a copy
of it. Select `Tasks`, then press `Shift-V` to paste `Source` there as a
clone instead of a copy:

```text
- Project
  - Source
    - main.rs
  - Documentation
  - Tasks
    - Source ↪ clone
```

`render` marks the repeated occurrence with `↪ clone` and does not repeat its
descendants — but it is still the same node, so editing it edits every
occurrence. Select either `Source` — the one under `Project` or the one
under `Tasks` — and press `h`. Rename it to `Sources`, `Enter`.

```text
- Project
  - Sources
    - main.rs
  - Documentation
  - Tasks
    - Sources ↪ clone
```

Both occurrences changed together, because there was only ever one headline
to change. `v` (plain paste) deliberately avoids this by giving the pasted
copy fresh identities instead.

## Edit a body

Select `main.rs` and press `o`. This opens the node's body in `$EDITOR` as a
temporary file. Add a line such as:

```rust
fn main() {
    println!("hello from the outline");
}
```

Save and close the editor. Back in the TUI, the status line reports the body
changed, and a small dot marker appears next to `main.rs` in the outline to
mark that it now carries body text — the same marker `render` and `inspect`
use for any node with a non-empty body.

## Find a headline

Press `Ctrl-P`, then type `main`. The list narrows as you type; `↓`/`↑` cycle
between matches, and `Enter` selects the active match and closes the prompt.
Press `Esc` instead to cancel and return to whatever was selected before you
opened find.

## Save and verify

Press `Ctrl-S` to write the file, then `q` to quit. From the shell, confirm
the result:

```sh
cub inspect project.leo
cub validate project.leo
```

`inspect` lists `Project` with `Sources`, `Documentation`, and `Tasks` as
children, `main.rs` under `Sources` with the body you typed, and the cloned
`Sources` under `Tasks` shown as a back-reference (`=<gnx>`) rather than a
repeated subtree — that back-reference is `inspect`'s way of marking the same
clone `render` marked with `↪ clone`. `validate` should print an empty JSON
list.
