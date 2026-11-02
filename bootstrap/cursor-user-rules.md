# Cursor user rules (bootstrap)

Copy into **Cursor → Settings → Rules → User rules** (or project rules). Adjust paths if you use `DOT_AGENTS_HOME`.

---

Load global agent home before other instructions:

- Read `~/.agents/AGENTS.md` completely. Follow links on demand; do not preload the whole tree.
- Scratch / drafts: `~/.agents/var/scratch/` (or `DOT_AGENTS_SCRATCH`).
- Git worktrees for agent tasks: `~/.agents/var/worktrees/` (or `DOT_AGENTS_WORKTREES`).
- Extend using upstream standards; run `dot-agents upstream` for current repo links — do not hard-code GitHub URLs.
- Prose in `~/.agents/` follows ASD-STE100 ([writing-standard](https://raw.githubusercontent.com/Madgeniusblink/dot-agents/main/.agents/context/writing-standard.md)).

Project work: prefer `<repo>/.agents/` and `<repo>/AGENTS.md` when present.

Do not commit contents of `var/scratch/` or `var/worktrees/`.

---

Replace raw links after `dot-agents upstream` if your installed CLI version differs from `main`.
