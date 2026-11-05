#!/usr/bin/env sh
# Run the checks that apply to any agent home. One exit code.
# Vendor neutrality is a template-repository rule, so CI runs check-neutrality.sh
# instead: a home carries the user's own files, and those may name any tool.
DIR="$(cd "$(dirname "$0")" && pwd)"
status=0
sh "$DIR/check-budget.sh" || status=1
python3 "$DIR/check-links.py" || status=1
exit "$status"
