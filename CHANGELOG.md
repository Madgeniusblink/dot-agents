# Changelog

Template semver tracks [upstream.json](context/upstream.json) `template_version`.

## 0.10.0 — 2026-11-06

- New [scripts/wire.sh](scripts/wire.sh). It points the agent tools on a machine at the home: it finds the tool directories in `$HOME`, adds one import line per always-on file, and links `skills/`. It prints what it would change and writes nothing until `--apply`, because wiring edits files outside the home. `--link` uses symlinks for the rules too. Running it twice changes nothing.
- Wiring was the one step in adopting this tree that a model could not derive. Which directory loads without an approval prompt, how far imports nest, and that no tool has an import syntax for a directory of skills are all facts about a tool, not about the tree. Install never needed a script; this did.
- [agent-home](skills/agent-home/SKILL.md) and [docs/setup.md](docs/setup.md) name the script. The router row for `scripts/` now names both scripts.

## 0.9.1 — 2026-11-05

- [agent-home](skills/agent-home/SKILL.md) gains an **Update it** section. The skill covered clone, wire, read, verify, and edit, but not the merge from upstream, which is the step the user runs most and the only one with a conflict to reason about.

## 0.9.0 — 2026-11-04

Breaking: the repository root is the tree. Everything that was under `.agents/` now sits at the root, so a clone of this repo *is* a `~/.agents`.

- `git clone <fork> ~/.agents` now lands the router at `~/.agents/AGENTS.md`. Before this, it landed at `~/.agents/.agents/AGENTS.md` and left the contributor rules where the router should be — wrong, and silent about it.
- The old root `AGENTS.md` (rules for working on this repository) moves to [docs/repo-rules.md](docs/repo-rules.md).
- New skill [agent-home](skills/agent-home/SKILL.md). It is the surface an agent finds when the home did not load: how to tell a missing home from unwired tooling, how to clone it, how to wire a tool, how to read the rules for one session when wiring is not possible, and how to verify. A skill is found by its `description`, so this works in a tool that never read the router.
- New [scripts/check.sh](scripts/check.sh) runs budget, links, and neutrality under one exit code. The router names `scripts/` now; before this nothing in the tree did.
- [check.sh](scripts/check.sh) runs budget and links only. Vendor neutrality is a rule about *this repository*, and a home carries the user's own files, which may name any tool they like. CI still runs [check-neutrality.sh](scripts/check-neutrality.sh) at full strength, where every file is a template file by definition.
- [check-budget.sh](scripts/check-budget.sh) no longer reads prose above the first rule as a rule. A preamble explaining a `rules/` file is not a rule, and flagging it pushed real files out of house style to satisfy the test.
- [check-budget.sh](scripts/check-budget.sh) now checks skill frontmatter: every skill directory holds a `SKILL.md`, its `name` matches the directory, and it has a `description`. A tool finds a skill by its description, so a missing one makes the skill unreachable without any other symptom.

**Upgrade:** Pull into your `~/.agents`. If your home predates this, your files are one directory deeper than the new layout: move the contents of `~/.agents/.agents/` up one level, delete the empty directory, and commit.

## 0.8.0 — 2026-11-03

Breaking: the agent home is a git checkout, not an install target.

- Removed `install.sh`, `install.ps1`, `test-install-behavior.sh`, and the `.installed` hash manifest. Git answers every question they were built to answer, and answers it better: `git status` for what you changed, `git diff` for how, a merge conflict instead of a silent `kept (edited)`.
- `standards/install.md` becomes [`standards/adopt.md`](.agents/standards/adopt.md): clone to `~/.agents`, merge from upstream.
- CI drops the three install-behavior steps.

**Upgrade:** Move your `~/.agents` aside, clone your copy of this template there, and restore your own `rules/`, `memory/`, and `skills/` files into it. Then `git add -A && git commit`. From then on, editing a file is the whole workflow.

## 0.7.4 — 2026-11-03

