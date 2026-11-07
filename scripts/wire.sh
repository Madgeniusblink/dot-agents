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

emit_dropin() {
  echo "<!-- $(tilde "$HOME_DIR")/scripts/wire.sh writes this file and --remove deletes it. Add a rule as a file in $(tilde "$HOME_DIR")/rules/, then re-run wire.sh --apply. -->"
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
    do_it "grep -v '^@.*/\.agents/' '$target' > '$target.tmp' && mv '$target.tmp' '$target'" \
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

# Undo: delete our drop-in, our legacy import lines, and our links. Leave
# everything else alone.
if [ "$remove" -eq 1 ]; then
  if [ -f "$DROPIN" ]; then
    do_it "rm '$DROPIN'" "delete ${DROPIN#$HOME/}"
  else
    say "no file at ${DROPIN#$HOME/}"
  fi
  drop_legacy_imports "$HOME/.claude/CLAUDE.md"
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

# Codex reads one global file and has no import syntax, so a link is the only
# live transport. It concatenates parents into children and stops at 32 KiB.
CODEX="${CODEX_HOME:-$HOME/.codex}"
if [ -d "$CODEX" ]; then
  found=1; echo "codex:"
  add_link "$HOME_DIR/AGENTS.md" "$CODEX/AGENTS.md"
  [ -e "$CODEX/AGENTS.md" ] && [ ! -L "$CODEX/AGENTS.md" ] &&
    say "it already has its own AGENTS.md. Move your rules into the home first."
fi

# Cursor keeps user-level rules in its application settings, not in a file.
if [ -d "$HOME/.cursor" ]; then
  found=1; echo "cursor:"
  say "user rules live in the app settings, not a file, so nothing here can"
  say "wire them. Paste this into Settings -> Rules by hand:"
  say "  Read ~/.agents/AGENTS.md and every file in ~/.agents/rules/ before starting work."
fi

[ "$found" -eq 1 ] || echo "  no known tool directory in \$HOME. see docs/setup.md."

echo
echo "verify: sh $HOME_DIR/scripts/check.sh"
