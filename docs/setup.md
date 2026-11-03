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
