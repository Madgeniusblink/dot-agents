//! A skill inside a git checkout belongs to that project. The CLI lists it,
//! reports copies of it in user directories, and never writes inside the
//! checkout.

use std::fs;
use std::path::Path;
use std::process::Command;

fn skill(dir: &Path, name: &str, extra: &str, body: &str) {
    let d = dir.join(name);
    fs::create_dir_all(&d).unwrap();
    fs::write(
        d.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: The {name} skill.\n{extra}---\n\n{body}\n"),
    )
    .unwrap();
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A user home and a committed checkout with one skill in each of the four
/// places, plus two copies in the user's skills: one byte for byte, one
/// rendered on the way that declares its source.
fn fixture() -> (tempfile::TempDir, tempfile::TempDir) {
    let (home, repo) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let r = repo.path();
    git(r, &["init", "-q"]);
    fs::write(r.join("AGENTS.md"), "# a project\n").unwrap();
    skill(&r.join(".claude/skills"), "alpha", "", "Alpha body.");
    skill(&r.join(".agents/skills"), "beta", "", "Beta body.");
    skill(&r.join(".codex/skills"), "gamma", "", "Gamma body.");
    skill(&r.join("skills"), "delta", "", "Delta body.");
    git(r, &["add", "-A"]);
    git(r, &["commit", "-qm", "init"]);

    let user = home.path().join(".claude/skills");
    skill(&user, "alpha", "", "Alpha body.");
    skill(
        &user,
        "beta",
        "metadata:\n  source: \"https://example.com/team/app/blob/main/.agents/skills/beta/SKILL.md\"\n",
        "<!-- rendered by an installer -->\nBeta body.",
    );
    skill(
        &user,
        "ghost",
        "metadata:\n  source: /nowhere/ghost\n",
        "Ghost body.",
    );
    (home, repo)
}

fn agents(home: &Path, cwd: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_agents"))
        .args(args)
        .current_dir(cwd)
        .env("HOME", home)
        .env("COLUMNS", "400")
        .env_remove("AGENTS_HOME")
        .env_remove("CODEX_HOME")
        .output()
        .expect("agents");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn skills_lists_the_project_and_reports_copies() {
    let (home, repo) = fixture();
    let out = agents(home.path(), repo.path(), &["skills"]);

    // B13: one row from each of the four places, in a project section.
    let section = out.split("\nproject ").nth(1).expect("a project section");
    assert!(section.contains("(4):"), "{out}");
    for name in ["alpha", "beta", "gamma", "delta"] {
        assert!(section.contains(name), "{name} missing: {out}");
    }

    // B16: an identical body and a declared source, and a source path that
    // names no file.
    assert!(
        out.contains("copy of .claude/skills/alpha (identical)"),
        "{out}"
    );
    assert!(
        out.contains("copy of .agents/skills/beta (declared)"),
        "{out}"
    );
    assert!(out.contains("source missing"), "{out}");
    let long = agents(home.path(), repo.path(), &["skills", "--long"]);
    assert!(
        long.contains("copy of .agents/skills/beta (declared)"),
        "{long}"
    );

    // B17: a name in two scopes says each scope and whether the bodies match.
    let same = out
        .split("same name in more than one place")
        .nth(1)
        .unwrap();
    let alpha = same.lines().find(|l| l.contains("alpha")).unwrap();
    assert!(
        alpha.contains("identical") && alpha.contains("yours ") && alpha.contains("project "),
        "{alpha}"
    );
    let beta = same.lines().find(|l| l.contains("beta")).unwrap();
    assert!(
        beta.contains("DIFFERENT bodies") && beta.contains("project "),
        "{beta}"
    );
}

#[test]
fn scan_counts_the_project_skills() {
    let (home, repo) = fixture();
    let root = repo.path().to_string_lossy().into_owned();
    let out = agents(home.path(), repo.path(), &["scan", "--root", &root]);
    assert!(
        out.contains("skills: .claude/skills 1, .agents/skills 1, .codex/skills 1, skills 1"),
        "{out}"
    );
}

#[test]
fn remove_and_keep_never_write_inside_the_checkout() {
    let (home, repo) = fixture();
    let (h, r) = (home.path(), repo.path());
    for args in [
        &["skills", "remove", "alpha", "delta"][..],
        &["skills", "remove", "alpha", "delta", "--apply"],
        &["skills", "keep", "beta"],
        &["skills", "keep", "beta", "--apply"],
    ] {
        let out = agents(h, r, args);
        assert!(out.contains(": the project owns it."), "{args:?}: {out}");
        assert_eq!(git(r, &["status", "--porcelain"]), "", "{args:?}");
    }
    // The user's own copy of alpha did move aside.
    assert!(!h.join(".claude/skills/alpha").exists());
    assert!(r.join(".claude/skills/alpha/SKILL.md").is_file());
}
