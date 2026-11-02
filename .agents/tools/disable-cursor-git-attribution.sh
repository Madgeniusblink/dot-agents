#!/usr/bin/env bash
# Turn off Cursor git/PR attribution on this machine (CLI config + commit-msg hook).
set -euo pipefail

ROOT="${AGENTS_HOME:-$HOME/.agents}"
HOOK_SRC="$ROOT/hooks/commit-msg-strip-cursor-attribution.sh"
CLI_CONFIG="$HOME/.cursor/cli-config.json"

if [[ ! -f "$HOOK_SRC" ]]; then
  echo "Missing hook template: $HOOK_SRC" >&2
  echo "Install ~/.agents from dot-agents first (see README)." >&2
  exit 1
fi

HOOKS_DIR="$(git config --global core.hooksPath 2>/dev/null || true)"
if [[ -z "$HOOKS_DIR" ]]; then
  HOOKS_DIR="$HOME/.agents/context/git-hooks"
  mkdir -p "$HOOKS_DIR"
  git config --global core.hooksPath "$HOOKS_DIR"
  echo "Set core.hooksPath -> $HOOKS_DIR"
fi

install -m 755 "$HOOK_SRC" "$HOOKS_DIR/commit-msg"
echo "Installed commit-msg hook -> $HOOKS_DIR/commit-msg"

python3 <<'PY'
import json
from pathlib import Path

path = Path.home() / ".cursor" / "cli-config.json"
path.parent.mkdir(parents=True, exist_ok=True)
if path.exists():
    data = json.loads(path.read_text())
else:
    data = {"version": 1}
data.setdefault("attribution", {})
data["attribution"]["attributeCommitsToAgent"] = False
data["attribution"]["attributePRsToAgent"] = False
path.write_text(json.dumps(data, indent=2) + "\n")
print(f"Updated {path} (attribution off)")
PY

echo ""
echo "Also disable in Cursor Settings -> Git & PRs -> Attribution (IDE-only toggle)."
