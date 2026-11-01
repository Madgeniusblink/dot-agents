# dot-agents

Public template for a repository **`.agents/`** base layer: the filesystem
home for project-scoped agent skills and small adapter notes beside
[`.cursor/`](https://cursor.com/docs) and root [`AGENTS.md`](https://agents.md/).

This repo is **not** personal Context canon. That lives in
[`Madgeniusblink/context`](https://github.com/Madgeniusblink/context) (private).
Use **dot-agents** when you want a minimal, copy-friendly **project** skeleton
anyone can fork or clone.

## What you get

```
.agents/
  AGENTS.md          # Base-layer notes (routes to root AGENTS.md when you add one)
  skills/
    README.md        # How skills are laid out here
    example-skill/   # Minimal SKILL.md stub you can delete or rename
```

Cursor documents discovery of project skills under `.agents/skills/` and
`.cursor/skills/`. User-level skills also load from `~/.agents/skills/` on
local Cursor; Cloud personal sync copies `~/.cursor/skills/` only.

## Adopt into a project

**Copy (simplest)**

```sh
cp -R .agents /path/to/your-repo/
```

**Use this repo as a GitHub template**

1. Click **Use this template** on GitHub (or fork).
2. Copy the `.agents/` tree into an existing repo, or keep it at the root of a
   new repo dedicated to agent tooling.

**Git subtree (optional, for tracking upstream)**

```sh
git subtree add --prefix=.agents \
  https://github.com/Madgeniusblink/dot-agents.git main --squash
```

Then customize `.agents/AGENTS.md` and add skills under `.agents/skills/`.

## Conventions

- One skill per directory: `.agents/skills/<name>/SKILL.md` with YAML front
  matter (`name`, `description`).
- Keep root `AGENTS.md` as the short router; put durable base-layer detail in
  `.agents/AGENTS.md` so you do not bloat the root file.
- Prefer skills and CI checks over committing provider hook trees in-repo.

## License

MIT — see [LICENSE](LICENSE).
