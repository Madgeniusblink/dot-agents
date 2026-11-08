//! A directory as a tree: structure and sizes, never content. Organizing
//! comes before reading, and the tree is the view that shows what is where
//! without opening anything. The home by default; any directory with
//! `--root`.

use crate::home::{tilde, Home};
use std::fs;
use std::path::{Path, PathBuf};

/// Build output and vendored trees, never a file anyone reads.
pub const SKIP: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".venv",
    "venv",
    "__pycache__",
    ".cache",
];

struct Counts {
    dirs: usize,
    files: usize,
    lines: usize,
}

pub fn tree(root: &Path, depth: usize, all: bool, home: Option<&Home>) {
    let always_on: Vec<PathBuf> = home
        .filter(|h| root.starts_with(&h.dir))
        .map(Home::always_on)
        .unwrap_or_default();
    println!("{}", tilde(root));
    let mut counts = Counts {
        dirs: 0,
        files: 0,
        lines: 0,
    };
    walk(root, "", depth, all, &always_on, &mut counts);
    println!();
    println!(
        "{} directories, {} files, {} lines",
        counts.dirs, counts.files, counts.lines
    );
}

fn walk(
    dir: &Path,
    prefix: &str,
    depth: usize,
    all: bool,
    always_on: &[PathBuf],
    counts: &mut Counts,
) {
    if depth == 0 {
        return;
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    entries.retain(|p| {
        all || !p
            .file_name()
            .is_some_and(|n| SKIP.contains(&n.to_string_lossy().as_ref()))
    });
    entries.sort_by_key(|p| (!p.is_dir(), p.file_name().map(|n| n.to_os_string())));
    let n = entries.len();
    for (i, p) in entries.iter().enumerate() {
        let last = i + 1 == n;
        let branch = if last { "└── " } else { "├── " };
        let name = p.file_name().unwrap_or_default().to_string_lossy();
        if p.is_symlink() {
            counts.files += 1;
            let target = fs::read_link(p).unwrap_or_default();
            println!("{prefix}{branch}{name} -> {}", tilde(&target));
        } else if p.is_dir() {
            counts.dirs += 1;
            println!("{prefix}{branch}{name}/");
            let next = format!("{prefix}{}", if last { "    " } else { "│   " });
            walk(p, &next, depth - 1, all, always_on, counts);
        } else {
            counts.files += 1;
            let lines = fs::read(p)
                .map(|b| b.iter().filter(|&&c| c == b'\n').count())
                .unwrap_or(0);
            counts.lines += lines;
            let mark = if always_on.contains(p) {
                "  always-on"
            } else {
                ""
            };
            println!("{prefix}{branch}{name}  {lines}{mark}");
        }
    }
}
