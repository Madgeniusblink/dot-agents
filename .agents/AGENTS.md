# `.agents` base layer

This directory is the **project-scoped agent base layer**: skills and small
notes that belong in-repo, not in a personal Context install.

## Entry routing

When this repository has a root [`AGENTS.md`](../AGENTS.md), that file stays the
**primary router** (bootstrap, safety, links). This file holds **base-layer**
detail that should not duplicate or inflate the root entry.

If there is no root `AGENTS.md` yet, treat this file as the starting point and
add a short root router when the project grows.

## Layout

| Path | Role |
|------|------|
| `skills/<name>/SKILL.md` | Project skills (Cursor/Codex/Claude discovery) |
| `skills/<name>/references/` | Optional on-demand material for a skill |

Do not commit local provider runtimes (for example repo-root `.claude/` hook
trees) unless the project explicitly documents why they must be shared.

## Skills

Add real procedures under `skills/`. Remove or replace `example-skill` once
you have your own skills.

Upstream template: [github.com/Madgeniusblink/dot-agents](https://github.com/Madgeniusblink/dot-agents).
