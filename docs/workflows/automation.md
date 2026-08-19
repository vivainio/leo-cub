# Automation and AI tools

`leo-cub` is designed to give scripts and coding agents a small, explicit
interface to an outline. A useful automation loop is:

1. Inspect the relevant subtree as JSON.
2. Construct a small operation batch.
3. Apply it with `--dry-run`.
4. Apply the same batch for real.
5. Validate the resulting outline.

```sh
cub inspect project.leo --format json > before.json
cub apply project.leo operations.json --dry-run
cub apply project.leo operations.json
cub validate project.leo
```

An operation batch is a JSON object with an `operations` array. For example:

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

The GNX in a real operation must come from the outline being edited. The
`expected` field provides conflict detection; omit it only when overwriting
the current value is intentional.

## Adding a whole tree at once

Building structure — rather than editing existing nodes — with one `insert`
per node means threading parent GNXs through the whole batch by hand. The
`insert-tree` operation instead takes a nested dict, so a script can just
build the structure it wants and hand it over in one shot:

```json
{
  "gnx-prefix": "acme",
  "operations": [
    {
      "op": "insert-tree",
      "parent": "ekr.1",
      "tree": {
        "Milestones": {
          "_body": "",
          "Kickoff": { "_body": "Draft agenda." },
          "Beta": { "_body": "Target date TBD." }
        }
      }
    }
  ]
}
```

`_gnx` is optional per node; omitted ones get a fresh id built from the
batch's `gnx-prefix` (default `"cub"`) — pass your own prefix to keep
scripted nodes easy to spot later. `_body` defaults to `""`. Since the tree
is a JSON object, siblings come out sorted by headline rather than in
writing order.

## Targeting a parent by headline that might not exist yet

A recurring import script — pulling in GitHub PRs or issues, say — usually
knows a stable destination like `"Imports/PRs"` but not its GNX, and the
destination may not exist on the first run. `insert-tree` and `merge-tree`
accept `"parent-headline"` in place of `"parent"` for exactly this: it
resolves the path the same way `cub add` does, reusing any prefix that
already exists, and creates whatever segments are missing instead of
failing:

```json
{
  "operations": [
    {
      "op": "insert-tree",
      "parent-headline": "Imports/PRs",
      "tree": {
        "PR #142: Fix flaky retry": { "_body": "https://github.com/.../142" }
      }
    }
  ]
}
```

Running that batch again with a different PR under the same
`"parent-headline"` reuses the existing `Imports/PRs` nodes rather than
creating duplicates. Give at most one of `"parent"`/`"parent-headline"`;
omitting both targets the outline root.

## Regenerating a section by its headline

A script that regenerates content — a changelog, a generated report section —
usually knows the section's headline but not its GNX, and doesn't care about
keeping the old GNX around. `replace-tree` removes the node at a headline
path (or GNX) along with its whole subtree, then inserts a fresh
`insert-tree`-shaped tree in the same spot:

```json
{
  "operations": [
    {
      "op": "replace-tree",
      "headline": "Docs/Changelog",
      "tree": {
        "Changelog": {
          "_body": "Regenerated from the latest release notes.",
          "0.4.0": { "_body": "..." }
        }
      }
    }
  ]
}
```

The headline path is resolved the same way as `cub add`'s paths, and fails
the batch if it's ambiguous or missing. The replaced node's GNX is not
reused; the new tree gets fresh ids the same way `insert-tree` does.

## Merging into a section without discarding it

`replace-tree` is destructive: everything under the target headline is
gone before the new tree goes in. When a script instead wants to update or
extend an existing section — bump a body, add a new child — without
touching siblings it doesn't know about, `merge-tree` matches `tree`'s
entries against `parent`'s existing children by headline:

```json
{
  "operations": [
    {
      "op": "merge-tree",
      "parent": "ekr.1",
      "tree": {
        "Milestones": {
          "Kickoff": { "_body": "Draft agenda — updated." },
          "Launch": { "_body": "New milestone." }
        }
      }
    }
  ]
}
```

A matching headline gets its body updated only if `_body` is given (leaving
it out preserves the existing body) and its children merged the same way,
recursively; a headline with no match is inserted fresh. `merge-tree` never
deletes a node — an entry not mentioned in `tree` is left exactly as-is.

## Search before loading

For large outlines, search headlines and body text directly:

```sh
cub inspect project.leo --search 'render_(compact|json)'
```

Search output includes excerpts rather than dumping every matching body. This
keeps agent context and command output manageable.

## Embed an outline in Zensical

The outline renderer emits ordinary Markdown, so Zensical can render it during
the documentation build with the Markdown Exec extension:

```toml
[project.plugins.markdown-exec]
```

Then include the selected outline in a page:

````markdown
```bash exec="on"
cub render project.leo --position 0/2
```
````

Only headlines are emitted. Repeated vnode occurrences are marked with
`↪ clone`, while their descendants are not repeated.

## Practical guardrails

- Keep the original `.leo` file or use version control before scripted edits.
- Prefer one focused batch over a long sequence of independent mutations.
- Use `--dry-run` for imports, syncs, and operation batches when available.
- Run `cub validate` after a write.
