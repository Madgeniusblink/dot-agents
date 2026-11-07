# 0010 A generated file for a tool that follows no pointer

## Status

Accepted

## Context

`docs/setup.md` ranks transports: a link, then an import line, then a
sentence naming the path, and never a copy. Codex expands no import and
reads one global file, so the home linked that file to its router. The router
is a table of paths. Codex then saw the table and had to choose to open the
rules, which the same document calls the weak transport. A review caught it
after a `reset` on a real machine: Codex had less guaranteed context than
before the reset.

Grok Build has the same shape. It scans `~/.grok/rules/` in every project and
loads each file in full, and it documents no import syntax.

## Decision

For a tool that follows neither a link to content nor an import line, `wire`
writes one file the home owns into the tool's slot or scanned directory,
holding `principles.md` and every `rules/*.md` in full under a header that
names the home. `wire` rewrites it whenever the home differs, `status` reports
it stale until then, `unwire` deletes it, and a file in that slot the home did
not write is left alone and reported.

The ban on copies stands. A bare copy goes stale in silence. This file is
rewritten by the same command that wires and checked by the same command
that reports, which is what the ban was protecting.

Decision 0009 item 2, that the binary embeds no content, also stands. The
content comes from the checkout at the moment `wire` runs.

## Consequences

Codex and Grok Build get the always-on set in full, the same bytes Claude
Code gets through its imports. `sync` keeps them current. A home edited
without a `wire` afterwards shows as stale in `status` and in the next
`wire` dry run.

`scripts/wire.sh` writes the same bytes, and the parity test holds the two
together until the scripts retire.
