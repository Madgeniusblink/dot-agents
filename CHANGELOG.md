# Changelog

Template semver tracks [upstream.json](.agents/context/upstream.json) `template_version`.

## 0.1.1 — 2026-11-02

- Install policy lists: `.user-owned`, `.managed-refresh` (single source for sh/ps1/CLI).
- Default install no longer overwrites edited `AGENTS.md`, checklists, or personal context files.
- Removed `--force`. Fixed neutrality script exit code and Windows CI assertions.

## 0.1.0 — 2026-11-02

- Initial public template (`v0.1.0` tag).
- Install policy: `.user-owned` and `.managed-refresh` lists; no `--force`.
- Managed refresh paths: `standards/`, `context/foundation.md`, `context/sources-policy.md`, `context/writing-standard.md`, `context/optional-extensions.md`, `context/upstream.json`.
- User-owned prefixes: `memory/`, `decisions/`.
- OSS: CONTRIBUTING, SECURITY, issue/PR templates, root AGENTS.md, admission test, multi-OS CI.
- CLI moved to [dot-agents-cli](https://github.com/Madgeniusblink/dot-agents-cli) (see `links.cli_install`).

When `.managed-refresh` changes, bump `template_version`, tag, and note here.
