#!/usr/bin/env sh
# Test scripts/merge-tier.sh over fixture path lists. Also check that
# .github/CODEOWNERS lists every red pattern and that every pattern names a
# path in this checkout. Prints one line per fault and exits 1 on any.
set -f
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 1
fail=0

expect() {
  want="$1"; shift
  got=$(printf '%s\n' "$@" | sh scripts/merge-tier.sh)
  case "$got" in
    "$want"*) ;;
    *) echo "merge-tier: want $(echo "$want" | head -n1), got $(echo "$got" | head -n1), for: $*"; fail=1 ;;
  esac
}

expect green
expect green README.md docs/setup.md cli/src/status.rs memory/x.md skills/a/SKILL.md rules/corrections.md
expect "yellow
scripts/wire.sh (scripts/*)" docs/setup.md scripts/wire.sh
expect yellow cli/Cargo.lock
expect yellow .github/workflows/check.yml
expect yellow context/upstream.json
expect "red
.github/CODEOWNERS (.github/CODEOWNERS)" scripts/wire.sh .github/CODEOWNERS
expect red .github/workflows/review.yml
expect red rules/boundaries.md docs/setup.md

tmp=$(mktemp)
printf 'red:\n  - README.md\n' > "$tmp"
got=$(echo README.md | MERGE_TIERS="$tmp" sh scripts/merge-tier.sh | head -n1)
[ "$got" = red ] || { echo "merge-tier: an edited tier file did not change the tier"; fail=1; }
rm -f "$tmp"

files=$(git ls-files)
for pat in $(sh scripts/merge-tier.sh --list red); do
  grep -q "^/$pat " .github/CODEOWNERS || { echo ".github/CODEOWNERS: no line for red path $pat"; fail=1; }
done
for owned in $(grep -v '^#' .github/CODEOWNERS | awk 'NF { sub(/^\//, "", $1); print $1 }'); do
  sh scripts/merge-tier.sh --list red | grep -qxF "$owned" || { echo ".github/CODEOWNERS: $owned is not a red path"; fail=1; }
done
for pat in $(sh scripts/merge-tier.sh --list red) $(sh scripts/merge-tier.sh --list yellow); do
  found=0
  for f in $files; do
    case "$f" in $pat) found=1; break ;; esac
  done
  [ "$found" = 1 ] || { echo ".github/merge-tiers.yml: $pat names no path here"; fail=1; }
done

[ "$fail" = 0 ] && echo "merge tiers OK"
exit "$fail"
