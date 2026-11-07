//! The checks that apply to any agent home: the context budget, relative
//! links, documented merge commands, and skill frontmatter. One exit code.

use crate::home::Home;
use crate::skills;
use std::fs;
use std::path::{Path, PathBuf};

pub fn check(home: &Home) -> bool {
    let budget_ok = budget(home);
    let links_ok = links(home);
    let skills_ok = skill_lint(home);
    budget_ok && links_ok && skills_ok
}

/// Each home skill against the Agent Skills standard. A warning prints; an
/// error fails the check.
fn skill_lint(home: &Home) -> bool {
    let root = &home.dir;
    let mut ok = true;
    for f in md_files(&root.join("skills"), true) {
        if f.file_name().is_none_or(|n| n != "SKILL.md") {
            continue;
        }
        let dir = f.parent().and_then(Path::file_name).unwrap_or_default();
        let text = fs::read_to_string(&f).unwrap_or_default();
        let front = skills::parse(&text).map(|(front, _)| front);
        for finding in skills::lint(front.as_ref(), &dir.to_string_lossy()) {
            println!("{}: {finding}", display(root, &f));
            ok &= finding.level != skills::Level::Error;
        }
    }
    if ok {
        println!("skill lint OK");
    }
    ok
}

/// Router <= 50 lines, one line per rule, skill <= 120 lines, every
/// correction dated with its retirement test.
fn budget(home: &Home) -> bool {
    let mut ok = true;
    let root = &home.dir;

    let router = line_count(&root.join("AGENTS.md"));
    if router > 50 {
        println!("AGENTS.md: {router} lines (max 50). Move detail to a linked file.");
        ok = false;
    }

    // A rule is one line: no wrapped or indented continuation, no multi-line paragraph.
    for f in md_files(&root.join("rules"), false) {
        let name = display(root, &f);
        let text = fs::read_to_string(&f).unwrap_or_default();
        let mut prev_is_text = false;
        // The intro before the first rule may be prose; the rules may not.
        let mut started = false;
        for (i, line) in text.lines().enumerate() {
            let n = i + 1;
            started |= line.starts_with("- ");
            if !started {
                continue;
            }
            let continues =
                line.starts_with([' ', '\t']) && line.trim_start().chars().next().is_some();
            if continues {
                println!("{name}:{n}: rule continues on an indented line");
                ok = false;
            }
            let is_text = !line.starts_with([' ', '\t', '#', '-']) && !line.is_empty();
            if is_text && prev_is_text {
                println!("{name}:{n}: paragraph spans more than one line");
                ok = false;
            }
            prev_is_text = is_text;
        }
    }

    // A correction carries its death condition: `YYYY-MM · task that proves it`.
    let corrections = root.join("rules").join("corrections.md");
    if let Ok(text) = fs::read_to_string(&corrections) {
        for (i, line) in text.lines().enumerate() {
            if line.starts_with("- ") && !dated(line) {
                println!(
                    "rules/corrections.md:{}: correction lacks `YYYY-MM · test task`",
                    i + 1
                );
                ok = false;
            }
        }
    }

    for f in md_files(&root.join("skills"), true) {
        if f.file_name().is_some_and(|n| n == "SKILL.md") {
            let n = line_count(&f);
            if n > 120 {
                println!(
                    "{}: {n} lines (max 120). Move detail to references/.",
                    display(root, &f)
                );
                ok = false;
            }
        }
    }

    if ok {
        println!("budget OK");
    }
    ok
}

/// Ends with `` `YYYY-MM · text` ``.
fn dated(line: &str) -> bool {
    let Some(body) = line.strip_suffix('`') else {
        return false;
    };
    let Some(start) = body.rfind('`') else {
        return false;
    };
    let tag = &body[start + 1..];
    let b = tag.as_bytes();
    b.len() > 10
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && tag[7..].starts_with(" · ")
        && tag[7..].len() > " · ".len()
}

/// Relative markdown links resolve, and a documented `git merge` carries
/// `--no-edit`. Without it, git opens an editor and waits, which in a shell
/// nobody is watching is indistinguishable from a hang.
fn links(home: &Home) -> bool {
    let root = &home.dir;
    let mut errors = 0;
    let mut files = Vec::new();
    for name in ["AGENTS.md", "principles.md", "README.md"] {
        let f = root.join(name);
        if f.is_file() {
            files.push(f);
        }
    }
    for d in [
        "rules",
        "context",
        "standards",
        "skills",
        "templates",
        "docs",
    ] {
        files.extend(md_files(&root.join(d), true));
    }
    for md in files {
        let text = fs::read_to_string(&md).unwrap_or_default();
        let name = display(root, &md);
        for target in link_targets(&text) {
            let target = target.split('#').next().unwrap_or("").trim();
            if target.is_empty()
                || target.starts_with("http://")
                || target.starts_with("https://")
                || target.starts_with("mailto:")
            {
                continue;
            }
            let resolved = md.parent().unwrap_or(root).join(target);
            if !resolved.exists() {
                println!("broken: {name} -> {target}");
                errors += 1;
            }
        }
        for (i, line) in text.lines().enumerate() {
            if merge_without_no_edit(line) {
                println!("{name}:{}: git merge without --no-edit", i + 1);
                errors += 1;
            }
        }
    }
    if errors == 0 {
        println!("link and command check OK");
    }
    errors == 0
}

/// Every `](target)` in the text.
fn link_targets(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("](") {
        let after = &rest[i + 2..];
        match after.find(')') {
            Some(j) => {
                out.push(&after[..j]);
                rest = &after[j + 1..];
            }
            None => break,
        }
    }
    out
}

/// `git merge X` or `git -C dir merge X` where X is not --no-edit, --abort,
/// or --continue.
fn merge_without_no_edit(line: &str) -> bool {
    let mut rest = line;
    while let Some(i) = rest.find("git ") {
        let mut after = &rest[i + 4..];
        if let Some(r) = after.strip_prefix("-C ") {
            match r.find(' ') {
                Some(j) => after = r[j + 1..].trim_start(),
                None => return false,
            }
        }
        if let Some(args) = after.strip_prefix("merge ") {
            if !["--no-edit", "--abort", "--continue"]
                .iter()
                .any(|flag| args.starts_with(flag))
            {
                return true;
            }
        }
        rest = &rest[i + 4..];
    }
    false
}

fn md_files(dir: &Path, recursive: bool) -> Vec<PathBuf> {
    if !dir.is_dir() {
        return Vec::new();
    }
    let depth = if recursive { usize::MAX } else { 1 };
    let mut v: Vec<PathBuf> = walkdir::WalkDir::new(dir)
        .max_depth(depth)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x == "md"))
        .map(|e| e.into_path())
        .collect();
    v.sort();
    v
}

fn line_count(f: &Path) -> usize {
    fs::read_to_string(f)
        .map(|t| t.lines().count())
        .unwrap_or(0)
}

fn display(root: &Path, f: &Path) -> String {
    f.strip_prefix(root).unwrap_or(f).display().to_string()
}
