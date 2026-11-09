# Standard: rules

Path: `rules/<topic>.md`

`rules/` is the primary surface. The router loads every `*.md` file in it each session, so a new file needs no router edit. Keep each file short; every line costs context in every session.

## Sort by decay clock

What a rule is about decides when it goes stale. Keep one clock per file.

| File | Class | Goes stale when |
|------|-------|-----------------|
| [boundaries.md](../rules/boundaries.md) (shipped) | What needs a human | Never |
| `preferences.md` (yours) | Your taste | You change your mind |
| [corrections.md](../rules/corrections.md) (shipped) | What the model misses | The model improves |

Facts go in `memory/`. Tool mechanisms go in a skill. Neither belongs in `rules/`.

## State the outcome, not the procedure

A better model finds a better route to the same outcome. A written route caps it at today's best idea. Test a line: would a much better model read it and do worse than unprompted? If yes, it is a procedure. Rewrite it as what must be true.

## Corrections expire

Each correction ends with the month added and the task that proves it: `` `2026-11 · paste a stack trace and ask for a fix` ``. On a model upgrade, run each task without its line. If the output holds, delete the line. CI rejects a correction without a test.

Install updates shipped files you have not touched. Put your own rules in a new file, such as `rules/preferences.md`.

Some tools support path-scoped front matter (for example `globs:`). If yours does, see its current docs. Otherwise write plain markdown rules.

## Rules

1. One topic per file. File name: `kebab-case` or single topic word.
2. Body: one line per bullet. Link live docs for detail.
3. Prose: [ASD-STE100](../context/writing-standard.md).

## Extend

Promote a lesson logged in `memory/` after it repeats. Do not duplicate [standards/](.) text in every rule.
