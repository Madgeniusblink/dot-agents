//! Every rule `standards/` writes, as a check with a stable name. A fault
//! prints as `path:line rule: message. fix: <what to change>`. The rule
//! names and messages are documented in `docs/check.md`.

use crate::home::Home;
use crate::out;
use crate::skills;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

/// Always-on bytes (principles plus every rule file) a home may load when
/// `context/upstream.json` sets no `budget.always_on_bytes`.
const ALWAYS_ON_CAP: u64 = 12 * 1024;
/// Skills the template ships, at most.
const TEMPLATE_SKILLS: usize = 3;

struct Fault {
    path: String,
    line: usize,
    rule: String,
    message: String,
    fix: &'static str,
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}:{} {}: {}. fix: {}",
            self.path, self.line, self.rule, self.message, self.fix
        )
    }
}

/// The always-on cap and where it came from.
struct Cap {
    bytes: u64,
    used: u64,
    source: &'static str,
}

/// Print each fault, or one line with the cap on a clean run.
pub fn check(home: &Home, template: bool, as_json: bool) -> bool {
    let (faults, cap) = run(home, template);
    if as_json {
        let faults: Vec<_> = faults
            .iter()
            .map(|f| {
                json!({
                    "path": f.path,
                    "line": f.line,
                    "rule": f.rule,
                    "message": f.message,
                    "fix": f.fix,
                })
            })
            .collect();
        out::json(
            "check",
            Some(&home.dir),
            json!({
                "ok": faults.is_empty(),
                "always_on": {"bytes": cap.used, "cap": cap.bytes, "cap_source": cap.source},
                "faults": faults,
            }),
        );
        return faults.is_empty();
    }
    for f in &faults {
        println!("{f}");
    }
    if faults.is_empty() {
        println!(
            "check OK: always-on {} of {} bytes ({})",
            cap.used, cap.bytes, cap.source
        );
    }
    faults.is_empty()
}

fn run(home: &Home, template: bool) -> (Vec<Fault>, Cap) {
    let root = &home.dir;
    let mut out = Vec::new();
    router(root, &mut out);
    rules(root, &mut out);
    skill_files(root, &mut out);
    memory(root, &mut out);
    templates(root, &mut out);
    links(root, &mut out);
    let cap = always_on(home, &mut out);
    if template {
        template_budget(root, &mut out);
    }
    out.sort_by(|a, b| (&a.path, a.line, &a.rule).cmp(&(&b.path, b.line, &b.rule)));
    (out, cap)
}

fn fault(
    out: &mut Vec<Fault>,
    path: String,
    line: usize,
    rule: &str,
    message: String,
    fix: &'static str,
) {
    out.push(Fault {
        path,
        line,
        rule: rule.to_string(),
        message,
        fix,
    });
}

fn router(root: &Path, out: &mut Vec<Fault>) {
    let n = line_count(&root.join("AGENTS.md"));
    if n > 50 {
        fault(
            out,
            "AGENTS.md".into(),
            51,
            "router.lines",
            format!("{n} lines, max 50"),
            "move detail to a linked file",
        );
    }
}

/// One line per rule, and every correction ends in a dated test.
fn rules(root: &Path, out: &mut Vec<Fault>) {
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
                fault(
                    out,
                    name.clone(),
                    n,
                    "rule.one-line",
                    "rule continues on an indented line".into(),
                    "join the rule into one line, or split it into two rules",
                );
            }
            let is_text = !line.starts_with([' ', '\t', '#', '-']) && !line.is_empty();
            if is_text && prev_is_text {
                fault(
                    out,
                    name.clone(),
                    n,
                    "rule.one-line",
                    "paragraph spans more than one line".into(),
                    "join the paragraph into one line",
                );
            }
            prev_is_text = is_text;
        }
    }

    // A correction carries its death condition: `YYYY-MM · task that proves it`.
    let corrections = root.join("rules").join("corrections.md");
    if let Ok(text) = fs::read_to_string(&corrections) {
        for (i, line) in text.lines().enumerate() {
            if line.starts_with("- ") && !dated(line) {
                fault(
                    out,
                    "rules/corrections.md".into(),
                    i + 1,
                    "correction.dated",
                    "correction does not end in a dated test".into(),
                    "end the line with `YYYY-MM · task that proves it`",
                );
            }
        }
    }
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

/// Each directory under `skills/` holds a `SKILL.md` whose frontmatter meets
/// the Agent Skills standard and whose body is at most 120 lines.
fn skill_files(root: &Path, out: &mut Vec<Fault>) {
    for dir in skill_dirs(root) {
        let f = dir.join("SKILL.md");
        let name = display(root, &f);
        if !f.is_file() {
            fault(
                out,
                display(root, &dir),
                1,
                "skill.missing",
                "directory has no SKILL.md".into(),
                "add SKILL.md, or move the directory out of skills/",
            );
            continue;
        }
        let text = fs::read_to_string(&f).unwrap_or_default();
        let parsed = skills::parse(&text);
        let dir_name = dir.file_name().unwrap_or_default().to_string_lossy();
        for finding in skills::lint(parsed.as_ref().map(|(front, _)| front), &dir_name) {
            fault(
                out,
                name.clone(),
                1,
                &format!("skill.{}", finding.rule),
                finding.text,
                "edit the frontmatter to meet the Agent Skills standard",
            );
        }
        let body = parsed.map_or(text.as_str(), |(_, body)| body);
        let n = body.lines().count();
        if n > 120 {
            fault(
                out,
                name,
                text.lines().count() - n + 121,
                "skill.lines",
                format!("body has {n} lines, max 120"),
                "move detail to references/",
            );
        }
    }
}

