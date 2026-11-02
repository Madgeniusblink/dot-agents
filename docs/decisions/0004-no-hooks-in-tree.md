# 0004 No hooks in the template tree

## Status

Accepted

## Context

Hooks belong to IDE and git providers, not a portable `.agents/` tree.

## Decision

Do not ship hook scripts under `.agents/`. Document provider paths in [adapters.md](../../.agents/standards/adapters.md).

## Consequences

Teams configure hooks in `.cursor/hooks/`, `core.hooksPath`, or equivalent locally.
