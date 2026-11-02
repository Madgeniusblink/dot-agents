# Rust CLI

Optional installer. Same commands as [../bin/dot-agents](../bin/dot-agents).

## Build

```sh
cd cli
cargo build --release
./target/release/dot-agents install
```

## Install to PATH

```sh
cargo install --path . --locked
dot-agents install
```

Set `DOT_AGENTS_SOURCE` to the repo `.agents/` directory when you run from a global install without a checkout.

## Flags

- `install --mirror` — remove files in `~/.agents/` that are not in the template (use with care).

Shell script users: `./bin/dot-agents install` is enough for most cases.
