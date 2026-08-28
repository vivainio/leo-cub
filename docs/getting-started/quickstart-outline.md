# Quickstart outline

Leo Editor ships a `quickstart.leo` that you open and read through node by
node, with a few nodes you're meant to act on rather than just read.
[`docs/quickstart.leo`](https://github.com/vivainio/leo-cub/blob/main/docs/quickstart.leo)
is the same idea, adapted to what `cub` itself supports - no Qt-specific
plugin, theme, or minibuffer content, since none of that exists here.

Open it directly:

```sh
cub docs/quickstart.leo
```

## What's inside

```text
- Welcome
- Move around
  - First stop
  - Second stop
  - Third stop
- Edit the tree
  - Scratch
- Find and search
  - A quiet node
- External files: @clean, @auto, @auto-dir, @file
  - @clean quickstart-files/greeting.txt
- Run an action
  - @action Say hello
  - @action Mark this node
- Automate from outside
- Where to go next
```

Each top-level node is a short, self-contained lesson - read its body for
instructions, then act on it right there. Nothing is written to disk unless
you press `Ctrl-S`, so there's nothing to undo by skipping around.

## Try an action live

The two nodes under **Run an action** are `@action` nodes: any node
headlined `@action <name>` is runnable from inside the TUI. Press `Shift-A` to
open the action palette, type to filter by name, and `Enter` runs the
selected node's body as a script. The body pane switches to the command's
output until you select a different node.

Press `Shift-A`, type `hello`, `Enter` - you'll see:

```text
Hello from an action!
This node's body just ran as a rhai script.
```

The second action, **Mark this node**, uses the two names every `@action`
body gets predefined: `doc`, bound to this outline, and `target`, the gnx
of the node you had selected when you invoked the action (here, the action
node itself). It appends a checkmark to that node's headline and prints
the result - a script can read and mutate the live outline directly,
without any serialization round trip.

## Pull in a real external file

**External files** has a real `@clean` node under it, mirroring
[`quickstart-files/greeting.txt`](https://github.com/vivainio/leo-cub/blob/main/docs/quickstart-files/greeting.txt)
next to the outline. From another terminal, change that file (any editor
works, or `echo "new text" > docs/quickstart-files/greeting.txt`), then
run:

```sh
cub sync docs/quickstart.leo
```

Back in the TUI, press `Ctrl-R` to reload - no need to quit, since reload
re-reads the outline from disk exactly like reopening it would. The node's
body now matches the file's new content. Sync is one-directional: it pulls
file changes into the outline, and never pushes an in-TUI body edit back
out to the file - editing the node's body directly, then syncing, leaves
the file untouched.

## How this differs from the other docs

- [First outline](first-outline.md) and
  [Interactive editing](interactive-editing.md) build a tree from nothing,
  from the shell and from the TUI respectively - use those to learn the
  editing commands in depth.
- [Tutorial outline](tutorial.md) uses a small fixture outline,
  `tutorial.leo`, to demonstrate `cub render`'s output - it's a fixture for
  *this documentation*, not something you're meant to open yourself.
- `docs/quickstart.leo` is the one file in this set meant to be opened and
  acted on directly; it links out to the pages above from its own
  **Where to go next** node instead of repeating them.
