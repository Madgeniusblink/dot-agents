//! Where each tool keeps its instructions, and which of those paths the home
//! may own. Nothing here writes.

use crate::home::user_home;
use std::path::PathBuf;

/// The names `[tools] present` in the per-machine config file takes.
pub const TOOLS: [&str; 4] = ["claude", "codex", "grok", "cursor"];

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

/// Where tools read a project's own skills, relative to the checkout root.
pub const PROJECT_SKILL_DIRS: [&str; 4] = [
    ".claude/skills",
    ".agents/skills",
    ".codex/skills",
    "skills",
];

/// Cursor keeps user rules in its application settings, not in a file.
pub fn cursor_dir() -> PathBuf {
    user_home().join(".cursor")
}

pub const CURSOR_LINE: &str =
    "Read ~/.agents/AGENTS.md and every file in ~/.agents/rules/ before starting work.";

impl Claude {
    /// Where Claude Code keeps auto memory: its own per-repository notes.
    /// `autoMemoryDirectory` in settings.json relocates it.
    pub fn auto_memory_root(&self) -> PathBuf {
        let settings = std::fs::read_to_string(self.settings()).unwrap_or_default();
        serde_json::from_str::<serde_json::Value>(&settings)
            .ok()
            .and_then(|v| {
                v["autoMemoryDirectory"]
                    .as_str()
                    .map(crate::home::expand_home)
            })
            .unwrap_or_else(|| self.root.join("projects"))
    }

    /// Every `memory/` directory with a `MEMORY.md` index. The first 200
    /// lines of that index load at the start of every session in its
    /// repository, so it shapes behavior like an instruction file does.
    pub fn auto_memory_dirs(&self) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(self.auto_memory_root())
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path().join("memory"))
                    .filter(|m| m.join("MEMORY.md").is_file())
                    .collect()
            })
            .unwrap_or_default();
        dirs.sort();
        dirs
    }
}

/// Grok Build scans `$GROK_HOME/rules/` (default `~/.grok/rules/`) in every
/// project and loads each file in full. It documents no import syntax, so
/// the home's drop-in there carries the content.
pub fn grok_dir() -> PathBuf {
    std::env::var_os("GROK_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| user_home().join(".grok"))
}

pub fn grok_dropin() -> PathBuf {
    grok_dir().join("rules").join("00-agents-home.md")
}
