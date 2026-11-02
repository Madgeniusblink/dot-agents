# Standard: install

Install the global home at **`~/.agents/`** (user home, not system root).

## Requirements

- Unix-like OS (macOS, Linux)
- `rsync` or the `dot-agents` CLI

## Option A — CLI (recommended)

From a clone of [Madgeniusblink/dot-agents](https://github.com/Madgeniusblink/dot-agents):

```sh
./bin/dot-agents install
```

Or build the Rust CLI (optional, faster for repeated use):

```sh
cd cli && cargo install --path . --locked
dot-agents install
```

## Option B — manual

```sh
mkdir -p ~/.agents
rsync -a --exclude='STRUCTURE.md' /path/to/dot-agents/.agents/ ~/.agents/
```

## Verify

```sh
./bin/dot-agents status
# or: dot-agents status
```

## Environment variables

| Variable | Default | Meaning |
|----------|---------|---------|
| `DOT_AGENTS_HOME` | `$HOME/.agents` | Install target |
| `DOT_AGENTS_SOURCE` | auto | Path to repo `.agents/` directory |

## Project-only install

Copy `.agents/` into a repository root. Do not overwrite `~/.agents/` unless you intend a global install.

## Update

Re-run `install`. The CLI uses `rsync` and does not delete unknown extra files in `~/.agents/` unless you pass `--mirror` (Rust CLI) or `rsync --delete` (manual).
