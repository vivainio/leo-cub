# Rhai API reference

This page lists the API available to [`cub` Rhai scripts](../workflows/scripting.md).
Use `Node` methods for ordinary outline work. The lower-level GNX methods are
useful when an operation specifically requires a node id.

## Opening and saving

| Signature | Description |
| --- | --- |
| `open(path: string) -> Doc` | Read a `.leo` file. In a TUI action, use the predefined `doc` instead. Also loads and merges `@auto`/`@file`/`@thin`/`@file-thin`/`@f`/`@clean` content, same as the TUI. |
| `doc.save()` | Write to the path from which the document was opened or bound, including any diverged `@file`/`@thin`/`@file-thin`/`@f`/`@clean` external file. |
| `doc.save_as(path: string)` | Write to another path and use it for later `save()` calls. |
| `doc.dir() -> string` | Directory containing the document, or `"."` when it has none. |
| `ARGS` | Array of strings: the command-line arguments after the script path (`cub run script.rhai a b` -> `["a", "b"]`). `[]` when none were given. |

## Nodes

| Signature | Description |
| --- | --- |
| `doc.node(gnx: string) -> Node` | Wrap an existing GNX as a node. |
| `doc.node_at(position: string) -> Node` | Get the exact occurrence at an index path such as `"0/2/1"`. |
| `doc.ensure(path: string) -> Node` | Find or create a slash-separated headline path. |
| `doc.find_h(pattern: string) -> array` | Find nodes whose headlines match a regular expression. |
| `doc.find_b(pattern: string) -> array` | Find nodes whose bodies match a regular expression. |
| `node.h: string` | Read or change the headline. |
| `node.b: string` | Read or change the body. |
| `node.gnx: string` | Read the node's GNX. |
| `node.position: string` | Read the exact index path, or `""` for a GNX-only handle. |
| `node.parent() -> Node` | Get the parent; a root's parent wraps `""`. |
| `node.children() -> array` | Get direct children as positioned nodes. |
| `node.subtree() -> array` | Get this node and its descendants, depth-first. |
| `node.path() -> string` | Get the slash-separated headline path. |
| `node.file_path() -> string` | Resolve this node's external-file path, or return `""`. |
| `node.remove()` | Remove this occurrence and its subtree; see clone behavior below. |

Search patterns use the same regular-expression syntax as `cub inspect
--search` and fail when the expression is invalid.

## Nodes, clones, and positions

A GNX identifies shared node content. A position identifies one occurrence of
that node in the outline.

`doc.node(gnx)`, `find_h()`, and `find_b()` produce GNX-based handles. When a
node has clones, structural operations on such a handle use its first
occurrence. Nodes returned by `node_at()`, `parent()`, `children()`, and
`subtree()` retain the exact occurrence in `node.position`.

This distinction is most important for removal:

```rust
let exact = doc.node_at("0/1");
exact.remove();                 // removes occurrence 0/1

let by_id = doc.node(exact.gnx);
by_id.remove();                 // removes the first remaining occurrence
```

Prefer a positioned `Node` when traversing or destructively editing cloned
content. Headline and body edits affect every occurrence because clones share
the same content.

## Document operations

| Signature | Description |
| --- | --- |
| `doc.gnx(path: string) -> string` | Resolve an existing, unambiguous headline path. |
| `doc.roots() -> array` | Get root GNXs in outline order. |
| `doc.all() -> array` | Get every GNX in depth-first position order; clones appear more than once. |
| `doc.clone_node(gnx, parent_gnx) -> string` | Append another occurrence beneath a parent. |
| `doc.clone_node(gnx, parent_gnx, index) -> string` | Insert another occurrence at an index. |
| `doc.apply(json: string) -> string` | Apply a transactional operation batch and return its JSON report. |
| `doc.render() -> string` | Render the outline as compact Markdown. |
| `doc.count() -> int` | Return the number of nodes. |
| `doc.validate() -> array` | Return structural validation errors. |

`doc.apply()` accepts the same operation batches as `cub apply`. See
[Automation and AI tools](../workflows/automation.md) for their JSON formats.

### GNX equivalents

These methods expose the operations underlying `Node`, using GNX strings
directly:

