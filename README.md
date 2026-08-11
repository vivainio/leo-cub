# leo-rs

An automation-safe Rust library and headless CLI for inspecting and modifying
Leo Editor `.leo` outlines. The model distinguishes shared node identity (GNX)
from outline positions, so clones remain explicit.

```sh
cargo run -- inspect outline.leo
cargo run -- validate outline.leo
cargo run -- apply outline.leo operations.json --dry-run
cargo run -- tui outline.leo
```

Operation batches are JSON, validated on a copy, and committed atomically. Use
`expected` on text edits for optimistic concurrency. Saving replaces only the
`vnodes` and `tnodes` sections and retains the rest of the Leo XML envelope.

The small TUI is currently a read-only browser. Use arrows or `hjkl` to navigate
and expand nodes, and `q` to quit. External `@file` synchronization is not yet
implemented; such headlines are displayed but not interpreted.
