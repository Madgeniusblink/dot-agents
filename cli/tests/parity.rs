//! The shell scripts stay until the CLI has shipped on every channel. Until
//! then each script and its command run against the same temporary `$HOME`
//! and any difference in the resulting tree is a failure.
//!
//! The round-trip tests need `sh`, so they run on unix only. On Windows the
//! helpers below are unused, and that is expected, not a warning.
#![cfg_attr(not(unix), allow(dead_code))]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `home/` in this repository: the tree a home is a checkout of.
fn template_home() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("home")
}

#[derive(Debug, PartialEq)]
enum Node {
    Dir,
    File(String),
    Link(PathBuf),
}

/// Every path under `root`, with symlink targets canonicalized so two
/// spellings of one directory compare equal.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Node> {
    let mut out = BTreeMap::new();
    for e in walkdir::WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .flatten()
    {
        let p = e.path();
        let rel = p.strip_prefix(root).unwrap().to_path_buf();
        let node = if e.path_is_symlink() {
            let t = fs::read_link(p).unwrap();
            Node::Link(fs::canonicalize(&t).unwrap_or(t))
        } else if e.file_type().is_dir() {
            Node::Dir
        } else {
            Node::File(fs::read_to_string(p).unwrap_or_default())
        };
        out.insert(rel, node);
    }
    out
}

/// A tool directory for each of Claude Code, Codex, Grok and Cursor, and a
/// CLAUDE.md that holds the user's own content, an unrelated import, and one
/// legacy line an earlier wire.sh appended.
fn fixture() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    fs::create_dir_all(t.path().join(".claude")).unwrap();
    fs::create_dir_all(t.path().join(".codex")).unwrap();
    fs::create_dir_all(t.path().join(".grok")).unwrap();
    fs::create_dir_all(t.path().join(".cursor")).unwrap();
    fs::write(
        t.path().join(".claude").join("CLAUDE.md"),
        "# my own content\n@~/other.md\n@/elsewhere/.agents/principles.md\n",
    )
    .unwrap();
    t
}

