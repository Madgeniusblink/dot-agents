# Agent rules (this repository)

You work on the **dot-agents template repository**, not only on `~/.agents/`.

## Read first

| Topic | Path |
|-------|------|
| Shipped agent tree | [.agents/AGENTS.md](.agents/AGENTS.md) |
| Admission test | [.agents/standards/extending.md](.agents/standards/extending.md) |
| Template repo decisions | [docs/decisions/](docs/decisions/) |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |

## Install behavior

[install.sh](install.sh) and [install.ps1](install.ps1) refresh **upstream-owned** paths under `.agents/`. They **never** overwrite existing `memory/` or `decisions/` files in the target home.

## Checks

CI runs acceptance tests in [.github/workflows/check.yml](.github/workflows/check.yml). Prefer those checks over running unchecked scripts from clones.
