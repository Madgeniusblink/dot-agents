//! The two commands that touch the home's git state. The home takes shared
//! files from the template by merge, not by an installer.

use crate::config::{self, Config};
use crate::home::{git_in, tilde, Home};
use crate::plan::{banner, Action, Plan};
use crate::{status, wire};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// Clone the user's own copy of the template to the home, add the template
/// it came from as `upstream`, set the merge driver its `.gitattributes`
/// asks for, then wire. A clone that carries the template's layout, with
/// the home under `home/`, or an empty repository, first gets one commit
/// whose tree is `home/`, so its root is the home (decision 0012).
pub fn init(flag: Option<PathBuf>, cfg: &Config, url: &str, apply: bool) -> Result<()> {
    let dir = std::path::absolute(flag.unwrap_or_else(Home::default_dir))?;
    if dir.exists() {
        bail!(
            "{} exists. To wire an existing home run: agents wire",
            tilde(&dir)
        );
    }
    banner(apply);
    println!("home:");
    println!("  git clone {url} {}", tilde(&dir));
    println!("  when the clone has home/AGENTS.md and no AGENTS.md: one commit whose tree is home/, then reset to it");
    println!("  git remote add upstream <repository from context/upstream.json>");
    println!("  git config merge.ours.driver true");
    let mut machine = Plan::new();
    machine.header("this machine:");
    if cfg.exists {
        machine.say(format!("present: {}", tilde(&cfg.path)));
    } else {
        machine.act(
            Action::Write {
                path: cfg.path.clone(),
                content: config::TEMPLATE.to_string(),
            },
            format!("write {} (every key at its default)", tilde(&cfg.path)),
        );
    }
    if !apply {
        machine.run(false)?;
        println!();
        println!("then the wiring plan, which needs the clone to exist.");
        return Ok(());
    }
    let out = std::process::Command::new("git")
        .args(["clone", "--quiet", url])
        .arg(&dir)
        .output()
        .context("run git")?;
    if !out.status.success() {
        bail!(
            "git clone failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    root_from_home_dir(&dir)?;
    let home = Home::locate(Some(dir.clone()))
        .context("the clone has no AGENTS.md at its root, so it is not an agent home")?;
    let upstream = template_url(&home);
    let mut plan = Plan::new();
    match upstream {
        Some(u) if u != url => plan.act(
            Action::Git {
                dir: dir.clone(),
                args: vec!["remote".into(), "add".into(), "upstream".into(), u.clone()],
            },
            format!("upstream: {u}"),
        ),
        Some(_) => plan.say("the clone is the template itself; no upstream to add"),
        None => plan.say("context/upstream.json names no repository; add upstream by hand"),
    }
    plan.act(
        Action::Git {
            dir: dir.clone(),
            args: vec!["config".into(), "merge.ours.driver".into(), "true".into()],
        },
        "merge.ours.driver true".to_string(),
    );
    plan.run(true)?;
    machine.run(true)?;
    println!();
    wire::wire(&home, cfg, false).run(true)
}

/// The template keeps the home under `home/` and its own files at the root.
/// A clone of it, or of a copy made with GitHub's template button, is not a
/// home until its root is `home/`. Make one commit whose tree is `home/` and
/// whose parent is the clone's head, then reset to it. The parent keeps the
/// template's history, so a later `merge -X subtree=home` has a merge base.
/// An empty clone first fetches the template. A clone that is already a
/// home, or that has no `home/AGENTS.md`, is left as it is.
fn root_from_home_dir(dir: &Path) -> Result<()> {
    if dir.join("AGENTS.md").is_file() {
        return Ok(());
    }
    let head = if git_in(dir, &["rev-parse", "--verify", "-q", "HEAD"]).is_some() {
        "HEAD"
    } else {
        let template = env!("CARGO_PKG_REPOSITORY");
        println!("  empty repository; git fetch {template} main");
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["fetch", "--quiet", template, "main"])
            .output()
            .context("run git")?;
        if !out.status.success() {
            bail!(
                "git fetch failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        "FETCH_HEAD"
    };
    let marker = format!("{head}:home/AGENTS.md");
    if git_in(dir, &["rev-parse", "--verify", "-q", &marker]).is_none() {
        return Ok(());
    }
    let tree = format!("{head}:home");
    let commit = git_in(
        dir,
        &[
            "commit-tree",
            "-p",
            head,
            "-m",
            "feat: make the root the home",
            &tree,
        ],
    )
    .context("git commit-tree")?;
    git_in(dir, &["reset", "--quiet", "--hard", &commit]).context("git reset")?;
    println!("  root is home/ as of {}", &commit[..7.min(commit.len())]);
    Ok(())
}

fn template_url(home: &Home) -> Option<String> {
    let text = std::fs::read_to_string(home.dir.join("context").join("upstream.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    json["repository"].as_str().map(str::to_string)
}

/// The merge command for this home. `-X subtree=home` when the template
/// keeps the home under `home/` and this checkout has the home at its root
/// (decision 0012): git then shifts the template tree down before the
/// three-way merge, so every file outside `home/` leaves the home.
/// `--allow-unrelated-histories` when the two histories share no commit,
/// as after GitHub's template button. Read after the fetch, so the flags
/// match what `upstream/main` is now.
fn merge_args(home: &Home) -> Vec<String> {
    let mut args = vec!["merge", "--no-edit"];
    if git_in(&home.dir, &["rev-parse", "--verify", "-q", "upstream/main"]).is_some() {
        if git_in(&home.dir, &["merge-base", "HEAD", "upstream/main"]).is_none() {
            args.push("--allow-unrelated-histories");
        }
        let shipped_in_home = git_in(
            &home.dir,
            &[
                "rev-parse",
                "--verify",
                "-q",
                "upstream/main:home/AGENTS.md",
            ],
        )
        .is_some();
        if shipped_in_home && !home.dir.join("home").join("AGENTS.md").is_file() {
            args.push("-X");
            args.push("subtree=home");
        }
    }
    args.push("upstream/main");
    args.into_iter().map(String::from).collect()
}

/// `git fetch upstream && git merge --no-edit upstream/main`, then the
/// report. Keep `--no-edit`: without it git opens an editor for the merge
/// message and waits, which looks like a hang in any shell nobody watches.
pub fn update(home: &Home, cfg: &Config, apply: bool) -> Result<()> {
    if git_in(&home.dir, &["remote", "get-url", "upstream"]).is_none() {
        bail!(
            "no upstream remote in {}. add it with: git -C {} remote add upstream <template-url>",
            tilde(&home.dir),
            tilde(&home.dir)
        );
    }
    banner(apply);
    let mut fetch = Plan::new();
    fetch.header("update:");
    fetch.act(
        Action::Git {
            dir: home.dir.clone(),
            args: vec!["fetch".into(), "upstream".into()],
        },
        "git fetch upstream",
    );
    fetch.run(apply)?;
    let args = merge_args(home);
    let mut merge = Plan::new();
    merge.act(
        Action::Git {
            dir: home.dir.clone(),
            args: args.clone(),
        },
        format!("git {}", args.join(" ")),
    );
    merge.run(apply)?;
    if apply {
        println!();
        status::status(home, cfg, false, false);
    }
    Ok(())
}

/// `update`, then `wire`, then the report: the one-step resync.
pub fn sync(home: &Home, cfg: &Config, apply: bool) -> Result<()> {
    update(home, cfg, apply)?;
    println!();
    wire::wire(home, cfg, false).run(apply)?;
    if apply {
        println!();
        status::status(home, cfg, false, false);
    }
    Ok(())
}
