# Standard: install

Target: **agent home** directory (default `~/.agents` on Unix, `%USERPROFILE%\.agents` on Windows). Not disk `/`.

## Scripts (no Rust required)

From a clone of [Madgeniusblink/dot-agents](https://github.com/Madgeniusblink/dot-agents):

```sh
./install.sh          # skip files that already exist
./install.sh --force  # overwrite every template file
```

Windows: `.\install.ps1` or `.\install.ps1 -Force`

Environment: `DOT_AGENTS_HOME` overrides the destination.

## Update behavior

- Default: adds **missing** upstream files only. Does **not** change your `memory/`, `decisions/`, or other edited files.
- `--force` / `-Force`: replaces all template paths with upstream copies.

## Project-only

Copy `.agents/` into a repository. Do not run `install.sh` against `~/.agents` unless you want a global home.

## Repo-only file

`.agents/STRUCTURE.md` is for maintainers in git; install scripts exclude it.
