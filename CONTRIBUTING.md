# Contributing

Thanks for improving the shared template.

## Before you open a PR

1. Read [AGENTS.md](AGENTS.md) and the [admission test](.agents/standards/extending.md#admission-test-before-you-add-a-file).
2. Run the same checks as CI locally if you can: neutrality greps, link check, install tests (see [.github/workflows/check.yml](.github/workflows/check.yml)).
3. Keep vendor names in [.agents/standards/adapters.md](.agents/standards/adapters.md) only.
4. Record **template** design choices in [docs/decisions/](docs/decisions/), not in `.agents/decisions/` (user-owned).

## Versioning

Template version and tag URLs live in [.agents/context/upstream.json](.agents/context/upstream.json) (`template_version`, `release_tag`, `links.raw_tag`). Tag releases on `main` when the template version bumps.

## GitHub repository settings (maintainer)

Enable **Template repository**, add a short description and topics (`agents`, `template`, `dot-agents`), and turn on **Keep my email addresses private** plus **Block command line pushes that expose my email**.

## Code of conduct

Be direct and kind. Minimize diff scope.
