//! Every package in Cargo.lock is named in crates.txt, so a new crate shows
//! in a diff as one line a reviewer reads.

use std::collections::BTreeSet;

const LOCK: &str = include_str!("../Cargo.lock");
const LIST: &str = include_str!("../crates.txt");

fn lock_names(lock: &str) -> BTreeSet<&str> {
    lock.lines()
        .filter_map(|l| l.strip_prefix("name = \"")?.strip_suffix('"'))
        .collect()
}

/// Returns the failure message, or None when the list and the lock agree.
fn compare(lock: &str, list: &str) -> Option<String> {
    let locked = lock_names(lock);
    let listed: BTreeSet<&str> = list
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let added: Vec<_> = locked.difference(&listed).copied().collect();
    let gone: Vec<_> = listed.difference(&locked).copied().collect();
    let mut msg = String::new();
    if !added.is_empty() {
        msg += &format!(
            "cli/Cargo.lock has packages that cli/crates.txt does not list: {}\n\
             Add each to cli/crates.txt and record the reason in a numbered decision in docs/decisions/.\n",
            added.join(", ")
        );
    }
    if !gone.is_empty() {
        msg += &format!(
            "cli/crates.txt lists packages that cli/Cargo.lock no longer has: {}\n\
             Delete these lines from cli/crates.txt.\n",
            gone.join(", ")
        );
    }
    (!msg.is_empty()).then_some(msg)
}

fn package(name: &str, version: &str) -> String {
    format!("[[package]]\nname = \"{name}\"\nversion = \"{version}\"\n\n")
}

#[test]
fn d1_list_is_sorted_and_unique() {
    let lines: Vec<&str> = LIST.lines().collect();
    let mut sorted = lines.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        lines, sorted,
        "cli/crates.txt must be sorted with one name per line"
    );
}

#[test]
fn d2_lock_matches_list() {
    if let Some(msg) = compare(LOCK, LIST) {
        panic!("{msg}");
    }
}

#[test]
fn d2_unlisted_package_fails() {
    let lock = package("anyhow", "1.0.0") + &package("left-pad", "1.0.0");
    let msg = compare(&lock, "anyhow\n").expect("an unlisted package must fail");
    assert!(msg.contains("does not list: left-pad\n"), "{msg}");
    assert!(msg.contains("numbered decision"), "{msg}");
}

#[test]
fn d3_version_bump_passes() {
    let lock = package("anyhow", "1.0.0") + &package("walkdir", "2.0.0");
    let bumped = package("anyhow", "9.9.9") + &package("walkdir", "2.0.0");
    let list = "anyhow\nwalkdir\n";
    assert_eq!(compare(&lock, list), None);
    assert_eq!(compare(&bumped, list), None);
}

#[test]
fn d3_removed_package_is_a_line_to_delete() {
    let lock = package("anyhow", "1.0.0");
    let msg = compare(&lock, "anyhow\nwalkdir\n").expect("a stale line must fail");
    assert!(msg.contains("no longer has: walkdir\n"), "{msg}");
    assert!(msg.contains("Delete these lines"), "{msg}");
}
