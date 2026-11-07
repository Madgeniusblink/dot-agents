//! Each listing fits one screen by default and prints the full listing with
//! `--long`. A per-machine config file limits what the commands touch.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn skill(root: &Path, dir: &str, name: &str) {
    write(
        root,
        &format!("{dir}/{name}/SKILL.md"),
        &format!("---\nname: {name}\ndescription: The {name} skill.\n---\n{name} body.\n"),
    );
}

/// A user home with an agent home and the directories of two tools.
fn fixture() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let h = t.path();
    write(h, ".agents/AGENTS.md", "# router\n");
    write(h, ".agents/principles.md", "# principles\n");
    write(
        h,
        ".agents/rules/boundaries.md",
        "- Never push to main without review.\n",
    );
    fs::create_dir_all(h.join(".agents/skills")).unwrap();
    fs::create_dir_all(h.join(".claude")).unwrap();
    fs::create_dir_all(h.join(".codex")).unwrap();
    t
}

/// Run `agents` in `home`, with `config` as the per-machine file when given.
fn agents(home: &Path, config: Option<&str>, args: &[&str]) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_agents"));
    cmd.env("AGENTS_UPDATE_CHECK", "off")
        .arg("--home")
        .arg(home.join(".agents"))
        .args(args)
        .current_dir(home)
        .env("HOME", home)
        .env("COLUMNS", "400")
        .env_remove("AGENTS_HOME")
        .env_remove("AGENTS_CONFIG")
        .env_remove("CODEX_HOME")
        .env_remove("GROK_HOME");
    if let Some(text) = config {
        let path = home.join("machine.toml");
        fs::write(&path, text).unwrap();
        cmd.env("AGENTS_CONFIG", path);
    }
    let out = cmd.output().expect("agents");
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn status_is_brief_and_says_stale() {
    let t = fixture();
    let h = t.path();
    agents(h, None, &["wire", "--apply"]);
    let out = agents(h, None, &["status"]);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.len() < 15, "{out}");
    assert!(lines[0].starts_with("machine: "), "{out}");
    assert!(
        lines[0].contains("config: ~/") && lines[0].contains("config.toml"),
        "{out}"
    );
    assert!(lines[1].starts_with("agents 0."), "{out}");
    assert!(lines[2].starts_with("home: ~/.agents"), "{out}");
    assert!(out.contains("always-on: 50 of 12288 bytes"), "{out}");
    assert!(out.contains("claude code: current\n"), "{out}");
    assert!(out.contains("codex: current\n"), "{out}");
    assert!(out.contains("links: 1, 0 broken"), "{out}");

    write(h, ".agents/rules/boundaries.md", "- Changed.\n");
    let out = agents(h, None, &["status"]);
    assert!(out.contains("codex: STALE."), "{out}");
    let long = agents(h, None, &["status", "--long"]);
    assert!(long.contains("what each harness loads (3):"), "{long}");
    assert!(long.contains("STALE: the home changed"), "{long}");
}

#[test]
fn codex_cap_counts_the_generated_file_once() {
    let t = fixture();
    let h = t.path();
    write(
        h,
        ".agents/rules/big.md",
        &format!("- {}\n", "x".repeat(17000)),
    );
    write(
        h,
        ".agents/context/upstream.json",
        r#"{"budget":{"always_on_bytes":20000}}"#,
    );
    agents(h, None, &["wire", "--apply"]);
    let bytes = fs::metadata(h.join(".codex/AGENTS.md")).unwrap().len();
    assert!(bytes < 32768, "{bytes}");
    let doc: Value = serde_json::from_str(&agents(h, None, &["status", "--json"])).unwrap();
    assert_eq!(doc["harnesses"][2]["over_cap"], Value::Null, "{doc}");
    let long = agents(h, None, &["status", "--long"]);
    assert!(!long.contains("project_doc_max_bytes"), "{long}");
}

