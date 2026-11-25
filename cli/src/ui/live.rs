//! Live refresh for `agents ui`. Every 2 seconds the view reads the path,
//! size, and modified time of each file it reports on, and reads its views
//! again when one changed. `R` does the same at once. This is a poll: no
//! file watcher and no crate (decision 0011).

use crate::skills::Skill;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The time between two reads of the file times.
pub const EVERY: Duration = Duration::from_secs(2);

/// How deep the poll reads under a scan root before the scan report is back.
pub const SCAN_DEPTH: usize = 2;

/// One directory the poll reads, and how many levels under it to walk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Root {
    pub path: PathBuf,
    /// `None` walks the whole tree; `Some(0)` is this path alone.
    pub depth: Option<usize>,
}

/// The home, each directory that holds a found skill, and paths the open
/// view shows. Scan roots are shallow until checkout paths are known.
pub fn roots(
    home: &Path,
    skills: &[Skill],
    files: &[PathBuf],
    scan_roots: &[PathBuf],
    scan_checkouts: &[PathBuf],
) -> Vec<Root> {
    let mut roots = vec![Root {
        path: home.to_path_buf(),
        depth: None,
    }];
    for p in skills.iter().filter_map(|s| s.dir.parent()) {
        push(&mut roots, p, None);
    }
    for p in files {
        push(&mut roots, p, None);
    }
    for p in scan_checkouts {
        push(&mut roots, p, None);
    }
    if scan_checkouts.is_empty() {
        for p in scan_roots {
            push(&mut roots, p, Some(SCAN_DEPTH));
        }
    }
    roots
}

fn push(roots: &mut Vec<Root>, path: &Path, depth: Option<usize>) {
    if roots.iter().any(|r| r.path == path) {
        return;
    }
    roots.push(Root {
        path: path.to_path_buf(),
        depth,
    });
}

/// One number from the path, size, and modified time of every file and
/// directory under `roots`. It changes when a file is added, removed, or
/// written. Hidden entries and the build and vendor trees that `agents scan`
/// skips, such as `target` and `node_modules`, are not read: they hold no
/// file a view reads, and a walk of them can outlast the poll period.
pub fn stamp(roots: &[Root]) -> u64 {
    let mut sum = 0;
    for root in roots {
        walk(&root.path, true, root.depth, &mut sum);
    }
    sum
}

/// One stamp per root.
pub type Stamps = Vec<(Root, u64)>;

/// The stamp of each root alone, so a root keeps its baseline while others
/// join or leave the watched set.
pub fn stamps(roots: Vec<Root>) -> Stamps {
    roots
        .into_iter()
        .map(|r| {
            let s = stamp(std::slice::from_ref(&r));
            (r, s)
        })
        .collect()
}

/// A root is followed when it is a link; a link below it is read but not
/// followed into, so a loop of links ends.
fn walk(path: &Path, root: bool, max_depth: Option<usize>, sum: &mut u64) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    let mut h = DefaultHasher::new();
    (path, meta.len(), meta.modified().ok()).hash(&mut h);
    *sum = sum.wrapping_add(h.finish());
    let dir = if root {
        meta.is_dir()
    } else {
        fs::symlink_metadata(path).is_ok_and(|m| m.is_dir())
    };
    if !dir {
        return;
    }
    let child_depth = match max_depth {
        None => None,
        Some(0) => return,
        Some(d) => Some(d - 1),
    };
    for entry in fs::read_dir(path).into_iter().flatten().flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with('.') && !crate::scan::SKIP.contains(&name.as_ref()) {
            walk(&entry.path(), false, child_depth, sum);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t10_stamp_changes_when_a_file_is_added_written_or_removed() {
        let t = tempfile::tempdir().unwrap();
        let roots = vec![Root {
            path: t.path().to_path_buf(),
            depth: None,
        }];
        fs::create_dir(t.path().join("rules")).unwrap();
        fs::write(t.path().join("rules/a.md"), "a\n").unwrap();
        let first = stamp(&roots);
        assert_eq!(stamp(&roots), first, "no change, the same stamp");

        fs::write(t.path().join("rules/b.md"), "b\n").unwrap();
        let added = stamp(&roots);
        assert_ne!(added, first);
        fs::write(t.path().join("rules/b.md"), "b and more\n").unwrap();
        let written = stamp(&roots);
        assert_ne!(written, added);
        fs::remove_file(t.path().join("rules/b.md")).unwrap();
        assert_ne!(stamp(&roots), written);

        let before = stamp(&roots);
        fs::create_dir(t.path().join(".git")).unwrap();
        fs::write(t.path().join(".git/HEAD"), "x\n").unwrap();
        fs::write(t.path().join("rules/.swp"), "x\n").unwrap();
        // The new hidden entries change their parents' times, not the files.
        let hidden = stamp(&roots);
        fs::write(t.path().join(".git/HEAD"), "y and more\n").unwrap();
        assert_eq!(stamp(&roots), hidden, "a hidden file is not read");
        assert_ne!(hidden, before, "a directory's time is read");
    }

    #[test]
    fn t155_stamp_skips_build_and_vendor_trees() {
        let t = tempfile::tempdir().unwrap();
        let roots = vec![Root {
            path: t.path().to_path_buf(),
            depth: None,
        }];
        for dir in ["target/debug", "node_modules/x", "src"] {
            fs::create_dir_all(t.path().join(dir)).unwrap();
        }
        fs::write(t.path().join("target/debug/out"), "a\n").unwrap();
        fs::write(t.path().join("node_modules/x/i.js"), "a\n").unwrap();
        let first = stamp(&roots);
        fs::write(t.path().join("target/debug/out"), "a build\n").unwrap();
        fs::write(t.path().join("target/debug/new"), "b\n").unwrap();
        fs::write(t.path().join("node_modules/x/i.js"), "an install\n").unwrap();
        assert_eq!(stamp(&roots), first, "build output is not read");
        fs::write(t.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        assert_ne!(stamp(&roots), first, "a source file is read");
    }

    #[test]
    fn t155_roots_watch_each_checkout_in_full_and_drop_the_shallow_roots() {
        let t = tempfile::tempdir().unwrap();
        let (home, scan) = (t.path().join("home"), t.path().join("code"));
        let (one, two) = (scan.join("one"), scan.join("two"));
        let r = roots(
            &home,
            &[],
            &[],
            std::slice::from_ref(&scan),
            &[one.clone(), two.clone(), one.clone()],
        );
        let full = |p: &Path| Root {
            path: p.to_path_buf(),
            depth: None,
        };
        assert_eq!(r, vec![full(&home), full(&one), full(&two)]);
    }

    #[test]
    fn t105_roots_watch_the_files_tree_and_scan_shallow_roots() {
        let t = tempfile::tempdir().unwrap();
        let home = t.path().join("home");
        let files = home.join("shown");
        fs::create_dir_all(&files).unwrap();
        let scan = t.path().join("scan");
        fs::create_dir_all(scan.join("a")).unwrap();
        let r = roots(
            &home,
            &[],
            std::slice::from_ref(&files),
            std::slice::from_ref(&scan),
            &[],
        );
        assert!(r.iter().any(|x| x.path == home && x.depth.is_none()));
        assert!(r.iter().any(|x| x.path == files && x.depth.is_none()));
        let shallow = r.iter().find(|x| x.path == scan).unwrap();
        assert_eq!(shallow.depth, Some(SCAN_DEPTH));
    }
}
