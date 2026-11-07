# Point your tool at `~/.agents`

Nothing here writes into a tool's own configuration. You wire it once per tool.
Your agent home is a git checkout, so after that a rule you edit is live at once.

## The one rule: use a live transport

| Transport | Resolves | Use it |
|---|---|---|
| Symlink | when the tool reads | first choice |
| Import line | when the tool reads | when links are not followed |
| A sentence naming the path | the model chooses to open it | last resort |
| A copy of the content | never again | not at all |

A copy is a snapshot. Edit a rule and nothing happens until you reinstall, and
nothing tells you the copy is stale. That failure is silent, which makes it
worse than a pointer the model sometimes skips.

Only `AGENTS.md` and `rules/` need to load every session. The router names
`memory/`, `templates/`, and `skills/`, so the model opens those when the task
matches.

## Take it back out

```sh
sh scripts/wire.sh --remove            # what it would undo
sh scripts/wire.sh --remove --apply    # undo it
```

It drops its own import lines and its own links, nothing else. Do this before
deleting a home. A deleted home whose wiring stayed behind leaves every tool
pointing at a path that is gone.

## See what loads now

```sh
sh scripts/context.sh
```

Reads only. Run it before and after wiring: it names every file a tool loads,
which imports resolve, and whether another tool owns the file you are about to
edit.

## Do it with the script

```sh
sh scripts/wire.sh            # prints what it would change
sh scripts/wire.sh --apply    # writes it
```

It covers the tools below and is idempotent. Read on for the detail, for a
tool it does not know, or to wire by hand.

## Claude Code

Claude Code scans `~/.claude/rules/` and loads every file in it, in every
project, with no approval prompt. That is a drop-in directory, and the rule
for one is the rule every package manager settled on: never edit a file you
do not own, put your own file in their directory. `~/.claude/CLAUDE.md` is the
harness's file. It may hold your own content or a generator's output, and a
generator discards anything appended to it. Nothing here writes to it.

`wire.sh` writes one file the home owns, `~/.claude/rules/00-agents-home.md`,
holding one import line per always-on file:

```text
@~/.agents/principles.md
@~/.agents/rules/boundaries.md
@~/.agents/rules/corrections.md
```

Imports expand before the model sees anything and nest up to four hops, so an
imported file may import others. Re-run `wire.sh --apply` when you add a rule
file; it rewrites the drop-in and nothing else. `--remove` deletes it.

`--link` uses a symlink instead, since the directory follows them:

```sh
ln -s ~/.agents/rules ~/.claude/rules/agents
```

Confirm either with `claude --debug`.

Source: [How Claude remembers your project](https://code.claude.com/docs/en/memory).

Skills are the exception to import-only wiring. No tool has an import syntax
for a directory of skills, so a link is the only live transport. Without one,
name `~/.agents/skills/` in the tool's instruction file and accept that the
model decides whether to look.

## Codex

Codex reads one global file, `~/.codex/AGENTS.md`, or `$CODEX_HOME/AGENTS.md`
when that is set. It has no import syntax, so a symlink is the only live
transport.

`project_doc_max_bytes` defaults to 32 KiB. It truncates **project** files
only; the global file is not cut off. Codex does compare global plus thread
instructions against that number and warns when they exceed it, and whatever
they use is pressure on what project files get. `scripts/context.sh` reports
the running total for that reason, not because the file is capped.

Sources: [Custom instructions with AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md),
and the wording in `codex-rs/core/src/agents_md_manager.rs`: "The limit applies
only to project files."

## Cursor

Cursor keeps user-level rules in its application settings rather than a file,
so nothing here can read or write them. Paste the pointer line into
Settings by hand, and accept that it is the weak transport. Project rules are
per repository, in `.cursor/rules/*.mdc`.

## A tool with no import syntax

Name the paths as text in the tool's global instruction file:

```text
Read ~/.agents/principles.md and every file in ~/.agents/rules/ before
starting work.
```

This is the weak transport, because the model decides whether to open them.
Short tasks and subagents are the usual misses. Accept that rather than
copying the files in. Check whether the tool reads `~/.agents/skills`
directly before linking a second skills path.

## Any other tool

Find its global instruction file or user-rules field and point it at
`~/.agents`. Prefer a link over a line, and a line over a copy.
