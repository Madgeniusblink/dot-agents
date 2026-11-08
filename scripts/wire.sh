#!/usr/bin/env sh
# Point the agent tools on this machine at this home, or take it back out.
# Prints what it would change. Pass --apply to write.
# --remove undoes exactly what this script added: its drop-in file and its
# links, and nothing else. Anything that writes without an undo leaves the
# other half behind when half is deleted.
# Never edit a file the tool owns. Where the tool scans a directory, drop one
# file of ours into it; where it follows a link, link. Never a copy: a copy is
# a snapshot and goes stale in silence.

HOME_DIR="$(cd "$(dirname "$0")/.." && pwd)"
apply=0; link=0; remove=0
for a in "$@"; do
  case "$a" in
    --apply) apply=1 ;;
    --link) link=1 ;;
    --remove) remove=1 ;;
    -h|--help) echo "usage: wire.sh [--apply] [--link] [--remove]"; exit 0 ;;
    *) echo "unknown option: $a" >&2; exit 2 ;;
  esac
done
[ "$apply" -eq 1 ] || echo "dry run. nothing is written. add --apply to write."

say() { echo "  $1"; }
do_it() { [ "$apply" -eq 1 ] && eval "$1"; say "$2"; }
tilde() { echo "$1" | sed "s|^$HOME|~|"; }

# One import line per always-on file. The router names the rest.
always_on() {
  echo "$HOME_DIR/principles.md"
  for f in "$HOME_DIR"/rules/*.md; do [ -e "$f" ] && echo "$f"; done
}

# Claude Code scans ~/.claude/rules/ and loads every file in it, so that is
# the drop-in directory. This is the one file in it that the home owns.
# ~/.claude/CLAUDE.md belongs to the harness and to whatever the user or a
# generator put there; nothing here writes to it.
DROPIN="$HOME/.claude/rules/00-agents-home.md"

# Cursor keeps user-level rules in its application settings, not in a file,
# but its user-level hooks file is read on every session. A sessionStart hook
# there runs scripts/cursor-context.sh, which sends the always-on set as it
# is now. Nothing is copied, so nothing goes stale.
# Source: https://cursor.com/docs/agent/hooks
CURSOR_HOOKS="$HOME/.cursor/hooks.json"
CURSOR_CMD="sh $(tilde "$HOME_DIR")/scripts/cursor-context.sh"

# The whole file when the home creates it. The CLI writes the same bytes.
emit_cursor_hooks() {
  printf '{\n  "hooks": {\n    "sessionStart": [\n      {\n        "command": "%s"\n      }\n    ]\n  },\n  "version": 1\n}\n' "$CURSOR_CMD"
}

emit_dropin() {
  echo "<!-- $(tilde "$HOME_DIR") owns this file. wire writes it and unwire deletes it. Add a rule as a file in $(tilde "$HOME_DIR")/rules/, then run wire again. -->"
  always_on | while read -r f; do echo "@$(tilde "$f")"; done
}

write_dropin() {
  if [ -f "$DROPIN" ] && [ "$(cat "$DROPIN")" = "$(emit_dropin)" ]; then
    say "present: ${DROPIN#$HOME/}"
  else
    do_it "mkdir -p '$(dirname "$DROPIN")' && emit_dropin > '$DROPIN'" \
          "write ${DROPIN#$HOME/} ($(always_on | wc -l | tr -d ' ') import lines)"
  fi
}

# An earlier version appended import lines to the harness's own file. Drop
# only those, so the drop-in is the one place they live.
drop_legacy_imports() {
  target="$1"
  [ -f "$target" ] || return 0
  n=$(grep -c "^@.*/\.agents/" "$target" 2>/dev/null) || n=0
  if [ "$n" -gt 0 ]; then
    # grep -v exits 1 when it filters every line, which is the case where the
    # file held nothing but our lines, so treat 1 as success and 2 as failure.
    do_it "{ grep -v '^@.*/\.agents/' '$target' || [ \$? -eq 1 ]; } > '$target.tmp' && mv '$target.tmp' '$target'" \
          "drop $n import line(s) from ${target#$HOME/}"
  fi
}

add_link() {
  src="$1"; dst="$2"
  if [ -L "$dst" ]; then say "linked: ${dst#$HOME/}"
  elif [ -e "$dst" ]; then say "SKIP ${dst#$HOME/} exists and is not a link"
  else do_it "mkdir -p '$(dirname "$dst")' && ln -s '$src' '$dst'" "link ${dst#$HOME/} -> ${src#$HOME/}"
  fi
}

# The file for a tool that follows neither links nor imports: one header,
# then every always-on file in full. wire rewrites it; status reports it stale.
emit_generated() {
  echo "<!-- $(tilde "$HOME_DIR") owns this file. wire writes it from principles.md and rules/*.md because this tool follows no import; unwire deletes it. Edit those files, then run wire again; status reports when this is stale. -->"
  always_on | while read -r f; do
    echo; echo "<!-- $(tilde "$f") -->"; cat "$f"
    [ -n "$(tail -c1 "$f")" ] && echo
  done
}

