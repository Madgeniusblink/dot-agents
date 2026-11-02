# Install protects user-owned paths and refreshes a managed list

## Status

Accepted (updated)

## Context

Re-run install must add missing template files, refresh maintainer-controlled standards and core context policy files, and never destroy user `memory/`, `decisions/`, or other edited files.

## Decision

- Lists live in `.agents/.user-owned` and `.agents/.managed-refresh` (repo-only; not copied to home).
- All installers (`install.sh`, `install.ps1`, `dot-agents` CLI) read the same lists.
- No `--force` flag.

## Consequences

Users opt into edits outside the managed list. Template updates ship via `.managed-refresh` and [CHANGELOG.md](../../CHANGELOG.md).
