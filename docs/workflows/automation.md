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