is_ours() { [ -f "$1" ] && head -1 "$1" | grep -q 'owns this file'; }

write_generated() {
  dst="$1"; was_link=0
  if [ -L "$dst" ] && case "$(readlink "$dst")" in "$HOME_DIR"/*) true ;; *) false ;; esac; then
    was_link=1
    do_it "rm '$dst'" "unlink ${dst#$HOME/} (a link to the router, which this tool cannot follow)"
  elif [ -e "$dst" ] && ! is_ours "$dst"; then
    say "SKIP ${dst#$HOME/} exists and is not the home's. Move your rules into the home first."
    return
  fi
  if [ "$was_link" -eq 0 ] && [ -f "$dst" ] && [ "$(cat "$dst")" = "$(emit_generated)" ]; then
    say "present: ${dst#$HOME/}"
  else
    do_it "mkdir -p '$(dirname "$dst")' && emit_generated > '$dst'" \
          "write ${dst#$HOME/} ($(always_on | wc -l | tr -d ' ') files)"
  fi
}

# Undo: delete our drop-in, our legacy import lines, and our links. Leave
# everything else alone.
if [ "$remove" -eq 1 ]; then
  for f in "$DROPIN" "${CODEX_HOME:-$HOME/.codex}/AGENTS.md" "${GROK_HOME:-$HOME/.grok}/rules/00-agents-home.md"; do
    if [ -f "$f" ] && [ ! -L "$f" ] && is_ours "$f"; then
      do_it "rm '$f'" "delete ${f#$HOME/}"
    fi
  done
  drop_legacy_imports "$HOME/.claude/CLAUDE.md"
  if [ -f "$CURSOR_HOOKS" ] && grep -q 'scripts/cursor-context.sh' "$CURSOR_HOOKS"; then
    if [ "$(cat "$CURSOR_HOOKS")" = "$(emit_cursor_hooks)" ]; then
      do_it "rm '$CURSOR_HOOKS'" "delete ${CURSOR_HOOKS#$HOME/}"
    else
      say "SKIP ${CURSOR_HOOKS#$HOME/}: it holds other hooks too, and this script"
      say "does not edit JSON. Run: agents unwire --apply, or remove the"
      say "sessionStart entry that names scripts/cursor-context.sh by hand."
    fi
  fi
  for l in "$HOME/.claude/rules/agents" "$HOME/.claude/skills/agents" \
           "${CODEX_HOME:-$HOME/.codex}/AGENTS.md"; do
    if [ -L "$l" ]; then
      do_it "rm '$l'" "unlink ${l#$HOME/}"
    fi
  done
  echo
  echo "the home itself is untouched. remove it with: rm -rf $HOME_DIR"
  exit 0
fi

found=0

if [ -d "$HOME/.claude" ]; then
  found=1; echo "claude code:"
  if [ "$link" -eq 1 ]; then
    add_link "$HOME_DIR/rules" "$HOME/.claude/rules/agents"
  else
    write_dropin
  fi
  drop_legacy_imports "$HOME/.claude/CLAUDE.md"
  # Skills have no import syntax. A link is the only live transport.
  add_link "$HOME_DIR/skills" "$HOME/.claude/skills/agents"
fi

# Codex reads one global file and expands no import, so that slot gets the
# generated file.
CODEX="${CODEX_HOME:-$HOME/.codex}"
if [ -d "$CODEX" ]; then
  found=1; echo "codex:"
  write_generated "$CODEX/AGENTS.md"
fi

# Grok Build scans ~/.grok/rules/ in every project and loads each file in
# full, with no import, so its drop-in holds the content.
GROK="${GROK_HOME:-$HOME/.grok}"
if [ -d "$GROK" ]; then
  found=1; echo "grok:"
  write_generated "$GROK/rules/00-agents-home.md"
fi

if [ -d "$HOME/.cursor" ]; then
  found=1; echo "cursor:"
  if [ ! -f "$CURSOR_HOOKS" ]; then
    do_it "emit_cursor_hooks > '$CURSOR_HOOKS'" "write ${CURSOR_HOOKS#$HOME/} (sessionStart hook)"
  elif grep -q 'scripts/cursor-context.sh' "$CURSOR_HOOKS"; then
    say "present: ${CURSOR_HOOKS#$HOME/}"
  else
    say "SKIP ${CURSOR_HOOKS#$HOME/}: it holds other hooks, and this script does"
    say "not edit JSON. Run: agents wire --apply, or add this to hooks.sessionStart:"
    say "  {\"command\": \"$CURSOR_CMD\"}"
  fi
fi

[ "$found" -eq 1 ] || echo "  no known tool directory in \$HOME. see docs/setup.md."

echo
echo "verify: sh $HOME_DIR/scripts/check.sh"
