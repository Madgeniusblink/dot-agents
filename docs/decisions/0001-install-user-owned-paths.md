# 0001 Install protects memory and decisions

## Status

Accepted

## Context

Re-run install must refresh upstream template files without destroying user `memory/` or `decisions/`.

## Decision

Install scripts treat `memory/` and `decisions/` as user-owned. Existing files there are never overwritten. All other template paths refresh on each install. `--force` does not change user-owned protection.

## Consequences

Users get template updates without losing lessons. `--force` is not a data-destructive flag for personal dirs.
