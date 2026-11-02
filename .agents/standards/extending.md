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
2. **Model-knowledge** — Run the task **without** the proposed file. If the result already passes, do not add it. If the model already knows the content (format specs, STE100 rules, generic tool docs), link or delete; do not restate.
3. **Generalize** — Remove names, chats, branches, and one-off paths. Would another team reuse this?
4. **Normalize** — One idea per file. Short sentences ([writing-standard.md](../context/writing-standard.md)).
5. **Decision** — Is this a settled template choice? Record it under [docs/decisions/](../../docs/decisions/) in the **git repo** (not shipped to `~/.agents/`). User choices stay in `.agents/decisions/`.

## Rules

1. Link live docs first ([sources-policy.md](../context/sources-policy.md)).
2. Open PRs on **upstream** to change the shared template.
3. Re-run [install.sh](../../install.sh) to refresh [managed paths](../.managed-refresh) and add missing files ([install.md](install.md)).

## Coexistence

Default install adds missing files and refreshes only `.managed-refresh` paths. It never overwrites `.user-owned` paths or other files you already edited.

Vendor names: [adapters.md](adapters.md) only.
