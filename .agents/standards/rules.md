# Standard: rules

## Live reference (read first)

- [Cursor — Rules](https://cursor.com/docs/context/rules)

## Local minimum

Path: `.agents/rules/<topic>.md`

Required front matter for **scoped** rules:

```yaml
---
globs:
  - "**/*.ts"
---
```

Use `globs` so the rule loads only for matching paths. This saves context.

## Rules

1. One topic per file. File name: `kebab-case` or single topic word.
2. Body: short bullets. Link [style-guide](../context/style-guide.md) or live docs for detail.
3. Prose: [ASD-STE100](../context/writing-standard.md).
4. Example: [typescript.md](../rules/typescript.md).

## Extend

Promote from [lessons.md](../memory/lessons.md). Do not duplicate [standards/](.) text in every rule.
