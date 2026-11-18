---
name: owner-decision
description: Record, read, and close Owner Decision Queue rows in the workspace Slack List via the agents CLI. Use when only a named person can approve, deny, or discuss a decision; require Checked evidence links and Why you before asking.
---

# owner-decision

Human answers live in the list **Answer** field (Approve, Deny, Discuss), not in chat. Machine config holds the list id and token (never commit values). See [issue #135](https://github.com/Madgeniusblink/dot-agents/issues/135).

## Before you run

1. Confirm `[owner_decision]` in `$AGENTS_CONFIG` or `~/.config/agents/config.toml`, or env `OWNER_DECISION_LIST_ID` and `OWNER_DECISION_SLACK_TOKEN` (token needs `lists:read` and `lists:write`).
2. Set `OWNER_DECISION_SESSION` when correlating this run to a row (optional; the CLI generates one if unset).

## Workflow

### Ask (open a row)

Required: **Checked** with at least one `http://` or `https://` link, and non-empty **Why you**. Cap: refuses when ten non-Done rows exist.

```sh
agents owner-decision ask \
  --decision "Short title" \
  --recommended "Your recommendation" \
  --checked "https://example.com/evidence" \
  --why-you "Why the owner must decide" \
  --project "TRACKER-KEY"
```

Save **record_id** and **session** from stdout.

### Read (poll Answer)

Returns **Answer**, **Status**, and column fields. Does **not** return list-item comment-thread text (Lists API has no thread read).

```sh
agents owner-decision read --session "$OWNER_DECISION_SESSION"
# or: agents owner-decision read --record-id Rec…
```

Pending Answer is success: stdout shows `answer: (pending)`.

### Close (Done + proof)

Requires Answer set in the list. **--proof-link** must be an http(s) URL; merged into **Checked**; **Status** → Done.

```sh
agents owner-decision close \
  --record-id Rec… \
  --proof-link "https://example.com/proof"
```

## Rules

- Do not ask without Checked link and Why you.
- Do not close until Answer is set.
- No private list ids, workspace names, or tokens in chat, commits, or public artifacts.