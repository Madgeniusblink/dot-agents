# var — ephemeral and machine-local

Canonical locations for **scratch** and **git worktrees**. Tools use different default paths; this tree gives one cross-tool convention.

| Path | Env override | Use |
|------|----------------|-----|
| [scratch/](scratch/) | `DOT_AGENTS_SCRATCH` | Drafts, scratchpad, temp exports — never commit |
| [worktrees/](worktrees/) | `DOT_AGENTS_WORKTREES` | `git worktree add` targets for agent tasks |

Default roots (when env is unset):

- Scratch: `~/.agents/var/scratch/`
- Worktrees: `~/.agents/var/worktrees/`

**Project work:** use a subfolder per repo, e.g. `scratch/<repo-name>/` or `worktrees/<repo-name>/`.

**In a git repo:** prefer `<repo>/.agents/var/scratch/` for code that must stay next to the project; use global `var/` for cross-repo or personal notes.

Wire these paths in user rules or [AGENTS.md](../AGENTS.md). Clients do not discover them automatically.
