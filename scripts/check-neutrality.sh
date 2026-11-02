#!/usr/bin/env sh
# Neutrality acceptance checks. Patterns are not stored in check.yml to avoid grep self-match.
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

mode="${1:?usage: check-neutrality.sh context|vendors}"

case "$mode" in
  context)
    pat='madgeniusblink/context|context-install'
    if grep -rniE "$pat" .agents README.md install.sh install.ps1 AGENTS.md CONTRIBUTING.md; then
      exit 1
    fi
    ;;
  vendors)
    hits=$(grep -rniE 'cursor|claude code|codex' .agents README.md AGENTS.md CONTRIBUTING.md \
      | grep -v '.agents/standards/adapters.md' || true)
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
