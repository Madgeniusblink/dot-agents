//! `agents ui` opens only on a terminal. Anywhere else it prints one line
//! on stderr and exits 1, and writes nothing on stdout.

use std::process::{Command, Stdio};

fn ui() -> std::process::Output {
    let t = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_agents"))
        .env("AGENTS_UPDATE_CHECK", "off")
        .env("HOME", t.path())
        .env_remove("AGENTS_HOME")
        .env_remove("AGENTS_CONFIG")
        .arg("ui")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

#[test]
fn t2_ui_without_a_terminal_is_one_line_and_exit_1() {
    let out = ui();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        out.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(err.lines().count(), 1, "{err}");
    if cfg!(unix) {
        assert!(err.contains("needs a terminal"), "{err}");
    } else {
        assert!(err.contains("Unix only"), "{err}");
    }
}
