# Agent rules (this repository)

You work on the **dot-agents template repository**. Its root is the agent home: a clone of it *is* a `~/.agents`.

## Read first

| Topic | Path |
|-------|------|
| Shipped agent tree | [AGENTS.md](../AGENTS.md) |
| Admission test | [standards/extending.md](../standards/extending.md) |
| Template repo decisions | [docs/decisions/](decisions/) |
| Contributing | [CONTRIBUTING.md](../CONTRIBUTING.md) |

## Adopting this tree

There is no installer. An agent home is a clone of a repository like this one, so an edited file is live at once and `git` reports every difference ([standards/adopt.md](../standards/adopt.md)).

## Checks

CI runs acceptance tests in [.github/workflows/check.yml](../.github/workflows/check.yml). Prefer those checks over running unchecked scripts from clones.
