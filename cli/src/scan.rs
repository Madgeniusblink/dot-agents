//! Find every instruction file on this machine. A tool loads the global ones
//! in every session and the project ones when a session starts inside that
//! repository, so an unlisted one is behavior you did not choose.

use crate::harness::Claude;
use crate::home::{tilde, user_home, Home};
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

pub fn scan(root: PathBuf, all: bool, home: Option<&Home>) {
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

    let skipped = if all {
        String::new()
    } else {
        format!(" (skipped {}; --all walks them)", SKIP.join(", "))
    };
    println!(
        "scanned {} in {:.1}s: {} instruction files{skipped}",
        tilde(&root),
        started.elapsed().as_secs_f32(),
        hits.len()
    );

    section(
        "global (loaded in every session)",
        hits.iter().filter(|h| matches!(h.scope, Scope::Global)),
    );
    if let Some(h) = home {
        section(
            &format!("home {}", tilde(&h.dir)),
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
        section(&format!("project {}", tilde(repo)), files.iter().copied());
    }
    section(
        "loose (no git repository)",
        hits.iter().filter(|h| matches!(h.scope, Scope::Loose)),
    );
    section(
        "moved aside by agents reset",
        hits.iter().filter(|h| matches!(h.scope, Scope::MovedAside)),
    );
    section(
        "plugin caches (loaded only through the plugin that ships them)",
        hits.iter()
            .filter(|h| matches!(h.scope, Scope::PluginCache)),
    );

    // Not an instruction file, but it loads like one: the tool's own notes
    // per repository, machine-local, the first 200 lines of each index.
    let memory = Claude::new().auto_memory_dirs();
    if !memory.is_empty() {
        println!();
        println!(
            "claude code auto memory ({} repositories; the first 200 lines of each index load every session there; agents reset --memory moves them):",
            memory.len()
        );
        for d in &memory {
            let index = d.join("MEMORY.md");
            let lines = fs::read_to_string(&index)
                .map(|t| t.lines().count())
                .unwrap_or(0);
            println!("  {:>5} lines  {}", lines, tilde(&index));
        }
    }

    let mut same: BTreeMap<u64, Vec<&Hit>> = BTreeMap::new();
    for h in hits.iter().filter(|h| h.bytes > 0 && h.link.is_none()) {
        same.entry(h.hash).or_default().push(h);
    }
    let groups: Vec<&Vec<&Hit>> = same.values().filter(|v| v.len() > 1).collect();
    if !groups.is_empty() {
        println!();
        println!("identical content ({} groups). A copy goes stale in silence; keep one and link or import it:", groups.len());
        for g in groups {
            let names: Vec<String> = g.iter().map(|h| tilde(&h.path)).collect();
            println!("  {}", names.join(" = "));
        }
    }
}

fn section<'a>(title: &str, items: impl Iterator<Item = &'a Hit>) {
    let items: Vec<&Hit> = items.collect();
    if items.is_empty() {
        return;
    }
    println!();
    println!("{title}:");
    for h in items {
        let mut note = String::new();
        if let Some(t) = &h.link {
            note = format!("  -> {}", tilde(t));
        } else if let Some(g) = &h.generated {
            note = format!("  GENERATED: {g}");
        }
        println!("  {:>5} lines  {}{note}", h.lines, tilde(&h.path));
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
