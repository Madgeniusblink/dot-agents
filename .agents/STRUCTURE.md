# Structure map (for agents)

This directory is the **full template tree**. Copy it to `~/.agents/` for a global
home, or into `<repo>/.agents/` for one project.

| Path | Purpose |
|------|---------|
| [AGENTS.md](AGENTS.md) | Short router — start here |
| [rules/](rules/) | File-scoped rules (`globs` in front matter) |
| [skills/](skills/) | Skills with `scripts/`, `references/`, `assets/` |
| [context/](context/) | Glossary, architecture, style |
| [checklists/](checklists/) | Done criteria |
| [memory/](memory/) | Durable facts and [lessons.md](memory/lessons.md) |
| [templates/](templates/) | Spec, PR, ADR shapes |
| [workflows/](workflows/) | Multi-step pipelines |
| [profiles/](profiles/) | Strict vs fast modes |
| [evals/](evals/) | Prompt + expected pairs |
| [decisions/](decisions/) | Settled choices |
| [handoffs/](handoffs/) | Session summaries |
| [schedules/](schedules/) | Recurring maintenance |
| [routing.json](routing.json) | Task → model |
| [permissions.json](permissions.json) | Allow/deny lists |
| [mcp.json](mcp.json) / [settings.json](settings.json) | Client config stubs |

Full narrative: [README.md](../README.md).
