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

User-level rules in `~/.claude/rules/` load in every project with no approval
prompt. That directory supports symlinks.

```sh
ln -s ~/.agents/rules ~/.claude/rules/agents
ln -s ~/.agents/skills ~/.claude/skills/agents
```

Confirm with `claude --debug`. If the link does not resolve, put import lines
in `~/.claude/CLAUDE.md` instead:

```text
@~/.agents/principles.md
@~/.agents/rules/boundaries.md
@~/.agents/rules/corrections.md
```

Imports expand before the model sees anything and nest up to four hops, so an
imported file may import others. Add a line when you add a rule file.

Source: [How Claude remembers your project](https://code.claude.com/docs/en/memory).

Skills are the exception to import-only wiring. No tool has an import syntax
for a directory of skills, so a link is the only live transport. Without one,
name `~/.agents/skills/` in the tool's instruction file and accept that the
model decides whether to look.

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
