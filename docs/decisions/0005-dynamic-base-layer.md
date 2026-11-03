# 0005 Dynamic base layer: add beside, do not edit

## Status

Accepted. Install and `AGENTS.local.md` parts superseded by [0006](0006-install-manifest.md).

## Context

Under 0.1.x the router listed files and users edited `AGENTS.md`. Install then
protected those edits, so base improvements to the router never reached users.
A personal overlay had to re-edit the router and the memory index to load.

## Decision

- The router names folders, not files. Every `rules/*.md` loads each session.
  `memory/`, `templates/`, and `skills/` are listed by the agent, not indexed.
- Shipped files are upstream-owned and refresh on install: router,
  principles, shipped rules, templates, and skills join `.managed-refresh`.
- Users customize by adding files: their own rules, memory, skills, and an
  optional `AGENTS.local.md` that wins over shipped files where they conflict.
- `rules/` is the primary surface. A skill is the exception.
- `standards/adapters.md` is removed. No shipped file names a vendor.
  Supersedes [0003](0003-adapters-only-vendors.md).

## Consequences

Base improvements arrive on the next install with nothing to merge. An overlay
installs by adding files only. Edits to a shipped file are lost on install;
the CHANGELOG upgrade note tells 0.1.x users to move `AGENTS.md` edits to
`AGENTS.local.md`. A user who wants to drop a shipped rule overrides it in
`AGENTS.local.md`.
