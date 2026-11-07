//! Find every instruction file on this machine. A tool loads the global ones
//! in every session and the project ones when a session starts inside that
//! repository, so an unlisted one is behavior you did not choose.

use crate::harness::Claude;
use crate::home::{tilde, user_home, Home};
use crate::{out, skills};
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::{DirEntry, WalkDir};

/// Directories that hold copies, caches, or vendored trees, never a file a
/// session of yours loads. `--all` walks them anyway.
const SKIP: &[&str] = &[
    ".git",
    "node_modules",
    "Library",
    ".Trash",
    ".cache",
    ".cargo",
    ".rustup",
    ".npm",
    "target",
    "vendor",
    ".venv",
    "venv",
    "__pycache__",
];

const NAMES: &[&str] = &[
    "claude.md",
    "claude.local.md",
    "agents.md",
    "agents.local.md",
    "agents.override.md",
    "context.md",
    ".cursorrules",
];

enum Scope {
    Home,
    Global,
    Project(PathBuf),
    PluginCache,
    MovedAside,
    Loose,
}

struct Hit {
    path: PathBuf,
    lines: usize,
    bytes: u64,
    link: Option<PathBuf>,
    generated: Option<String>,
    hash: u64,
    scope: Scope,
}

pub fn scan(root: PathBuf, all: bool, long: bool, home: Option<&Home>) {
    let width = out::width();
    let started = Instant::now();
    let user = user_home();
    let globals = [
        user.join(".claude"),
        user.join(".codex"),
        user.join(".cursor"),
    ];
    let aside = user.join(".agents-reset");

    let mut hits: Vec<Hit> = WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| all || keep(e))
        .flatten()
        .filter(|e| !e.file_type().is_dir() && instruction_file(e.path()))
        .map(|e| {
            let path = e.into_path();
            let text = fs::read_to_string(&path).unwrap_or_default();
            let mut hasher = DefaultHasher::new();
            text.hash(&mut hasher);
            let scope = if home.is_some_and(|h| path.starts_with(&h.dir)) {
                Scope::Home
            } else if path.starts_with(&aside) {
                Scope::MovedAside
            } else if globals.iter().any(|g| path.starts_with(g)) {
                // A tool's plugin cache holds files the plugin ships, loaded
                // only through the plugin, never as the user's own rules.
                if path
                    .components()
                    .any(|c| c.as_os_str() == "plugins" || c.as_os_str() == ".tmp")
                {
                    Scope::PluginCache
                } else {
                    Scope::Global
                }
            } else {
                match repo_root(&path) {
                    Some(r) => Scope::Project(r),
                    None => Scope::Loose,
                }
            };
            Hit {
                lines: text.lines().count(),
                bytes: text.len() as u64,
                link: fs::read_link(&path).ok(),
                generated: text
                    .lines()
                    .take(3)
                    .find(|l| {
                        let l = l.to_ascii_lowercase();
                        l.contains("generated") || l.contains("do not edit")
                    })
                    .map(|l| l.trim().to_string()),
                hash: hasher.finish(),
                scope,
                path,
            }
        })
        .collect();
    hits.sort_by(|a, b| a.path.cmp(&b.path));

    println!(
        "scanned {} in {:.1}s: {} instruction files.",
        tilde(&root),
        started.elapsed().as_secs_f32(),
        hits.len()
    );
    if !all {
        out::say(
            &format!(
                "The scan skipped {}. To walk them too, add --all.",
                SKIP.join(", ")
            ),
            0,
            width,
        );
    }
    let view = View { long, width };

    view.section(
        "global, loaded in every session",
        None,
        hits.iter().filter(|h| matches!(h.scope, Scope::Global)),
    );
    if let Some(h) = home {
        view.section(
            &format!("home {}", tilde(&h.dir)),
            Some(&h.dir),
            hits.iter().filter(|h| matches!(h.scope, Scope::Home)),
        );
    }
    let mut projects: BTreeMap<&Path, Vec<&Hit>> = BTreeMap::new();
    for h in &hits {
        if let Scope::Project(r) = &h.scope {
            projects.entry(r.as_path()).or_default().push(h);
        }
    }
    for (repo, files) in &projects {
        view.section(
            &format!("project {}", tilde(repo)),
            Some(repo),
            files.iter().copied(),
        );
        let counts = skills::project_counts(repo);
        if counts.iter().any(|(_, n)| *n > 0) {
            let parts: Vec<String> = counts.iter().map(|(p, n)| format!("{p} {n}")).collect();
            out::say(&format!("skills: {}", parts.join(", ")), 2, width);
        }
    }
    view.section(
        "loose, in no git repository",
        None,
        hits.iter().filter(|h| matches!(h.scope, Scope::Loose)),
    );
    view.section(
        &format!("moved aside by agents reset, under {}", tilde(&aside)),
        Some(&aside),
        hits.iter().filter(|h| matches!(h.scope, Scope::MovedAside)),
    );
    view.section(
        "plugin caches",
        None,
        hits.iter()
            .filter(|h| matches!(h.scope, Scope::PluginCache)),
    );
    if hits.iter().any(|h| matches!(h.scope, Scope::PluginCache)) {
        out::say(
            "A tool loads these only through the plugin that ships them.",
            2,
            width,
        );
    }

    // Not an instruction file, but it loads like one: the tool's own notes
    // per repository, machine-local, the first 200 lines of each index.
    let claude = Claude::new();
    let memory = claude.auto_memory_dirs();
    if !memory.is_empty() {
        let base = claude.auto_memory_root();
        out::section(
            &format!("claude code auto memory, under {}", tilde(&base)),
            memory.len(),
            width,
        );
        let items: Vec<out::Row> = memory
            .iter()
            .map(|d| {
                let index = d.join("MEMORY.md");
                let lines = fs::read_to_string(&index)
                    .map(|t| t.lines().count())
                    .unwrap_or(0);
                view.row(&index, Some(&base), lines, String::new())
            })
            .collect();
        out::print(&items, long, width);
        out::say(
            "The first 200 lines of each index load in every session in that repository. To move them aside, run: agents reset --memory",
            2,
            width,
        );
    }

    let mut same: BTreeMap<u64, Vec<&Hit>> = BTreeMap::new();
    for h in hits.iter().filter(|h| h.bytes > 0 && h.link.is_none()) {
        same.entry(h.hash).or_default().push(h);
    }
    let groups: Vec<&Vec<&Hit>> = same.values().filter(|v| v.len() > 1).collect();
    if !groups.is_empty() {
        out::section("identical content", groups.len(), width);
        let sep = if long { "\n" } else { " = " };
        let items: Vec<out::Row> = groups
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let names: Vec<String> = g.iter().map(|h| tilde(&h.path)).collect();
                let name = if long {
                    format!("group {}", i + 1)
                } else {
                    String::new()
                };
                out::Row::new(name, "", names.join(sep))
            })
            .collect();
        out::print(&items, long, width);
        out::say(
            "A copy goes stale in silence. Keep one, and link or import it.",
            2,
            width,
        );
    }
}

