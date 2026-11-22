//! `agents check` fails on any file the standard cannot parse. Each
//! directory under `fixtures/` is a home that breaks one rule, named after
//! the rule. The two rules that compare a home with a base ref need git
//! history, so their tests build a repository.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// `home/` in this repository: the tree a home is a checkout of.
fn template_home() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("home")
}

fn check(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agents"))
        .env("AGENTS_UPDATE_CHECK", "off")
        .arg("--home")
        .arg(home)
        .arg("check")
        .args(args)
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn each_fixture_fails_on_its_rule() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut seen = 0;
    for e in fs::read_dir(&dir).unwrap().flatten() {
        let rule = e.file_name().to_string_lossy().into_owned();
        let out = check(&e.path(), &["--template"]);
        let text = stdout(&out);
        assert_eq!(out.status.code(), Some(1), "{rule}:\n{text}");
        assert!(
            text.lines().any(|l| l.contains(&format!(" {rule}: "))),
            "{rule} not caught:\n{text}"
        );
        seen += 1;
    }
    assert_eq!(seen, 23, "one fixture per rule that needs no base ref");
}

#[test]
fn a_failing_run_prints_one_line_per_fault_and_nothing_else() {
    let t = tempfile::tempdir().unwrap();
    fs::write(t.path().join("AGENTS.md"), "# Agent rules\n").unwrap();
    fs::create_dir(t.path().join("rules")).unwrap();
    fs::write(
        t.path().join("rules").join("corrections.md"),
        "# Corrections\n\n- No test here.\n- See [x](gone.md). `2026-11 · a task`\n",
    )
    .unwrap();
    let out = check(t.path(), &[]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        stdout(&out),
        "rules/corrections.md:3 correction.dated: correction does not end in a dated test. \
         fix: end the line with `YYYY-MM · task that proves it`\n\
         rules/corrections.md:4 link.relative: link target gone.md does not exist. \
         fix: point the link at a file that exists\n"
    );
}

/// Copy the template's home, the tree the budget reads.
fn template_copy() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    copy_template(t.path());
    t
}

fn copy_template(to: &Path) {
    let root = template_home();
    for e in walkdir::WalkDir::new(&root)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| {
            let n = e.file_name().to_string_lossy();
            !(n == "target" || n == ".git" || n == ".claude")
        })
    {
        let e = e.unwrap();
        let to = to.join(e.path().strip_prefix(&root).unwrap());
        if e.file_type().is_dir() {
            fs::create_dir_all(&to).unwrap();
        } else if e.file_type().is_file() {
            fs::copy(e.path(), &to).unwrap();
        }
    }
}

#[test]
fn the_template_fits_the_budget_and_1_kib_more_does_not() {
    let t = template_copy();
    let out = check(t.path(), &["--template"]);
    let text = stdout(&out);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("of 12288 bytes (default cap)"), "{text}");

    let rule = t.path().join("rules").join("boundaries.md");
    let mut body = fs::read_to_string(&rule).unwrap();
    let used: usize = [
        "principles.md",
        "rules/boundaries.md",
        "rules/corrections.md",
    ]
    .iter()
    .map(|f| fs::metadata(t.path().join(f)).unwrap().len() as usize)
    .sum();
    // 1 KiB more, or enough to pass the cap if the tree has grown since.
    let add = 1024.max(12 * 1024 + 1 - used.min(12 * 1024));
    body.push_str(&format!("- {}\n", "x".repeat(add)));
    fs::write(&rule, body).unwrap();
    let text = stdout(&check(t.path(), &["--template"]));
    let line = text
        .lines()
        .find(|l| l.contains(" budget.always-on: "))
        .unwrap_or_else(|| panic!("no budget line in:\n{text}"));
    assert!(
        line.ends_with(". fix: remove words from the file that owns the topic"),
        "{line}"
    );
}

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .status()
        .expect("git")
        .success();
    assert!(ok, "git {args:?}");
}

/// A repository whose `sub` directory is a copy of the template's home, in
/// one commit. The template repository keeps its home in `home/`.
fn repo(sub: &str) -> (tempfile::TempDir, PathBuf) {
    let t = tempfile::tempdir().unwrap();
    let home = t.path().join(sub);
    fs::create_dir_all(&home).unwrap();
    copy_template(&home);
    git(t.path(), &["init", "-q"]);
    git(t.path(), &["add", "-A"]);
    git(t.path(), &["commit", "-qm", "base"]);
    (t, home)
}

