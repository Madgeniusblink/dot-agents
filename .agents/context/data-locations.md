# Data locations

One map for paths agents and humans share. Update when your layout changes.

## Global home

| What | Default path | Override |
|------|----------------|----------|
| Agent home | `~/.agents/` | `DOT_AGENTS_HOME` |

## Project (optional)

| What | Path |
|------|------|
| Project agent layer | `<repo>/.agents/` |
| Project router | `<repo>/AGENTS.md` |

## Provider defaults (do not assume)

Your tool may use its own config dirs. Prefer the table above in **user rules** so every model gets the same path.

Optional scratch or worktree paths you define yourself: [optional-extensions.md](optional-extensions.md).

## Git

- **Canon** (this template): repository URL in [upstream.json](upstream.json).
- **`~/.agents/`:** usually not a git repo; if you version it, ignore ephemeral dirs you add locally.
