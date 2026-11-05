---
name: agent-home
description: Set up, wire, verify, or read the shared agent home at ~/.agents. Use when the home is missing, when its rules did not load this session, when a tool needs pointing at it, or when a rule or skill the user names is not in context.
---

# agent-home

`~/.agents` is one git checkout that holds the user's rules, memory, skills,
and standards for every agent tool. Each tool is wired to it separately, so a
tool can be unwired while the home is present and correct. This skill is how
you reach the home when it did not reach you.

## Is it there, and did it load?

```sh
test -d ~/.agents && git -C ~/.agents log --oneline -1
```

The home loaded this session only if you can already see `principles.md` and
the files in `rules/` in your context. Judge by that, not by the directory
existing. If the directory is there and the rules are not in context, the home
is fine and the wiring is not. Skip to **Wire a tool**.

## Get it

```sh
git clone <the user's dot-agents fork> ~/.agents
```

Ask for the fork URL. Do not guess it, and do not clone the upstream template
over a home the user already owns. The repository root *is* the tree: after
the clone, `~/.agents/AGENTS.md` is the router.

If `~/.agents` exists but is not a git checkout, stop and report it. Moving a
home the user filled by hand is their decision, not yours.

## Wire a tool

Full per-tool steps are in `~/.agents/docs/setup.md`. Read it before wiring.
The one rule it enforces: the transport must resolve when the tool reads, so a
symlink or an import line, never a copy. A copy goes stale and says nothing.

Only `AGENTS.md` and `rules/` need to load every session. The router names
`memory/`, `templates/`, and `skills/`, so leave those for the model to open
when the task matches.

Wiring writes into a tool's own configuration outside the home. Get the user's
approval first, and name the exact file you will change.

## Read it without wiring

When you cannot wire the tool, or the user wants the rules for this session
only, read them directly and say that you did:

```sh
cat ~/.agents/principles.md ~/.agents/rules/*.md
```

That covers the always-on set. For anything else, list the directory the
router names and open only the file that matches the task. Do not read the
whole tree; most of it is there to be opened on demand.

## Verify

```sh
sh ~/.agents/scripts/check.sh
```

Context budget and relative links. Run it after any edit you make inside the
home.

## Edit it

The home is a checkout, so an edited file is live at once and `git diff` shows
every change. Add behavior as a file in `rules/`, a fact in `memory/`, or a
procedure in `skills/`. `~/.agents/standards/` states what belongs where.
Never edit `AGENTS.md`; it is a router, and a new file in `rules/` needs no
entry in it.

Commit in the home like any other repository. Leave pushing to the user unless
they ask.
