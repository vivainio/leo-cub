# Installation

## Install a released version

The easiest way to install the `cub` command is with `uv`:

```sh
uv tool install leo-cub
```

`pip` works as well:

```sh
python -m pip install leo-cub
```

Check that the command is available:

```sh
cub --help
```

## Build from the repository

You need a recent Rust toolchain and Cargo:

```sh
cargo install --path .
```

From the checkout, this installs the binary defined by the package. Install
the bundled agent skill separately if you use Claude Code or another tool that
reads local skills:

```sh
cub install-skills
```

That command writes `~/.claude/skills/leo-cub/SKILL.md` and can be run again
after an upgrade.