| Signature | Description |
| --- | --- |
| `doc.children(gnx) -> array` | Get direct child GNXs. |
| `doc.subtree(gnx) -> array` | Get this GNX and its descendant GNXs. |
| `doc.parent(gnx) -> string` | Get the first occurrence's parent GNX, or `""`. |
| `doc.path(gnx) -> string` | Get the first occurrence's headline path. |
| `doc.file_path(gnx) -> string` | Resolve the node's external-file path. |
| `doc.headline(gnx) -> string` | Read a headline. |
| `doc.set_headline(gnx, text)` | Change a headline. |
| `doc.body(gnx) -> string` | Read a body. |
| `doc.set_body(gnx, text)` | Change a body. |
| `doc.remove(gnx)` | Remove the GNX's first occurrence and its subtree. |

## Assertions and output

| Signature | Description |
| --- | --- |
| `assert(condition)` | Stop when the condition is false. |
| `assert(condition, message)` | Stop with a custom message. |
| `assert_eq(actual, expected)` | Stop when supported values differ. |
| `print(...)` | Write to standard output or the TUI action output. |
| `debug(...)` | Write diagnostic output with its source position. |

## Subprocesses and files

### Subprocesses

| Signature | Description |
| --- | --- |
| `sh(command) -> map` | Run through `sh -c` relative to `cub`'s working directory. |
| `sh(command, #{ cwd: path }) -> map` | Run in an explicit directory, e.g. `doc.dir()`. |
| `env_var(name: string) -> string` | Read an environment variable, `""` when unset. |

Each `sh` form returns `#{ stdout, stderr, code }`. A command's non-zero
status is returned in `code` rather than thrown. A process terminated by a
signal uses `-1`.

### Paths and directories

| Signature | Description |
| --- | --- |
| `path(value: string) -> Path` | Wrap a path for path properties and operators. |
| `cwd() -> Path` | Get `cub`'s current working directory. |
| `p.exists`, `p.is_dir`, `p.is_file` | Inspect a path. |
| `p.is_absolute`, `p.is_relative`, `p.is_symlink` | Inspect a path. |
| `p.canonicalize() -> Path` | Resolve an absolute canonical path. |
| `create_dir(path)` | Create a directory and missing parents. |
| `remove_dir(path)` | Remove an empty directory. |
| `open_dir(path) -> array` | List entries as `Path` values. |
| `remove_file(path)` | Delete a file. |

### Files

| Signature | Description |
| --- | --- |
| `open_file(path) -> File` | Open for reading and writing, creating if needed. |
| `open_file(path, mode) -> File` | Open with mode `r`, `r+`, `w`, `wx`, `w+`, `a`, `ax`, `a+`, or `ax+`. |
| `file.read_string() -> string` | Read remaining UTF-8 text. |
| `file.read_string(length) -> string` | Read up to a number of bytes. |
| `file.write(text)` | Write at the current position. |
| `file.seek(position) -> int` | Move the file cursor. |
| `file.position() -> int` | Read the file cursor. |
| `file.bytes() -> int` | Get the file length. |
| `file.read_blob() -> blob` | Read binary data. |
| `blob.write_to_file(file)` | Write binary data. |

Paths passed as strings are relative to `cub`'s current working directory.
Use `doc.dir() + "/name"` for a path beside the open outline.

## TUI script contexts

An `@action` body receives:

| Name | Type |
| --- | --- |
| `doc` | The open `Doc`. |
| `p` | A positioned `Node` for the selection. |
| `target` | The selection's GNX string. |

An `@import` command must have the signature `fn name(doc, target)`, where
`target` is a positioned `Node`. Only functions named in the top-level
`COMMANDS` array appear in the action palette.

## Gotchas

Rhai's string case/whitespace/substitution methods -- `trim()`,
`trim_start()`, `trim_end()`, `to_upper()`, `to_lower()`, `replace()`, and
similar -- mutate their target in place and return nothing. Chaining one onto
a temporary throws the result away:

```rust
print(result.stdout.trim());   // prints nothing; the trimmed copy is discarded
```

Call it as a statement on a variable instead, then use that variable:

```rust
let hash = result.stdout;
hash.trim();
print(hash);                   // prints the trimmed value
```

Rhai caps expression nesting depth to guard against pathological input --
64 levels at a script's top level, 32 (raised here to 64, matching the
top-level limit) inside a `fn`. A long `+`-chained string build or a few
levels of `if`/`for` around a compound condition can reach that inside one
function faster than it looks; if a script fails to parse with "Expression
exceeds maximum complexity", split the expression across a few `let`/`+=`
statements or an extra helper function rather than one large one.
