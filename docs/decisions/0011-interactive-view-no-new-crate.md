# 0011 One interactive command, with no new crate

## Status

Proposed

## Context

Every `agents` command prints once and exits. To go from a report to one
item, then to its file, a person runs the next command by hand. Issue #82
asks for a terminal view that moves through the reports and the home, and
copies a file or sends it to an agent pane with one key. The view sits
beside the agent tool. It is not an AI application.

No command is interactive today, so the view is a new command. Decision 0009
lists the commands, and each one prints a plan and writes nothing until
`--apply`. A view that takes keys could write a file outside that path. This
record closes that gap before any view code lands.

The crate count, measured with `cargo tree -e normal` on macOS arm64 at the
time of #82:

| Option | Crates added |
|---|---|
| `rustix` with `termios`, ANSI written in the crate | 0 |
| `crossterm` 0.29 | about 9 |
| `ratatui` 0.30 with `crossterm` | about 49 |

The tree has 32 crates. `rustix` 1.x is already in it through
`terminal_size`, which pulls it only on Unix. Each added crate is a
supply-chain risk.

## Decision

1. The command is `agents ui`. It opens a full-screen view of the same
   reports the other commands print. The views read the same structs that
   `--json` prints, so the view and the report cannot disagree.
2. The view adds no crate. Raw mode comes from `rustix` with the `termios`
   feature, declared as a direct dependency on the version already in the
   tree. Drawing is ANSI escape sequences written in the crate. A change
   that adds a crate for the view needs a new decision.
3. The view compiles only under `cfg(unix)`, because `rustix` termios is
   Unix only and CI also builds on Windows. On Windows, `agents ui` prints
   one line that says the view is Unix only, and exits 1. Every other
   command is unchanged on Windows.
4. The view changes no file except through the existing plan and `--apply`
   code in `cli/src/plan.rs`. An action that writes shows its plan first.
   One key then runs the same `Plan::run(true)` that `--apply` runs, so
   files move to `~/.agents-reset/` as they do today. The view has no
   write path of its own.
5. Actions that write no file are allowed without a plan: copy through
   OSC 52, open in `$EDITOR`, OSC 8 links, and text sent to a tmux pane
   after the person confirms it. A pane label comes from
   `cli/src/harness.rs`, so no vendor name enters other files.
6. The terminal is restored on exit and on panic. A view that leaves the
   terminal in raw mode is a bug, and the restore path has its own test.

## Consequences

The tree stays at 32 crates, and the release binary grows only by the view
code. The crate owns its drawing, so layout, resize, and key decoding are
code this repository maintains. That is the cost of item 2, and it is
bounded by the small set of views #82 names.

Windows users get every report but no view. A Windows view needs its own
console code and a new decision.

The only write path stays `plan.rs`. A review of a view change checks that
it builds a `Plan` and does not call `fs` itself.

A graph view of skills, a live feed from tool hooks, and a staging tray for
context are out of scope for this record.

## Plan

1. This record.
2. The frame and the Status view, with the restore test. CI passes on all
   three systems.
3. The Scan view.
4. The Skills view.
5. The Files and Tree views.
6. Copy, open, and links.
7. Send to a tmux pane.
8. `skills keep` and `skills remove` from the view, through item 4.
