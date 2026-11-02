#!/usr/bin/env sh
# Install .agents/ to DOT_AGENTS_HOME (default ~/.agents).
set -e

for arg in "$@"; do
  case "$arg" in
    -h|--help)
      echo "Usage: install.sh"
      echo "  DOT_AGENTS_HOME  target directory (default: ~/.agents)"
      echo ""
      echo "Adds missing files. Refreshes paths listed in .agents/.managed-refresh."
      echo "Never overwrites paths listed in .agents/.user-owned once present."
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

# Repo-only policy files (read from template source, not copied to DEST).
skip_install_meta() {
  case "$1" in
    .user-owned|.managed-refresh) return 0 ;;
    *) return 1 ;;
  esac
}

# rel uses / separators.
matches_rule_line() {
  _rel="$1"
  _line="$2"
  [ -z "$_line" ] && return 1
  case "$_line" in
    */)
      case "$_rel" in
        "${_line%/}"|"${_line%/}"/*) return 0 ;;
      esac
      ;;
    *)
      [ "$_rel" = "$_line" ] && return 0
      ;;
  esac
  return 1
}

matches_list() {
  _rel="$1"
  _listfile="$2"
  while IFS= read -r line || [ -n "$line" ]; do
    line="${line%%#*}"
    line=$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
    if matches_rule_line "$_rel" "$line"; then
      return 0
    fi
  done < "$_listfile"
  return 1
}

is_user_owned() {
  matches_list "$1" "$SRC/.user-owned"
}

is_managed_refresh() {
  matches_list "$1" "$SRC/.managed-refresh"
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
  if is_managed_refresh "$_rel"; then
    return 0
  fi
  return 1
}

mkdir -p "$DEST"

find "$SRC" -type f | while IFS= read -r file; do
  rel="${file#"$SRC"/}"
  if skip_install_meta "$rel"; then
    continue
  fi
  target="$DEST/$rel"
  if should_copy "$rel" "$target"; then
    mkdir -p "$(dirname "$target")"
    cp "$file" "$target"
  fi
done

echo "Installed (missing added; managed paths refreshed; user-owned unchanged) -> $DEST"
