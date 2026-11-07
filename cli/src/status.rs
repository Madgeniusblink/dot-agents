//! Report what loads into an agent's context, from where, and what is
//! broken. Reads only. Writes nothing.

use crate::harness::{codex_dir, cursor_dir, Claude, CODEX_PROJECT_DOC_MAX, CURSOR_LINE};
use crate::home::{expand_home, tilde, Home};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn status(home: &Home) {
    println!("home: {}", tilde(&home.dir));
    if home.git(&["rev-parse", "--is-inside-work-tree"]).is_some() {
        let branch = home
            .git(&["rev-parse", "--abbrev-ref", "HEAD"])
            .unwrap_or_default();
        let sha = home
            .git(&["rev-parse", "--short", "HEAD"])
            .unwrap_or_default();
        let dirty = home
            .git(&["status", "--porcelain"])
            .map(|o| o.lines().count())
            .unwrap_or(0);
        println!("  {branch} at {sha}, {dirty} uncommitted");
        if home
            .git(&["rev-parse", "--verify", "-q", "upstream/main"])
            .is_some()
        {
            let n = home
                .git(&["rev-list", "--count", "HEAD..upstream/main"])
                .unwrap_or_default();
            println!("  behind upstream by {n} commits");
        }
    } else {
        println!("  NOT a git checkout. You cannot see what you changed.");
    }

    // merge=ours without the driver set falls back to a normal merge, silently.
    let attributes = fs::read_to_string(home.dir.join(".gitattributes")).unwrap_or_default();
    if attributes.contains("merge=ours")
        && home.git(&["config", "merge.ours.driver"]).as_deref() != Some("true")
    {
        println!("  .gitattributes marks files merge=ours but merge.ours.driver is unset.");
        println!("  Those files will conflict on every upstream change. Fix:");
        println!(
            "    git -C {} config merge.ours.driver true",
            tilde(&home.dir)
        );
    }

    println!();
    println!("always-on (loads every session):");
    let mut total = 0u64;
    for f in home.always_on() {
        let n = fs::metadata(&f).map(|m| m.len()).unwrap_or(0);
        total += n;
        println!(
            "  {}  {n} bytes",
            f.file_name().unwrap_or_default().to_string_lossy()
        );
    }
    println!("  total {total} bytes");

    println!();
    println!("on demand (the router names these; opened when a task matches):");
    for d in [
        "memory",
        "templates",
        "skills",
        "decisions",
        "standards",
        "context",
    ] {
        let p = home.dir.join(d);
        if p.is_dir() {
            println!("  {d}/  {} files", count_files(&p));
        }
    }

    println!();
    println!("what each harness loads:");
    let claude = Claude::new();
    report_file(
        home,
        &claude.dropin(),
        "claude code, the home's drop-in",
        None,
        total,
    );
    report_file(
        home,
        &claude.own_file(),
        "claude code, the harness's own file",
        None,
        total,
    );
    report_file(
        home,
        &codex_dir().join("AGENTS.md"),
        "codex",
        Some(CODEX_PROJECT_DOC_MAX),
        total,
    );
    if cursor_dir().is_dir() {
        println!();
        println!("cursor: user rules live in the app settings, not a file.");
        println!("  Nothing here can read or wire them. Paste the pointer line by hand:");
        println!("    {CURSOR_LINE}");
    }

    dead_hooks(&claude.settings());

    println!();
    println!("links into this home:");
    let mut found = false;
    for l in [claude.rules_link(), claude.skills_link()] {
        if !l.is_symlink() {
            continue;
        }
        found = true;
        let target = fs::read_link(&l).unwrap_or_default();
        if l.exists() {
            println!("  {} -> {}", tilde(&l), tilde(&target));
        } else {
            println!("  BROKEN {} -> {}", tilde(&l), target.display());
        }
    }
    if !found {
        println!("  none");
    }
}

fn count_files(dir: &Path) -> usize {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .count()
}

/// One harness file: which imports resolve, what it holds, and who owns it.
fn report_file(home: &Home, f: &Path, label: &str, cap: Option<u64>, always_on: u64) {
    println!();
    if !f.exists() && !f.is_symlink() {
        println!(
            "{label}: no file at {}. This harness loads nothing from here.",
            tilde(f)
        );
        return;
    }
    println!("{label}: {}", tilde(f));
    let text = fs::read_to_string(f).unwrap_or_default();
    let (mut mine, mut foreign, mut broken) = (0, 0, 0);
    for line in text.lines() {
        let Some(target) = line.strip_prefix('@') else {
            continue;
        };
        let path = expand_home(target);
        if !path.exists() {
            println!("  BROKEN {line}");
            broken += 1;
        } else if path.starts_with(&home.dir) {
            mine += 1;
        } else {
            foreign += 1;
        }
    }
    let lines = text.lines().count();
    let bytes = text.len() as u64;
    if f.is_symlink() {
        println!(
            "  symlink -> {}",
            tilde(&fs::read_link(f).unwrap_or_default())
        );
    }
    println!(
        "  {lines} lines, {bytes} bytes: {mine} imports from this home, {foreign} from elsewhere, {broken} broken"
    );
    // Codex compares global plus thread instructions against project_doc_max_bytes
    // and warns, but truncates only project files. Crossing it is a signal, not a
    // cut-off. Source: codex-rs/core/src/agents_md_manager.rs.
    if let Some(cap) = cap {
        if bytes + always_on > cap {
            println!(
                "  {} bytes with the always-on set, over the {cap} byte",
                bytes + always_on
            );
            println!("  project_doc_max_bytes setting. This file is not truncated, but the");
            println!("  harness warns, and project files then compete for what is left.");
        }
    }
    // Content in the file itself outranks nothing and duplicates the home,
    // unless the file is a link into the home, in which case it is the home.
    let into_home = f.is_symlink() && canonical(f).starts_with(canonical(&home.dir));
    let body = text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('@') && !l.starts_with("<!--"))
        .count();
    if body > 10 && !into_home {
        println!(
            "  {body} lines of rules live in this file, not in the home. Two sources of truth."
        );
    }
    for (i, l) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| {
            l.contains("Generated from") || l.contains("generated by") || l.contains("do not edit")
        })
        .take(2)
    {
        println!("  OWNED BY A GENERATOR: {}:{l}", i + 1);
        println!("  A regeneration will discard anything added here.");
    }
}

fn canonical(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// A hook naming a path that is gone blocks the tool it guards, and the error
/// names the script rather than the config that still calls for it.
fn dead_hooks(settings: &Path) {
    let Ok(text) = fs::read_to_string(settings) else {
        return;
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        println!();
        println!(
            "{} is not valid JSON. The harness ignores it.",
            tilde(settings)
        );
        return;
    };
    let mut gone = BTreeSet::new();
    commands(&json["hooks"], &mut |cmd| {
        for token in cmd.split_whitespace() {
            let path = expand_home(token.trim_matches('"').trim_matches('\''));
            if path.is_absolute() && !path.exists() {
                gone.insert(path);
            }
        }
    });
    if gone.is_empty() {
        return;
    }
    println!();
    println!(
        "hooks in {} naming a path that does not exist:",
        tilde(settings)
    );
    for t in &gone {
        println!("  {}", tilde(t));
    }
    println!("  Each one fails the tool it guards. Remove the hook or restore the file.");
}

fn commands(v: &Value, f: &mut dyn FnMut(&str)) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                match (k.as_str(), x.as_str()) {
                    ("command", Some(s)) => f(s),
                    _ => commands(x, f),
                }
            }
        }
        Value::Array(a) => a.iter().for_each(|x| commands(x, f)),
        _ => {}
    }
}
