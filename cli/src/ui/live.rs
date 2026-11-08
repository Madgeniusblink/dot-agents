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

/// The home, and each directory that holds a found skill, so a new skill
/// beside the others is seen too.
pub fn roots(home: &Path, skills: &[Skill]) -> Vec<PathBuf> {
    let mut roots = vec![home.to_path_buf()];
    for p in skills.iter().filter_map(|s| s.dir.parent()) {
        if !roots.iter().any(|r| r == p) {
            roots.push(p.to_path_buf());
        }
    }
    roots
}

/// One number from the path, size, and modified time of every file and
/// directory under `roots`. It changes when a file is added, removed, or
/// written. Hidden entries are skipped: `.git` and the worktrees under
/// `.claude` hold no file a view reads.
pub fn stamp(roots: &[PathBuf]) -> u64 {
    let mut sum = 0;
    for root in roots {
        walk(root, true, &mut sum);
    }
    sum
}

/// A root is followed when it is a link; a link below it is read but not
/// followed into, so a loop of links ends.
fn walk(path: &Path, root: bool, sum: &mut u64) {
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
    for entry in fs::read_dir(path).into_iter().flatten().flatten() {
        if !entry.file_name().to_string_lossy().starts_with('.') {
            walk(&entry.path(), false, sum);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t10_stamp_changes_when_a_file_is_added_written_or_removed() {
        let t = tempfile::tempdir().unwrap();
        let roots = vec![t.path().to_path_buf()];
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
}
