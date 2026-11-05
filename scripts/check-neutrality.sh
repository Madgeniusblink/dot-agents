#!/usr/bin/env sh
# Neutrality acceptance checks. Patterns are not stored in check.yml to avoid grep self-match.
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

mode="${1:?usage: check-neutrality.sh context|vendors}"

case "$mode" in
  context)
    pat='madgeniusblink/context|context-install'
    if grep -rniE "$pat" AGENTS.md principles.md README.md CONTRIBUTING.md rules context standards skills templates; then
      exit 1
    fi
    ;;
  vendors)
    hits=$(grep -rniE 'cursor|claude code|codex' AGENTS.md principles.md README.md CONTRIBUTING.md rules context standards skills templates || true)
    if [ -n "$hits" ]; then
      echo "$hits"
      exit 1
    fi
    ;;
  *)
    echo "unknown mode: $mode" >&2
    exit 1
    ;;
esac

echo "neutrality OK ($mode)"
