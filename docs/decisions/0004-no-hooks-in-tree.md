# 0004 No hooks in the template tree

## Status

Accepted

## Context

Hooks belong to IDE and git providers, not a portable `.agents/` tree.

## Decision

Do not ship hook scripts under `.agents/`. Use your tool's hook paths and its current docs.

## Consequences

Teams configure hooks in their tool's hook directory, `core.hooksPath`, or equivalent locally.
