# Extending this tree

## Words we use

| Term | Meaning |
|------|---------|
| **Upstream** | The canonical template repo ([upstream.json](upstream.json)). You install from upstream; you do not fork the meaning of `standards/` without a decision. |
| **Overlay** | Your files beside upstream: extra skills, `memory/`, scratch, custom tools. Default `dot-agents install` keeps overlays. |
| **Extend** | Add artifacts using [standards/](.) — skills, rules, checklists — without replacing upstream files unless you intend to. |

Run **`dot-agents upstream`** anytime for current repo URLs (embedded at CLI build; matches [upstream.json](upstream.json) in git).

## Before you add anything

1. Read [sources-policy.md](../context/sources-policy.md) — link to live docs first.
2. Read the standard for that artifact (skills, rules, var layout, etc.).
3. Prefer **project** `.agents/` for repo-specific work; use **global** `~/.agents/` for cross-repo memory and paths.

## Ways to relate to upstream

| Goal | Action |
|------|--------|
| One-time global home | `dot-agents install` ([install.md](install.md)) |
| Refresh template files | Re-run `install` after CLI upgrade; overlays stay |
| Track upstream in a project | `git subtree` or copy — see upstream `links.repo` via `dot-agents upstream` |
| Change template for everyone | Open a PR on **upstream**, not only in your home |
| Local-only experiment | Overlay in `~/.agents/`; record in [decisions/](../decisions/) if permanent |

## Coexistence with other tooling

Other skills or runtimes may already live under `~/.agents/`. Default install **merges**; it does not delete your extras. Only `install --mirror` removes files not in the template. See **Overlays** below and `dot-agents upstream` for the spec on upstream.

## Do not

- Paste upstream `standards/` into issues; link with `dot-agents upstream` output.
- Commit scratch or worktree contents ([var/](../var/)).
- Maintain a second copy of vendor docs; use [sources-policy.md](../context/sources-policy.md).

## Overlays

**Template-owned (refresh from upstream):** `standards/`, shipped checklists, STE100 context files, example skill, `var/` layout files.

**Yours (upstream never deletes on default install):** `memory/`, your `decisions/`, extra `skills/`, `rules/*.md`, contents of `var/scratch/` and `var/worktrees/`.

Conflict at the **same path** (two files named alike): last install wins for that path. Rename your artifact or fork upstream intentionally.
