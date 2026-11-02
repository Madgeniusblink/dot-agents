# dot-agents

Public template for agent filesystem layout: a **project** `.agents/` layer plus a
documented **global** `~/.agents/` home. Cursor discovers project skills under
`.agents/skills/` and `.cursor/skills/`; local user skills also load from
`~/.agents/skills/`.

Personal Context canon stays in
[`Madgeniusblink/context`](https://github.com/Madgeniusblink/context) (private).
**dot-agents** is the shareable skeleton, [standards](.agents/standards/) contract, and installer for both layers.

---

## Philosophy (short)

1. **Minimal core** — [foundation](.agents/context/foundation.md): route, do not encyclopedia.
2. **Link first** — [sources policy](.agents/context/sources-policy.md): credible live URLs; local [templates](.agents/templates/) only for gaps.
3. **ASD-STE100** — [writing standard](.agents/context/writing-standard.md) for all prose in this tree.
4. **Standards folder** — When someone asks how to write a skill, rule, or schedule, point to [`.agents/standards/`](.agents/standards/README.md).

---

## Install at `~/.agents/` (home directory)

Global target is **`$HOME/.agents`**, not disk `/`.

**Standalone CLI (recommended)** — template is **built into the binary**; no `bin/` script and no clone required after install:

```sh
cargo install --git https://github.com/Madgeniusblink/dot-agents --path cli --locked
dot-agents install
dot-agents doctor
```

One-shot (installs CLI via cargo, then `~/.agents/`): `curl -fsSL …/install.sh | sh` from repo [install.sh](install.sh).

Details: [cli/README.md](cli/README.md) and [`.agents/standards/install.md`](.agents/standards/install.md).

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
with the CLI:

```sh
dot-agents install
```

Manual equivalent (development only):

```sh
mkdir -p ~/.agents
rsync -a --exclude='STRUCTURE.md' ./.agents/ ~/.agents/
```

Base **core** layout (what this repo ships):

```
~/.agents/
├── AGENTS.md              # Short router
├── standards/             # How to add skills, rules, checklists (public contract)
├── context/               # Foundation, STE100, glossary, architecture, optional-extensions
├── checklists/
│   ├── before-commit.md
│   └── before-release.md
├── memory/
│   ├── MEMORY.md
│   ├── preferences.md
│   └── lessons.md
├── decisions/
│   └── 0001-<title>.md
├── skills/
│   └── self-verify/       # Example skill (checklist only; no hooks)
├── rules/
│   └── README.md          # Add scoped rules when you need them
└── templates/
    └── pr-description.md  # Only when no live URL fits
```

**Not in base:** hooks (use `.cursor/hooks/` or provider paths), profiles, evals, routing JSON, MCP JSON — see [optional-extensions.md](.agents/context/optional-extensions.md).

Full **optional** tree (teams that outgrow the base) is documented in [standards/](.agents/standards/) without shipping every folder.

### What makes an agent smarter (ranked)

1. **Verification** — [checklists/](.agents/checklists/) before you claim done.
2. **Lessons loop** — [memory/lessons.md](.agents/memory/lessons.md); promote repeats to [rules/](.agents/rules/) when needed.
3. **Short entry** — [AGENTS.md](.agents/AGENTS.md); detail in [context/](.agents/context/) and [standards/](.agents/standards/).
4. **Decisions log** — [decisions/](.agents/decisions/) so settled choices stay settled.
5. **Link-first** — [sources-policy.md](.agents/context/sources-policy.md); do not maintain vendor copies.

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
`~/.agents/` for memory, checklists, and decisions.

In this Git repo, `.agents/` is the **base core** template. Optional folders are documented, not shipped.

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
