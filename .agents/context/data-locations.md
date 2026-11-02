# Data locations

One map for paths agents and humans share. Update when your layout changes.

## Global home

| What | Default path | Override |
|------|----------------|----------|
| Agent home | `~/.agents/` | `DOT_AGENTS_HOME` |
| Scratch / scratchpad | `~/.agents/var/scratch/` | `DOT_AGENTS_SCRATCH` |
| Git worktrees | `~/.agents/var/worktrees/` | `DOT_AGENTS_WORKTREES` |

## Project (optional)

| What | Path |
|------|------|
| Project agent layer | `<repo>/.agents/` |
| Project scratch | `<repo>/.agents/var/scratch/` |
| Project router | `<repo>/AGENTS.md` |

Use project scratch when artifacts must live in the repo workspace. Use global scratch for personal or cross-repo notes.

## Provider defaults (do not assume)

Tools may use their own dirs (Cursor, Claude Code, Codex, etc.). Prefer the table above in **user rules** so every model gets the same path.

## Git

- **Canon** (this template): tracked in [Madgeniusblink/dot-agents](https://github.com/Madgeniusblink/dot-agents).
- **Scratch and worktrees:** ignored by `.gitignore`; never commit contents.
- **`~/.agents/`:** usually not a git repo; if you version it, keep `var/` ignored.

See [var/README.md](../var/README.md).
