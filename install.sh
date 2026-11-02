#!/usr/bin/env sh
# Install .agents/ to DOT_AGENTS_HOME (default ~/.agents).
# Updates upstream-owned files. Never overwrites existing memory/ or decisions/ files.
set -e

FORCE=0
for arg in "$@"; do
  case "$arg" in
    --force) FORCE=1 ;;
    -h|--help)
      echo "Usage: install.sh [--force]"
      echo "  DOT_AGENTS_HOME  target directory (default: ~/.agents)"
      echo ""
      echo "Re-run refreshes upstream-owned template files."
      echo "Existing files under memory/ and decisions/ are never overwritten."
      echo "--force does not change that protection."
      exit 0
      ;;
    *)
      echo "error: unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

ROOT="$(cd "$(dirname "$0")" && pwd)"
SRC="$ROOT/.agents"
if [ -n "${DOT_AGENTS_HOME:-}" ]; then
  DEST="$DOT_AGENTS_HOME"
else
  DEST="${HOME:?set HOME or DOT_AGENTS_HOME}/.agents"
fi

if [ ! -d "$SRC" ]; then
  echo "error: missing $SRC" >&2
  exit 1
fi

is_user_owned() {
  case "$1" in
    memory|memory/*|decisions|decisions/*) return 0 ;;
    *) return 1 ;;
  esac
}

should_copy() {
  _rel="$1"
  _target="$2"
  if [ ! -e "$_target" ]; then
    return 0
  fi
  if is_user_owned "$_rel"; then
    return 1
  fi
  return 0
}

mkdir -p "$DEST"

find "$SRC" -type f | while IFS= read -r file; do
  rel="${file#"$SRC"/}"
  target="$DEST/$rel"
  if should_copy "$rel" "$target"; then
    mkdir -p "$(dirname "$target")"
    cp "$file" "$target"
  fi
done

if [ "$FORCE" = 1 ]; then
  echo "Installed (upstream refreshed; memory/ and decisions/ left unchanged) -> $DEST"
else
  echo "Installed (upstream refreshed; memory/ and decisions/ left unchanged) -> $DEST"
fi