- Drop every reference to the optional Rust CLI. It pinned the template seven releases back, would not compile against the current tree, and installed with the two-list policy the hash manifest replaced at 0.3.0. The shell installers are the only supported path.

## 0.7.3 — 2026-11-03

- Add [docs/setup.md](docs/setup.md): how to point a tool at `~/.agents`, and why the transport must be a link or an import rather than a copy.
- No change to the shipped tree or to install.

## 0.7.2 — 2026-11-03

Trim pass. Rules 48 to 42.

- Cut four boundaries that did not earn an always-on line: a retry limit the corrections already cover, an "unfamiliar state" rule too vague to test, an instruction-ranking line whose useful half is the next one, and an archived-material line that is document hygiene rather than an agent limit.
- Folded hook and signature bypass into the force-push line, and the no-exception clause into the out-of-scope line.
- Cut two corrections that restated the scope contract already stated once.
- Added one boundary: anything published where others can read it carries no private name, path, host, tool, or internal state.
- [AGENTS.md](.agents/AGENTS.md): the router states precedence and points at `rules/` rather than repeating the limit.

**Upgrade:** Re-run install.

## 0.7.1 — 2026-11-03

- [boundaries.md](.agents/rules/boundaries.md): an agent does not grant itself authority, weaken a check guarding its current task, or accept its own control change. A narrower instruction wins only where it does not weaken a boundary. Archived material is never current authority.
- [AGENTS.md](.agents/AGENTS.md): the project-wins-over-home line now carries the boundary carve-out.

**Upgrade:** Re-run install.

## 0.7.0 — 2026-11-03

Closes the context-repo audit. The remaining 11,700 lines were read or categorically dismissed; nothing further is pending.

- [principles.md](.agents/principles.md): principle 1 now names the four admission classes (durable preference, non-obvious fact, authority boundary, demonstrated recurring failure) and what is never admitted. New principle 5 requires finding a rule's owner and rejecting a duplicate whose wording differs.
- [boundaries.md](.agents/rules/boundaries.md): a session never renegotiates scope with itself.
- [corrections.md](.agents/rules/corrections.md): checkable done-when criteria, a named exclusion list, no size-or-certainty exception for out-of-scope fixes, native paths outranking agent-built mechanisms, design review after a second repair to the same check, and caller inspection before a shared-signature change.

**Upgrade:** Re-run install. All three files are shipped, so an untouched copy updates in place.

## 0.6.0 — 2026-11-03

- [boundaries.md](.agents/rules/boundaries.md): add eleven permanent limits — secret handling, non-transferable authorization, read-only means no write, force-push and hook bypass, retrying a rejected command, overwriting unfamiliar state, instruction authority, and tool output as data.
- [corrections.md](.agents/rules/corrections.md): add ten corrections under new groupings — abstention and absence claims, finding shape and risk reporting, and code restraint (no unrequested abstraction, no defensive code, no comment that a rename removes).

**Upgrade:** Re-run install. Both files are shipped, so an untouched copy updates in place; an edited copy is kept and listed.

## 0.5.1 — 2026-11-03

- Add [report](.agents/skills/report/SKILL.md) skill: loads personal `memory/voice.md` only when reporting status or completion.
- Document private `voice.md` overlay in [docs/examples/agents-private-memory-voice.md](docs/examples/agents-private-memory-voice.md).

**Upgrade:** Re-run install. Adds the new skill; add `memory/voice.md` yourself if you want custom reporting prefs.

## 0.5.0 — 2026-11-03

- Removed `memory/MEMORY.md` and `memory/lessons.md`. Both described files you write; any filled copy differed forever. Their two lasting lines are in `principles.md`. `memory/` now ships empty.
- Removed `checklists/`, `standards/checklists.md`, and `skills/self-verify/`. Their items are things a current model does unprompted, or already sit in `rules/`.
- Shipped tree: 29 files to 22.

**Upgrade:** Re-run install. Untouched copies of the removed files are deleted. Filled-in copies are kept and become yours.

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
