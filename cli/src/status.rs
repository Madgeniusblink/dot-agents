//! Report what loads into an agent's context, from where, and what is
//! broken. Reads only. Writes nothing.

use crate::harness::{
    codex_dir, cursor_dir, grok_dir, grok_dropin, Claude, CODEX_PROJECT_DOC_MAX, CURSOR_LINE,
};
use crate::home::{expand_home, tilde, Home};
use crate::out;
use crate::wire::{dropin_content, generated_content, is_ours};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn status(home: &Home) {
    let width = out::width();
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
        out::say(
            ".gitattributes marks files merge=ours, but merge.ours.driver is not set. Those files conflict on every upstream change. To fix, run:",
            2,
            width,
        );
        println!(
            "    git -C {} config merge.ours.driver true",
            tilde(&home.dir)
        );
    }

    let always_on = home.always_on();
    out::section("always-on, loads in every session", always_on.len(), width);
    let mut total = 0u64;
    let mut items = Vec::new();
    for f in &always_on {
        let n = fs::metadata(f).map(|m| m.len()).unwrap_or(0);
        total += n;
        items.push(out::Row::new(
            f.file_name().unwrap_or_default().to_string_lossy(),
            format!("{n} bytes"),
            "",
        ));
    }
    items.push(out::Row::new("total", format!("{total} bytes"), ""));
    out::print(&items, false, width);

    let demand: Vec<out::Row> = [
        "memory",
        "templates",
        "skills",
        "decisions",
        "standards",
        "context",
    ]
    .iter()
    .map(|d| home.dir.join(d))
    .filter(|p| p.is_dir())
    .map(|p| {
        out::Row::new(
            format!("{}/", p.file_name().unwrap_or_default().to_string_lossy()),
            format!("{} files", count_files(&p)),
            "",
        )
    })
    .collect();
    out::section("on demand", demand.len(), width);
    out::print(&demand, false, width);
    out::say(
        "The router names these. A tool opens one when a task matches it.",
        2,
        width,
    );

    let harnesses = 3 + usize::from(grok_dir().is_dir()) + usize::from(cursor_dir().is_dir());
    out::section("what each harness loads", harnesses, width);
    let claude = Claude::new();
    let generated = generated_content(home);
    report_file(
        home,
        &claude.dropin(),
        "claude code, the home's drop-in",
        None,
        total,
        Some(&dropin_content(home)),
    );
    report_file(
        home,
        &claude.own_file(),
        "claude code, the harness's own file",
        None,
        total,
        None,
    );
    report_file(
        home,
        &codex_dir().join("AGENTS.md"),
        "codex, the home's generated file",
        Some(CODEX_PROJECT_DOC_MAX),
        total,
        Some(&generated),
    );
    if grok_dir().is_dir() {
        report_file(
            home,
            &grok_dropin(),
            "grok, the home's generated file",
            None,
            total,
            Some(&generated),
        );
    }
    if cursor_dir().is_dir() {
        println!();
        println!("cursor: user rules live in the app settings, not in a file.");
        println!("  Nothing here can read or wire them. Paste the pointer line by hand:");
        println!("    {CURSOR_LINE}");
    }

    dead_hooks(&claude.settings(), width);

    let links: Vec<PathBuf> = [claude.rules_link(), claude.skills_link()]
        .into_iter()
        .filter(|l| l.is_symlink())
        .collect();
    out::section("links into this home", links.len(), width);
    for l in &links {
        let target = fs::read_link(l).unwrap_or_default();
        if l.exists() {
            println!("  {} -> {}", tilde(l), tilde(&target));
        } else {
            println!("  BROKEN {} -> {}", tilde(l), target.display());
        }
    }
    if links.is_empty() {
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

/// One harness file: which imports resolve, what it holds, who owns it, and
/// whether a file the home wrote still matches the home.
fn report_file(
    home: &Home,
    f: &Path,
    label: &str,
    cap: Option<u64>,
    always_on: u64,
    expected: Option<&str>,
) {
    let width = out::width();
    println!();
    if !f.exists() && !f.is_symlink() {
        out::say(
            &format!(
                "{label}: no file at {}. This harness loads nothing from here.",
                tilde(f)
            ),
            0,
            width,
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
            println!("{}", out::fit(&format!("  BROKEN {line}"), width));
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
    out::say(
        &format!("{lines} lines, {bytes} bytes: {mine} imports from this home, {foreign} from elsewhere, {broken} broken."),
        2,
        width,
    );
    // Codex compares global plus thread instructions against project_doc_max_bytes
    // and warns, but truncates only project files. Crossing it is a signal, not a
    // cut-off. Source: codex-rs/core/src/agents_md_manager.rs.
    if let Some(cap) = cap {
        if bytes + always_on > cap {
            out::say(
                &format!(
                    "{} bytes with the always-on set. That is over the {cap} byte project_doc_max_bytes setting. The harness does not truncate this file, but it warns. Project files then compete for what is left.",
                    bytes + always_on
                ),
                2,
                width,
            );
        }
    }
    let ours = is_ours(f);
    if ours && expected.is_some_and(|want| want != text) {
        out::say(
            "STALE: the home changed after wire wrote this file. Run: agents wire --apply",
            2,
            width,
        );
    }
    // Content in the file itself outranks nothing and duplicates the home,
    // unless the file is a link into the home or the home wrote it.
    let into_home = f.is_symlink() && canonical(f).starts_with(canonical(&home.dir));
    let body = text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('@') && !l.starts_with("<!--"))
        .count();
    if body > 10 && !into_home && !ours {
        out::say(
            &format!("{body} lines of rules live in this file, not in the home. That makes two sources of truth."),
            2,
            width,
        );
    }
    // A file the home wrote is a generator's, but it is ours, and status
    // already says whether it is current.
    for (i, l) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| {
            !ours
                && (l.contains("Generated from")
                    || l.contains("generated by")
                    || l.contains("do not edit"))
        })
        .take(2)
    {
        println!(
            "{}",
            out::fit(&format!("  OWNED BY A GENERATOR: {}:{l}", i + 1), width)
        );
        println!("  A regeneration discards anything that you add here.");
    }
}

fn canonical(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// A hook naming a path that is gone blocks the tool it guards, and the error
/// names the script rather than the config that still calls for it.
fn dead_hooks(settings: &Path, width: usize) {
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
    out::section(
        &format!("hooks in {} that name a missing path", tilde(settings)),
        gone.len(),
        width,
    );
    for t in &gone {
        println!("  {}", tilde(t));
    }
    out::say(
        "Each one fails the tool it guards. Remove the hook or restore the file.",
        2,
        width,
    );
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
