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
sh ~/.agents/scripts/context.sh
```

It reports the home's branch and state, the always-on files and their byte
cost, and for each tool what it loads, what resolves, and what does not. Three
findings mean the wiring is not what it looks like:

- **BROKEN** — an import points at a file that is not there, so that rule
  never loads and nothing says so.
- **lines of rules live in this file** — the tool's own config holds rules the
  home should own. Two sources of truth, and they will disagree.
- **OWNED BY A GENERATOR** — another tool writes this file. Anything added to
  it is discarded on the next run. Take the file over before wiring it.

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

```sh
sh ~/.agents/scripts/wire.sh            # what it would change
sh ~/.agents/scripts/wire.sh --apply    # write it
```

It finds the tools in `$HOME`, drops one file it owns into each tool's rules
directory with one import line per always-on file, and links `skills/`, which
no tool can import. It never edits a file the tool owns. `--link` links the
rules too. Running it twice changes nothing. `docs/setup.md` covers the rest.

Every transport must resolve when the tool reads, so a symlink or an import
line, never a copy. A copy goes stale and says nothing.

Wiring writes outside the home. Get the user's approval first, and name the
exact file you will change. `--remove` takes out exactly what `--apply` added
and leaves the rest of the file alone, so a home can be removed without
stranding the half that pointed at it.

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

## Update it

The home takes shared files from a template by merge, not by an installer:

```sh
git -C ~/.agents fetch upstream
git -C ~/.agents merge --no-edit upstream/main
```

Keep `--no-edit`. Without it git opens an editor for the merge message and
waits, which looks like a hang in any shell the user is not watching.

Read `CHANGELOG.md` after the merge; a release may ask for a one-time move.

A conflict appears only where the user changed a file the template also
changed, and that conflict is the report an installer could not give. Resolve
it, do not discard either side, and do not re-run the merge with a strategy
that picks a winner for them. If `upstream` is not set, ask for the template's
URL rather than guessing it.

## Edit it

The home is a checkout, so an edited file is live at once and `git diff` shows
every change. Add behavior as a file in `rules/`, a fact in `memory/`, or a
procedure in `skills/`. `~/.agents/standards/` states what belongs where.
Never edit `AGENTS.md`; it is a router, and a new file in `rules/` needs no
entry in it.

Commit in the home like any other repository. Leave pushing to the user unless
they ask.
