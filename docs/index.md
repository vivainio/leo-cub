---
icon: lucide/boxes
hide:
  - toc
---

# leo-cub

`leo-cub` is an automation-safe Rust tool for reading, validating, browsing,
and modifying [Leo Editor](https://leo-editor.github.io/leo-editor/) outlines.

It treats a Leo outline as both a tree and a graph: a node's GNX identifies
shared content, while a position identifies one occurrence in the outline.
That distinction makes clones visible and makes scripted changes predictable.

<div class="guide-grid" markdown>

[:lucide-download: **Install it**  
Get the `cub` command from PyPI or build it with Cargo.](getting-started/installation.md)

[:lucide-rocket: **Try the first outline**  
Create an outline, add a small tree, and inspect the result.](getting-started/first-outline.md)

[:lucide-book-open: **Understand the model**  
Learn how GNXs, positions, clones, and external files fit together.](reference/concepts.md)

[:lucide-bot: **Automate safely**  
Use compact inspection and transactional JSON operations from scripts or agents.](workflows/automation.md)

</div>

## What it is good at

- Inspecting `.leo` files without opening a graphical editor.
- Validating outline structure in CI or pre-commit checks.
- Importing source trees as `@auto`, `@edit`, or `@clean` nodes.
- Applying a complete batch of edits atomically, with optional preconditions.
- Browsing outlines in a small terminal UI.
- Synchronizing supported external and thin files.

## Status

This is an early project. The format support is deliberately conservative, so
keep backups of important outlines and use `--dry-run` when a command offers it.
The [repository](https://github.com/vivainio/leo-cub) is the source of truth for
implementation details and current limitations.
