#!/usr/bin/env sh
# Install .agents/ to DOT_AGENTS_HOME (default ~/.agents). Skips existing files unless --force.
set -e

FORCE=0
for arg in "$@"; do
  case "$arg" in
    --force) FORCE=1 ;;
    -h|--help)
      echo "Usage: install.sh [--force]"
      echo "  DOT_AGENTS_HOME  target directory (default: ~/.agents)"
      exit 0
      ;;
  esac
done

ROOT="$(cd "$(dirname "$0")" && pwd)"
SRC="$ROOT/.agents"
DEST="${DOT_AGENTS_HOME:-$HOME/.agents}"

if [ ! -d "$SRC" ]; then
  echo "error: missing $SRC" >&2
  exit 1
fi

mkdir -p "$DEST"

# Plain copy (no Rust, no rsync required)
find "$SRC" -type f | while IFS= read -r file; do
  rel="${file#"$SRC"/}"
  case "$rel" in
    STRUCTURE.md) continue ;;
  esac
  target="$DEST/$rel"
  mkdir -p "$(dirname "$target")"
  if [ "$FORCE" = 1 ] || [ ! -e "$target" ]; then
    cp "$file" "$target"
  fi
done

if [ "$FORCE" = 1 ]; then
  echo "Installed (overwrote existing files) -> $DEST"
else
  echo "Installed (skipped existing files; use --force to overwrite) -> $DEST"
fi
