#!/usr/bin/env sh
# Context budget: router <= 50 lines, one line per rule, skill <= 120 lines,
# every correction dated with its retirement test.
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
status=0

n=$(wc -l < AGENTS.md | tr -d ' ')
if [ "$n" -gt 50 ]; then
  echo "AGENTS.md: $n lines (max 50). Move detail to a linked file."
  status=1
fi

# A rule is one line: no wrapped or indented continuation, no multi-line paragraph.
for f in rules/*.md; do
  [ -e "$f" ] || continue
  awk -v f="$f" '
    /^[ \t]+[^ \t]/ { print f ":" NR ": rule continues on an indented line"; bad=1 }
    /^[^ \t#-]/ && prev ~ /^[^ \t#-]/ { print f ":" NR ": paragraph spans more than one line"; bad=1 }
    { prev = $0 }
    END { exit bad }
  ' "$f" || status=1
done

# A correction carries its death condition: `YYYY-MM · task that proves it`.
if [ -f rules/corrections.md ]; then
  awk '/^- / && $0 !~ /`[0-9][0-9][0-9][0-9]-[0-9][0-9] · [^`]+`$/ { print "rules/corrections.md:" NR ": correction lacks `YYYY-MM · test task`"; bad=1 } END { exit bad }' rules/corrections.md || status=1
fi

# A skill is <= 120 lines and carries frontmatter whose name is its directory.
for f in skills/*/SKILL.md; do
  [ -e "$f" ] || continue
  n=$(wc -l < "$f" | tr -d ' ')
  if [ "$n" -gt 120 ]; then
    echo "$f: $n lines (max 120). Move detail to references/."
    status=1
  fi
  dir=$(basename "$(dirname "$f")")
  name=$(awk 'NR==1 && $0 != "---" { exit } NR>1 && $0 == "---" { exit } /^name:/ { gsub(/^name:[ \t]*|["'"'"']/, ""); print; exit }' "$f")
  if [ "$name" != "$dir" ]; then
    echo "$f: frontmatter name is [$name], directory is [$dir]."
    status=1
  fi
  grep -q '^description:' "$f" || { echo "$f: frontmatter has no description. A tool finds a skill by it."; status=1; }
done

for d in skills/*/; do
  [ -e "$d/SKILL.md" ] || { echo "$d: no SKILL.md."; status=1; }
done

[ "$status" -eq 0 ] && echo "budget OK"
exit "$status"
