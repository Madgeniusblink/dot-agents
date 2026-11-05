#!/usr/bin/env sh
# Run every check in this directory. One exit code.
DIR="$(cd "$(dirname "$0")" && pwd)"
status=0
sh "$DIR/check-budget.sh" || status=1
python3 "$DIR/check-links.py" || status=1
sh "$DIR/check-neutrality.sh" context || status=1
sh "$DIR/check-neutrality.sh" vendors || status=1
exit "$status"
