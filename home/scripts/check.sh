#!/usr/bin/env sh
# Run the checks that apply to any agent home. One exit code.
# With the agents binary on PATH, agents check owns every rule, the always-on
# byte cap included. Without it, the shell checks run without that cap.
# Vendor neutrality is a template-repository rule, so CI runs check-neutrality.sh
# instead: a home carries the user's own files, and those may name any tool.
DIR="$(cd "$(dirname "$0")" && pwd)"
if command -v agents >/dev/null 2>&1; then
  exec agents --home "$DIR/.." check
fi
status=0
sh "$DIR/check-budget.sh" || status=1
python3 "$DIR/check-links.py" || status=1
exit "$status"
