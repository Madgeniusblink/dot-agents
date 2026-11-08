//! `agents ui`: one full-screen view of the reports the other commands
//! print, per decision 0011. Each view reads the document `--json` prints
//! and returns the lines of its pane for a width and a height, so a render
//! test needs no terminal. The frame that draws the lines and reads the keys
//! is the next slice of #82.

pub mod scan;
