//! `agents check` fails on any file the standard cannot parse. Each
//! directory under `fixtures/` is a home that breaks one rule, named after
//! the rule.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn check(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agents"))
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
    assert_eq!(seen, 23, "one fixture per rule");
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

/// Copy the template's always-on files and skills, the parts the budget reads.
fn template_copy() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let root = repo_root();
    for e in walkdir::WalkDir::new(&root)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| {
            let n = e.file_name().to_string_lossy();
            !(n == "target" || n == ".git" || n == ".claude")
        })
    {
        let e = e.unwrap();
        let to = t.path().join(e.path().strip_prefix(&root).unwrap());
        if e.file_type().is_dir() {
            fs::create_dir_all(&to).unwrap();
        } else if e.file_type().is_file() {
            fs::copy(e.path(), &to).unwrap();
        }
    }
    t
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
    assert!(text.contains(" budget.always-on: "), "{text}");
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
