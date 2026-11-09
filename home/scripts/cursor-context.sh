#!/usr/bin/env sh
# Cursor's sessionStart hook. ~/.cursor/hooks.json names this script; Cursor
# runs it when a session starts and adds the `additional_context` in its
# output to that session. It reads principles.md and rules/*.md as they are
# now, so an edit in the home reaches the next session with nothing rewritten
# and nothing to go stale. Output: one JSON object on stdout.
# Source: https://cursor.com/docs/agent/hooks

HOME_DIR="$(cd "$(dirname "$0")/.." && pwd)"
tilde() { echo "$1" | sed "s|^$HOME|~|"; }

always_on() {
  echo "$HOME_DIR/principles.md"
  for f in "$HOME_DIR"/rules/*.md; do [ -e "$f" ] && echo "$f"; done
}

content() {
  echo "<!-- $(tilde "$HOME_DIR") owns this context. Cursor's sessionStart hook runs scripts/cursor-context.sh, which sends principles.md and rules/*.md when a session starts. Edit those files; the next session has them. -->"
  always_on | while read -r f; do
    [ -f "$f" ] || continue
    echo; echo "<!-- $(tilde "$f") -->"; cat "$f"
    [ -n "$(tail -c1 "$f")" ] && echo
  done
}

# JSON string escaping: backslash, quote, tab, carriage return, then one \n
# between lines. Markdown holds no other control characters.
TAB=$(printf '\t'); CR=$(printf '\r')
content |
  sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' -e "s/$TAB/\\\\t/g" -e "s/$CR/\\\\r/g" |
  awk 'BEGIN { printf "{\"additional_context\": \"" }
       { if (NR > 1) printf "\\n"; printf "%s", $0 }
       END { printf "\"}\n" }'
