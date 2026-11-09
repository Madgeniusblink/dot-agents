# Agent rules (this repository)

You work on the **dot-agents template repository**. Its `home/` directory is the agent home: a checkout of `home/` *is* a `~/.agents`. The root holds the glue a home does not carry: the `agents` binary's source, the maintainer docs, CI, and the release files ([decision 0012](decisions/0012-home-directory-is-the-home.md)).

## Read first

| Topic | Path |
|-------|------|
| Shipped agent tree | [home/AGENTS.md](../home/AGENTS.md) |
| Admission test | [home/standards/extending.md](../home/standards/extending.md) |
| Template repo decisions | [docs/decisions/](decisions/) |
| Contributing | [CONTRIBUTING.md](../CONTRIBUTING.md) |

## Adopting this tree

There is no installer. An agent home is a git checkout whose root is this repository's `home/`, so an edited file is live at once and `git` reports every difference. A home merges the template with `-X subtree=home` ([home/standards/adopt.md](../home/standards/adopt.md)).

## Checks

CI runs acceptance tests in [.github/workflows/check.yml](../.github/workflows/check.yml). `sh scripts/check.sh` at the root runs the same checks: the home's checks on `home/`, the root docs' links, neutrality, and the merge tiers. A link inside `home/` resolves inside `home/`; a link from `home/` to a root file is a GitHub URL.
