# Standard: var (scratch and worktrees)

## Purpose

Give every model the **same** ephemeral paths. Providers use different defaults; this standard is the contract you cite in rules.

## Layout

```
~/.agents/var/
├── scratch/      # Ephemeral files (gitignored contents)
└── worktrees/    # git worktree add targets (gitignored contents)
```

## Environment

| Variable | Default |
|----------|---------|
| `DOT_AGENTS_SCRATCH` | `$DOT_AGENTS_HOME/var/scratch` or `~/.agents/var/scratch` |
| `DOT_AGENTS_WORKTREES` | `$DOT_AGENTS_HOME/var/worktrees` or `~/.agents/var/worktrees` |

## Rules

1. Scratch is **never** committed. Each folder ships `.gitignore` with `*` and negated README only.
2. Worktrees are **real git worktrees**, not copies. Register path under `var/worktrees/<repo>/`.
3. STE100 in README files only ([writing-standard](../context/writing-standard.md)).
4. Link [data-locations.md](../context/data-locations.md) from [AGENTS.md](../AGENTS.md).
