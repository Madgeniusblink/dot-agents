//! Integration tests for docs/specs/2026-11-02-deterministic-core.md behaviors.

use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

fn bin() -> String {
    env!("CARGO_BIN_EXE_dot-agents").to_string()
}

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().expect("repo root").to_path_buf()
}

#[test]
fn b1_install_writes_core_and_var_gitignore() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path();
    let repo = repo_root();
    let status = Command::new(bin())
        .env("DOT_AGENTS_HOME", home)
        .env_remove("DOT_AGENTS_SOURCE")
        .current_dir(&repo)
        .args(["install"])
        .status()
        .expect("run install");
    assert!(status.success(), "install failed");
    for rel in [
        "AGENTS.md",
        "standards/README.md",
        "context/writing-standard.md",
        "var/scratch/.gitignore",
        "var/worktrees/.gitignore",
    ] {
        assert!(home.join(rel).is_file(), "missing {rel}");
    }
}

#[test]
fn b2_doctor_fails_then_passes_after_install() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path();
    let fail = Command::new(bin())
        .env("DOT_AGENTS_HOME", home)
        .arg("doctor")
        .status()
        .expect("doctor");
    assert!(!fail.success());
    assert!(
        Command::new(bin())
            .env("DOT_AGENTS_HOME", home)
            .env_remove("DOT_AGENTS_SOURCE")
            .current_dir(repo_root())
            .arg("install")
            .status()
            .expect("install")
            .success()
    );
    let ok = Command::new(bin())
        .env("DOT_AGENTS_HOME", home)
        .arg("doctor")
        .status()
        .expect("doctor ok");
    assert!(ok.success());
}

#[test]
fn b3_status_shows_template_version() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path();
    Command::new(bin())
        .env("DOT_AGENTS_HOME", home)
        .env_remove("DOT_AGENTS_SOURCE")
        .arg("install")
        .status()
        .expect("install");
    let out = Command::new(bin())
        .env("DOT_AGENTS_HOME", home)
        .arg("status")
        .output()
        .expect("status");
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let ver = env!("CARGO_PKG_VERSION");
    assert!(stdout.contains(&format!("dot-agents {ver}")));
    assert!(stdout.contains(&format!("Template: v{ver}")));
}

#[test]
fn b4_paths_honors_env() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path().join("home");
    let scratch = tmp.path().join("scratch");
    let wt = tmp.path().join("wt");
    fs::create_dir_all(&home).unwrap();
    let out = Command::new(bin())
        .env("DOT_AGENTS_HOME", &home)
        .env("DOT_AGENTS_SCRATCH", &scratch)
        .env("DOT_AGENTS_WORKTREES", &wt)
        .args(["paths"])
        .output()
        .expect("paths");
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains(&format!("home:      {}", home.display())));
    assert!(stdout.contains(&format!("scratch:   {}", scratch.display())));
    assert!(stdout.contains(&format!("worktrees: {}", wt.display())));
}

#[test]
fn b8_bootstrap_file_exists() {
    let root = repo_root();
    let p = root.join("bootstrap/cursor-user-rules.md");
    assert!(p.is_file(), "missing bootstrap/cursor-user-rules.md");
    let body = fs::read_to_string(p).unwrap();
    assert!(body.contains("~/.agents/AGENTS.md"));
    assert!(body.contains("DOT_AGENTS_SCRATCH"));
}