fn script(home: &Path, args: &[&str]) -> String {
    let out = Command::new("sh")
        .arg(template_home().join("scripts").join("wire.sh"))
        .args(args)
        .env("HOME", home)
        .env_remove("CODEX_HOME")
        .env_remove("GROK_HOME")
        .output()
        .expect("sh");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn cli(home: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_agents"))
        .env("AGENTS_UPDATE_CHECK", "off")
        .arg("--home")
        .arg(template_home())
        .args(args)
        .env("HOME", home)
        .env_remove("CODEX_HOME")
        .env_remove("GROK_HOME")
        .env_remove("AGENTS_HOME")
        .env_remove("AGENTS_CONFIG")
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
#[cfg(unix)]
fn wire_and_unwire_match_the_scripts() {
    let (a, b) = (fixture(), fixture());

    script(a.path(), &["--apply"]);
    let cli_out = cli(b.path(), &["wire", "--apply"]);
    assert_eq!(snapshot(a.path()), snapshot(b.path()), "after wire");
    assert!(cli_out.contains("write .claude/rules/00-agents-home.md"));
    assert!(cli_out.contains("drop 1 import line(s) from .claude/CLAUDE.md"));
    let codex = fs::read_to_string(b.path().join(".codex").join("AGENTS.md")).unwrap();
    assert!(codex.starts_with("<!-- ") && codex.contains("owns this file"));
    assert!(
        codex.contains("# Principles"),
        "the generated file carries the content"
    );
    assert_eq!(
        codex,
        fs::read_to_string(
            b.path()
                .join(".grok")
                .join("rules")
                .join("00-agents-home.md")
        )
        .unwrap(),
        "codex and grok get the same generated file"
    );

    let hooks = fs::read_to_string(b.path().join(".cursor").join("hooks.json")).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&hooks).unwrap();
    assert_eq!(
        doc["hooks"]["sessionStart"][0]["command"].as_str().unwrap(),
        format!("sh {}/scripts/cursor-context.sh", template_home().display()),
        "cursor's sessionStart hook runs the home's script"
    );

    let again = cli(b.path(), &["wire", "--apply"]);
    assert!(again.contains("present: .claude/rules/00-agents-home.md"));
    assert!(again.contains("present: .cursor/hooks.json"));
    assert_eq!(snapshot(a.path()), snapshot(b.path()), "after second wire");

    script(a.path(), &["--remove", "--apply"]);
    cli(b.path(), &["unwire", "--apply"]);
    assert_eq!(snapshot(a.path()), snapshot(b.path()), "after unwire");
    assert_eq!(
        fs::read_to_string(b.path().join(".claude").join("CLAUDE.md")).unwrap(),
        "# my own content\n@~/other.md\n",
        "the user's own lines survive the round trip"
    );
}

/// The user's own Cursor hooks are not ours to touch: `wire` adds one
/// `sessionStart` entry beside them and `unwire` takes only that one out.
/// An entry from a home at another path is replaced, not joined.
/// `unwire` removes the skills link too, and on Windows that unlink of a
/// directory link fails (issue filed), so this runs on unix only.
#[test]
#[cfg(unix)]
fn cursor_hooks_of_the_users_own_survive_wire_and_unwire() {
    let t = fixture();
    let path = t.path().join(".cursor").join("hooks.json");
    let before = r#"{"version": 1, "hooks": {"beforeShellExecution": [{"command": "./guard.sh"}], "sessionStart": [{"command": "sh /old/home/scripts/cursor-context.sh"}, {"command": "./banner.sh"}]}, "extra": true}"#;
    fs::write(&path, before).unwrap();

    let shown = Path::new(".cursor")
        .join("hooks.json")
        .display()
        .to_string();
    let out = cli(t.path(), &["wire", "--apply"]);
    assert!(
        out.contains(&format!("write {shown} (sessionStart hook)")),
        "{out}"
    );
    let doc: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let start = doc["hooks"]["sessionStart"].as_array().unwrap();
    let commands: Vec<&str> = start
        .iter()
        .map(|e| e["command"].as_str().unwrap())
        .collect();
    assert_eq!(
        commands,
        vec![
            "./banner.sh",
            &format!("sh {}/scripts/cursor-context.sh", template_home().display())
        ],
        "the stale entry goes, the user's stays, ours is last"
    );
    assert_eq!(
        doc["hooks"]["beforeShellExecution"][0]["command"],
        "./guard.sh"
    );
    assert_eq!(doc["extra"], true);

    cli(t.path(), &["unwire", "--apply"]);
    let doc: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(doc["hooks"]["sessionStart"].as_array().unwrap().len(), 1);
    assert_eq!(doc["hooks"]["sessionStart"][0]["command"], "./banner.sh");
    assert_eq!(
        doc["hooks"]["beforeShellExecution"][0]["command"],
        "./guard.sh"
    );
    assert_eq!(doc["extra"], true);
}

/// A hooks file that is not JSON is not ours to repair.
#[test]
fn an_unreadable_cursor_hooks_file_is_skipped() {
    let t = fixture();
    let path = t.path().join(".cursor").join("hooks.json");
    fs::write(&path, "{ not json").unwrap();
    let shown = Path::new(".cursor")
        .join("hooks.json")
        .display()
        .to_string();
    let out = cli(t.path(), &["wire", "--apply"]);
    assert!(
        out.contains(&format!("SKIP {shown}: not valid JSON")),
        "{out}"
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), "{ not json");
}

/// What the hook prints is one JSON object whose `additional_context` holds
/// the always-on set, as Cursor's sessionStart hook reads it.
#[test]
#[cfg(unix)]
fn cursor_context_script_prints_the_always_on_set_as_json() {
    let out = Command::new("sh")
        .arg(template_home().join("scripts").join("cursor-context.sh"))
        .output()
        .expect("sh");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("one JSON object");
    let text = doc["additional_context"].as_str().unwrap();
    assert!(text.starts_with("<!-- "), "a header names the owner");
    assert!(text.contains("# Principles"), "principles.md is in it");
    assert!(
        text.contains("rules/") && text.contains("\n<!-- "),
        "each rules file follows under its own path comment"
    );
}

