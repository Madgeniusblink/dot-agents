# 0008 Ship no files the user is meant to fill in or that restate model defaults

## Status

Accepted

## Context

The install manifest replaces only files the user has not touched. A shipped file that exists to be filled in (a glossary, a lessons log, a memory index) differs from the shipped copy forever, so install reports it as edited on every run. Shipped checklists and a skill that read them aloud listed steps a current model takes unprompted.

## Decision

Ship no file whose purpose is to be edited, and no file that restates model defaults. A format convention for user files goes in one line of `principles.md`. `memory/` ships empty.

## Consequences

The edited report lists only real edits. The shipped tree drops from 29 files to 22. Filled-in copies in existing homes are kept.
