//! `scan` finds every tool settings file, and `scan --from` says which
//! apply from a directory, in order, what each changes, and where a deny
//! blocks an allow. It never writes.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// A user home with each tool's user file, and a checkout with a nested
/// project. One project file is broken JSON, one is empty, one hook names a
/// missing script, and one deny blocks an allow in the user file.
fn fixture() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let h = t.path();
    write(
        h,
        ".claude/settings.json",
        r#"{"permissions": {"allow": ["Bash(git push)", "Bash(ls)"]}, "model": "big"}"#,
    );
    write(
        h,
        ".codex/config.toml",
        "model = \"m1\"\nmodel_reasoning_effort = \"high\"\n",
    );
    write(h, ".cursor/mcp.json", "{\"mcpServers\": {}}\n");
    let r = h.join("work/app");
    fs::create_dir_all(r.join(".git")).unwrap();
    write(&r, ".mcp.json", "");
    write(
        &r,
        ".claude/settings.json",
        "{\n  \"permissions\": {\n    \"deny\": [\"Bash(git push)\"]\n  },\n  \"env\": {\"API_URL\": \"x\"}\n}\n",
    );
    write(&r, ".claude/settings.local.json", "{\n  \"model\": \n");
    write(&r, ".codex/config.toml", "model = \"m2\"\n");
    write(&r, ".cursor/mcp.json", "{\"mcpServers\": {}}\n");
    write(
        &r,
        "pkg/.claude/settings.json",
        r#"{"hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "/nowhere/guard.sh --strict"}]}]}, "effortLevel": "low"}"#,
    );
    write(
        &r,
        "pkg/.codex/config.toml",
        "model = [\n[shell_environment_policy.set]\nTOKEN_PATH = \"x\"\n",
    );
    t
}

fn agents(home: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_agents"))
        .args(args)
        .current_dir(home)
        .env("HOME", home)
        .env("COLUMNS", "400")
        .env("EDITOR", "vi")
        .env("AGENTS_UPDATE_CHECK", "off")
        .env_remove("AGENTS_HOME")
        .env_remove("AGENTS_CONFIG")
        .env_remove("CODEX_HOME")
        .output()
        .expect("agents");
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Every file under the tree, with its bytes, so a test can show nothing changed.
fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = walkdir(root);
    v.sort();
    v
}

fn walkdir(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut v = Vec::new();
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            v.extend(walkdir(&p));
        } else {
            v.push((p.display().to_string(), fs::read(&p).unwrap()));
        }
    }
    v
}

