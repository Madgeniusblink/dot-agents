#!/usr/bin/env sh
# Shared install policy tests for install.sh (sourced paths relative to repo root).
set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

run_install_tests() {
  tmp=$(mktemp -d)
  export DOT_AGENTS_HOME="$tmp/home"

  ./install.sh

  echo "USER LESSON" >> "$DOT_AGENTS_HOME/memory/lessons.md"
  ./install.sh
  grep -q 'USER LESSON' "$DOT_AGENTS_HOME/memory/lessons.md"

  echo "MY AGENT RULES" > "$DOT_AGENTS_HOME/AGENTS.md"
  ./install.sh
  grep -q 'MY AGENT RULES' "$DOT_AGENTS_HOME/AGENTS.md"

  echo "STALE" > "$DOT_AGENTS_HOME/standards/README.md"
  ./install.sh
  grep -q 'Public contract' "$DOT_AGENTS_HOME/standards/README.md"
  ! grep -q 'STALE' "$DOT_AGENTS_HOME/standards/README.md"

  rm -rf "$tmp"
  echo "install behavior OK"
}

run_install_tests
