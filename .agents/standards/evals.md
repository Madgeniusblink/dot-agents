# Standard: evals

Evals test whether rules and routing still work after you change them.

Path: `.agents/evals/<task-id>/`

| File | Content |
|------|---------|
| `prompt.md` | Task you give the agent |
| `expected.md` | Pass criteria (behavior, not exact wording) |

Rules:

1. Keep 5–10 evals that match **your** real work.
2. Run evals after you change [rules/](../rules/), [routing.json](../routing.json), or models.
3. STE100 in both files ([writing-standard](../context/writing-standard.md)).

Example: [smoke-verify](../evals/smoke-verify/).
