# Global agent home (`~/.agents`)

Short router. Load detail only when the task needs it.

**Foundation:** [context/foundation.md](context/foundation.md) · **Writing:** [ASD-STE100](context/writing-standard.md) · **Standards:** [standards/](standards/) · **Extend:** [standards/extending.md](standards/extending.md) · **Upstream:** [context/upstream.json](context/upstream.json) or `dot-agents upstream`

## Smart habits (ranked)

1. **Verification** — [checklists/before-commit.md](checklists/before-commit.md).
2. **Lessons loop** — [memory/lessons.md](memory/lessons.md); promote repeats to [rules/](rules/) when you add them.
3. **Short entry** — this file stays small; detail lives in [context/](context/), [standards/](standards/), skill `references/`.
4. **Decisions** — [decisions/](decisions/) before reopening settled choices.

## Core map (shipped in this template)

| Path | Purpose |
|------|---------|
| [standards/](standards/) | How to add skills, rules, checklists (link-first) |
| [context/](context/) | Foundation, STE100, glossary, architecture |
| [checklists/](checklists/) | Done criteria before commit or release |
| [memory/](memory/) | Lessons and preferences |
| [decisions/](decisions/) | Settled choices (ADR pattern) |
| [skills/](skills/) | Optional procedures (example: self-verify) |
| [rules/](rules/) | Optional scoped rules — add when you need `globs` |
| [templates/](templates/) | Only when no live URL exists |
| [var/](var/) | Scratch and git worktrees (contents gitignored) |

## Scratch and worktrees

Canonical ephemeral paths (same for every model if you wire them in user rules):

- Scratch: `~/.agents/var/scratch/` — `DOT_AGENTS_SCRATCH`
- Worktrees: `~/.agents/var/worktrees/` — `DOT_AGENTS_WORKTREES`

Map: [context/data-locations.md](context/data-locations.md). Standard: [standards/var-layout.md](standards/var-layout.md).

## Hooks and provider config

**Do not put git or IDE hooks in this tree.** Cursor, Claude Code, and other tools use their own hook locations (for example project `.cursor/hooks/`). This template does not ship hooks so we do not constrain models or duplicate provider wiring.

## Optional extensions

See [context/optional-extensions.md](context/optional-extensions.md) for profiles, evals, routing, MCP JSON, and other power-user folders. Add them in your own fork or home; they are not in the base install.

**Note:** `context-install` may use `~/.agents/context` as a full runtime snapshot. See [decisions/0001-reference-context-folder.md](decisions/0001-reference-context-folder.md).

Most clients do not auto-load `~/.agents/`. Wire [AGENTS.md](AGENTS.md) in user rules. For repo URLs when extending, run **`dot-agents upstream`** (canon: [upstream.json](context/upstream.json)).

## Maintenance

One idea per file. Review [memory/](memory/) monthly; delete stale entries.
