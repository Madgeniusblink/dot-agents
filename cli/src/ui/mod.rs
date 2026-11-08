//! The views of `agents ui`, one per file. Each view draws lines for a
//! width and a height and keeps its own selection. The frame owns the
//! terminal, the tabs, and the keys, per decision 0011.

pub mod files;
pub mod skills;
pub mod tree;
