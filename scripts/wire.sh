#!/usr/bin/env sh
# Point the agent tools on this machine at this home, or take it back out.
# Prints what it would change. Pass --apply to write.
# --remove undoes exactly what this script added: its import lines and its
# links, and nothing else. Anything that writes without an undo leaves the
# other half behind when half is deleted.
# Default transport is an import line; --link uses a symlink where the tool
# follows one. Never a copy: a copy is a snapshot and goes stale in silence.

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

# One import line per always-on file. The router names the rest.
always_on() {
  echo "$HOME_DIR/principles.md"
  for f in "$HOME_DIR"/rules/*.md; do [ -e "$f" ] && echo "$f"; done
}

add_imports() {
  target="$1"
  always_on | while read -r f; do
    rel=$(echo "$f" | sed "s|^$HOME|~|")
    if [ -f "$target" ] && grep -qF "@$rel" "$target"; then
      say "present: @$rel"
    else
      do_it "mkdir -p '$(dirname "$target")' && printf '@%s\n' '$rel' >> '$target'" "add @$rel -> ${target#$HOME/}"
    fi
  done
}

add_link() {
  src="$1"; dst="$2"
  if [ -L "$dst" ]; then say "linked: ${dst#$HOME/}"
  elif [ -e "$dst" ]; then say "SKIP ${dst#$HOME/} exists and is not a link"
  else do_it "mkdir -p '$(dirname "$dst")' && ln -s '$src' '$dst'" "link ${dst#$HOME/} -> ${src#$HOME/}"
  fi
}

# Undo: drop our import lines and our links. Leave everything else alone.
if [ "$remove" -eq 1 ]; then
  for target in "$HOME/.claude/CLAUDE.md"; do
    [ -f "$target" ] || continue
    n=$(grep -c "^@.*/\.agents/" "$target" 2>/dev/null) || n=0
    if [ "$n" -gt 0 ]; then
      do_it "grep -v '^@.*/\.agents/' '$target' > '$target.tmp' && mv '$target.tmp' '$target'" \
            "drop $n import line(s) from ${target#$HOME/}"
    else
      say "no import lines in ${target#$HOME/}"
    fi
  done
  for l in "$HOME/.claude/rules/agents" "$HOME/.claude/skills/agents"; do
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
    add_imports "$HOME/.claude/CLAUDE.md"
  fi
  # Skills have no import syntax. A link is the only live transport.
  add_link "$HOME_DIR/skills" "$HOME/.claude/skills/agents"
fi

for d in .cursor .codex; do
  [ -d "$HOME/$d" ] || continue
  found=1; echo "${d#.}:"
  say "no documented import syntax. add this line to its global instructions:"
  say "  Read ~/.agents/AGENTS.md and every file in ~/.agents/rules/ before starting work."
done

[ "$found" -eq 1 ] || echo "  no known tool directory in \$HOME. see docs/setup.md."

echo
echo "verify: sh $HOME_DIR/scripts/check.sh"
