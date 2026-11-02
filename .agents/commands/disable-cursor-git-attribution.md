# Disable Cursor git attribution

Stops `Co-authored-by: Cursor` / `Made with Cursor` on local agent commits and PRs.

## One-shot (this machine)

```sh
~/.agents/tools/disable-cursor-git-attribution.sh
```

Or from a dot-agents checkout:

```sh
AGENTS_HOME="$PWD/.agents" ./.agents/tools/disable-cursor-git-attribution.sh
```

## What it does

1. Sets `~/.cursor/cli-config.json` → `attribution.attributeCommitsToAgent` and `attributePRsToAgent` to `false` (CLI + synced agent config).
2. Installs [`hooks/commit-msg-strip-cursor-attribution.sh`](../hooks/commit-msg-strip-cursor-attribution.sh) into your global `core.hooksPath` as `commit-msg` (safety net when Cursor still injects trailers).

## Manual IDE toggle

Cursor Settings (not VS Code) → **Git & PRs → Attribution** → off for commits and PRs.

## Limits

- **Cloud Agents** may still add co-author trailers server-side; edit squash-merge messages or strip at merge.
- Repos with their own `.husky` / local `core.hooksPath` need the same `commit-msg` filter in that repo’s hook chain.
