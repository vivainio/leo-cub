# leo-rs

An automation-safe Rust library and headless CLI for inspecting and modifying
Leo Editor `.leo` outlines. The model distinguishes shared node identity (GNX)
from outline positions, so clones remain explicit.

```sh
cargo run -- inspect outline.leo
cargo run -- validate outline.leo
cargo run -- apply outline.leo operations.json --dry-run
cargo run -- tui outline.leo
cargo run -- refresh-derived outline.leo 0 path/to/thin-derived-file --dry-run
cargo run -- inspect-derived path/to/thin-derived-file --summary
```

Operation batches are JSON, validated on a copy, and committed atomically. Use
`expected` on text edits for optimistic concurrency. Saving replaces only the
`vnodes` and `tnodes` sections and retains the rest of the Leo XML envelope.

The small TUI is currently a read-only browser. Use arrows or `hjkl` to navigate
and expand nodes, and `q` to quit. It automatically resolves ancestor `@path`
directives and overlays `@file`, `@thin`, and `@file-thin` descendants in memory;
use `--no-derived` to disable this. `refresh-derived` parses Leo 5 thin sentinels
and transactionally reflects the reconstructed external hierarchy and bodies in
the outline. It requires an explicit outline position and external path; path
resolution from Leo directives and writing derived files are not yet included.
