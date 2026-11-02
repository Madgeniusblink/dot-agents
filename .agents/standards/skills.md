# Standard: skills

## Live reference (read first)

- [Cursor — Agent Skills](https://cursor.com/docs/skills)
- [AGENTS.md](https://agents.md/) — repository entry pattern

## Local minimum (when you add a skill)

Path: `.agents/skills/<skill-id>/SKILL.md`

Required front matter:

```yaml
---
name: <skill-id>
description: One line. State when the agent must load this skill.
---
```

Optional directories:

| Directory | Use |
|-----------|-----|
| `references/` | Links and short notes; prefer URLs over pasted docs |
| `assets/` | Files the skill copies or fills |
| `scripts/` | Executable helpers |

## Rules

1. One skill per directory. Use `kebab-case` for `<skill-id>`.
2. The `description` field is the **trigger**. Write it for discovery, not marketing.
3. Do not paste vendor manuals. Link them in `references/`.
4. Body text: [ASD-STE100](../context/writing-standard.md).
5. Example in this repo: [self-verify](../skills/self-verify/SKILL.md).

## Extend

Add a skill only when a procedure repeats. Promote from [memory/lessons.md](../memory/lessons.md) after the same mistake three times.
