//! Where each tool keeps its instructions, and which of those paths the home
//! may own. Nothing here writes.

use crate::home::user_home;
use std::path::{Path, PathBuf};

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

/// The keys in a settings file that change behavior, as JSON pointers. A
/// TOML file is read into the same shape.
#[derive(Default)]
pub struct SettingsKeys {
    pub hooks: Option<&'static str>,
    pub allow: Option<&'static str>,
    pub deny: Option<&'static str>,
    pub env: Option<&'static str>,
    pub model: Option<&'static str>,
    pub effort: Option<&'static str>,
}

/// The settings files in each tool's own directory.
pub fn settings_globals() -> [PathBuf; 3] {
    [
        Claude::new().settings(),
        codex_dir().join("config.toml"),
        cursor_dir().join("mcp.json"),
    ]
}

/// The tool that reads a settings file at this path, or None.
pub fn settings_tool(p: &Path) -> Option<&'static str> {
    let name = p.file_name()?.to_str()?;
    let parent = p
        .parent()
        .and_then(Path::file_name)
        .and_then(|n| n.to_str());
    match (name, parent) {
        ("settings.json" | "settings.local.json", Some(".claude")) | (".mcp.json", _) => {
            Some("claude code")
        }
        ("config.toml", Some(".codex")) => Some("codex"),
        ("config.toml", _) if p == codex_dir().join("config.toml") => Some("codex"),
        ("mcp.json", Some(".cursor")) => Some("cursor"),
        _ => None,
    }
}

/// Where each key that changes behavior sits in this settings file.
/// <https://code.claude.com/docs/en/settings>,
/// <https://learn.chatgpt.com/docs/config-file/config-reference>.
pub fn settings_keys(p: &Path) -> SettingsKeys {
    match p.file_name().and_then(|n| n.to_str()) {
        Some("settings.json" | "settings.local.json") => SettingsKeys {
            hooks: Some("/hooks"),
            allow: Some("/permissions/allow"),
            deny: Some("/permissions/deny"),
            env: Some("/env"),
            model: Some("/model"),
            effort: Some("/effortLevel"),
        },
        Some("config.toml") => SettingsKeys {
            hooks: Some("/hooks"),
            env: Some("/shell_environment_policy/set"),
            model: Some("/model"),
            effort: Some("/model_reasoning_effort"),
            ..SettingsKeys::default()
        },
        _ => SettingsKeys::default(),
    }
}

/// The settings files one tool reads from a directory, lowest precedence
/// first, each with its scope. `known` is false when the tool documents no
/// order.
pub struct SettingsOrder {
    pub tool: &'static str,
    pub known: bool,
    pub files: Vec<(PathBuf, &'static str)>,
}

/// Every settings file each tool reads in a session started in `dir`, inside
/// the checkout at `root` (or `dir` itself outside a checkout), lowest
/// precedence first. `main` is the main checkout of a worktree, else `root`.
pub fn settings_order(dir: &Path, root: Option<&Path>, main: Option<&Path>) -> Vec<SettingsOrder> {
    let top = root.unwrap_or(dir);
    // Claude Code: user, then shared project from the working directory,
    // then local. The local file sits at the main checkout root, and an older
    // one in the working directory is still read below it; outside a
    // checkout, on Windows, or with the checkout at the user home it stays
    // in the working directory. `.mcp.json` sits at the project root and
    // holds only servers, so its place among the settings files changes no
    // key. <https://code.claude.com/docs/en/settings#settings-precedence>,
    // <https://code.claude.com/docs/en/mcp>.
    let claude = Claude::new();
    let mut c = vec![
        (claude.settings(), "user"),
        (top.join(".mcp.json"), "project"),
        (dir.join(".claude").join("settings.json"), "project"),
    ];
    let local = |d: &Path| d.join(".claude").join("settings.local.json");
    match main {
        Some(r) if !cfg!(windows) && r != user_home() => {
            if r != dir {
                c.push((local(dir), "local"));
            }
            c.push((local(r), "local"));
        }
        _ => c.push((local(dir), "local")),
    }
    // Codex: user, then each project layer from the project root down to
    // the working directory, closest wins, in a trusted project only.
    // <https://learn.chatgpt.com/docs/config-file/config-basic>.
    let mut x = vec![(codex_dir().join("config.toml"), "user")];
    let mut layers: Vec<&Path> = dir.ancestors().take_while(|a| a.starts_with(top)).collect();
    layers.reverse();
    x.extend(
        layers
            .into_iter()
            .map(|d| (d.join(".codex").join("config.toml"), "project")),
    );
    // Cursor names a global and a project file and no order between them.
    // <https://cursor.com/docs/context/mcp>.
    let u = vec![
        (cursor_dir().join("mcp.json"), "user"),
        (top.join(".cursor").join("mcp.json"), "project"),
    ];
    vec![
        SettingsOrder {
            tool: "claude code",
            known: true,
            files: c,
        },
        SettingsOrder {
            tool: "codex",
            known: true,
            files: x,
        },
        SettingsOrder {
            tool: "cursor",
            known: false,
            files: u,
        },
    ]
}

/// What a tool does with a project layer that the settings files alone do
/// not show.
pub fn settings_note(tool: &str) -> Option<&'static str> {
    (tool == "codex").then_some("Codex reads a project layer only in a project you trust.")
}
