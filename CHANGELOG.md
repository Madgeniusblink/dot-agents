# Changelog

Template semver tracks [upstream.json](.agents/context/upstream.json) `template_version`.

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

When `.managed-refresh` changes, bump `template_version`, tag, and note here.
