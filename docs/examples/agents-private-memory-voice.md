# Example: `memory/voice.md` in a private overlay

Keep personal reporting prefs out of the public template. Maintain them in a
private repo (for example `agents-private`) and install into `~/.agents`.

## File: `.agents/memory/voice.md`

```markdown
# Voice

## How to report to me

- Give directions, not options. Decide, state the decision, then one line of why.
- Do not lay out alternatives for me to choose between unless I ask for them.
- Put the reason after the directive, never before it.
- Keep any closing offer to one line.
- Cut reasoning I did not ask for. Show the result, not the path to it.
```

## Workflow

1. Commit `memory/voice.md` in your private repo (only the tree you own:
   `memory/`, `rules/`, `decisions/`, extra `skills/`).
2. Clone that private repo to `~/.agents`. It is your agent home, not a copy
   of one.
3. Commit the change. The home is the checkout, so it is live at once.

## Token efficiency

- [AGENTS.md](../../.agents/AGENTS.md) does **not** load `voice.md` every session.
- [report skill](../../.agents/skills/report/SKILL.md) loads `voice.md` only when
  the model is reporting status or completion.
- Do **not** duplicate these lines in Cursor user rules unless you need a host
  that never reads skills.

Optional one-line user rule for Cursor:

```markdown
When reporting status or completion, follow ~/.agents/skills/report/SKILL.md.
```
