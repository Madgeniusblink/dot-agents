# Optional extensions (not in base template)

The base template stays small so any engineer, PM, founder, or CTO can adopt it without noise.

Add these **only when you need them**. None of them load automatically from `~/.agents/` unless you link them from `AGENTS.local.md` or a skill.

| Extension | What it is | How it works |
|-----------|------------|--------------|
| **Profiles** | Short markdown mode notes (e.g. "strict review") | You say in chat or user rules: "Follow `profiles/strict.md`." We do not ship profiles in the base tree. |
| **Evals** | `prompt.md` + `expected.md` test pairs | You run them manually after rule changes. Add a folder such as `evals/` when you need them. |
| **Routing / permissions / MCP JSON** | Config stubs | Your client must read them; this template does not ship routing JSON. |
| **Workflows / schedules / handoffs** | Process docs | Useful for teams; optional folders you create. |
| **Scratch / worktrees** | Ephemeral files or agent git worktrees | e.g. `var/scratch/`, `var/worktrees/` under your agent home; set env vars only if **you** wire them in user rules. |
| **Hooks** | Shell or IDE hooks | Belong in your tool's hook paths, not in this tree. |

Standards for core artifact types stay in [standards/](../standards/) so you can extend without guessing shape.
