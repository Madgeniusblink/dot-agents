# dot-agents

Public template for agent filesystem layout: a **project** `.agents/` layer plus a
documented **global** `~/.agents/` home. Cursor discovers project skills under
`.agents/skills/` and `.cursor/skills/`; local user skills also load from
`~/.agents/skills/`.

Personal Context canon stays in
[`Madgeniusblink/context`](https://github.com/Madgeniusblink/context) (private).
**dot-agents** is the shareable skeleton and the map agents read to navigate
both layers.

---

## For agents: read this first

1. **Start at the entry file** — [`AGENTS.md`](.agents/AGENTS.md) in a project, or
   `~/.agents/AGENTS.md` for global rules. Both stay **short**; follow links
   instead of loading everything.
2. **Load on demand** — Open `rules/`, `context/`, skill `references/`, and
   `checklists/` only when the task matches. Do not preload the whole tree.
3. **Verify before done** — Run [`checklists/before-commit.md`](.agents/checklists/before-commit.md)
   (project) or `~/.agents/checklists/before-commit.md` (global) when finishing
   work.
4. **Most tools do not auto-scan these folders** — If something is not linked from
   `AGENTS.md` or a skill, assume it is invisible until you open it.

**Path convention**

| Location | Scope |
|----------|--------|
| `~/.agents/` | Global: your machine, all repos |
| `<repo>/.agents/` | Project: one repository |
| `<repo>/AGENTS.md` | Project router (when present); wins for repo-specific routing |

When both exist, prefer **project** rules and skills for repo work; use **global**
for personal checklists, memory, routing, and cross-repo reference.

---

## Global home: `~/.agents/`

The **full scaffold** lives in this repo under [`.agents/`](.agents/). Install
it once on your machine:

```sh
mkdir -p ~/.agents
rsync -a --exclude='STRUCTURE.md' /path/to/dot-agents/.agents/ ~/.agents/
# or: cp -R dot-agents/.agents/. ~/.agents/
```

Then edit `~/.agents/context/*` and `memory/*` for your environment. Layout:

```
~/.agents/
├── AGENTS.md              # Global rules. Keep short. Link out.
├── rules/
│   └── <topic>.md         # YAML front matter with `globs:` for scoped load
├── skills/
│   └── <skill-id>/
│       ├── SKILL.md
│       ├── scripts/
│       ├── references/    # Extra docs on demand
│       └── assets/        # Templates and samples
├── commands/
├── agents/                # Subagent personas
├── memory/
│   ├── MEMORY.md
│   ├── <topic>.md
│   └── lessons.md         # Mistakes and fixes
├── tools/
├── hooks/
├── mcp.json
├── settings.json
├── context/               # Stable reference (glossary, architecture, style)
│   ├── glossary.md
│   ├── architecture.md
│   └── style-guide.md
├── templates/
│   ├── spec.md
│   ├── pr-description.md
│   └── adr.md
├── workflows/
│   └── plan-build-review.md
├── checklists/
│   ├── before-commit.md
│   └── before-release.md
├── profiles/
│   ├── strict-reviewer.md
│   └── fast-prototype.md
├── routing.json           # Task type → model
├── evals/
│   └── <task>/
│       ├── prompt.md
│       └── expected.md
├── decisions/
│   └── 0001-<title>.md
├── handoffs/              # Session summaries
├── schedules/             # Recurring tasks
└── permissions.json       # Tool/path allow and deny
```

### What makes an agent smarter (ranked)

1. **Verification** — `checklists/` + a `before-commit` hook.
2. **Lessons loop** — Append to `memory/lessons.md`; promote repeats to `rules/`.
3. **Short entry file** — Detail in `context/`, `rules/`, skill `references/`.
4. **Scoped rules** — `globs` in rule front matter saves context.
5. **Decisions log** — Stops re-asking settled questions.
6. **Evals** — Keep 5–10 real tasks; run after rule or model changes.
7. **Routing** — Cheap tasks → fast model; hard tasks → strong model.
8. **Handoffs** — Short end-of-session summary for the next run.

### Rules for every addition

- Add a folder only when you have a real file for it.
- One idea per file.
- Review `lessons.md` and `memory/` monthly; delete stale entries.

**Context install collision:** `context-install` may use `~/.agents/context` as
a full runtime directory. Keep glossary/architecture/style either merged into
Context source or documented in `decisions/` before running `-update`.

---

## Project layer: `<repo>/.agents/`

Copy the same [`.agents/`](.agents/) tree into a repository (or a subset if you
want a lighter project). Prefer project rules/skills for repo work; keep global
`~/.agents/` for cross-repo memory, routing, and evals.

In this Git repo, `.agents/` is the canonical template — what you see in the
file tree is the full layout (not a trimmed stub).

---

## Adopt into a project

**Copy**

```sh
cp -R .agents /path/to/your-repo/
```

**GitHub template**

Use **Use this template**, then customize `.agents/AGENTS.md`.

**Subtree (track upstream)**

```sh
git subtree add --prefix=.agents \
  https://github.com/Madgeniusblink/dot-agents.git main --squash
```

**Bootstrap global home**

```sh
rsync -a --exclude='STRUCTURE.md' ./.agents/ ~/.agents/
```

Point Cursor user rules at `~/.agents/AGENTS.md` when you want global behavior.

---

## Conventions

- Skill: `.agents/skills/<id>/SKILL.md` with `name` and `description` in front matter.
- Rule: `.agents/rules/<topic>.md` with `globs:` list in front matter.
- Keep routers small; put durable text in linked files.

## License

MIT — see [LICENSE](LICENSE).