struct View {
    long: bool,
    width: usize,
}

impl View {
    /// A file, shown relative to `base` when it is under it. `--long`
    /// shows the full path.
    fn row(&self, path: &Path, base: Option<&Path>, lines: usize, note: String) -> out::Row {
        let shown = match base.and_then(|b| path.strip_prefix(b).ok()) {
            Some(r) if !self.long => r.display().to_string(),
            _ => tilde(path),
        };
        if self.long {
            out::Row::new(shown, format!("{lines} lines"), note)
        } else {
            out::Row::new("", format!("{lines:>5} lines"), format!("{shown}{note}"))
        }
    }

    fn section<'a>(&self, title: &str, base: Option<&Path>, items: impl Iterator<Item = &'a Hit>) {
        let items: Vec<&Hit> = items.collect();
        if items.is_empty() {
            return;
        }
        out::section(title, items.len(), self.width);
        let sep = if self.long { "" } else { "  " };
        let rows: Vec<out::Row> = items
            .iter()
            .map(|h| {
                let note = if let Some(t) = &h.link {
                    format!("{sep}-> {}", tilde(t))
                } else if let Some(g) = &h.generated {
                    format!("{sep}GENERATED: {g}")
                } else {
                    String::new()
                };
                self.row(&h.path, base, h.lines, note)
            })
            .collect();
        out::print(&rows, self.long, self.width);
    }
}

fn keep(e: &DirEntry) -> bool {
    let name = e.file_name().to_string_lossy();
    if SKIP.contains(&name.as_ref()) {
        return false;
    }
    // Session transcripts, not instructions, and a lot of them.
    !(name == "projects"
        && e.path()
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|p| p == ".claude"))
}

fn instruction_file(p: &Path) -> bool {
    let Some(name) = p
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
    else {
        return false;
    };
    if NAMES.contains(&name.as_str()) {
        return true;
    }
    let parent = p.parent().and_then(Path::file_name);
    let grand = p.parent().and_then(Path::parent).and_then(Path::file_name);
    let under =
        |tool: &str| parent.is_some_and(|d| d == "rules") && grand.is_some_and(|d| d == tool);
    (name.ends_with(".md") && under(".claude")) || (name.ends_with(".mdc") && under(".cursor"))
}

fn repo_root(p: &Path) -> Option<PathBuf> {
    p.ancestors()
        .skip(1)
        .find(|d| d.join(".git").exists())
        .map(Path::to_path_buf)
}
