//! Once a day, ask GitHub how far main is ahead of the commit this binary
//! was built from, and say so on stderr. The answer is cached under
//! `~/.cache/agents`, a miss is cached too, and nothing here can fail or
//! slow a command: curl gets three seconds, and any error means silence.
//! `AGENTS_UPDATE_CHECK=off` or a set `CI` variable turns the check off.

use crate::home::user_home;
use serde_json::{json, Value};
use std::cell::Cell;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const COMMIT: &str = env!("AGENTS_COMMIT");
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("AGENTS_COMMIT"), ")");
const REPO: &str = "Madgeniusblink/dot-agents";
const INSTALL: &str = "cargo install --git https://github.com/Madgeniusblink/dot-agents dot-agents";
const DAY: u64 = 86_400;

thread_local! {
    static AHEAD: Cell<Option<Option<u64>>> = const { Cell::new(None) };
}

pub fn warn() {
    if let Some(line) = ahead().and_then(|n| message(COMMIT, n)) {
        eprintln!("agents: {line}");
    }
}

/// The `build` object of a `--json` document: the commit, and how many
/// commits main is ahead, or `null` when the check is off or failed.
pub fn json() -> Value {
    json!({"commit": COMMIT, "main_ahead": ahead()})
}

/// `agents 0.17.0 (0e2c48a)`, then `, main is N commits ahead` when it is.
pub fn line() -> String {
    let mut s = format!("agents {VERSION}");
    if let Some(n) = ahead().filter(|n| *n > 0) {
        let plural = if n == 1 { "" } else { "s" };
        s.push_str(&format!(", main is {n} commit{plural} ahead"));
    }
    s
}

/// How many commits main is ahead of this build, found once per process.
fn ahead() -> Option<u64> {
    AHEAD.with(|a| match a.get() {
        Some(n) => n,
        None => {
            let n = behind();
            a.set(Some(n));
            n
        }
    })
}

fn behind() -> Option<u64> {
    let off = std::env::var("AGENTS_UPDATE_CHECK").is_ok_and(|v| v == "off")
        || std::env::var_os("CI").is_some();
    if off || COMMIT == "unknown" {
        return None;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let cache = user_home()
        .join(".cache")
        .join("agents")
        .join("update.json");
    let cached: Option<Value> = fs::read_to_string(&cache)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .filter(|c: &Value| {
            c["build"] == COMMIT
                && c["checked"]
                    .as_u64()
                    .is_some_and(|t| now.saturating_sub(t) < DAY)
        });
    match cached {
        Some(c) => c["ahead"].as_u64(),
        None => {
            let ahead = fetch();
            if let Some(dir) = cache.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(
                &cache,
                json!({"build": COMMIT, "checked": now, "ahead": ahead}).to_string(),
            );
            ahead
        }
    }
}

/// How many commits main has that the build does not, from GitHub's compare.
fn fetch() -> Option<u64> {
    let url = format!("https://api.github.com/repos/{REPO}/compare/{COMMIT}...main");
    let out = Command::new("curl")
        .args([
            "-fsS",
            "--max-time",
            "3",
            "-H",
            "Accept: application/vnd.github+json",
        ])
        .arg(url)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    serde_json::from_slice::<Value>(&out.stdout).ok()?["ahead_by"].as_u64()
}

fn message(build: &str, ahead: u64) -> Option<String> {
    (ahead > 0).then(|| {
        let s = if ahead == 1 { "" } else { "s" };
        format!("built from {build}; main is {ahead} commit{s} ahead. Update: {INSTALL}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn says_nothing_when_main_has_not_moved() {
        assert_eq!(message("abc1234", 0), None);
    }

    #[test]
    fn counts_the_commits_and_names_the_install() {
        let one = message("abc1234", 1).unwrap();
        assert!(
            one.starts_with("built from abc1234; main is 1 commit ahead."),
            "{one}"
        );
        let three = message("abc1234", 3).unwrap();
        assert!(
            three.contains("main is 3 commits ahead. Update: cargo install --git"),
            "{three}"
        );
    }
}
