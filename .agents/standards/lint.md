# Standard: lint (keep the contract)

Run **`dot-agents lint`** from the **upstream repo root** before you merge template changes.

## What lint checks

| Check | Keeps |
|-------|--------|
| `readme check` | README tree matches `.agents/` |
| `upstream.json` vs `Cargo.toml` `repository` | One canonical GitHub URL |
| `var/scratch` and `var/worktrees` `.gitignore` | Ephemeral files never commit |
| Root `.gitignore` var patterns | Same rule in git checkout |
| Required files | `lint.md`, `extending.md`, `upstream.json`, `data-locations.md` |

## Commands (full stack)

| Command | When |
|---------|------|
| `dot-agents lint` | Maintainer: before PR merge (repo root) |
| `dot-agents doctor` | User: after `install` on `~/.agents/` |
| `dot-agents readme sync` | After you change `.agents/` layout |
| `dot-agents upstream` | Before you paste repo links in overlays |
| `cargo test --manifest-path cli/Cargo.toml` | CI and local — behaviors B1–B4 |

## CI

GitHub Actions runs `cargo test` and `dot-agents lint` on pull requests. See [.github/workflows/contract.yml](../../.github/workflows/contract.yml).

## User install

Lint does **not** require a clone. Users run **`dot-agents doctor`** only.

## STE100

Lint output is CLI text. Standards prose stays [ASD-STE100](../context/writing-standard.md).