fn skill_dirs(root: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(root.join("skills"))
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// A `memory/` file starts with one `# ` title and holds one fact.
fn memory(root: &Path, out: &mut Vec<Fault>) {
    for f in md_files(&root.join("memory"), true) {
        let name = display(root, &f);
        let text = fs::read_to_string(&f).unwrap_or_default();
        if !text.starts_with("# ") {
            fault(
                out,
                name.clone(),
                1,
                "memory.title",
                "file does not start with a `# ` title".into(),
                "make the first line `# <the fact>`",
            );
        }
        let mut fence = false;
        let mut titles = 0;
        for (i, line) in text.lines().enumerate() {
            if line.starts_with("```") {
                fence = !fence;
            }
            if !fence && line.starts_with("# ") {
                titles += 1;
                if titles == 2 {
                    fault(
                        out,
                        name.clone(),
                        i + 1,
                        "memory.one-fact",
                        "second `# ` title".into(),
                        "split the file, one fact per file",
                    );
                }
            }
        }
    }
}

/// `standards/` names no frontmatter for a template.
fn templates(root: &Path, out: &mut Vec<Fault>) {
    for f in md_files(&root.join("templates"), true) {
        let text = fs::read_to_string(&f).unwrap_or_default();
        if text.lines().next() == Some("---") {
            fault(
                out,
                display(root, &f),
                1,
                "template.frontmatter",
                "template has frontmatter, and standards/ names none".into(),
                "remove the frontmatter block",
            );
        }
    }
}

/// Relative markdown links resolve, and a documented `git merge` carries
/// `--no-edit`. Without it, git opens an editor and waits, which in a shell
/// nobody is watching is indistinguishable from a hang.
fn links(root: &Path, out: &mut Vec<Fault>) {
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
        "memory",
    ] {
        files.extend(md_files(&root.join(d), true));
    }
    for md in files {
        let text = fs::read_to_string(&md).unwrap_or_default();
        let name = display(root, &md);
        for (i, line) in text.lines().enumerate() {
            for target in link_targets(line) {
                let target = target.split('#').next().unwrap_or("").trim();
                if target.is_empty()
                    || target.starts_with("http://")
                    || target.starts_with("https://")
                    || target.starts_with("mailto:")
                {
                    continue;
                }
                if !md.parent().unwrap_or(root).join(target).exists() {
                    fault(
                        out,
                        name.clone(),
                        i + 1,
                        "link.relative",
                        format!("link target {target} does not exist"),
                        "point the link at a file that exists",
                    );
                }
            }
            if merge_without_no_edit(line) {
                fault(
                    out,
                    name.clone(),
                    i + 1,
                    "link.merge-no-edit",
                    "a documented merge command lacks --no-edit".into(),
                    "add --no-edit to the command",
                );
            }
        }
    }
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

/// Principles plus every rule file, against the cap. A fork may set
/// `budget.always_on_bytes` in `context/upstream.json`.
fn always_on(home: &Home, out: &mut Vec<Fault>) -> Cap {
    let root = &home.dir;
    let set = fs::read_to_string(root.join("context").join("upstream.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|j| j["budget"]["always_on_bytes"].as_u64());
    let (bytes, source) = match set {
        Some(b) => (b, "cap from context/upstream.json"),
        None => (ALWAYS_ON_CAP, "default cap"),
    };
    let files: Vec<(PathBuf, u64)> = home
        .always_on()
        .into_iter()
        .map(|f| {
            let n = fs::metadata(&f).map(|m| m.len()).unwrap_or(0);
            (f, n)
        })
        .collect();
    let used = files.iter().map(|(_, n)| n).sum();
    if used > bytes {
        if let Some((largest, _)) = files.iter().max_by_key(|(_, n)| *n) {
            fault(
                out,
                display(root, largest),
                1,
                "budget.always-on",
                format!("principles and rules hold {used} bytes, {source} is {bytes}"),
                "move text out of the always-on files, or raise budget.always_on_bytes in context/upstream.json",
            );
        }
    }
    Cap {
        bytes,
        used,
        source,
    }
}

/// The template ships at most three skills, and nothing in `memory/` or
/// `decisions/` beyond a README.
fn template_budget(root: &Path, out: &mut Vec<Fault>) {
    for dir in skill_dirs(root).iter().skip(TEMPLATE_SKILLS) {
        fault(
            out,
            display(root, &dir.join("SKILL.md")),
            1,
            "budget.skills",
            format!("the template ships more than {TEMPLATE_SKILLS} skills"),
            "move the skill out of the template",
        );
    }
    for d in ["memory", "decisions"] {
        let dir = root.join(d);
        let mut files: Vec<PathBuf> = walkdir::WalkDir::new(&dir)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .filter(|p| {
                p.parent() != Some(dir.as_path())
                    || !p
                        .file_name()
                        .is_some_and(|n| n == "README.md" || n == ".gitkeep")
            })
            .collect();
        files.sort();
        for f in files {
            fault(
                out,
                display(root, &f),
                1,
                "budget.template-files",
                format!("the template ships a file in {d}/"),
                "move the file out of the template",
            );
        }
    }
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

/// Relative to the home, with `/` on every platform, so output is the same
/// everywhere.
fn display(root: &Path, f: &Path) -> String {
    let rel = f.strip_prefix(root).unwrap_or(f);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
