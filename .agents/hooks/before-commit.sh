#!/usr/bin/env bash
# Wire this from your provider or run manually before commit.
set -euo pipefail
ROOT="${AGENTS_HOME:-$HOME/.agents}"
echo "=== before-commit checklist ==="
cat "$ROOT/checklists/before-commit.md"
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git diff --stat
  git status -sb
fi
