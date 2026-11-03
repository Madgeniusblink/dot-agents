# Contributing

Thanks for improving the shared template.

## Before you open a PR

1. Read [AGENTS.md](AGENTS.md) and the [admission test](.agents/standards/extending.md#admission-test-before-you-add-a-file).
2. Run the same checks as CI locally if you can: neutrality greps, link check, budget check, install tests (see [.github/workflows/check.yml](.github/workflows/check.yml)).
3. Do not name vendors in shipped files. Say "your tool."
4. Record **template** design choices in [docs/decisions/](docs/decisions/), not in `.agents/decisions/` (user-owned).

## Versioning

Template version and tag URLs live in [.agents/context/upstream.json](.agents/context/upstream.json) (`template_version`, `release_tag`, `links.raw_tag`). Tag releases on `main` when the template version bumps.

## GitHub account settings (maintainer)

Turn on both options under [GitHub → Settings → Emails](https://github.com/settings/emails):

1. **Keep my email addresses private** — web edits and **merge commits** use your `@users.noreply.github.com` address instead of your personal email.
2. **Block command line pushes that expose my email** — rejects `git push` when commit metadata contains a private address.

Also set local Git author email to the same noreply address (see **set your email in Git** on that page). Use **Squash merge** or ensure PR commits already use noreply before merging.

**Already public merge commits** keep their old committer metadata until history is rewritten (requires maintainer approval, retags, and submodule updates). New merges after the settings above use noreply.

Enable **Private vulnerability reporting** on each public repo (**Settings → Security**).

Repository hygiene: **Template repository**, description, topics (`agents`, `template`, `dot-agents`).

## Code of conduct

Be direct and kind. Minimize diff scope.
