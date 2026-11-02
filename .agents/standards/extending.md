# Extending this tree

## Words

| Term | Meaning |
|------|---------|
| **Upstream** | This template repo ([upstream.json](../context/upstream.json)) |
| **Overlay** | Your files beside upstream (memory, extra skills) |
| **Extend** | Add artifacts using [standards/](.) |

## Rules

1. Link live docs first ([sources-policy.md](../context/sources-policy.md)).
2. Open PRs on **upstream** to change the shared template.
3. Default install keeps overlays; use `--force` only when you intend to overwrite.

## Coexistence

If other skills or tools already live in your agent home, default install **merges**. Same path + same filename: `--force` overwrites; default install skips existing files.

Vendor names: [adapters.md](adapters.md) only.
