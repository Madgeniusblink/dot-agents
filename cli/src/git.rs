//! The two commands that touch the home's git state. The home takes shared
//! files from the template by merge, not by an installer.

use crate::home::{git_in, tilde, Home};
use crate::plan::{banner, Action, Plan};
use crate::{status, wire};
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

/// Clone the user's own copy of the template to the home, add the template
/// it came from as `upstream`, set the merge driver its `.gitattributes`
/// asks for, then wire.
pub fn init(flag: Option<PathBuf>, url: &str, apply: bool) -> Result<()> {
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
    println!("  git remote add upstream <repository from context/upstream.json>");
    println!("  git config merge.ours.driver true");
    if !apply {
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
    println!();
    wire::wire(&home, false).run(true)
}

fn template_url(home: &Home) -> Option<String> {
    let text = std::fs::read_to_string(home.dir.join("context").join("upstream.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    json["repository"].as_str().map(str::to_string)
}

/// `git fetch upstream && git merge --no-edit upstream/main`, then the
/// report. Keep `--no-edit`: without it git opens an editor for the merge
/// message and waits, which looks like a hang in any shell nobody watches.
pub fn update(home: &Home, apply: bool) -> Result<()> {
    if git_in(&home.dir, &["remote", "get-url", "upstream"]).is_none() {
        bail!(
            "no upstream remote in {}. add it with: git -C {} remote add upstream <template-url>",
            tilde(&home.dir),
            tilde(&home.dir)
        );
    }
    banner(apply);
    let mut plan = Plan::new();
    plan.header("update:");
    for args in [
        vec!["fetch", "upstream"],
        vec!["merge", "--no-edit", "upstream/main"],
    ] {
        plan.act(
            Action::Git {
                dir: home.dir.clone(),
                args: args.iter().map(|s| s.to_string()).collect(),
            },
            format!("git {}", args.join(" ")),
        );
    }
    plan.run(apply)?;
    if apply {
        println!();
        status::status(home, false);
    }
    Ok(())
}

/// `update`, then `wire`, then the report: the one-step resync.
pub fn sync(home: &Home, apply: bool) -> Result<()> {
    update(home, apply)?;
    println!();
    wire::wire(home, false).run(apply)?;
    if apply {
        println!();
        status::status(home, false);
    }
    Ok(())
}
