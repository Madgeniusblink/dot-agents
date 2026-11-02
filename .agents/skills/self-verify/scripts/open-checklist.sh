#!/usr/bin/env bash
set -euo pipefail
ROOT="${AGENTS_HOME:-$HOME/.agents}"
exec "${PAGER:-cat}" "$ROOT/checklists/before-commit.md"
