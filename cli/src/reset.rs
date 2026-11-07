//! Move each tool's own instruction files aside, so the home is the only
//! thing a session loads, then wire. Nothing is deleted: every file goes to
//! `~/.agents-reset/<timestamp>/` under its original path, with a manifest,
//! and `--restore` puts it back.

use crate::harness::{codex_dir, Claude};
use crate::home::{rel, tilde, user_home, Home};
use crate::plan::{Action, Plan};
use crate::wire;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn reset(home: &Home) -> Plan {
    let mut plan = Plan::new();
    let user = user_home();
    let stamp = timestamp();
    let aside = user.join(".agents-reset").join(&stamp);
    let claude = Claude::new();

    let mut candidates: Vec<PathBuf> = Vec::new();
    if claude.exists() {
        for f in [claude.own_file(), claude.root.join("CLAUDE.local.md")] {
            if f.is_file() && fs::metadata(&f).map(|m| m.len()).unwrap_or(0) > 0 {
                candidates.push(f);
            }
        }
        let keep = [claude.dropin(), claude.rules_link()];
        if let Ok(rd) = fs::read_dir(claude.root.join("rules")) {
            let mut extra: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| !keep.contains(p) && !p.is_dir())
                .collect();
            extra.sort();
            candidates.extend(extra);
        }
    }
    let codex_file = codex_dir().join("AGENTS.md");
    if codex_file.is_file() && !codex_file.is_symlink() {
        candidates.push(codex_file);
    }

    plan.header("move aside (nothing is deleted):");
    if candidates.is_empty() {
        plan.say("nothing. no tool holds an instruction file of its own.");
    }
    let mut manifest = String::new();
    for from in &candidates {
        let to = aside.join(from.strip_prefix(&user).unwrap_or(from));
        manifest.push_str(&format!("{}\t{}\n", from.display(), to.display()));
        plan.act(
            Action::Move {
                from: from.clone(),
                to: to.clone(),
            },
            format!("move {} -> {}", rel(from), rel(&to)),
        );
    }
    if !candidates.is_empty() {
        plan.act(
            Action::Write {
                path: aside.join("manifest.txt"),
                content: manifest,
            },
            format!("write {}", rel(&aside.join("manifest.txt"))),
        );
        plan.say(format!(
            "put it back with: agents reset --restore {} --apply",
            tilde(&aside)
        ));
    }

    plan.blank();
    let wiring = wire::wire(home, false);
    plan.append(wiring);
    plan
}

pub fn restore(dir: &Path) -> Plan {
    let mut plan = Plan::new();
    let manifest = dir.join("manifest.txt");
    let Ok(text) = fs::read_to_string(&manifest) else {
        plan.say(format!("no manifest at {}", tilde(&manifest)));
        return plan;
    };
    plan.header("restore:");
    for line in text.lines() {
        let Some((from, to)) = line.split_once('\t') else {
            continue;
        };
        let (original, moved) = (PathBuf::from(from), PathBuf::from(to));
        if !moved.exists() {
            plan.say(format!("gone: {}", rel(&moved)));
        } else if original.exists() {
            plan.say(format!(
                "SKIP {} exists; the moved copy stays at {}",
                rel(&original),
                rel(&moved)
            ));
        } else {
            plan.act(
                Action::Move {
                    from: moved.clone(),
                    to: original.clone(),
                },
                format!("move {} -> {}", rel(&moved), rel(&original)),
            );
        }
    }
    plan
}

/// `YYYYMMDD-HHMMSS` in UTC, without a date crate.
pub fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = civil_from_days((secs / 86400) as i64);
    let rem = secs % 86400;
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Days since 1970-01-01 to a proleptic Gregorian date. Howard Hinnant's
/// algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    #[test]
    fn epoch_and_leap_day() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
        assert_eq!(civil_from_days(20_763), (2026, 11, 6));
    }
}
