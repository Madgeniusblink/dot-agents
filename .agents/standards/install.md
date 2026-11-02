# Standard: install

Target: **agent home** directory (default `~/.agents` on Unix, `$HOME\.agents` on Windows when `$HOME` is set). Not disk `/`.

## Scripts (no Rust required)

From a clone of this repository (see [upstream.json](../context/upstream.json) → `links.repo`):

```sh
./install.sh          # refresh upstream-owned files; add missing paths
./install.sh --force  # same protection for memory/ and decisions/
```

Windows: `.\install.ps1` (optional `-Force`; same file rules)

Environment: `DOT_AGENTS_HOME` overrides the destination. On Unix, `HOME` must be set when `DOT_AGENTS_HOME` is unset.

Optional CLI: URL in [upstream.json](../context/upstream.json) → `links.cli_install`.

## Upstream-owned vs user-owned

| Area | On re-run |
|------|-----------|
| **Upstream-owned** (standards, context, skills, rules, templates, checklists, shipped stubs) | Existing files are **updated** from the template. Missing files are added. |
| **User-owned** (`memory/`, `decisions/`) | Existing files are **never** overwritten. Missing files are added once. |

`--force` / `-Force` does **not** overwrite existing files under `memory/` or `decisions/`.

## Project copy

```sh
DOT_AGENTS_HOME=/path/to/repo/.agents ./install.sh
```

Do not use `cp -R` if the project tree already has edited `memory/` or `decisions/`.
