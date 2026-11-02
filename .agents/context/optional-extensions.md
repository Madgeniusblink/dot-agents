# Optional extensions (not in base template)

The base template stays small so any engineer, PM, founder, or CTO can adopt it without noise.

Add these **only when you need them**. None of them load automatically from `~/.agents/` unless you link them from [AGENTS.md](../AGENTS.md) or a skill.

| Extension | What it is | How it works |
|-----------|------------|--------------|
| **Profiles** | Short markdown mode notes (e.g. "strict review") | Not a Cursor feature by themselves. You say in chat or user rules: "Follow `profiles/strict.md`." We do not ship profiles in the base tree. |
| **Evals** | `prompt.md` + `expected.md` test pairs | You run them manually after rule changes. See [standards/evals.md](../standards/evals.md). |
| **Routing / permissions / MCP JSON** | Config stubs | Your client must read them; Cursor does not load `~/.agents/routing.json` by default. |
| **Workflows / schedules / handoffs** | Process docs | Useful for teams; optional. |
| **Hooks** | Shell or IDE hooks | Belong in **provider paths** (e.g. `.cursor/hooks/`, `core.hooksPath`), not here. |

Standards for each type stay in [standards/](../standards/) so you can extend without guessing shape.
