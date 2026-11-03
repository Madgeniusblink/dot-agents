#!/usr/bin/env sh
# Install .agents/ to DOT_AGENTS_HOME (default ~/.agents).
# Replaces a file only if it is unchanged since install wrote it (.installed manifest).
set -e

for arg in "$@"; do
  case "$arg" in
    -h|--help)
      echo "Usage: install.sh"
      echo "  DOT_AGENTS_HOME  target directory (default: ~/.agents)"
      echo ""
      echo "Adds missing files. Replaces a file only if it is unchanged since the last install."
      echo "Keeps any file you edited, and lists it. Removes unchanged files the template no longer ships."
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

if command -v sha256sum >/dev/null 2>&1; then
  hash_file() { sha256sum "$1" | cut -c1-64; }
else
  hash_file() { shasum -a 256 "$1" | cut -c1-64; }
fi

MANIFEST="$DEST/.installed"
mkdir -p "$DEST"
[ -f "$MANIFEST" ] || : > "$MANIFEST"
NEW="$MANIFEST.new"
: > "$NEW"

# Manifest line: <sha256>  <relative path>
recorded() {
  awk -v r="$1" 'substr($0, 67) == r { print substr($0, 1, 64); exit }' "$MANIFEST"
}

LIST="$(mktemp)"
trap 'rm -f "$LIST"' EXIT
(cd "$SRC" && find . -type f | sed 's|^\./||' | sort) > "$LIST"

while IFS= read -r rel; do
  file="$SRC/$rel"
  target="$DEST/$rel"
  new=$(hash_file "$file")
  if [ ! -e "$target" ]; then
    mkdir -p "$(dirname "$target")"
    cp "$file" "$target"
    echo "$new  $rel" >> "$NEW"
    continue
  fi
  have=$(hash_file "$target")
  old=$(recorded "$rel")
  if [ "$have" = "$new" ]; then
    echo "$new  $rel" >> "$NEW"
  elif [ -n "$old" ] && [ "$have" = "$old" ]; then
    cp "$file" "$target"
    echo "$new  $rel" >> "$NEW"
    echo "updated: $rel"
  else
    # Edited, or present before the manifest existed. Never guess: keep it.
    [ -n "$old" ] && echo "$old  $rel" >> "$NEW"
    echo "kept (edited): $rel"
  fi
done < "$LIST"

# Files install wrote that the template no longer ships: remove if unchanged.
while IFS= read -r line; do
  rel="${line#????????????????????????????????????????????????????????????????  }"
  old="${line%%  *}"
  grep -qxF "$rel" "$LIST" && continue
  target="$DEST/$rel"
  [ -e "$target" ] || continue
  if [ "$(hash_file "$target")" = "$old" ]; then
    rm "$target"
    echo "removed: $rel"
  else
    echo "kept (edited, no longer shipped): $rel"
  fi
done < "$MANIFEST"

mv "$NEW" "$MANIFEST"
echo "Installed -> $DEST"
echo "A kept file gets no updates. Delete one you never edited to take the current version."
