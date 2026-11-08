//! The views `agents ui` shows, one per file. Each view reads what the
//! report prints, renders into lines of a given width and height, and
//! writes no file. The frame owns the terminal and the keys.
// The frame lands in a later slice. Until it wires these views in, nothing
// in the binary calls them.
#![allow(dead_code)]

pub mod files;
pub mod tree;
