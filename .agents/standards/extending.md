# Extending this tree

## Words

| Term | Meaning |
|------|---------|
| **Upstream** | This template repo ([upstream.json](../context/upstream.json)) |
| **Overlay** | Your files beside upstream: new rules, memory, skills |
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
3. `git merge upstream/main` to take base improvements ([adopt.md](adopt.md)).

## Coexistence

Your home is a checkout, so `git status` shows exactly what you changed and a merge shows you any line that moved under it ([adopt.md](adopt.md)).

So add beside the base where you can. A file you leave alone merges cleanly; a file you rewrote conflicts, which is the signal you want.

Do not name vendors. Say "your tool."
