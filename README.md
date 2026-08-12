# leo-cub

`leo-cub` is an experimental Rust library and command-line tool for reading,
validating, browsing, and modifying [Leo Editor](https://leo-editor.github.io/leo-editor/)
outlines.

The installed command is `cub`; the Rust library namespace is `leo`.

## Screenshot

<img width="1836" height="933" alt="image" src="https://github.com/user-attachments/assets/2ff24163-62a2-487b-aaf5-a9ec8c2beb73" />

## Why

Leo outlines are not ordinary XML trees. A GNX identifies shared vnode content,
while an outline position identifies one occurrence of that vnode. Cloned nodes
can therefore appear in several places. `leo-cub` keeps those concepts separate
and exposes transactional operations intended for scripts and AI tools.

## Current features

- Parse and validate `.leo` XML outlines.
- Preserve XML outside the rewritten `<vnodes>` and `<tnodes>` sections.
- Represent clone identity separately from outline positions.
- Apply atomic JSON operation batches with optional text preconditions.
- Parse Leo 5 thin derived-file sentinels.
- Reconstruct `@file`, `@thin`, and `@file-thin` hierarchies and bodies.
- Resolve ancestor `@path` directives in the TUI.
- Browse outlines with a small Ratatui interface.
- Highlight node bodies with Syntect, using `@language` or source extensions.
- Open a derived node's full source file at its sentinel line using `$VISUAL` or
  `$EDITOR`.

## Install

The recommended installation method is [`uv`](https://docs.astral.sh/uv/):

```sh
uv tool install leo-cub
```

This installs the `cub` command. You can also use `pip install leo-cub`,
or download the appropriate archive
from the [latest GitHub release](https://github.com/vivainio/leo-cub/releases/latest).

### Installation from source

From the repository root, install the `cub` command with Cargo:

```sh
cargo install --path .
```

## TUI

```sh
cub tui outline.leo
```

The browser resolves external thin files in memory.

## TUI keybindings

### Browsing and display

| Key | Action |
| --- | --- |
| `j`, `↓` / `k`, `↑` | Select next/previous node |
| `l`, `→`, `Enter` | Expand selected node |
| `h`, `←` | Collapse selected node |
| `Home` / `End` | Select the first/last visible node |
| `Ctrl-P` | Find a headline incrementally; use `↑`/`↓` to cycle matches |
| `o` | Open the full external source file at the node sentinel |
| `y` | Toggle syntax highlighting |

### Outline editing

| Key | Action |
| --- | --- |
| `Ctrl-I` or `Tab` | Insert a new sibling and enter headline editing |
| `Ctrl-H` or `Backspace` | Edit the selected headline |
| `Ctrl-↑`, `Ctrl-↓` | Move among siblings |
| `Ctrl-←`, `Ctrl-→` | Promote or demote the selected node |
| `Ctrl-S` | Save outline changes |
| `q` or `Esc` | Quit; press twice to discard unsaved changes |

### Headline editing

| Key | Action |
| --- | --- |
| Printable characters | Append to the headline |
| `Backspace` | Delete the previous character |
| `Enter` | Accept the headline |
| `Esc` | Cancel editing; a newly inserted node is removed |

Use `--no-derived` to display only the hierarchy physically present in the
`.leo` XML file.

For source navigation, `cub` recognizes common position arguments for Vim,
Neovim, Nano, Emacs, VS Code, Helix, and Kakoune. Other editors receive the file
path without a line argument.

## Headless commands

```sh
cub inspect outline.leo
cub validate outline.leo
cub diff before.leo after.leo
cub inspect-derived path/to/derived.py --summary
cub refresh-derived outline.leo 0 path/to/derived.py --dry-run
cub apply outline.leo operations.json --dry-run
```

An operation batch is a JSON object:

```json
{
  "operations": [
    {
      "op": "set-body",
      "node": "ekr.20260811210000.1",
      "expected": "old body",
      "body": "new body"
    }
  ]
}
```

Operations are applied to a copy and committed only if the complete batch is
valid. `expected` provides optimistic conflict detection for headline and body
edits.

## Status and safety

This project is early and the file format support is incomplete. In particular,
it does not yet write thin derived files, dynamically interpret every
`@comment`/`@delims` change, or fully reconstruct all doc-part forms. Keep
backups and use `--dry-run` when testing write operations on important outlines.

The TUI overlays derived files without modifying either the outline or external
source files. Derived descendants are read-only in the outline editor; use `o`
to edit their full external source. Unsaved outline changes require a second
`q` before they are discarded.

## License

MIT
