# 0006 Install replaces only files the user has not touched

## Status

Accepted. Supersedes [0001](0001-install-user-owned-paths.md) and the install part of [0005](0005-dynamic-base-layer.md).

## Context

0.1.1 protected edits with two lists, `.user-owned` and `.managed-refresh`. Both guess in advance which files a user will edit. 0.2.0 moved `AGENTS.md` into the managed list so base improvements would arrive, which broke the 0.1.1 promise for anyone who had edited it.

`AGENTS.md` only needed protecting because it mixed template content with user content.

## Decision

- Install writes `.installed` in the target home: a SHA-256 per file, as installed.
- Per shipped file: missing, add it. Unchanged since install wrote it, replace it. Otherwise keep it and print `kept (edited)`.
- An existing file with no manifest entry is adopted only if it equals the new version. Otherwise it is kept. Recording an unknown on-disk hash would make the next install replace a pre-manifest edit.
- A file the template stops shipping is removed if unchanged.
- `AGENTS.md` is a router of links only. `AGENTS.local.md` is dropped; user content goes in `rules/`, `memory/`, `decisions/`.
- CI budget: `AGENTS.md` at 50 lines or fewer, one line per rule, `SKILL.md` at 120 lines or fewer.

## Consequences

Untouched files always get the newest version, and no edit is lost. The first install from an older version updates nothing that differs; it lists those files. The optional CLI must adopt the manifest before it pins this template version.
