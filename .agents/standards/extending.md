# Extending this tree

## Words

| Term | Meaning |
|------|---------|
| **Upstream** | This template repo ([upstream.json](../context/upstream.json)) |
| **Overlay** | Your files beside upstream (memory, extra skills) |
| **Extend** | Add artifacts using [standards/](.) |

## Admission test (before you add a file)

Apply this in order. If any step fails, do not add the file.

1. **Source** — Is there a primary URL? Link it ([sources-policy.md](../context/sources-policy.md)). Skip a local copy unless the URL is missing or unstable.
2. **Model-knowledge** — Does the model already know this (format specs, STE100 rules, generic tool docs)? Do not restate it. Link or delete.
3. **Generalize** — Remove names, chats, branches, and one-off paths. Would another team reuse this?
4. **Normalize** — One idea per file. Short sentences ([writing-standard.md](../context/writing-standard.md)).
5. **Decision** — Is this a settled template choice? Record it under [docs/decisions/](../../docs/decisions/) in the **git repo** (not shipped to `~/.agents/`). User choices stay in `.agents/decisions/`.

## Rules

1. Link live docs first ([sources-policy.md](../context/sources-policy.md)).
2. Open PRs on **upstream** to change the shared template.
3. Re-run [install.sh](../../install.sh) to refresh upstream-owned files; `memory/` and `decisions/` stay yours ([install.md](install.md)).

## Coexistence

Default install **merges**: it updates upstream-owned paths and adds missing files. It never overwrites your existing `memory/` or `decisions/` files.

Vendor names: [adapters.md](adapters.md) only.
