# Agent rules

Short router. Load linked files only when the task needs them.

**Foundation:** [context/foundation.md](context/foundation.md) · **Writing:** [context/writing-standard.md](context/writing-standard.md) · **Standards:** [standards/](standards/) · **Vendor wiring:** [standards/adapters.md](standards/adapters.md)

## Global vs project

| Copy location | Use |
|---------------|-----|
| User agent home (e.g. `~/.agents/`) | Cross-repo memory, checklists, personal decisions |
| `<repo>/.agents/` | Rules and skills for one codebase |

When both exist, prefer **project** files for repo work.

## Core map

| Path | Purpose |
|------|---------|
| [standards/](standards/) | How to extend this tree (link-first) |
| [context/](context/) | Foundation, STE100 pointer, glossary, architecture |
| [checklists/](checklists/) | Done criteria |
| [memory/](memory/) | Your lessons and preferences (**you own**) |
| [decisions/](decisions/) | Your settled choices (**you own**) |
| [skills/](skills/) | Example: [self-verify](skills/self-verify/SKILL.md) |
| [rules/](rules/) | Add scoped rules when needed |
| [templates/](templates/) | Only when no live URL exists |

## Hooks

Do not put git or IDE hooks in this tree. Use your tool’s hook paths. See [adapters.md](standards/adapters.md).

## Optional extensions

[context/optional-extensions.md](context/optional-extensions.md) — profiles, evals, scratch dirs, worktrees, and other folders you add yourself.

## Maintenance

One idea per file. Review [memory/](memory/) monthly. Upstream template: [upstream.json](context/upstream.json).

Most tools do **not** auto-load this tree. Wire [AGENTS.md](AGENTS.md) in your tool’s user or project rules.