fn set_cap(home: &Path, cap: Option<u64>) {
    let path = home.join("context").join("upstream.json");
    let mut json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let doc = json.as_object_mut().unwrap();
    match cap {
        Some(n) => doc.insert("budget".into(), serde_json::json!({"always_on_bytes": n})),
        None => doc.remove("budget"),
    };
    fs::write(&path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
}

#[test]
fn a_cap_higher_than_at_the_base_fails() {
    let (t, home) = repo("");
    set_cap(&home, Some(20000));
    let out = check(&home, &["--base", "HEAD"]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    let line = text
        .lines()
        .find(|l| l.contains(" budget.raised: "))
        .unwrap_or_else(|| panic!("no budget.raised in:\n{text}"));
    assert!(
        line.starts_with("context/upstream.json:")
            && line.contains("budget.always_on_bytes is 20000, and 12288 at HEAD")
            && !line.split(". fix: ").nth(1).unwrap().contains("raise"),
        "{line}"
    );

    // A lower cap passes, and removing it again raises it to the default.
    set_cap(&home, Some(12000));
    let out = check(&home, &["--base", "HEAD"]);
    assert!(out.status.success(), "{}", stdout(&out));
    git(t.path(), &["commit", "-qam", "lower the cap"]);
    set_cap(&home, None);
    let text = stdout(&check(&home, &["--base", "HEAD"]));
    assert!(
        text.contains(" budget.raised: budget.always_on_bytes is 12288, and 12000 at HEAD."),
        "{text}"
    );
}

const TASK: &str = "ask for a config value that is not in the repo";

/// Drop the correction whose test is `TASK`, and reword the line above it
/// with its test kept.
fn drop_task(home: &Path) {
    let path = home.join("rules").join("corrections.md");
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains(TASK), "the template no longer has the task");
    let kept: Vec<String> = text
        .lines()
        .filter(|l| !l.contains(TASK))
        .map(|l| {
            l.replace(
                "- Pushed, merged, deployed,",
                "- Pushed, merged, and deployed,",
            )
        })
        .collect();
    fs::write(&path, kept.join("\n") + "\n").unwrap();
}

#[test]
fn a_dropped_test_fails_until_a_commit_retires_it() {
    // Below the repository root, where git lists a tree only with --full-tree.
    let (t, home) = repo("home");
    drop_task(&home);
    let out = check(&home, &["--base", "HEAD"]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    let faults: Vec<&str> = text
        .lines()
        .filter(|l| l.contains(" proof.dropped: "))
        .collect();
    assert_eq!(
        faults.len(),
        1,
        "only the dropped test, not the reworded line:\n{text}"
    );
    assert!(
        faults[0].starts_with("rules/corrections.md:")
            && faults[0].contains(&format!("`2026-11 · {TASK}` is at HEAD")),
        "{text}"
    );

    git(t.path(), &["commit", "-qam", "docs: drop a correction"]);
    let other = "Retired: some other task";
    git(
        t.path(),
        &[
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "docs: retire",
            "-m",
            other,
        ],
    );
    let out = check(&home, &["--base", "HEAD~2"]);
    assert_eq!(out.status.code(), Some(1), "{}", stdout(&out));

    let retired = format!("Retired: {TASK}");
    git(
        t.path(),
        &[
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "docs: retire",
            "-m",
            &retired,
        ],
    );
    let out = check(&home, &["--base", "HEAD~3"]);
    let text = stdout(&out);
    assert!(out.status.success(), "{text}");
    assert!(text.contains(", base HEAD~3 ("), "{text}");
}

#[test]
fn a_base_that_is_not_a_commit_is_an_error() {
    let (_t, home) = repo("");
    let out = check(&home, &["--base", "no-such-ref"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--base no-such-ref is not a commit"));
}

#[test]
fn json_lists_the_faults_with_the_same_fields() {
    let home = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/correction.dated");
    let out = check(&home, &["--json"]);
    assert_eq!(out.status.code(), Some(1));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["command"], "check");
    assert_eq!(doc["ok"], false);
    let f = &doc["faults"][0];
    assert_eq!(f["path"], "rules/corrections.md");
    assert_eq!(f["line"], 3);
    assert_eq!(f["rule"], "correction.dated");
    assert!(f["message"].is_string() && f["fix"].is_string());
}
