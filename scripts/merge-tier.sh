#!/usr/bin/env sh
# Print a PR's merge tier from its changed paths, one path per line on stdin.
# Line 1 is green, yellow, or red. Each later line is a path that set the tier
# and the pattern it matched in .github/merge-tiers.yml, or $MERGE_TIERS.
# With --list <tier>, print that tier's patterns instead.
set -ef
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TIERS="${MERGE_TIERS:-$ROOT/.github/merge-tiers.yml}"

patterns() {
  awk -v t="$1:" '/^[a-z]+:/ { s = $1; next } s == t && /^ *- / { sub(/^ *- */, ""); print }' "$TIERS"
}

if [ "$1" = "--list" ]; then
  patterns "$2"
  exit 0
fi

paths=$(cat)
for tier in red yellow; do
  pats=$(patterns "$tier")
  hits=$(printf '%s\n' "$paths" | while IFS= read -r p; do
    [ -n "$p" ] || continue
    for pat in $pats; do
      case "$p" in ($pat) echo "$p ($pat)"; break ;; esac
    done
  done)
  if [ -n "$hits" ]; then
    echo "$tier"
    echo "$hits"
    exit 0
  fi
done
echo green
