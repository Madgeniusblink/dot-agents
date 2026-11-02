# dot-agents CLI

Standalone installer. The **base `.agents` template is embedded in the binary** at build time. You do not need a git clone or `bin/` script to run `install`.

## Install the CLI

**From GitHub (recommended when Rust is installed):**

```sh
cargo install --git https://github.com/Madgeniusblink/dot-agents --path cli --locked
```

**From a local clone (development):**

```sh
cd cli && cargo install --path . --locked
```

**Without Rust:** download a release binary from GitHub Releases when published, or use the repo [install.sh](../install.sh) helper (requires Rust or an existing `dot-agents` on PATH).

## Install `~/.agents/`

```sh
dot-agents install
dot-agents doctor
dot-agents status
```

## Commands

| Command | Action |
|---------|--------|
| `install` | Write template to `~/.agents/` (or `DOT_AGENTS_HOME`) |
| `install --mirror` | Also remove files in home not in template |
| `install --from PATH` | Dev: use a checkout instead of embedded template |
| `status` | Show home path and file count |
| `doctor` | Check core foundation files |

## Environment

| Variable | Default |
|----------|---------|
| `DOT_AGENTS_HOME` | `$HOME/.agents` |
| `DOT_AGENTS_SOURCE` | If set, `install` prefers this directory over embedded (unless `--from`) |

## Why not `bin/`?

`bin/` scripts only work inside a clone. The product is the **`dot-agents` binary**: one name on your PATH, same behavior everywhere, template baked in at compile time.
