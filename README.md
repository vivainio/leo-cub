# leo-cub

`leo-cub` is an experimental Rust library and command-line tool for reading,
validating, browsing, and modifying [Leo Editor](https://leo-editor.github.io/leo-editor/)
outlines.

The installed command is `cub`; the Rust library namespace is `leo`.

## Documentation

The project guide is built with [Zensical](https://zensical.org/) and published
to [GitHub Pages](https://vivainio.github.io/leo-cub/) from the `main` branch.
The source files live in [`docs/`](docs/), and the local preview can be started
with:

```sh
python -m pip install zensical
zensical serve
```

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
- Highlight node bodies with Syntect, using `@language`, `@rst` ancestors, or
  source extensions, including bundled reStructuredText syntax support.
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

### Termux

The PyPI release includes an Android API 24 ARM64 wheel suitable for current
64-bit Termux installations:

```sh
uv tool install leo-cub
```

### Installation from source

From the repository root, install the `cub` command with Cargo:

```sh
cargo install --path .
```

Install the bundled local agent skill after installing the command:

```sh
cub install-skills
```

This writes `~/.claude/skills/leo-cub/SKILL.md` and overwrites an existing
copy, so it is safe to rerun after upgrading.

## TUI

```sh
cub tui outline.leo
```

The browser resolves external thin files in memory. Outline headlines highlight
Leo directives, external-file names, and section-reference markers. A red `*`
marks each node changed since the outline was loaded or last saved; saving or
reloading clears the markers.

## TUI keybindings

### Browsing and display

| Key | Action |
| --- | --- |
| `↓` / `↑` | Select next/previous node |
| `Shift-↓` / `Shift-↑` | Extend or shrink a contiguous multi-node selection |
| `→`, `Enter` | Expand selected node |
| `←` | Collapse selected node |
| `Home` / `End` | Select the first/last visible node |
| `PageUp` / `PageDown` | Scroll the selected node's body by one page |
| `f` | Toggle a full-width body pane |
| `Shift-F` | Toggle a full-width outline pane |
| `↑` / `↓` in full-width mode | Scroll the body vertically by one line |
| `←` / `→` in full-width mode | Scroll the body horizontally |
| `Ctrl-P` | Find a headline incrementally; use `↑`/`↓` to cycle matches |
| `o` | Edit the node body in `$VISUAL`/`$EDITOR`; for derived nodes, open the real source at its sentinel |
| `y` | Toggle syntax highlighting |
| `?` | Show command help |

### Outline editing

| Key | Action |
| --- | --- |
| `Ctrl-I` or `Tab` | Insert a new sibling and enter headline editing |
| `Ctrl-H` or `Backspace` | Edit the selected headline |
| `c` | Copy the selected tree |
| `x` | Cut the selected tree |
| `v` | Paste an independent copy with fresh node identities |
| `Shift-V` | Paste as clones, retaining node identities |
| `Ctrl-↑`, `Ctrl-↓` | Move the selected node or multi-selection among siblings |
| `Ctrl-←`, `Ctrl-→` | Promote or demote the selected node or multi-selection |
| `Ctrl-R` | Reload from disk; press twice to discard unsaved changes |
| `Ctrl-S` | Save outline changes |
| `q` or `Esc` | Quit; press twice to discard unsaved changes |

### Headline editing

| Key | Action |
| --- | --- |
| Printable characters | Replace the initial selection, or insert at the cursor |
| `←` / `→`, `Home` / `End` | Keep the headline and position the cursor |
| `Backspace` / `Delete` | Delete the selection or a character |
| `Enter` | Accept the headline |
| `Esc` | Cancel editing; a newly inserted node is removed |

Use `--no-derived` to display only the hierarchy physically present in the
`.leo` XML file.

For source navigation, `cub` recognizes common position arguments for Vim,
Neovim, Nano, Emacs, VS Code, Microsoft Edit, Helix, and Kakoune. Other editors
receive the file path without a line argument.

## Demo flow

Starting in a project containing `README.md` and a `src/` directory, create a
new outline with destinations for source code, documentation, and tasks:

```sh
cub new project.leo --headline "Project"
cub add project.leo \
  "Project/Source" \
  "Project/Documentation" \
  "Project/Tasks/Backlog"
```

Import the source tree below `Project/Source`, preserving its directory
structure, then import the README as an editable node below the documentation
branch:

```sh
cub import project.leo src \
  --recursive --mode auto --paths \
  --parent "Project/Source"
cub import project.leo README.md \
  --mode edit \
  --parent "Project/Documentation"
```

Finally, inspect the resulting tree and validate the file:

```sh
cub inspect project.leo
cub validate project.leo
```

`@auto` source nodes are reconstructed from their files when inspected or
opened, while the `@edit` README node stores its text in the outline.

## Headless commands

```sh
cub new outline.leo
cub new notes.leo --headline "Notes"
cub add outline.leo "Project/Tasks/First task" "Project/Notes"
cub inspect outline.leo
cub inspect outline.leo src/main.rs
cub inspect outline.leo --gnx ekr.20260811210000.1
cub inspect outline.leo --position 0/2/1
cub inspect outline.leo --search 'render_(compact|json)'
cub inspect outline.leo --search TODO --search FIXME
cub inspect outline.leo src/main.rs --format json
cub validate outline.leo
cub import outline.leo src --recursive --mode auto --paths
cub import outline.leo README.md --mode edit --no-paths
cub import outline.leo README.md --parent "Project/Notes"
cub sync outline.leo
cub sync outline.leo src/main.rs --dry-run
cub sync outline.leo --gnx ekr.20260811210000.1
cub diff before.leo after.leo
cub inspect-derived path/to/derived.py --summary
cub apply outline.leo operations.json --dry-run
```

`new` creates a valid outline with one empty root node. It refuses to overwrite
an existing file.

`add` creates nodes from slash-separated headline paths and reuses shared or
existing prefixes. `import --parent` accepts either an exact GNX or a unique
slash-separated headline path. Paths with duplicate matching siblings are
rejected as ambiguous.

`import` creates Leo external-file nodes in `auto`, `edit`, or `clean` mode.
Markdown, Python, Rust, C#, Go, JavaScript/JSX, and TypeScript/TSX `@auto` files
are expanded transiently with Tree-sitter when they are loaded by `inspect` or
the TUI; the generated tree is not stored in the `.leo` file. Unsupported
source types remain available as a plain root node. Markdown also supports
Leo's `@auto-md` and `@auto-markdown` headlines and `leo-noheader` markers.
Directory imports are recursive only with `--recursive` and preserve their
layout with `@path` nodes by default. Use `--no-paths` to put all imported
files directly below the destination, `--parent GNX_OR_PATH` to choose that
destination, and `--dry-run` to validate without saving.

`inspect` uses a compact text format containing position paths, GNXs,
headlines, and bodies. Repeated clone content is shown as `=GNX`. Use
`--format json` for structured output in scripts.
`--search` accepts a Rust regular expression and searches headlines and body
lines. Search results include line-numbered excerpts with two surrounding lines
instead of printing entire matching bodies. Repeat `--search` to match any of
several expressions. Thin external files are scanned first and reconstructed
only when they may contain a search or GNX match.

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
it does not dynamically interpret every `@comment`/`@delims` change or fully
reconstruct all doc-part forms. Keep backups and use `--dry-run` when testing
write operations on important outlines.

The TUI permits structural and headline edits in `@file`, `@thin`, and
`@file-thin` trees and writes changed thin files on `Ctrl-S`. It validates and
stages generated files before replacing their external sources; unchanged
external files are not rewritten. Generated `@auto` descendants remain
read-only because writing them requires language-specific exporters. Use `o`
to edit a derived node's full external source directly. Unsaved changes require
a second `q` before they are discarded.

## License

MIT