#[test]
fn skills_collapse_managed_sources() {
    let t = fixture();
    let h = t.path();
    for i in 0..40 {
        skill(
            h,
            ".claude/plugins/cache/market/kit/1.0/skills",
            &format!("kit{i}"),
        );
    }
    for name in ["alpha", "beta", "gamma"] {
        skill(h, ".claude/skills", name);
    }
    let out = agents(h, None, &["skills"]);
    assert!(out.lines().count() < 20, "{out}");
    assert!(
        out.contains("  40  claude code, plugin skills, managed"),
        "{out}"
    );
    assert!(out.contains("yours, BROKEN, or a copy (3):"), "{out}");
    assert!(out.contains("  alpha"), "{out}");
    assert!(!out.contains("kit0"), "{out}");
    let long = agents(h, None, &["skills", "--long"]);
    assert!(long.contains("kit0"), "{long}");
    let hint = long.lines().find(|l| l.starts_with("each in full:"));
    assert!(
        hint.is_some_and(|l| l.ends_with(" agents skills --long --long")),
        "{long}"
    );
    assert!(!long.contains("read in full"), "{long}");
    let full = agents(h, None, &["skills", "--long", "--long"]);
    assert!(full.contains("The alpha skill."), "{full}");
}

#[test]
fn a_broken_link_names_its_target_from_the_user_home() {
    let t = fixture();
    let h = t.path();
    agents(h, None, &["wire", "--apply"]);
    fs::remove_dir(h.join(".agents/skills")).unwrap();
    let long = agents(h, None, &["status", "--long"]);
    assert!(
        long.contains("BROKEN ~/.claude/skills/agents -> ~/.agents/skills\n"),
        "{long}"
    );
}

#[test]
fn status_long_says_when_a_file_wire_writes_is_not_the_homes() {
    let t = fixture();
    let h = t.path();
    write(h, ".codex/AGENTS.md", "# my own codex rules\n");
    let out = agents(h, None, &["status"]);
    assert!(
        out.contains("codex: the file there is not the home's"),
        "{out}"
    );
    let long = agents(h, None, &["status", "--long"]);
    assert!(
        long.contains("0 broken.\n  The file here is not the home's. wire leaves it alone."),
        "{long}"
    );
}

