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
        .env("AGENTS_UPDATE_CHECK", "off")
        .args(args)
        .current_dir(cwd)
        .env("HOME", home)
        .env("COLUMNS", "400")
        .env_remove("AGENTS_HOME")
        .env_remove("AGENTS_CONFIG")
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
    let out = agents(home.path(), repo.path(), &["skills", "--long"]);

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
    let long = agents(home.path(), repo.path(), &["skills", "--long", "--long"]);
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
        beta.contains("different bodies") && beta.contains("project "),
        "{beta}"
    );
}

#[test]
fn scan_counts_the_project_skills() {
    let (home, repo) = fixture();
    let root = repo.path().to_string_lossy().into_owned();
    let out = agents(
        home.path(),
        repo.path(),
        &["scan", "--long", "--root", &root],
    );
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

/// T8: the moves of `remove_and_keep_never_write_inside_the_checkout`, made
/// from the Skills view of `agents ui`. `r` and `K` show the plan and move
/// nothing; `A` runs that plan, as `--apply` does.
#[cfg(unix)]
#[test]
fn t8_remove_and_keep_from_the_view_never_write_inside_the_checkout() {
    let (home, repo) = fixture();
    let (h, r) = (home.path(), repo.path());
    // The view opens only with an agent home.
    fs::create_dir_all(h.join(".agents")).unwrap();
    fs::write(h.join(".agents/AGENTS.md"), "# home\n").unwrap();
    let mut ui = view::Ui::open(h, r);
    ui.press("2", "yours, BROKEN, or a copy");

    ui.select("alpha");
    let plan = ui.press("r", "dry run of agents skills");
    assert!(plan.contains(": the project owns it."), "{plan}");
    assert!(plan.contains("move ~/.claude/skills/alpha -> "), "{plan}");
    assert!(
        h.join(".claude/skills/alpha").exists(),
        "the plan moved alpha"
    );
    assert_eq!(git(r, &["status", "--porcelain"]), "");
    let done = ui.press("A", "applied.");
    assert!(done.contains(": the project owns it."), "{done}");
    assert_eq!(git(r, &["status", "--porcelain"]), "");
    assert!(!h.join(".claude/skills/alpha").exists());
    assert!(r.join(".claude/skills/alpha/SKILL.md").is_file());

    ui.press("\x1b", "yours, BROKEN, or a copy");
    ui.select("beta");
    let plan = ui.press("K", "dry run of agents skills");
    assert!(plan.contains(": the project owns it."), "{plan}");
    assert!(
        h.join(".claude/skills/ghost").exists(),
        "the plan moved ghost"
    );
    assert_eq!(git(r, &["status", "--porcelain"]), "");
    let done = ui.press("A", "applied.");
    assert!(done.contains(": the project owns it."), "{done}");
    assert_eq!(git(r, &["status", "--porcelain"]), "");
    assert!(!h.join(".claude/skills/ghost").exists());
    assert!(h.join(".claude/skills/beta").exists());
    ui.quit();
}

/// T10: a skill an agent adds while the view is open shows within the 2 s
/// poll, with no key pressed, and the selection stays on its row.
#[cfg(unix)]
#[test]
fn t10_the_open_view_shows_a_new_skill_with_no_key() {
    let (home, repo) = fixture();
    let (h, r) = (home.path(), repo.path());
    fs::create_dir_all(h.join(".agents")).unwrap();
    fs::write(h.join(".agents/AGENTS.md"), "# home\n").unwrap();
    let mut ui = view::Ui::open(h, r);
    ui.press("2", "yours, BROKEN, or a copy");
    ui.select("ghost");
    assert!(!ui.frame().contains("zeta"));

    skill(&h.join(".claude/skills"), "zeta", "", "Zeta body.");
    let f = ui.wait("the new row", |f| f.contains("zeta"));
    assert_eq!(view::selected(&f).as_deref(), Some("ghost"), "{f}");
    ui.quit();
}

/// `agents ui` on a pseudo-terminal, driven one key at a time as a person
/// drives it. The screen redraws every 100 ms, so each wait reads the last
/// whole frame.
#[cfg(unix)]
mod view {
    use rustix::pty::{grantpt, openpt, ptsname, unlockpt, OpenptFlags};
    use rustix::termios::{tcsetwinsize, Winsize};
    use std::fs::File;
    use std::io::{Read, Write};
    use std::path::Path;
    use std::process::{Child, Command, Stdio};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    pub struct Ui {
        master: File,
        out: Arc<Mutex<Vec<u8>>>,
        child: Child,
    }

    impl Ui {
        pub fn open(home: &Path, cwd: &Path) -> Ui {
            let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).unwrap();
            grantpt(&master).unwrap();
            unlockpt(&master).unwrap();
            let name = ptsname(&master, Vec::new()).unwrap();
            let slave = File::options()
                .read(true)
                .write(true)
                .open(name.to_str().unwrap())
                .unwrap();
            let size = Winsize {
                ws_row: 40,
                ws_col: 400,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            tcsetwinsize(&slave, size).unwrap();
            let child = Command::new(env!("CARGO_BIN_EXE_agents"))
                .env("AGENTS_UPDATE_CHECK", "off")
                .arg("ui")
                .current_dir(cwd)
                .env("HOME", home)
                .env_remove("AGENTS_HOME")
                .env_remove("AGENTS_CONFIG")
                .env_remove("CODEX_HOME")
                .stdin(slave.try_clone().unwrap())
                .stdout(slave)
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let master = File::from(master);
            let out = Arc::new(Mutex::new(Vec::new()));
            let (mut reader, sink) = (master.try_clone().unwrap(), out.clone());
            std::thread::spawn(move || {
                let mut buf = [0u8; 8192];
                while let Ok(n @ 1..) = reader.read(&mut buf) {
                    sink.lock().unwrap().extend_from_slice(&buf[..n]);
                }
            });
            // Raw mode flushes the input, so a key sent before the first
            // frame is lost.
            let ui = Ui { master, out, child };
            ui.wait("the tab bar", |f| f.contains(" 1 Status "));
            ui
        }

        /// The last frame drawn in full, without escape sequences.
        pub fn frame(&self) -> String {
            let out = String::from_utf8_lossy(&self.out.lock().unwrap()).into_owned();
            let mut frames: Vec<&str> = out.split("\x1b[H").collect();
            frames.pop();
            strip(frames.last().copied().unwrap_or_default())
        }

        /// The first frame that `ok` accepts, within ten seconds.
        pub fn wait(&self, what: &str, ok: impl Fn(&str) -> bool) -> String {
            let end = Instant::now() + Duration::from_secs(10);
            loop {
                let f = self.frame();
                if ok(&f) {
                    return f;
                }
                assert!(Instant::now() < end, "no frame with {what}:\n{f}");
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        /// Press `key`, then wait for a frame that shows `text`.
        pub fn press(&mut self, key: &str, text: &str) -> String {
            self.master.write_all(key.as_bytes()).unwrap();
            self.wait(text, |f| f.contains(text))
        }

        /// Move down the Skills list until the selected row is `name`.
        pub fn select(&mut self, name: &str) {
            for _ in 0..20 {
                let f = self.wait("a selected row", |f| selected(f).is_some());
                let now = selected(&f).unwrap();
                if now == name {
                    return;
                }
                self.master.write_all(b"j").unwrap();
                self.wait("the next row", |f| selected(f).is_some_and(|s| s != now));
            }
            panic!("no row {name}:\n{}", self.frame());
        }

        pub fn quit(mut self) {
            self.master.write_all(b"q").unwrap();
            let end = Instant::now() + Duration::from_secs(10);
            while self.child.try_wait().unwrap().is_none() {
                assert!(Instant::now() < end, "agents ui did not quit");
                std::thread::sleep(Duration::from_millis(20));
            }
            assert!(self.child.wait().unwrap().success());
        }
    }

    /// The name on the row that starts with `>`.
    pub fn selected(frame: &str) -> Option<String> {
        let row = frame.split("\r\n").find(|l| l.starts_with('>'))?;
        let name = row[1..].trim_start_matches('*').split_whitespace().next()?;
        Some(name.to_string())
    }

    /// `s` without its CSI and OSC sequences. An OSC ends in BEL or ESC `\`.
    fn strip(s: &str) -> String {
        let mut text = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c != '\x1b' {
                text.push(c);
                continue;
            }
            match chars.next() {
                Some('[') => {
                    for c in chars.by_ref() {
                        if ('\x40'..='\x7e').contains(&c) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(c) = chars.next() {
                        if c == '\x07' || (c == '\x1b' && chars.next() == Some('\\')) {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        text
    }
}

/// `git` with a fixed commit date, to backdate a commit.
fn git_at(dir: &Path, date: &str, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Two clones of one remote, a worktree of the first, a submodule inside the
/// second, and a worktree whose source directory was moved. The remote's one
/// commit is old.
fn checkouts() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    // git reports resolved long paths: macOS /var is a link, and a Windows
    // temp dir can be an 8.3 short name. Resolve it, then drop the `\\?\`
    // prefix Windows adds, which git cannot use.
    let resolved = tmp.path().canonicalize().unwrap();
    let text = resolved.to_string_lossy();
    let root = std::path::PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text));
    let s = |p: &Path| p.to_string_lossy().into_owned();
    let seed = root.join("seed");
    fs::create_dir_all(&seed).unwrap();
    git(&seed, &["init", "-q", "-b", "main"]);
    fs::write(seed.join("AGENTS.md"), "# app\n").unwrap();
    git(&seed, &["add", "-A"]);
    git_at(&seed, "2020-01-02T00:00:00Z", &["commit", "-qm", "init"]);
    let remote = root.join("remotes/team/app.git");
    git(&root, &["clone", "-q", "--bare", &s(&seed), &s(&remote)]);
    fs::remove_dir_all(&seed).unwrap();
    for name in ["a", "b"] {
        git(&root, &["clone", "-q", &s(&remote), name]);
    }
    let a = root.join("a");
    fs::write(a.join("notes.txt"), "x\n").unwrap();
    git(&a, &["add", "-A"]);
    git(&a, &["commit", "-qm", "work"]);
    fs::write(a.join("scratch.txt"), "x\n").unwrap();
    git(
        &a,
        &["worktree", "add", "-q", "-b", "side", &s(&root.join("wt"))],
    );
    let b = root.join("b");
    git(
        &b,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            &s(&remote),
            "sub",
        ],
    );
    git_at(&b, "2020-01-03T00:00:00Z", &["commit", "-qm", "sub"]);

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    git(&src, &["init", "-q", "-b", "main"]);
    fs::write(src.join("AGENTS.md"), "# src\n").unwrap();
    git(&src, &["add", "-A"]);
    git(&src, &["commit", "-qm", "init"]);
    git(
        &src,
        &["worktree", "add", "-q", "-b", "o", &s(&root.join("orphan"))],
    );
    fs::rename(&src, root.join("src-moved")).unwrap();
    (tmp, root)
}

#[test]
fn scan_reports_each_checkout_without_the_network() {
    let (_tmp, root) = checkouts();
    let trace = root.join("trace.log");
    let out = Command::new(env!("CARGO_BIN_EXE_agents"))
        .env("AGENTS_UPDATE_CHECK", "off")
        .args(["scan", "--long", "--root"])
        .arg(&root)
        .env("HOME", &root)
        .env("COLUMNS", "400")
        .env("GIT_TRACE", &trace)
        .env_remove("AGENTS_HOME")
        .env_remove("AGENTS_CONFIG")
        .env_remove("CODEX_HOME")
        .output()
        .expect("agents");
    assert!(out.status.success());
    let out = String::from_utf8_lossy(&out.stdout).into_owned();
    let row = |name: &str| {
        out.split("\nproject ~/")
            .find(|s| s.starts_with(&format!("{name} (")))
            .and_then(|s| s.lines().find(|l| l.contains("checkout: ")))
            .unwrap_or_else(|| panic!("no checkout row for {name}: {out}"))
            .to_string()
    };

    // B1: the three kinds, and every field of a row.
    let a = row("a");
    assert!(
        a.contains(
            "remote team/app, clone, branch main, 1 ahead 0 behind, 1 uncommitted, last commit "
        ),
        "{a}"
    );
    let wt = row("wt");
    assert!(
        wt.contains("remote team/app, worktree of ~/a, branch side, no upstream"),
        "{wt}"
    );
    assert!(
        row("orphan").contains("remote none, orphan worktree"),
        "{out}"
    );
    assert!(
        row("b/sub").contains("remote team/app, submodule of ~/b, branch main"),
        "{out}"
    );

    // B3: an old commit with nothing ahead is stale; work ahead is not.
    assert!(
        row("b").contains("1 ahead 0 behind, 0 uncommitted, last commit 2020-01-03"),
        "{out}"
    );
    assert!(!row("b").contains("stale"), "{out}");
    assert!(
        row("b/sub").contains("0 ahead 0 behind, 0 uncommitted, last commit 2020-01-02, stale"),
        "{out}"
    );
    assert!(!a.contains("stale"), "{a}");
    assert!(row("orphan").ends_with("stale"), "{out}");

    // B2: the clones in one group, newest last commit first.
    let group = out
        .split("same remote in more than one place (1):")
        .nth(1)
        .expect("a same-remote section");
    let at = |p: &str| group.find(&format!("~/{p}\n")).unwrap();
    assert!(at("a") < at("b"), "{group}");

    // B4: the identical file is attributed to its checkouts.
    assert!(
        out.contains("same file in 4 checkouts of team/app"),
        "{out}"
    );

    // B6: one prune command per orphan, and nothing that deletes.
    assert!(
        out.contains("~/orphan is an orphan worktree. After you remove it, clear its record with: git -C ~/src-moved worktree prune"),
        "{out}"
    );
    assert!(!out.contains("rm "), "{out}");

    // B5: git ran, never fetch, and no checkout changed.
    let trace = fs::read_to_string(&trace).unwrap();
    assert!(trace.contains("status"), "{trace}");
    assert!(!trace.contains("fetch"), "{trace}");
    assert_eq!(git(&root.join("b"), &["status", "--porcelain"]), "");

    // B7: the same fields and groups under --json.
    let doc = Command::new(env!("CARGO_BIN_EXE_agents"))
        .env("AGENTS_UPDATE_CHECK", "off")
        .args(["scan", "--json", "--root"])
        .arg(&root)
        .env("HOME", &root)
        .env_remove("AGENTS_HOME")
        .env_remove("AGENTS_CONFIG")
        .env_remove("CODEX_HOME")
        .output()
        .expect("agents");
    let doc: serde_json::Value = serde_json::from_slice(&doc.stdout).unwrap();
    let project = |name: &str| {
        doc["projects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["root"] == format!("~/{name}"))
            .unwrap()
            .clone()
    };
    let a = project("a");
    assert_eq!(a["remote"], "team/app");
    assert_eq!(a["kind"], "clone");
    assert_eq!(a["branch"], "main");
    assert_eq!(
        (a["ahead"].clone(), a["behind"].clone()),
        (1.into(), 0.into())
    );
    assert_eq!(a["uncommitted"], 1);
    assert_eq!(a["stale"], false);
    let wt = project("wt");
    assert_eq!(wt["kind"], "worktree");
    assert_eq!(wt["main_checkout"], "~/a");
    assert!(wt["ahead"].is_null(), "{wt}");
    let sub = project("b/sub");
    assert_eq!(sub["kind"], "submodule");
    assert_eq!(sub["main_checkout"], "~/b");
    assert_eq!(sub["last_commit"], "2020-01-02");
    assert_eq!(sub["stale"], true);
    let orphan = project("orphan");
    assert_eq!(orphan["kind"], "orphan worktree");
    assert_eq!(orphan["prune"], "git -C ~/src-moved worktree prune");
    assert_eq!(doc["same_remote"][0]["remote"], "team/app");
    assert_eq!(
        doc["same_remote"][0]["checkouts"].as_array().unwrap().len(),
        4
    );
    assert!(doc["identical_remotes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g[0]["remote"] == "team/app" && g[0]["checkouts"] == 4));
}
