---
name: agent-home
description: Set up, wire, verify, or update the shared agent home at ~/.agents. Use when the home is missing, when its rules did not load this session, when a tool needs pointing at it, or when a rule changed.
---

# agent-home

The home is a git checkout at `~/.agents`. The `agents` binary does every step below. Without the binary, the scripts in `scripts/` do the same with no build.

Run the brief command first, then ask for one item with `--long`.

## Did it load?

Run `agents status`, or `sh ~/.agents/scripts/context.sh`. It says what each tool loads, from where, and whether a generated file is STALE. If `~/.agents` is missing, get it.

## Get it

```sh
agents init <your-copy-url> --apply
```

This clones to `~/.agents`, adds the template as `upstream`, and wires every tool it finds. Without the binary: `git clone <your-copy-url> ~/.agents`, then `sh ~/.agents/scripts/wire.sh --apply`.

## Wire a tool

`agents wire --apply`. It writes one file the home owns into each tool's drop-in directory, or a generated file for a tool that follows no import, and never edits a file the tool owns. `agents unwire --apply` removes exactly that. The transports per tool are in [docs/setup.md](../../docs/setup.md).

## Verify

`agents check`. One line when all is well; one line per fault with the file, the rule, and the fix.

## Update

`agents sync --apply`. It merges the template's main into the home, rewires, and reports. A conflict is a rule you rewrote that moved upstream; resolve it in the file and commit.

## Edit

Add a file in `rules/`, `memory/`, or `skills/`. Do not edit `AGENTS.md`. Then `agents check`, commit, push, and `agents wire --apply` so tools that follow no import get the new text.
