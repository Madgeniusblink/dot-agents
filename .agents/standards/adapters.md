# Tool adapters

Vendor-specific paths and formats live **only in this file**. Other docs say "your tool."

## Skills discovery

| Tool | Document |
|------|----------|
| Cursor | [cursor.com/docs/skills](https://cursor.com/docs/skills) |
| agents.md convention | [agents.md](https://agents.md/) |

Project skills: `.agents/skills/<id>/SKILL.md`. Some tools also read `.cursor/skills/`.

## Scoped rules (optional)

Some tools support path-scoped rules with YAML front matter, for example:

```yaml
---
globs:
  - "**/*.ts"
---
```

Cursor documents this under [Rules](https://cursor.com/docs/context/rules). If your tool does not use `globs`, ignore this block or map it in your adapter config.

## Hooks

| Tool | Typical hook location |
|------|------------------------|
| Cursor | Project `.cursor/hooks/` |
| Git | `core.hooksPath` or repo `.git/hooks` |

Do not commit hook scripts into this template unless the whole team agrees.

## User rules bootstrap

Point your tool’s **user rules** at the installed copy of this file:

- Global home: `<agent-home>/AGENTS.md`
- Project: `<repo>/.agents/AGENTS.md` plus optional root `AGENTS.md`

Load scratch or worktree paths only if **you** define them in optional extensions.

## Cloud vs local

Tool docs change often. Link out; do not copy vendor manuals into this repo ([sources-policy.md](../context/sources-policy.md)).
