use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;

pub fn agents_home() -> PathBuf {
    env::var("DOT_AGENTS_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            env::var("HOME")
                .map(|h| PathBuf::from(h).join(".agents"))
                .unwrap_or_else(|_| PathBuf::from(".agents"))
        })
}

pub fn resolve_scratch(home: &PathBuf) -> PathBuf {
    env::var("DOT_AGENTS_SCRATCH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join("var/scratch"))
}

pub fn resolve_worktrees(home: &PathBuf) -> PathBuf {
    env::var("DOT_AGENTS_WORKTREES")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join("var/worktrees"))
}

pub fn resolved_paths() -> Result<(PathBuf, PathBuf, PathBuf)> {
    let home = agents_home();
    Ok((home.clone(), resolve_scratch(&home), resolve_worktrees(&home)))
}

pub fn format_paths_human(home: &PathBuf, scratch: &PathBuf, worktrees: &PathBuf) -> String {
    format!(
        "home:      {}\nscratch:   {}\nworktrees: {}\n",
        home.display(),
        scratch.display(),
        worktrees.display()
    )
}

pub fn format_paths_json(home: &PathBuf, scratch: &PathBuf, worktrees: &PathBuf) -> Result<String> {
    let obj = serde_json::json!({
        "home": home,
        "scratch": scratch,
        "worktrees": worktrees,
    });
    serde_json::to_string_pretty(&obj).context("serialize paths json")
}
