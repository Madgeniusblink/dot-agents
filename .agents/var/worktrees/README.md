# Worktrees

Git **linked worktrees** for agent tasks (parallel branches, isolated checkouts).

Example:

```sh
REPO=~/Code/work/my-app
WT_ROOT="${DOT_AGENTS_WORKTREES:-$HOME/.agents/var/worktrees}/my-app"
mkdir -p "$WT_ROOT"
git -C "$REPO" worktree add "$WT_ROOT/feature-x" -b agent/feature-x
```

- One subdirectory per source repo under `worktrees/`.
- Remove worktrees with `git worktree remove` when done.
- Do **not** commit worktree checkouts into the dot-agents template repo.

Default root: `~/.agents/var/worktrees/` (override with `DOT_AGENTS_WORKTREES`).