#[test]
#[cfg(unix)]
fn dry_run_writes_nothing() {
    let t = fixture();
    let before = snapshot(t.path());
    let out = cli(t.path(), &["wire"]);
    assert!(out.starts_with("dry run."));
    assert_eq!(before, snapshot(t.path()));
}

/// `wire` already copies the always-on set into the Codex file, so
/// `context.sh` must compare that file's own bytes with the cap, not add
/// the set again. A 17 KB set sits under the 32768 cap once, over it twice.
#[test]
#[cfg(unix)]
fn context_sh_counts_the_codex_file_once_against_its_cap() {
    let t = tempfile::tempdir().unwrap();
    let home = t.path().join("home");
    let user = t.path().join("user");
    fs::create_dir_all(home.join("scripts")).unwrap();
    fs::create_dir_all(home.join("rules")).unwrap();
    fs::create_dir_all(user.join(".codex")).unwrap();
    fs::copy(
        template_home().join("scripts").join("context.sh"),
        home.join("scripts").join("context.sh"),
    )
    .unwrap();
    let always_on = "x".repeat(17_000) + "\n";
    fs::write(home.join("principles.md"), &always_on).unwrap();
    fs::write(
        user.join(".codex").join("AGENTS.md"),
        format!("<!-- dot-agents owns this file -->\n{always_on}"),
    )
    .unwrap();
    let out = Command::new("sh")
        .arg(home.join("scripts").join("context.sh"))
        .env("HOME", &user)
        .env_remove("CODEX_HOME")
        .output()
        .expect("sh");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("project_doc_max_bytes"),
        "17 KB counted once is under the cap:\n{stdout}"
    );
}

#[test]
fn check_passes_on_this_tree() {
    let out = Command::new(env!("CARGO_BIN_EXE_agents"))
        .env("AGENTS_UPDATE_CHECK", "off")
        .arg("--home")
        .arg(template_home())
        .arg("check")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn crate_version_is_the_template_version() {
    let text = fs::read_to_string(template_home().join("context").join("upstream.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        json["template_version"].as_str().unwrap(),
        env!("CARGO_PKG_VERSION"),
        "cli/Cargo.toml and home/context/upstream.json must carry the same version"
    );
}

/// A skill whose name is not its directory, with an owner, one reference
/// that resolves and one that does not.
fn broken_skill(skills: &Path) {
    let dir = skills.join("data_query");
    fs::create_dir_all(dir.join("references")).unwrap();
    fs::write(dir.join("references").join("GOOD.md"), "ok\n").unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: data-query\ndescription: Query the example database.\n\
         metadata:\n  owner: example-team\n---\n\
         Read [the guide](references/GOOD.md) and references/MISSING.md.\n",
    )
    .unwrap();
}

#[test]
fn skills_marks_a_broken_skill_and_long_says_why() {
    let t = fixture();
    broken_skill(&t.path().join(".claude").join("skills"));
    let row = cli(t.path(), &["skills"]);
    assert!(
        row.lines()
            .any(|l| l.contains("data-query") && l.contains("BROKEN")),
        "{row}"
    );
    let long = cli(t.path(), &["skills", "--long", "--long"]);
    for want in [
        "error name-matches-dir: name is data-query, directory is data_query",
        "owner: example-team",
        "broken reference: references/MISSING.md",
    ] {
        assert!(long.contains(want), "{want} not in:\n{long}");
    }
}

#[test]
fn check_fails_on_a_skill_name_that_is_not_its_directory() {
    let t = tempfile::tempdir().unwrap();
    fs::write(t.path().join("AGENTS.md"), "# Agent rules\n").unwrap();
    broken_skill(&t.path().join("skills"));
    let out = Command::new(env!("CARGO_BIN_EXE_agents"))
        .env("AGENTS_UPDATE_CHECK", "off")
        .arg("--home")
        .arg(t.path())
        .arg("check")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "{stdout}");
    let line = stdout
        .lines()
        .find(|l| l.contains("name-matches-dir"))
        .unwrap_or_else(|| panic!("no lint line in:\n{stdout}"));
    assert_eq!(
        line,
        "skills/data_query/SKILL.md:1 skill.name-matches-dir: name is data-query, \
         directory is data_query. fix: edit the frontmatter to meet the Agent Skills standard"
    );
}
