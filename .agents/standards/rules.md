# Standard: rules

Path: `.agents/rules/<topic>.md`

`rules/` is the primary surface. The router loads every `*.md` file in it each session, so a new file needs no router edit. Keep each file short; every line costs context in every session.

The shipped [behavior.md](../rules/behavior.md) refreshes on install. Add your own rules in a separate file.

Some tools support path-scoped front matter (for example `globs:`). If yours does, see its current docs. Otherwise write plain markdown rules.

## Rules

1. One topic per file. File name: `kebab-case` or single topic word.
2. Body: short bullets. Link [style-guide](../context/style-guide.md) or live docs for detail.
3. Prose: [ASD-STE100](../context/writing-standard.md).

## Extend

Promote from [lessons.md](../memory/lessons.md). Do not duplicate [standards/](.) text in every rule.
