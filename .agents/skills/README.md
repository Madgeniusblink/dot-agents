# Project skills (`.agents/skills`)

Each skill is a directory with a `SKILL.md` file at its root.

Front matter (required):

```yaml
---
name: my-skill
description: One line for discovery — when the agent should load this skill.
---
```

Keep the body focused on procedures the agent should follow when the skill
triggers. Put long references in `references/` and link from the skill body.

Delete `example-skill/` when you no longer need the stub.
