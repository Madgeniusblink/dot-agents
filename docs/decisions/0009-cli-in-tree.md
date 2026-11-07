# 0009 One CLI, in this tree, that owns no content

## Status

Proposed

## Context

`dot-agents-cli` was archived on 2026-11-02. It embedded the template at build
time, so it pinned users seven releases back, and it carried an install policy
the template had already dropped. Four shell scripts replaced it: `context.sh`,
`wire.sh`, `check.sh`, `check-neutrality.sh`. They need no binary, but they are
not a product anyone else can install, and they cannot be published through
Homebrew, crates.io, or PyPI.

The goal now is one install command that a developer on any machine runs, after
which every agent tool on that machine reads `~/.agents/` and nothing else.
The tools in scope first are Claude Code and Codex; Cursor has no user-rules
file, so it gets the pointer line to paste.

## Decision

1. The CLI lives in this repository at `cli/`, as one crate. It is versioned,
   tagged, and released with the template, so it can never pin the template:
   the binary at tag `vX.Y.Z` is the one that matches tree `vX.Y.Z`.
2. The binary is `agents`. It embeds no content. A home is a git checkout, so
   the CLI runs `git` for every content operation and owns only the wiring,
   the report, and the checks.
3. The commands are the four scripts, one to one, plus three the scripts could
   not be. Each one prints what it would change and writes nothing until
   `--apply`, like `wire.sh` today.

   | Command | Replaces | Does |
   |---|---|---|
   | `agents status` | `context.sh` | What each tool loads, from where, what is broken |
   | `agents wire` | `wire.sh` | Drop one owned file into each tool's rules directory; link skills |
   | `agents unwire` | `wire.sh --remove` | Remove exactly what `wire` added |
   | `agents check` | `check.sh` | Budget and links |
   | `agents init <url>` | none | Clone the user's home to `~/.agents`, add `upstream`, set `merge.ours.driver`, then `wire` |
   | `agents update` | none | `git fetch upstream && git merge --no-edit upstream/main`, then `status` |
   | `agents reset` | none | Move every instruction file a tool owns aside, then `wire` |

4. `reset` moves, never deletes. Each file it takes out of a tool goes to
   `~/.agents-reset/<timestamp>/` under its original path, with a manifest, and
   the command prints the one line that puts it back. The archived CLI and the
   system before it both overwrote files they did not own; that is the failure
   this record exists to stop, and a flag that deletes would reintroduce it.
5. `wire` follows the drop-in rule from 0.14.0: never edit a file the tool
   owns; put one owned file in the tool's scanned directory, or a link where
   the tool follows one. The one exception is `agents harness claude --auto`,
   which merges a documented set of permission keys into
   `~/.claude/settings.json`, because that file has no drop-in directory.
   It records the previous values beside the `reset` manifest so `unwire`
   can restore them.
6. Distribution is three channels from one release: `cargo install dot-agents`,
   a Homebrew tap at `Madgeniusblink/homebrew-tap`, and `pip install dot-agents`
   as a binary wheel. GitHub Releases carries the binaries. The template's
   `release_tag` and the crate version are the same string, checked in CI.
   Checked 2026-11-06: `dot-agents` is free on crates.io, the tap does not
   exist yet, and `dot-agents` on PyPI is a 0.0.1 "Reserved package name"
   upload from 2025-10-18 with no author. If that reservation is not ours,
   the PyPI name is `dot-agents-cli`.
7. The shell scripts stay until the CLI has shipped one release on all three
   channels. Until then the CLI's tests run each script and its command on
   the same temporary `$HOME` and fail on any difference in the resulting
   tree. After that release the scripts are removed and `docs/setup.md`
   names the binary.
8. Validation for other developers is the existing `agent-home` skill, which
   gains `agents status` and `agents check` as its first commands. No second
   skill.

## Consequences

One install command, and one place the wiring logic lives. The neutrality
check lists the directories it greps and `cli/` is not among them, like
`scripts/`, so the crate may name the tools it wires.

Two implementations exist for the length of item 7, and a change to wiring
must land in both or the tests fail. That cost is bounded by the release that
retires the scripts.

`harness claude --auto` is the only command that edits a file the tool owns.
It is reversible through the recorded previous values, and it stays a separate
command, never part of `wire` or `init`.

Project-level `.agents/` directories are out of scope for this record.

## Plan

1. `cli/` scaffold, `status`, and the fixture test against `context.sh`.
2. `wire` and `unwire`, with the round-trip test from 0.12.0.
3. `check` and `update`.
4. `reset` with its manifest and restore line.
5. Release on the three channels; versions checked in CI.
6. `harness claude --auto`.
7. Retire the scripts; point `docs/setup.md` and the skill at the binary.
