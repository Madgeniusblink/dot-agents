//! `--json` prints one document with a shared envelope, and the numbers in
//! it match the text output of the same command.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

/// A user home with an agent home, one skill in two places, one auto memory
/// note the home covers, and one instruction file outside any checkout.
fn fixture() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let h = t.path();
    let write = |p: &str, s: &str| {
        let p = h.join(p);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, s).unwrap();
    };
    write(".agents/AGENTS.md", "# router\n");
    write(".agents/principles.md", "# principles\n");
    write(
        ".agents/rules/boundaries.md",
        "- Never push to main without review.\n",
    );
    let skill = "---\nname: demo\ndescription: A demo skill.\nmetadata:\n  owner: me\n---\nBody.\n";
    write(".agents/skills/demo/SKILL.md", skill);
    write(".claude/skills/demo/SKILL.md", skill);
    write(
        ".claude/projects/p1/memory/push.md",
        "---\nname: push\ndescription: never push to main without review\ntype: feedback\n---\nNever push to main without review.\n",
    );
    write(
        ".claude/projects/p1/memory/MEMORY.md",
        "- [push](push.md)\n",
    );
    write("notes/CLAUDE.md", "# loose\n");
    t
}

fn agents(home: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_agents"))
        .arg("--home")
        .arg(home.join(".agents"))
        .args(args)
        .current_dir(home)
        .env("HOME", home)
        .env("COLUMNS", "400")
        .env_remove("AGENTS_HOME")
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

/// B7 and B8: stdout is one JSON document, and it opens with the envelope.
fn document(home: &Path, command: &str, extra: &[&str]) -> Value {
    let mut args = vec![command, "--json"];
    args.extend(extra);
    let out = agents(home, &args);
    let doc: Value = serde_json::from_str(&out).unwrap_or_else(|e| panic!("{e}: {out}"));
    assert_eq!(doc["agents"], env!("CARGO_PKG_VERSION"));
    assert_eq!(doc["command"], command);
    let g = doc["generated"].as_str().unwrap();
    assert!(
        g.len() == 20 && g.ends_with('Z') && &g[10..11] == "T",
        "{g}"
    );
    assert_eq!(doc["home"], "~/.agents");
    doc
}

/// The number just before `tail` in the text output.
fn number_before(text: &str, tail: &str) -> u64 {
    let head = &text[..text.find(tail).unwrap_or_else(|| panic!("{tail}: {text}"))];
    let digits: String = head
        .trim_end()
        .chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.chars().rev().collect::<String>().parse().unwrap()
}

#[test]
fn skills_json_has_the_record_shape_and_the_duplicates() {
    let t = fixture();
    let doc = document(t.path(), "skills", &[]);
    let text = agents(t.path(), &["skills"]);
    assert_eq!(doc["count"], number_before(&text, " skills."));

    // B9: each record has the keys the spec defines, plus the copy note.
    let keys = |v: &Value| {
        let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        k.sort();
        k
    };
    let skill = &doc["skills"][0];
    assert_eq!(
        keys(skill),
        [
            "body",
            "copy",
            "dir",
            "files",
            "frontmatter",
            "lint",
            "name",
            "owner",
            "project",
            "refs",
            "scope",
            "source"
        ]
    );
    assert_eq!(
        keys(&skill["frontmatter"]),
        [
            "allowed_tools",
            "compatibility",
            "description",
            "license",
            "metadata",
            "name",
            "unknown_keys"
        ]
    );
    assert_eq!(keys(&skill["body"]), ["bytes", "hash", "lines"]);
    assert_eq!(
        keys(&skill["files"]),
        ["assets", "bytes", "other", "references", "scripts"]
    );
    assert_eq!(keys(&skill["refs"]), ["broken", "ok"]);
    assert_eq!(skill["owner"], "me");
    assert_eq!(skill["source"], "home");
    assert_eq!(doc["skills"][1]["source"], "claude-user");

    let same = &doc["duplicates"]["same_name"];
    assert_eq!(same.as_array().unwrap().len(), 1);
    assert_eq!(same[0]["name"], "demo");
    assert_eq!(same[0]["identical"], true);
    assert!(doc["duplicates"]["same_body"].is_array());
}

#[test]
fn status_json_matches_the_text() {
    let t = fixture();
    let doc = document(t.path(), "status", &[]);
    let text = agents(t.path(), &["status"]);
    let total = text
        .lines()
        .find(|l| l.trim_start().starts_with("total"))
        .unwrap();
    assert_eq!(doc["always_on"]["bytes"], number_before(total, " bytes"));
    assert_eq!(doc["always_on"]["files"].as_array().unwrap().len(), 2);
}

#[test]
fn scan_json_matches_the_text() {
    let t = fixture();
    let root = t.path().to_string_lossy().into_owned();
    let doc = document(t.path(), "scan", &["--root", &root]);
    let text = agents(t.path(), &["scan", "--root", &root]);
    assert_eq!(doc["count"], number_before(&text, " instruction files."));
    assert_eq!(doc["count"], 2, "{doc}");
    assert_eq!(doc["auto_memory"]["indexes"].as_array().unwrap().len(), 1);
}

#[test]
fn memory_json_matches_the_text() {
    let t = fixture();
    let doc = document(t.path(), "memory", &[]);
    let text = agents(t.path(), &["memory"]);
    assert_eq!(doc["count"], number_before(&text, " notes."));
    assert_eq!(doc["covered"], number_before(&text, " of these notes."));
    assert_eq!(doc["covered"], 1);
    assert_eq!(doc["notes"][0]["verdict"], "covered");
}
