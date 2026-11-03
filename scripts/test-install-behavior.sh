#!/usr/bin/env sh
# Install manifest tests for install.sh. Runs against a temp copy of the template
# so a test can simulate an upstream change.
set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/tpl"
cp -R "$ROOT/install.sh" "$ROOT/.agents" "$tmp/tpl/"
H="$tmp/home"
export DOT_AGENTS_HOME="$H"
inst() { "$tmp/tpl/install.sh" > "$tmp/out"; }
fail() { echo "FAIL: $1"; cat "$tmp/out"; exit 1; }

# Fresh install writes files and a manifest.
inst
[ -f "$H/.installed" ] || fail "no manifest"
[ -f "$H/AGENTS.md" ] || fail "AGENTS.md missing"

# Clean re-run changes nothing.
inst
grep -qE '^(updated|kept|removed)' "$tmp/out" && fail "clean re-run changed something"

# User edits and additions.
echo "MY EDIT" >> "$H/AGENTS.md"
echo "USER LESSON" >> "$H/memory/lessons.md"
echo "MY RULE" > "$H/rules/mine.md"

# Upstream changes three files and drops one.
echo "UPSTREAM V2" >> "$tmp/tpl/.agents/AGENTS.md"
echo "UPSTREAM V2" >> "$tmp/tpl/.agents/memory/lessons.md"
echo "UPSTREAM V2" >> "$tmp/tpl/.agents/principles.md"
rm "$tmp/tpl/.agents/templates/brief.md"
inst
grep -q 'MY EDIT' "$H/AGENTS.md" || fail "edited AGENTS.md replaced"
grep -q 'UPSTREAM V2' "$H/AGENTS.md" && fail "edited AGENTS.md merged"
grep -q 'kept (edited): AGENTS.md' "$tmp/out" || fail "edit not reported"
grep -q 'USER LESSON' "$H/memory/lessons.md" || fail "memory edit replaced"
grep -q 'MY RULE' "$H/rules/mine.md" || fail "user rule touched"
grep -q 'UPSTREAM V2' "$H/principles.md" || fail "untouched file not updated"
grep -q 'updated: principles.md' "$tmp/out" || fail "update not reported"
[ -e "$H/templates/brief.md" ] && fail "unchanged dropped file not removed"

# Edited file stays edited on every later install.
inst
grep -q 'MY EDIT' "$H/AGENTS.md" || fail "edit lost on second install"

# Migration: home from an older install, no manifest.
M="$tmp/old"
export DOT_AGENTS_HOME="$M"
mkdir -p "$M/rules"
cp "$tmp/tpl/.agents/principles.md" "$M/principles.md"
echo "OLD EDITED ROUTER" > "$M/AGENTS.md"
echo "OLD" > "$M/rules/boundaries.md"
inst
grep -q 'OLD EDITED ROUTER' "$M/AGENTS.md" || fail "migration replaced AGENTS.md"
grep -q '^OLD$' "$M/rules/boundaries.md" || fail "migration replaced rules/boundaries.md"
echo "UPSTREAM V3" >> "$tmp/tpl/.agents/AGENTS.md"
echo "UPSTREAM V3" >> "$tmp/tpl/.agents/principles.md"
inst
grep -q 'OLD EDITED ROUTER' "$M/AGENTS.md" || fail "second install after migration replaced edit"
grep -q 'UPSTREAM V3' "$M/principles.md" || fail "identical file not adopted at migration"

echo "install behavior OK"
