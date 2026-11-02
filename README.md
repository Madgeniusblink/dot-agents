# dot-agents

Public template for agent filesystem layout: a **project** `.agents/` layer plus a
documented **global** `~/.agents/` home. Cursor discovers project skills under
`.agents/skills/` and `.cursor/skills/`; local user skills also load from
`~/.agents/skills/`.

Personal Context canon stays in
[`Madgeniusblink/context`](https://github.com/Madgeniusblink/context) (private).
**dot-agents** is the shareable skeleton, [standards](.agents/standards/) contract, and installer for both layers.

- **Upstream:** [`.agents/context/upstream.json`](.agents/context/upstream.json) — single source for repo URLs; **`dot-agents upstream`** prints current links from the installed CLI.

---

## Extend (upstream)

**Upstream** is the canonical template repo ([Madgeniusblink/dot-agents](https://github.com/Madgeniusblink/dot-agents)). **Overlay** is what you add locally (skills, memory, scratch).

1. Read [`.agents/standards/extending.md`](.agents/standards/extending.md).
2. Run **`dot-agents upstream`** for repo, raw, and tree links (always matches the CLI you installed).
3. Add artifacts per [standards/](.agents/standards/); open PRs on **upstream** to change the shared template.

Do not hard-code GitHub URLs in your overlay; link via `dot-agents upstream --json` or copy from its output.

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

## Shipped tree (`~/.agents/`)

The tree below is **generated from `.agents/`** in this repo. When you add or remove files, run:

```sh
dot-agents readme sync    # from repo root (after cargo install --path cli)
dot-agents readme check   # CI / pre-push
```

Install copies this layout to `$HOME/.agents` (excluding repo-only `STRUCTURE.md`):

```sh
dot-agents install
```

<!-- dot-agents-tree:start -->
```
~/.agents/
├── AGENTS.md
├── checklists/
│   ├── before-commit.md
│   └── before-release.md
├── context/
│   ├── architecture.md
│   ├── data-locations.md
│   ├── foundation.md
│   ├── glossary.md
│   ├── optional-extensions.md
│   ├── sources-policy.md
│   ├── style-guide.md
│   ├── upstream.json
│   └── writing-standard.md
├── decisions/
│   └── 0001-reference-context-folder.md
├── memory/
│   ├── MEMORY.md
│   ├── lessons.md
│   └── preferences.md
├── rules/
│   └── README.md
├── skills/
│   ├── README.md
│   └── self-verify/
│       ├── SKILL.md
│       ├── assets/
│       │   └── done-report-template.md
│       ├── references/
│       │   └── checklist-map.md
│       └── scripts/
│           └── open-checklist.sh
├── standards/
│   ├── README.md
│   ├── checklists.md
│   ├── decisions.md
│   ├── evals.md
│   ├── extending.md
│   ├── install.md
│   ├── lint.md
│   ├── rules.md
│   ├── schedules.md
│   ├── skills.md
│   ├── templates.md
│   ├── var-layout.md
│   └── workflows.md
├── templates/
│   └── pr-description.md
└── var/
    ├── PROJECT-GITIGNORE-SNIPPET.md
    ├── README.md
    ├── scratch/
    │   ├── .gitignore
    │   └── README.md
    └── worktrees/
        ├── .gitignore
        └── README.md
```

Same layout in this repo under [`.agents/`](.agents/). Repo-only file `.agents/STRUCTURE.md` is not installed to `~/.agents/`.
<!-- dot-agents-tree:end -->

**Not in base template:** hooks (use `.cursor/hooks/` or provider paths), profiles, evals, routing JSON — see [optional-extensions.md](.agents/context/optional-extensions.md). **`var/scratch/` and `var/worktrees/`** ship with `.gitignore` so ephemeral files never commit — see [data-locations.md](.agents/context/data-locations.md).

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