#[test]
fn memory_shows_only_flagged_notes() {
    let t = fixture();
    let h = t.path();
    let note = |name: &str, text: &str| {
        write(
            h,
            &format!(".claude/projects/p1/memory/{name}.md"),
            &format!("---\nname: {name}\ndescription: {text}\ntype: feedback\n---\n{text}\n"),
        )
    };
    note("push", "never push to main without review");
    note("tea", "prefers green tea in the afternoon");
    write(
        h,
        ".claude/projects/p1/memory/MEMORY.md",
        "- [push](push.md)\n- [tea](tea.md)\n",
    );
    let out = agents(h, None, &["memory"]);
    assert!(out.contains("2 notes  1 covered, 0 similar"), "{out}");
    assert!(out.contains("covered or similar (1):"), "{out}");
    assert!(out.contains("  push"), "{out}");
    assert!(!out.contains("  tea"), "{out}");
    let long = agents(h, None, &["memory", "--long"]);
    assert!(long.contains("  tea"), "{long}");
    let hint = long.lines().find(|l| l.starts_with("each in full:"));
    assert!(
        hint.is_some_and(|l| l.ends_with(" agents memory --long --long")),
        "{long}"
    );
    assert!(!long.contains("read in full"), "{long}");
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

#[test]
fn status_takes_no_lock_in_the_home() {
    let t = fixture();
    let h = t.path();
    let home = h.join(".agents");
    git(&home, &["init", "-q"]);
    git(&home, &["add", "-A"]);
    git(&home, &["commit", "-qm", "home"]);
    let modified = || fs::metadata(home.join(".git")).unwrap().modified().unwrap();
    let before = modified();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    for args in [
        &["status"][..],
        &["status", "--long"],
        &["status", "--json"],
    ] {
        agents(h, None, args);
    }
    assert_eq!(modified(), before, "status wrote into the home's .git");
}

/// Two clones of one remote under `work/`, and one checkout under `tasks/`.
fn checkouts(h: &Path) {
    for dir in ["work/one", "work/two", "tasks/web-task-1"] {
        let d = h.join(dir);
        fs::create_dir_all(&d).unwrap();
        git(&d, &["init", "-q"]);
        git(
            &d,
            &[
                "remote",
                "add",
                "origin",
                "https://example.com/acme/widget.git",
            ],
        );
        write(&d, "AGENTS.md", "# widget\n");
        git(&d, &["add", "-A"]);
        git(&d, &["commit", "-qm", "init"]);
    }
}

#[test]
fn scan_prints_one_row_per_remote_and_no_files() {
    let t = fixture();
    let h = t.path();
    checkouts(h);
    let root = h.to_string_lossy().into_owned();
    let out = agents(h, None, &["scan", "--root", &root]);
    assert!(out.contains("in 3 checkouts."), "{out}");
    let row = out.lines().find(|l| l.contains("acme/widget")).unwrap();
    assert!(row.contains("3 checkouts") && row.contains("copy"), "{row}");
    assert!(!out.contains("AGENTS.md"), "{out}");
    assert!(out.contains("identical content: 1 groups"), "{out}");
    let long = agents(h, None, &["scan", "--long", "--root", &root]);
    assert!(long.contains("AGENTS.md"), "{long}");
}

#[test]
fn a_skip_glob_removes_a_root_from_scan() {
    let t = fixture();
    let h = t.path();
    checkouts(h);
    let root = h.to_string_lossy().into_owned();
    let config = "[scan]\nskip = [\"*-task-*\"]\n";
    let out = agents(h, Some(config), &["scan", "--long", "--root", &root]);
    assert!(out.contains("one (1):"), "{out}");
    assert!(!out.contains("web-task-1"), "{out}");
    let roots = format!("[scan]\nroots = [\"{}\"]\n", h.join("tasks").display()).replace('\\', "/");
    let out = agents(h, Some(&roots), &["scan", "--long"]);
    assert!(
        out.contains("web-task-1") && !out.contains("one (1):"),
        "{out}"
    );
}

#[test]
fn tools_present_limits_wire() {
    let t = fixture();
    let h = t.path();
    let out = agents(h, Some("[tools]\npresent = [\"claude\"]\n"), &["wire"]);
    assert!(out.contains("claude code:"), "{out}");
    assert!(!out.contains("codex:"), "{out}");
    let out = agents(h, None, &["wire"]);
    assert!(out.contains("codex:"), "{out}");
}

#[test]
fn init_writes_the_config_file() {
    let t = fixture();
    let h = t.path();
    let src = h.join("src");
    write(&src, "AGENTS.md", "# router\n");
    git(&src, &["init", "-q"]);
    git(&src, &["add", "-A"]);
    git(&src, &["commit", "-qm", "init"]);
    let out = Command::new(env!("CARGO_BIN_EXE_agents"))
        .env("AGENTS_UPDATE_CHECK", "off")
        .arg("--home")
        .arg(h.join("new-home"))
        .args(["init", &src.to_string_lossy(), "--apply"])
        .env("HOME", h)
        .env_remove("AGENTS_CONFIG")
        .env_remove("CODEX_HOME")
        .env_remove("GROK_HOME")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = fs::read_to_string(h.join(".config/agents/config.toml")).unwrap();
    for key in ["editor = ", "roots = ", "skip = ", "present = "] {
        assert!(text.contains(key), "{text}");
    }
}

#[test]
fn json_names_the_machine_and_the_config() {
    let t = fixture();
    let h = t.path();
    let root = h.to_string_lossy().into_owned();
    let config = "editor = \"vi\"\n";
    for args in [
        &["status", "--json"][..],
        &["scan", "--json", "--root", &root],
    ] {
        let doc: Value = serde_json::from_str(&agents(h, Some(config), args)).unwrap();
        assert!(
            doc["machine"].as_str().is_some_and(|m| !m.is_empty()),
            "{doc}"
        );
        assert_eq!(doc["config"]["exists"], true, "{doc}");
        assert_eq!(doc["config"]["editor"], "vi", "{doc}");
        assert_eq!(doc["config"]["tools"]["present"], serde_json::json!([]));
    }
}

#[test]
fn status_counts_one_finding_in_the_singular() {
    let t = fixture();
    let h = t.path();
    write(
        h,
        ".codex/AGENTS.md",
        "<!-- generated by a tool, do not edit -->\n",
    );
    let out = agents(h, None, &["status"]);
    assert!(
        out.contains("codex: the file there is not the home's, 1 finding: agents status --long"),
        "{out}"
    );
}

#[test]
fn status_long_holds_every_fact_of_the_brief() {
    let t = fixture();
    let h = t.path();
    agents(h, None, &["wire", "--apply"]);
    let brief = agents(h, None, &["status"]);
    let long = agents(h, None, &["status", "--long"]);
    let machine = brief.lines().next().unwrap();
    assert!(long.contains(machine), "{machine}\n{long}");
    for n in brief
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
    {
        assert!(
            long.contains(n),
            "{n} is in the brief, not in --long:\n{long}"
        );
    }
    assert!(long.contains("(default cap)"), "{long}");
}
