#!/usr/bin/env sh
# Install dot-agents CLI then ~/.agents template. Requires Rust (cargo) or dot-agents already on PATH.
set -e

if command -v dot-agents >/dev/null 2>&1; then
  echo "dot-agents already on PATH"
else
  if ! command -v cargo >/dev/null 2>&1; then
    echo "Install Rust from https://rustup.rs then re-run this script," >&2
    echo "or: cargo install --git https://github.com/Madgeniusblink/dot-agents --path cli --locked" >&2
    exit 1
  fi
  REPO="${DOT_AGENTS_REPO:-https://github.com/Madgeniusblink/dot-agents.git}"
  TMP="${TMPDIR:-/tmp}/dot-agents-install-$$"
  git clone --depth 1 "$REPO" "$TMP"
  cargo install --path "$TMP/cli" --locked
  rm -rf "$TMP"
fi

dot-agents install
dot-agents doctor
