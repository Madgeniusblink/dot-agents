# Agent rules (this repository)

You work on the **dot-agents template repository**, not only on `~/.agents/`.

## Read first

| Topic | Path |
|-------|------|
| Shipped agent tree | [.agents/AGENTS.md](.agents/AGENTS.md) |
| Admission test | [.agents/standards/extending.md](.agents/standards/extending.md) |
| Template repo decisions | [docs/decisions/](docs/decisions/) |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |

## Adopting this tree

There is no installer. An agent home is a clone of a repository like this one, so an edited file is live at once and `git` reports every difference ([.agents/standards/adopt.md](.agents/standards/adopt.md)).

## Checks

CI runs acceptance tests in [.github/workflows/check.yml](.github/workflows/check.yml). Prefer those checks over running unchecked scripts from clones.
