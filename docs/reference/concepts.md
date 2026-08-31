# Concepts

## Nodes and positions

A node has an identity, called a GNX, and content such as a headline and body.
A position is one occurrence of that node in the outline hierarchy. A cloned
node therefore has one GNX but can have several positions.

This is why commands that edit node content and commands that remove a
position are different operations. Editing a node can affect every clone;
removing a position removes only that occurrence.

## External files

Leo can represent source files through directives such as `@file`, `@thin`,
`@file-thin`, `@auto`, and `@clean`. `leo-cub` keeps the outline model and the
external source distinct, then reconstructs supported external content when it
is inspected or synchronized.

`@auto-dir <dir-or-glob>` expands a directory or glob pattern into one `@auto`
child per matching file, re-enumerated on every load. See
[Files and paths](files-and-paths.md) for the full directive reference,
including path resolution, `@auto-dir`'s pattern syntax, and how file
selection and jump-to-source look a node's real path back up.

The `import` command creates external-file nodes. The `sync` command reads
changes from supported external nodes into the outline. Use `inspect-derived`
for Leo thin-derived files.

## Safe changes

Operations are applied to a copy and committed only if the complete batch is
valid. A `set-headline` or `set-body` operation can include an `expected` value
as an optimistic conflict check. If the value changed since it was read, the
batch fails instead of silently overwriting somebody else's edit.

For scripts, prefer:

```sh
cub validate outline.leo
cub inspect outline.leo --format json
cub apply outline.leo operations.json --dry-run
```