/// B1: every settings file, with scope, size, and validity.
#[test]
fn scan_lists_every_settings_file_with_its_validity() {
    let t = fixture();
    let h = t.path();
    let root = h.to_string_lossy().into_owned();
    let out = agents(h, &["scan", "--long", "--root", &root]);
    let row = |path: &str| {
        out.lines()
            .find(|l| l.ends_with(path))
            .unwrap_or_else(|| panic!("{path}: {out}"))
            .to_string()
    };
    assert!(row("~/.claude/settings.json").contains("global  valid"));
    assert!(row("~/.codex/config.toml").contains("global  valid"));
    assert!(row("~/.cursor/mcp.json").contains("global  valid"));
    assert!(row("~/work/app/.mcp.json").contains("project  empty"));
    assert!(row("~/work/app/.claude/settings.json").contains("project  valid"));
    let local = row("~/work/app/.claude/settings.local.json");
    assert!(local.contains("invalid at line 3"), "{local}");
    assert!(row("~/work/app/.codex/config.toml").contains("project  valid"));
    assert!(row("~/work/app/.cursor/mcp.json").contains("project  valid"));
    assert!(row("~/work/app/pkg/.codex/config.toml").contains("invalid at line"));
    assert!(out.contains("tool settings files (10):"), "{out}");

    let brief = agents(h, &["scan", "--root", &root]);
    assert!(
        brief.contains("tool settings files: 10, 2 invalid"),
        "{brief}"
    );

    let doc: Value =
        serde_json::from_str(&agents(h, &["scan", "--json", "--root", &root])).unwrap();
    let local = doc["settings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["path"] == "~/work/app/.claude/settings.local.json")
        .unwrap();
    assert_eq!(local["validity"], "invalid");
    assert_eq!(local["error_line"], 3);
    assert_eq!(local["scope"], "project");
    assert_eq!(local["project"], "~/work/app");
}

/// B2 to B5: the order from a nested directory, the entries, `dead`,
/// `conflict`, and the one edit line last. Nothing is written.
#[test]
fn from_a_nested_directory_prints_the_order_and_the_entries() {
    let t = fixture();
    let h = t.path();
    let before = snapshot(h);
    let pkg = h.join("work/app/pkg").to_string_lossy().into_owned();
    let out = agents(h, &["scan", "--from", &pkg]);
    let paths: Vec<&str> = out
        .lines()
        .filter(|l| l.ends_with("  applies") || l.ends_with("  broken"))
        .map(|l| l.split("  ").nth(3).unwrap())
        .collect();
    // The local file sits at the checkout root, except on Windows, where it
    // stays in the directory the session starts in.
    let local = if cfg!(windows) {
        vec![]
    } else {
        vec!["~/work/app/.claude/settings.local.json"]
    };
    let rest = [
        "~/.codex/config.toml",
        "~/work/app/.codex/config.toml",
        "~/work/app/pkg/.codex/config.toml",
        "~/.cursor/mcp.json",
        "~/work/app/.cursor/mcp.json",
    ];
    let mut want = vec![
        "~/.claude/settings.json",
        "~/work/app/.mcp.json",
        "~/work/app/pkg/.claude/settings.json",
    ];
    want.extend(local);
    want.extend(rest);
    assert_eq!(paths, want, "{out}");
    assert!(
        out.contains("invalid at line 2  ~/work/app/pkg/.codex/config.toml  broken"),
        "{out}"
    );
    assert!(out.contains("cursor, order unknown (2):"), "{out}");
    assert!(
        out.contains("hook PreToolUse: /nowhere/guard.sh --strict  dead"),
        "{out}"
    );
    assert!(out.contains("effort: low"), "{out}");
    assert!(out.contains("allow: 2 entries"), "{out}");
    assert!(out.contains("model: big"), "{out}");
    assert!(out.contains("effort: high"), "{out}");
    // The project's shared file applies from the root, not from pkg.
    assert!(!out.contains("env: API_URL"), "{out}");

    let app = h.join("work/app").to_string_lossy().into_owned();
    let out = agents(h, &["scan", "--from", &app]);
    assert!(out.contains("deny: Bash(git push)"), "{out}");
    assert!(out.contains("env: API_URL"), "{out}");
    assert!(
        out.contains("conflict: Bash(git push) is denied in ~/work/app/.claude/settings.json and allowed in ~/.claude/settings.json"),
        "{out}"
    );
    let last = out.trim_end().lines().last().unwrap();
    assert_eq!(last, "$EDITOR ~/work/app/.claude/settings.json");
    assert_eq!(snapshot(h), before);
}

/// B6: `--json` carries every field.
#[test]
fn from_json_carries_every_field() {
    let t = fixture();
    let h = t.path();
    let app = h.join("work/app").to_string_lossy().into_owned();
    let doc: Value = serde_json::from_str(&agents(h, &["scan", "--from", &app, "--json"])).unwrap();
    assert_eq!(doc["command"], "scan");
    assert_eq!(doc["from"], "~/work/app");
    assert_eq!(doc["checkout"], "~/work/app");
    let tools = doc["tools"].as_array().unwrap();
    assert_eq!(tools[0]["tool"], "claude code");
    assert_eq!(tools[2]["order"], "unknown");
    let shared = &tools[0]["files"][2];
    assert_eq!(shared["path"], "~/work/app/.claude/settings.json");
    assert_eq!(shared["applies"], true);
    assert_eq!(shared["deny"][0], "Bash(git push)");
    assert_eq!(shared["env"][0], "API_URL");
    assert_eq!(tools[0]["files"][0]["allow"], 2);
    assert_eq!(tools[0]["files"][3]["error_line"], 3);
    assert_eq!(
        doc["conflicts"][0]["deny"],
        "~/work/app/.claude/settings.json"
    );
    assert_eq!(doc["conflicts"][0]["allow"], "~/.claude/settings.json");
    assert_eq!(tools[0]["files"][3]["applies"], false);
    assert_eq!(doc["edit"], "$EDITOR ~/work/app/.claude/settings.json");
}
