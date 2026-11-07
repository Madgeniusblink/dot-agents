//! The once-a-day update warning: read from the cache, printed on stderr,
//! and silent when main has not moved or the check is off.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn home() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".agents")).unwrap();
    fs::write(tmp.path().join(".agents/AGENTS.md"), "# home\n").unwrap();
    tmp
}

fn cache(home: &Path, ahead: u64) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let dir = home.join(".cache/agents");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("update.json"),
        format!(
            r#"{{"build":"{}","checked":{now},"ahead":{ahead}}}"#,
            env!("AGENTS_COMMIT")
        ),
    )
    .unwrap();
}

fn stderr(home: &Path, check: Option<&str>) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_agents"));
    cmd.args(["--home"])
        .arg(home.join(".agents"))
        .arg("tree")
        .env("HOME", home)
        .env_remove("CI")
        .env_remove("AGENTS_HOME")
        .env_remove("AGENTS_CONFIG");
    match check {
        Some(v) => cmd.env("AGENTS_UPDATE_CHECK", v),
        None => cmd.env_remove("AGENTS_UPDATE_CHECK"),
    };
    let out = cmd.output().expect("agents");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn warns_from_the_cache_when_main_is_ahead() {
    let tmp = home();
    cache(tmp.path(), 3);
    let err = stderr(tmp.path(), None);
    assert!(
        err.contains(&format!(
            "agents: built from {}; main is 3 commits ahead. Update: cargo install --git",
            env!("AGENTS_COMMIT")
        )),
        "{err}"
    );
}

#[test]
fn stays_silent_when_current_or_off() {
    let tmp = home();
    cache(tmp.path(), 0);
    assert_eq!(stderr(tmp.path(), None), "");
    cache(tmp.path(), 3);
    assert_eq!(stderr(tmp.path(), Some("off")), "");
}

#[test]
fn status_and_json_carry_the_build() {
    let tmp = home();
    cache(tmp.path(), 2);
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_agents"))
            .arg("--home")
            .arg(tmp.path().join(".agents"))
            .args(args)
            .env("HOME", tmp.path())
            .env_remove("CI")
            .env_remove("AGENTS_UPDATE_CHECK")
            .env_remove("AGENTS_HOME")
            .env_remove("AGENTS_CONFIG")
            .output()
            .expect("agents");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let text = run(&["status"]);
    assert!(
        text.contains(&format!(
            "agents {} ({}), main is 2 commits ahead\n",
            env!("CARGO_PKG_VERSION"),
            env!("AGENTS_COMMIT")
        )),
        "{text}"
    );
    let doc: serde_json::Value = serde_json::from_str(&run(&["status", "--json"])).unwrap();
    assert_eq!(doc["build"]["commit"], env!("AGENTS_COMMIT"));
    assert_eq!(doc["build"]["main_ahead"], 2);
    let off = Command::new(env!("CARGO_BIN_EXE_agents"))
        .arg("--home")
        .arg(tmp.path().join(".agents"))
        .args(["status", "--json"])
        .env("HOME", tmp.path())
        .env("AGENTS_UPDATE_CHECK", "off")
        .output()
        .expect("agents");
    let doc: serde_json::Value = serde_json::from_slice(&off.stdout).unwrap();
    assert!(doc["build"]["main_ahead"].is_null(), "{doc}");
}

#[test]
fn version_names_the_commit() {
    let out = Command::new(env!("CARGO_BIN_EXE_agents"))
        .env("AGENTS_UPDATE_CHECK", "off")
        .arg("--version")
        .output()
        .expect("agents");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.trim()
            .ends_with(&format!("({})", env!("AGENTS_COMMIT"))),
        "{text}"
    );
}
