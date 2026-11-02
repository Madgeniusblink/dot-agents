# 0001: Reference material lives in context/

## Status

Accepted

## Context

Global stable glossary and architecture belong in-repo for agents.

## Decision

Keep `~/.agents/context/{glossary,architecture,style-guide}.md`. When Context runtime install occupies the same path, merge or copy these files into the Context source repo instead of fighting the installer.

## Consequences

Document collision in AGENTS.md; revisit if install path changes.
