//! Where each tool keeps its instructions, and which of those paths the home
//! may own. Nothing here writes.

use crate::home::user_home;
use std::path::PathBuf;

/// Claude Code scans `~/.claude/rules/` and loads every file in it, so that is
/// its drop-in directory. `~/.claude/CLAUDE.md` is the harness's own file.
pub struct Claude {
    pub root: PathBuf,
}

impl Claude {
    pub fn new() -> Claude {
        Claude {
            root: user_home().join(".claude"),
        }
    }

    pub fn exists(&self) -> bool {
        self.root.is_dir()
    }

    /// The one file in the drop-in directory that the home owns.
    pub fn dropin(&self) -> PathBuf {
        self.root.join("rules").join("00-agents-home.md")
    }

    /// The harness's own file. Never written; legacy import lines are
    /// dropped from it because an earlier version put them there.
    pub fn own_file(&self) -> PathBuf {
        self.root.join("CLAUDE.md")
    }

    pub fn rules_link(&self) -> PathBuf {
        self.root.join("rules").join("agents")
    }

    pub fn skills_link(&self) -> PathBuf {
        self.root.join("skills").join("agents")
    }

    pub fn settings(&self) -> PathBuf {
        self.root.join("settings.json")
    }
}

/// Codex reads one global file and has no import syntax, so a link is the
/// only live transport.
pub fn codex_dir() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| user_home().join(".codex"))
}

/// Codex compares global plus thread instructions against this setting and
/// warns past it. It truncates project files only.
pub const CODEX_PROJECT_DOC_MAX: u64 = 32768;

/// Cursor keeps user rules in its application settings, not in a file.
pub fn cursor_dir() -> PathBuf {
    user_home().join(".cursor")
}

pub const CURSOR_LINE: &str =
    "Read ~/.agents/AGENTS.md and every file in ~/.agents/rules/ before starting work.";
