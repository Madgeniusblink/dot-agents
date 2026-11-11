//! What is true on this machine only: where to scan, what to skip, which
//! tools to touch. The home travels to every machine, so it cannot hold
//! these. A missing file means the defaults.

use crate::harness::TOOLS;
use crate::home::{expand_home, tilde, user_home};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// What `agents init` writes: every key, each with one comment, set to its
/// default.
pub const TEMPLATE: &str = r#"# What is true on this machine only. The home holds what every machine shares.
# Every agents command reads this file. A missing file means the defaults.

# The command that opens a file on this machine, such as "vim". Empty means unset.
editor = ""

[scan]
# The directories that scan walks. Empty means your home directory.
roots = []
# Globs of paths that scan skips, such as "~/work/*-task-*". ** crosses directories.
skip = []

[tools]
# The tools that status and wire touch: claude, codex, grok, cursor. Empty means every tool found.
present = []
"#;

#[derive(Clone, Default)]
pub struct Config {
    pub path: PathBuf,
    pub exists: bool,
    pub editor: Option<String>,
    pub roots: Vec<String>,
    pub skip: Vec<String>,
    pub present: Vec<String>,
}

/// `$AGENTS_CONFIG`, else `~/.config/agents/config.toml`.
pub fn path() -> PathBuf {
    std::env::var_os("AGENTS_CONFIG")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            user_home()
                .join(".config")
                .join("agents")
                .join("config.toml")
        })
}

/// The name this machine gives itself, or `unknown`.
pub fn machine() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

impl Config {
    pub fn load() -> Result<Config> {
        let path = path();
        let Ok(text) = fs::read_to_string(&path) else {
            return Ok(Config {
                path,
                ..Config::default()
            });
        };
        let doc: Value = basic_toml::from_str(&text)
            .with_context(|| format!("{} is not valid TOML", tilde(&path)))?;
        let list = |table: &str, key: &str| -> Result<Vec<String>> {
            match &doc[table][key] {
                Value::Null => Ok(Vec::new()),
                Value::Array(a) if a.iter().all(Value::is_string) => Ok(a
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()),
                _ => bail!("{}: {table}.{key} must be a list of strings", tilde(&path)),
            }
        };
        let present = list("tools", "present")?;
        if let Some(bad) = present.iter().find(|t| !TOOLS.contains(&t.as_str())) {
            bail!(
                "{}: tools.present names {bad}. Use one of: {}",
                tilde(&path),
                TOOLS.join(", ")
            );
        }
        let editor = match &doc["editor"] {
            Value::Null => None,
            Value::String(s) if s.is_empty() => None,
            Value::String(s) => Some(s.clone()),
            _ => bail!("{}: editor must be a string", tilde(&path)),
        };
        Ok(Config {
            editor,
            roots: list("scan", "roots")?,
            skip: list("scan", "skip")?,
            present,
            exists: true,
            path,
        })
    }

    /// True when `[tools] present` is empty or names this tool.
    pub fn tool(&self, name: &str) -> bool {
        self.present.is_empty() || self.present.iter().any(|t| t == name)
    }

    pub fn roots(&self) -> Vec<PathBuf> {
        self.roots.iter().map(|r| expand_home(r)).collect()
    }

    /// True when a `[scan] skip` glob matches the path.
    pub fn skipped(&self, path: &Path) -> bool {
        if self.skip.is_empty() {
            return false;
        }
        let p = path.to_string_lossy().replace('\\', "/");
        let name = path.file_name().map(|n| n.to_string_lossy());
        self.skip.iter().any(|g| {
            if g.contains('/') {
                let g = expand_home(g).to_string_lossy().replace('\\', "/");
                glob(g.as_bytes(), p.as_bytes())
            } else {
                name.as_deref()
                    .is_some_and(|n| glob(g.as_bytes(), n.as_bytes()))
            }
        })
    }

    /// The `config` key of a `--json` document.
    pub fn json(&self) -> Value {
        json!({
            "path": tilde(&self.path),
            "exists": self.exists,
            "editor": self.editor,
            "scan": {"roots": self.roots, "skip": self.skip},
            "tools": {"present": self.present},
        })
    }
}

/// `*` and `?` stay inside one path part; `**` crosses `/`.
fn glob(g: &[u8], s: &[u8]) -> bool {
    match g {
        [] => s.is_empty(),
        [b'*', b'*', rest @ ..] => (0..=s.len()).any(|i| glob(rest, &s[i..])),
        [b'*', rest @ ..] => (0..=s.len())
            .take_while(|&i| i == 0 || s[i - 1] != b'/')
            .any(|i| glob(rest, &s[i..])),
        [b'?', rest @ ..] => s.first().is_some_and(|&c| c != b'/') && glob(rest, &s[1..]),
        [c, rest @ ..] => s.first() == Some(c) && glob(rest, &s[1..]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_keeps_star_inside_one_part() {
        assert!(glob(b"/a/*-task-*", b"/a/web-task-7"));
        assert!(!glob(b"/a/*", b"/a/b/c"));
        assert!(glob(b"/a/**", b"/a/b/c"));
        assert!(glob(b"**/tmp?", b"/x/y/tmp1"));
        assert!(!glob(b"/a/?", b"/a/"));
    }

    #[test]
    fn the_template_parses_to_the_defaults() {
        let doc: Value = basic_toml::from_str(TEMPLATE).unwrap();
        assert_eq!(doc["scan"]["roots"], json!([]));
        assert_eq!(doc["scan"]["skip"], json!([]));
        assert_eq!(doc["tools"]["present"], json!([]));
        assert_eq!(doc["editor"], "");
        let comments = TEMPLATE.lines().filter(|l| l.starts_with("# ")).count();
        let keys = TEMPLATE.lines().filter(|l| l.contains(" = ")).count();
        assert_eq!(comments, keys + 2, "one comment per key, plus the header");
    }
}
