#!/usr/bin/env sh
# The template repository's checks, one exit code: the home's own checks on
# home/, the links in the root docs, vendor neutrality, and the merge tiers.
# A home runs home/scripts/check.sh alone; the rest are template rules.
DIR="$(cd "$(dirname "$0")" && pwd)"
status=0
sh "$DIR/../home/scripts/check.sh" || status=1
python3 "$DIR/../home/scripts/check-links.py" "$DIR/.." || status=1
sh "$DIR/check-neutrality.sh" context || status=1
sh "$DIR/check-neutrality.sh" vendors || status=1
sh "$DIR/merge-tier-test.sh" || status=1
exit "$status"
