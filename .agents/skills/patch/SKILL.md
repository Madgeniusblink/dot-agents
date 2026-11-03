---
name: patch
description: Author file changes as an anchored Python patch script when interactive editing is blocked or cannot be replayed. Use for classifier-gated edits, guard-parsed shells, and cross-machine handoffs.
---

# patch

Some edits cannot go through the normal edit tool. A classifier refuses
agent-authored guardrail changes. A shell guard lexes the whole command and
rejects a heredoc over one stray quote. A change described in chat cannot be
replayed by another session. Route those through one anchored Python script.

## When

- The change is gated and must travel to a channel that has authority: a direct
  user paste, an unsandboxed session, or a spec issue.
- The edit must apply identically on more than one machine or session.
- The surrounding UI makes hand-editing lossy: approval prompts, flaky editors,
  remote shells.

Not for ordinary edits. The edit tool on a file you have read is simpler and
just as exact.

## Shape

```python
python3 - <<'PYEOF'
import pathlib

def patch(path, old, new, tag):
    p = pathlib.Path(path)
    s = p.read_text()
    assert old in s, "anchor missing: " + tag
    p.write_text(s.replace(old, new, 1))
    print("ok", tag)

patch("bin/tool", "exact current text", "exact replacement", "short-tag")
print("applied")
PYEOF
```

## Rules

- Copy every anchor verbatim from the current file. Never retype one.
- Assert before writing. A missing anchor must stop the script and name its
  tag. Partial application is worse than none.
- Replace once unless the duplication is the point.
- Write no stray quote characters in shell-carried text. A guard that lexes the
  whole command reads an apostrophe inside a heredoc as an unbalanced quote and
  refuses it before it runs.
- Print one line per patch and a final sentinel.
- Do not assume idempotence. A second run fails on the consumed anchor, and
  that failure is correct.

## After

The script edits files. It does not ship them. The applying channel still runs
the repository's gates, commits, and opens the PR. A patch script that bypasses
review is a defect, not a convenience.
