#!/usr/bin/env sh
# Run the checks that apply to any agent home. One exit code.
# agents check owns every rule and budget. Without the binary, no other script
# measures them, so this check fails instead of passing on part of the rules.
# Vendor neutrality is a template-repository rule, so CI runs check-neutrality.sh
# instead: a home carries the user's own files, and those may name any tool.
DIR="$(cd "$(dirname "$0")" && pwd)"
if command -v agents >/dev/null 2>&1; then
  exec agents --home "$DIR/.." check
fi
echo "check: agents is not on PATH, so no rule or budget was measured."
echo "fix: cargo install --git https://github.com/Madgeniusblink/dot-agents dot-agents"
exit 1
