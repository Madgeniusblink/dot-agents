# Standard: install

Target: **agent home** directory (default `~/.agents` on Unix, `$HOME\.agents` on Windows when `$HOME` is set). Not disk `/`.

## Scripts (no Rust required)

From a clone (see [upstream.json](../context/upstream.json) → `links.repo` or `links.raw_tag`):

```sh
./install.sh
```

Windows: `.\install.ps1`

Environment: `DOT_AGENTS_HOME` overrides the destination. On Unix, `HOME` must be set when `DOT_AGENTS_HOME` is unset.

Optional CLI: [upstream.json](../context/upstream.json) → `links.cli_install` (same rules; reads `.user-owned` and `.managed-refresh`).

## Policy files (single source of truth)

| File | Role |
|------|------|
| [`.user-owned`](../.user-owned) | Prefixes never overwritten once a file exists (`memory/`, `decisions/`). |
| [`.managed-refresh`](../.managed-refresh) | Paths replaced on every install when present (standards + selected context files). |

Installers do **not** copy these two files into your home. They read them from the template checkout.

## Update behavior

| Case | On re-run |
|------|-----------|
| Missing file | Added |
| Path under `.user-owned` and file exists | Skipped |
| Path under `.managed-refresh` and file exists | Refreshed from template |
| Any other existing file (e.g. edited `AGENTS.md`, `checklists/`, `data-locations.md`) | Skipped |

See [CHANGELOG.md](../../CHANGELOG.md) when managed paths change.

## Project copy

```sh
DOT_AGENTS_HOME=/path/to/repo/.agents ./install.sh
```
