# Changelog

Template semver tracks [upstream.json](.agents/context/upstream.json) `template_version`.

## 0.4.0 — 2026-11-02

- `rules/behavior.md` split by decay clock: `rules/boundaries.md` (what needs a human; never expires) and `rules/corrections.md` (what the model misses; expires).
- Corrections rewritten as outcomes, not procedures. Each ends with the month added and the task that proves it.
- CI rejects a correction without `` `YYYY-MM · task` ``.
- New principle: state the outcome, not the procedure.
- The template no longer ships empty files. Removed `context/architecture.md`, `context/glossary.md`, `context/style-guide.md`, and `memory/preferences.md`: each existed to be filled, so every copy differed forever and showed as edited on every install. The two style-guide lines moved to `principles.md`.

**Upgrade:** Re-run install. An untouched `rules/behavior.md` is removed and the two new files are added. If you edited it, install keeps it and lists it; move your lines to `rules/preferences.md` and delete it. Your filled-in copies of the four removed files are kept and become yours.

## 0.3.0 — 2026-11-02

- Install keeps a hash manifest (`.installed`). It replaces a file only if you have not touched it, keeps and lists every file you edited, and removes unchanged files the template stopped shipping.
- Removed `.user-owned` and `.managed-refresh`. The manifest measures what you edited instead of guessing.
- `AGENTS.md` is a router of links. No `AGENTS.local.md`: your additions go in `rules/`, `memory/`, and `skills/`.
- Every rule in `rules/behavior.md` is one line.
- CI budget check: `AGENTS.md` at 50 lines or fewer, one line per rule, `SKILL.md` at 120 lines or fewer.

**Upgrade:** No action needed. The first install under 0.3.0 has no manifest, so it keeps every existing file that differs from 0.3.0 and lists it. Delete a listed file you never edited to take the current version. If you used `AGENTS.local.md` in 0.2.0, move its lines into a file in `rules/`.

## 0.2.0 — 2026-11-02

- Router names folders, not files. Every `rules/*.md` loads each session; `memory/`, `templates/`, and `skills/` are listed, not indexed. Optional `AGENTS.local.md` holds your router additions.
- New shipped files: `principles.md`, `rules/behavior.md`, `templates/handoff.md`, `templates/brief.md`, `skills/patch/`.
- `rules/` is the primary surface; skills are the exception.
- `.managed-refresh` now includes `AGENTS.md`, `principles.md`, `rules/behavior.md`, `templates/`, and shipped skills.
- Removed `standards/adapters.md`. No shipped file names a vendor.
- Removed `rules/README.md` (the router loads every file in `rules/`); guidance moved to `standards/rules.md`.

**Upgrade from 0.1.x:** Re-run install **replaces** `AGENTS.md`. Before you upgrade, move your own `AGENTS.md` lines into `AGENTS.local.md` beside it. Delete an old `rules/README.md` from your home if present.

## 0.1.2 — 2026-11-02

- Remove shipped `templates/` folder; PR shape uses `.github/pull_request_template.md`.
- CHANGELOG corrects 0.1.0 install behavior; README shows full clone command.

## 0.1.1 — 2026-11-02

- Install policy lists: `.user-owned`, `.managed-refresh` (single source for sh/ps1/CLI).
- Default install no longer overwrites edited `AGENTS.md`, checklists, or personal context files.
- Removed `--force`. Fixed neutrality script exit code and Windows CI assertions.

**Upgrade from 0.1.0:** Re-run `./install.sh` once on 0.1.1+. If you edited `AGENTS.md`, checklists, or personal context under 0.1.0, those edits may have been overwritten by a re-run on 0.1.0; restore from backup if needed. 0.1.1 protects those paths.

## 0.1.0 — 2026-11-02

- Initial public template (`v0.1.0` tag).
- Re-run install **overwrote all existing files** except later policy work; do not use 0.1.0 install behavior for edited trees (upgrade to 0.1.1).
- OSS: CONTRIBUTING, SECURITY, issue/PR templates, root AGENTS.md, admission test, multi-OS CI.
- CLI moved to [dot-agents-cli](https://github.com/Madgeniusblink/dot-agents-cli) (see `links.cli_install`).

When install behavior or shipped files change, bump `template_version`, tag, and note here.
