#!/usr/bin/env sh
# Point the agent tools on this machine at this home.
# Prints what it would change. Pass --apply to write.
# Default transport is an import line; --link uses a symlink where the tool
# follows one. Never a copy: a copy is a snapshot and goes stale in silence.

HOME_DIR="$(cd "$(dirname "$0")/.." && pwd)"
apply=0; link=0
for a in "$@"; do
  case "$a" in
    --apply) apply=1 ;;
    --link) link=1 ;;
    -h|--help) echo "usage: wire.sh [--apply] [--link]"; exit 0 ;;
    *) echo "unknown option: $a" >&2; exit 2 ;;
  esac
done
[ "$apply" -eq 1 ] || echo "dry run. nothing is written. add --apply to wire."

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
