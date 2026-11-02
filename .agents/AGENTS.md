# Global agent home (`~/.agents`)

Short router. Load detail only when the task needs it.

**Foundation:** [context/foundation.md](context/foundation.md) · **Writing:** [ASD-STE100](context/writing-standard.md) · **Standards:** [standards/](standards/) · **Install:** [standards/install.md](standards/install.md)

## Smart habits (ranked)

1. **Verification** — [checklists/before-commit.md](checklists/before-commit.md) and [hooks/before-commit.sh](hooks/before-commit.sh).
2. **Lessons loop** — [memory/lessons.md](memory/lessons.md); promote repeats to [rules/](rules/).
3. **Short entry** — this file stays small; detail lives in [context/](context/), [rules/](rules/), skill `references/`.
4. **Scoped rules** — each rule file has a `globs` header for matching paths only.
5. **Decisions** — [decisions/](decisions/) before reopening settled choices.
6. **Evals** — [evals/](evals/) after rule or model changes.
7. **Routing** — [routing.json](routing.json) for task-type → model.
8. **Handoffs** — [handoffs/](handoffs/) at session end.

## Map

| Path | Purpose |
|------|---------|
| [rules/](rules/) | File-scoped rules (`globs` in front matter) |
| [skills/](skills/) | Procedures; on-demand `references/` and `assets/` |
| [commands/](commands/) | Repeatable command docs |
| [agents/](agents/) | Subagent personas |
| [memory/](memory/) | Durable facts and lessons |
| [tools/](tools/) | Local CLI notes and wrappers |
| [hooks/](hooks/) | Shell hooks you wire in providers |
| [context/](context/) | Glossary, architecture, style, foundation, sources policy |
| [standards/](standards/) | How to add skills, rules, schedules, evals (link-first) |
| [templates/](templates/) | Fixed output shapes |
| [workflows/](workflows/) | Multi-step pipelines |
| [checklists/](checklists/) | Done criteria |
| [profiles/](profiles/) | Behavior modes |
| [evals/](evals/) | Prompt + expected result pairs |
| [decisions/](decisions/) | ADR-style settled choices |
| [handoffs/](handoffs/) | Session summaries |
| [schedules/](schedules/) | Recurring tasks |
| [routing.json](routing.json) | Model routing |
| [permissions.json](permissions.json) | Tool/path allow and deny |
| [mcp.json](mcp.json) | MCP registry (wire in your client) |
| [settings.json](settings.json) | Non-secret defaults |

**Note:** If you install personal [Context](https://github.com/Madgeniusblink/context) with `context-install`, that tool uses `~/.agents/context` as a full runtime snapshot. Merge or relocate this reference `context/` tree before `-update`, or keep canon in `~/Code/work/context` and treat these three files as your personal overlay copies.

Most clients do not auto-load these folders. Point skills and rules at paths here, or symlink from a project `.agents/` template ([dot-agents](https://github.com/Madgeniusblink/dot-agents)).

## Maintenance

One idea per file. Add a folder only when it has a real file. Review [memory/](memory/) monthly; delete stale entries.
