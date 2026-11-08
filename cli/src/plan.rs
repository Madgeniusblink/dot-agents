//! A plan is the list of what a command would change, printed in order. With
//! `--apply` each action runs before its line is printed, so the output reads
//! the same either way and a failure names the step that failed.

use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub enum Action {
    Write {
        path: PathBuf,
        content: String,
    },
    Delete(PathBuf),
    Link {
        src: PathBuf,
        dst: PathBuf,
    },
    Unlink(PathBuf),
    /// Rewrite the file without the lines `matches` selects.
    DropLines {
        path: PathBuf,
        matches: Box<dyn Fn(&str) -> bool>,
    },
    Move {
        from: PathBuf,
        to: PathBuf,
    },
    Git {
        dir: PathBuf,
        args: Vec<String>,
    },
}

enum Step {
    Header(String),
    Say(String),
    Text(String),
    Blank,
    Do(Action, String),
}

#[derive(Default)]
pub struct Plan {
    steps: Vec<Step>,
}

pub fn banner(apply: bool) {
    if !apply {
        println!("dry run. nothing is written. add --apply to write.");
    }
}

impl Plan {
    pub fn new() -> Plan {
        Plan::default()
    }

    pub fn header(&mut self, s: impl Into<String>) {
        self.steps.push(Step::Header(s.into()));
    }

    pub fn say(&mut self, s: impl Into<String>) {
        self.steps.push(Step::Say(s.into()));
    }

    pub fn text(&mut self, s: impl Into<String>) {
        self.steps.push(Step::Text(s.into()));
    }

    pub fn blank(&mut self) {
        self.steps.push(Step::Blank);
    }

    pub fn act(&mut self, action: Action, s: impl Into<String>) {
        self.steps.push(Step::Do(action, s.into()));
    }

    pub fn run(&self, apply: bool) -> Result<()> {
        self.run_to(apply, &mut std::io::stdout().lock())
    }

    /// `run`, printing to `out`. `agents ui` draws the lines itself.
    pub fn run_to(&self, apply: bool, out: &mut impl Write) -> Result<()> {
        for step in &self.steps {
            match step {
                Step::Header(h) => writeln!(out, "{h}")?,
                Step::Say(t) => writeln!(out, "  {t}")?,
                Step::Text(t) => writeln!(out, "{t}")?,
                Step::Blank => writeln!(out)?,
                Step::Do(action, t) => {
                    if apply {
                        exec(action).with_context(|| format!("failed: {t}"))?;
                    }
                    writeln!(out, "  {t}")?;
                }
            }
        }
        Ok(())
    }
}

fn parent_dir(p: &Path) -> Result<()> {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn exec(action: &Action) -> Result<()> {
    match action {
        Action::Write { path, content } => {
            parent_dir(path)?;
            fs::write(path, content)?;
        }
        Action::Delete(path) => fs::remove_file(path)?,
        Action::Link { src, dst } => {
            parent_dir(dst)?;
            symlink(src, dst)?;
        }
        Action::Unlink(path) => unlink(path)?,
        Action::DropLines { path, matches } => {
            let text = fs::read_to_string(path)?;
            let kept: String = text
                .lines()
                .filter(|l| !matches(l))
                .map(|l| format!("{l}\n"))
                .collect();
            let tmp = path.with_extension("md.tmp");
            fs::write(&tmp, kept)?;
            fs::rename(&tmp, path)?;
        }
        Action::Move { from, to } => {
            parent_dir(to)?;
            if fs::rename(from, to).is_err() {
                fs::copy(from, to)?;
                fs::remove_file(from)?;
            }
        }
        Action::Git { dir, args } => {
            let out = Command::new("git").arg("-C").arg(dir).args(args).output()?;
            if !out.status.success() {
                anyhow::bail!(
                    "git {} failed: {}",
                    args.join(" "),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn symlink(src: &Path, dst: &Path) -> Result<()> {
    Ok(std::os::unix::fs::symlink(src, dst)?)
}

#[cfg(windows)]
fn symlink(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        Ok(std::os::windows::fs::symlink_dir(src, dst)?)
    } else {
        Ok(std::os::windows::fs::symlink_file(src, dst)?)
    }
}

#[cfg(unix)]
fn unlink(path: &Path) -> Result<()> {
    Ok(fs::remove_file(path)?)
}

#[cfg(windows)]
fn unlink(path: &Path) -> Result<()> {
    if fs::symlink_metadata(path)?.is_dir() {
        Ok(fs::remove_dir(path)?)
    } else {
        Ok(fs::remove_file(path)?)
    }
}
