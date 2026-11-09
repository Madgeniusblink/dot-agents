//! The agent home: one git checkout, and the paths the tools are pointed at.

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The user's home directory.
pub fn user_home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// `~/x` for a path under the user's home, else the path as given. The
/// part under `~` always uses `/`, on every platform.
pub fn tilde(p: &Path) -> String {
    match p.strip_prefix(user_home()) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => {
            let parts: Vec<_> = rest
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect();
            format!("~/{}", parts.join("/"))
        }
        Err(_) => p.display().to_string(),
    }
}

/// `x` for a path under the user's home, else the path as given.
pub fn rel(p: &Path) -> String {
    match p.strip_prefix(user_home()) {
        Ok(rest) if !rest.as_os_str().is_empty() => rest.display().to_string(),
        _ => p.display().to_string(),
    }
}

/// `~/x`, `$HOME/x`, and `${HOME}/x` as the path they name.
pub fn expand_home(s: &str) -> PathBuf {
    for prefix in ["~/", "$HOME/", "${HOME}/"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            return user_home().join(rest);
        }
    }
    PathBuf::from(s)
}

pub struct Home {
    pub dir: PathBuf,
}

impl Home {
    pub fn default_dir() -> PathBuf {
        std::env::var_os("AGENTS_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| user_home().join(".agents"))
    }

    pub fn locate(flag: Option<PathBuf>) -> Result<Home> {
        let dir = std::path::absolute(flag.unwrap_or_else(Self::default_dir))?;
        if !dir.join("AGENTS.md").is_file() {
            if dir.join("home").join("AGENTS.md").is_file() {
                bail!(
                    "{d} is the template repository, not a home; its home is {d}/home. \
                     run: agents --home {d}/home <command>, or agents init <your-repo-url>",
                    d = tilde(&dir)
                );
            }
            bail!(
                "no agent home at {}. run: agents init <your-repo-url>",
                tilde(&dir)
            );
        }
        Ok(Home { dir })
    }

    /// One import line per always-on file: `principles.md`, then every
    /// `rules/*.md` in byte order. The router names the rest.
    pub fn always_on(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let principles = self.dir.join("principles.md");
        if principles.is_file() {
            files.push(principles);
        }
        let mut rules: Vec<PathBuf> = std::fs::read_dir(self.dir.join("rules"))
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
                    .collect()
            })
            .unwrap_or_default();
        rules.sort();
        files.extend(rules);
        files
    }

    /// Trimmed stdout of a git command run in the home, or None on failure.
    pub fn git(&self, args: &[&str]) -> Option<String> {
        git_in(&self.dir, args)
    }
}

pub fn git_in(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}
