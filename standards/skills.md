# Standard: skills

Format and discovery: your tool's current docs. Prose: [ASD-STE100](../context/writing-standard.md).

Rules come first. A skill is the exception: use one only when a procedure has steps whose order the model cannot derive ([principles.md](../principles.md)).

Example in this repo: [patch](../skills/patch/SKILL.md).

## Where a skill goes

A skill lives with its owner, and only there.

| The skill is | It lives in | Who may move it |
|---|---|---|
| yours, used in any repository | your home, `skills/` | you, with `agents skills` |
| a project's, used only in that repository | the project's checkout, in the tool's project folder | the project's team |
| a tool's or a service's, shipped to you | the tool's own store | the tool |

A copy of a project's skill in your tool directory is the project's to make and to refresh. When a project ships copies, it marks each one with `metadata.source` and `metadata.version` in the frontmatter, so a scanner can name the origin and say when the copy is behind. `agents skills` reports every copy it can see and never moves one the project owns.

## Extend

Add a skill only when a procedure repeats. Promote a lesson logged in `memory/` after the same mistake three times.
