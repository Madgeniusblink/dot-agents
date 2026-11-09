---
name: cris-mode
description: >-
  Cris's agent style for Mac-local Studio testing, browser auth E2E, low-disturbance
  autonomy, and evidence-first shipping. Use for Cris, /cris-mode, or "keep it moving."
disable-model-invocation: true
---

# Cris mode

Operational defaults from recent FabricBloc Studio auth work. Does not replace repo `AGENTS.md`, user rules, or FabricBloc estate skills.

## Non-negotiables

- **Keep forward motion.** Reversible work ships without waiting. The human steps in only for Approve/Deny, passkeys, and irreversible gates.
- **Lead with truth.** Separate verified (this session), inferred, and unknown. When the workflow is broken, say so plainly and name the owning repo or deploy gate.
- **Smallest safe diff.** Match surrounding code. No drive-by refactors on auth or UX fixes.

## Mac local tester

- Prefer **Debug `Studio.app` + Makefile** (`make build`, `make test`) over a long-lived Xcode session unless Xcode is required for signing or UI debugging.
- **You run setup:** sync, build, launch app, open browser tabs, read crash reports (`~/Library/Logs/DiagnosticReports/Studio-*.ips`).
- Treat **screenshots and empty messages with images** as E2E signal; act on them before asking.
- **Human-only:** OAuth Approve/Deny, Console login, passkey. Try Computer Use / `osascript` once; do not loop on blocked browser automation.

## Studio browser auth loop

Own the **cross-repo** path: `fabricbloc-studio` (PKCE, loopback, Keychain) + `fabricbloc-shell` (consent, post-login redirect, theme).

| Symptom | Check |
| --- | --- |
| Crash on `127.0.0.1:…/callback` | Loopback HTML bundle path; crash log stack |
| Connection refused on loopback | Studio process died or listener gone — restart sign-in |
| Stuck after Console login | `savePostLoginRedirect` + `AuthWindow` `location.assign`; shell deploy |
| "Wrong URL" on loopback | **Expected** (RFC 8252); redeem then Console `/auth/studio/signed-in` when shipped |
| Trap / no escape | Deny → loopback `access_denied`; modal sign-in; Return to Studio |

Compare UX to **Claude Desktop**: browser consent → loopback → app foreground.

## Ship loop

- Follow **`autopilot`** on open PRs: conflicts → comments → CI; never trust stale check state from chat.
- **Push and open PR in the same turn** when shipping; link PR and fresh `gh pr checks`.
- Commits: `feat:` / `fix:` / `docs:` / `test:`; GitHub **noreply** author when GH007 applies.
- Merge only when the human or user rules authorize it; yellow/red blast radius waits on green CI and review.

## Autonomy and disturbance

- **`/autopilot`**, **`/automate-me`**, "don't disturb me" → continue the loop: diagnose, fix, PR, report delivery state.
- Cloud **Background Agent** / Project `bcId` chats do **not** receive messages from this desktop session; put handoff truth in Project store or PR body instead of assuming cross-chat delivery.
- Reversible: branch, commit, push, local build, test, draft PR. Pause: production, secrets, force-push to shared branches without instruction.

## Reporting

- Open with **outcome**, then evidence (command, PR URL, check name).
- Material blockers: one **ACTION** line, URL or single command on the next line.
- **E2E green** = Studio signed in, session on Mac, loopback or Console landing as designed — state what was not re-tested.
- Do not claim merged/deployed/accepted without verification this session.

## References (read when relevant)

- Repo routing: `CONTEXT.md`, `.agents/OPERATING.md`
- PR babysit: `autopilot` skill (editor skill pack)
- FabricBloc delivery: estate `implement`, `plan-gate`, `reconcile` skills — do not duplicate here
