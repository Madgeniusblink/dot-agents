# Standard: install

Install the global home at **`~/.agents/`** (user home, not system root).

## Recommended: standalone CLI

The **`dot-agents`** binary embeds the template. No clone and no `bin/` script required.

```sh
cargo install --git https://github.com/Madgeniusblink/dot-agents --path cli --locked
dot-agents install
dot-agents doctor
```

One-shot helper (clone + cargo + install): [install.sh](../../install.sh) at repo root.

## Without a checkout

After the CLI is on your PATH, only:

```sh
dot-agents install
```

Re-run after upgrading the CLI to refresh files from the new embedded template.

## Development

From a repo clone, `install` prefers `./.agents/` when present. Override:

```sh
dot-agents install --from /path/to/dot-agents/.agents
```

Or set `DOT_AGENTS_SOURCE`.

## Verify

```sh
dot-agents status
dot-agents doctor
```

## Environment

| Variable | Default | Meaning |
|----------|---------|---------|
| `DOT_AGENTS_HOME` | `$HOME/.agents` | Install target |

## Project-only

Copy `.agents/` into a repository. Do not overwrite `~/.agents/` unless you want a global install.

## Update

Run `dot-agents install` again. Use `install --mirror` only if you want to delete extra files in `~/.agents/` that are not in the template.

## Maintainers (this repo)

After you change `.agents/`, refresh the README tree:

```sh
dot-agents readme sync
```

CI runs `dot-agents readme check` on pull requests.
