//! Find every instruction file on this machine. A tool loads the global ones
//! in every session and the project ones when a session starts inside that
//! repository, so an unlisted one is behavior you did not choose.

use crate::config::{self, Config};
use crate::harness::{self, Claude};
use crate::home::{expand_home, git_in, tilde, user_home, Home};
use crate::{out, skills, status};
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
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

/// No `long`: one row per remote and counts. Once: every file. Twice: each
/// file in full. `[scan] skip` globs prune the walk, even with `--all`.
pub fn scan(
    roots: Vec<PathBuf>,
    all: bool,
    long: u8,
    json: bool,
    home: Option<&Home>,
    cfg: &Config,
) {
    let width = out::width();
    let started = Instant::now();
    let user = user_home();
    let globals = [
        user.join(".claude"),
        user.join(".codex"),
        user.join(".cursor"),
    ];
    let aside = user.join(".agents-reset");

    let scope_of = |path: &Path| {
        if home.is_some_and(|h| path.starts_with(&h.dir)) {
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
            match repo_root(path) {
                Some(r) => Scope::Project(r),
                None => Scope::Loose,
            }
        }
    };
    let (found, mut setting_paths): (Vec<PathBuf>, Vec<PathBuf>) = roots
        .iter()
        .flat_map(|root| {
            WalkDir::new(root)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| (all || keep(e)) && !cfg.skipped(e.path()))
        })
        .flatten()
        .filter(|e| {
            !e.file_type().is_dir()
                && (instruction_file(e.path()) || harness::settings_tool(e.path()).is_some())
        })
        .map(DirEntry::into_path)
        .partition(|p| instruction_file(p));
    setting_paths.extend(
        harness::settings_globals()
            .into_iter()
            .filter(|p| p.is_file()),
    );
    setting_paths.sort();
    setting_paths.dedup();
    let settings: Vec<(Setting, Scope)> = setting_paths
        .iter()
        .map(|p| (Setting::read(p), scope_of(p)))
        .collect();
    let mut hits: Vec<Hit> = found
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).unwrap_or_default();
            let mut hasher = DefaultHasher::new();
            text.hash(&mut hasher);
            let scope = scope_of(&path);
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
    hits.dedup_by(|a, b| a.path == b.path);
    let mut projects: BTreeMap<&Path, Vec<&Hit>> = BTreeMap::new();
    for h in &hits {
        if let Scope::Project(r) = &h.scope {
            projects.entry(r.as_path()).or_default().push(h);
        }
    }
    let checkouts: BTreeMap<&Path, Checkout> =
        projects.keys().map(|r| (*r, Checkout::read(r))).collect();
    if json {
        let data = data(
            &roots,
            cfg,
            all,
            started.elapsed().as_secs_f32(),
            &hits,
            &checkouts,
            &settings,
        );
        out::json("scan", home.map(|h| h.dir.as_path()), data);
        return;
    }

    let shown: Vec<String> = roots.iter().map(|r| tilde(r)).collect();
    if long == 0 {
        brief(
            &shown.join(", "),
            started,
            home,
            &hits,
            &checkouts,
            &settings,
            width,
        );
        return;
    }
    let long = long >= 2;
    println!(
        "scanned {} in {:.1}s: {} instruction files.",
        shown.join(", "),
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
    for (repo, files) in &projects {
        view.section(
            &format!("project {}", tilde(repo)),
            Some(repo),
            files.iter().copied(),
        );
        out::say(&checkouts[repo].line(), 2, width);
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

    if !settings.is_empty() {
        out::section("tool settings files", settings.len(), width);
        let rows: Vec<out::Row> = settings
            .iter()
            .map(|(f, scope)| {
                out::Row::new(
                    f.tool,
                    format!("{:>7} bytes", f.bytes),
                    format!(
                        "{}  {}  {}",
                        scope_name(scope),
                        f.validity(),
                        tilde(&f.path)
                    ),
                )
            })
            .collect();
        out::print(&rows, long, width);
        out::say(
            "To see which apply from a directory, and in what order, run: agents scan --from <dir>",
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

    let remotes = same_remote(&checkouts);
    if !remotes.is_empty() {
        out::section("same remote in more than one place", remotes.len(), width);
        let mut items = Vec::new();
        for (remote, group) in &remotes {
            for repo in group {
                let date = checkouts[*repo].when.as_ref().map_or("no commit", |w| &w.1);
                items.push(out::Row::new(*remote, date, tilde(repo)));
            }
        }
        out::print(&items, long, width);
    }

    let groups = identical(&hits);
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
                let mut text: Vec<String> = per_remote(g, &checkouts)
                    .iter()
                    .map(|(r, n)| format!("same file in {n} checkouts of {r}"))
                    .collect();
                text.push(names.join(sep));
                out::Row::new(name, "", text.join(if long { "\n" } else { ": " }))
            })
            .collect();
        out::print(&items, long, width);
        out::say(
            "A copy goes stale in silence. Keep one, and link or import it.",
            2,
            width,
        );
    }

    let orphans: Vec<&Path> = checkouts
        .iter()
        .filter(|(_, c)| matches!(c.kind, Kind::Orphan))
        .map(|(r, _)| *r)
        .collect();
    if !orphans.is_empty() {
        println!();
    }
    for repo in orphans {
        let text = match orphan_source(repo, &checkouts) {
            Some(s) => format!(
                "{} is an orphan worktree. After you remove it, clear its record with: git -C {} worktree prune",
                tilde(repo),
                tilde(s)
            ),
            None => format!(
                "{} is an orphan worktree. Its source repository is not under the root.",
                tilde(repo)
            ),
        };
        out::say(&text, 0, width);
    }
}

/// One row per remote: how many checkouts, the newest, and `stale`,
/// `orphan`, or `copy` where they apply. Then counts, and no file list.
fn brief(
    roots: &str,
    started: Instant,
    home: Option<&Home>,
    hits: &[Hit],
    checkouts: &BTreeMap<&Path, Checkout>,
    settings: &[(Setting, Scope)],
    width: usize,
) {
    out::say(
        &format!(
            "scanned {roots} in {:.1}s: {} instruction files in {} checkouts.",
            started.elapsed().as_secs_f32(),
            hits.len(),
            checkouts.len()
        ),
        0,
        width,
    );
    let mut groups: BTreeMap<&str, Vec<(&Path, &Checkout)>> = BTreeMap::new();
    for (repo, c) in checkouts {
        groups
            .entry(c.remote.as_deref().unwrap_or("no remote"))
            .or_default()
            .push((repo, c));
    }
    let items: Vec<out::Row> = groups
        .iter()
        .map(|(remote, group)| {
            let newest = group
                .iter()
                .max_by(|a, b| a.1.when.cmp(&b.1.when))
                .map_or(String::new(), |(r, _)| tilde(r));
            let stale = group.iter().filter(|(_, c)| c.stale()).count();
            let orphan = group
                .iter()
                .filter(|(_, c)| matches!(c.kind, Kind::Orphan))
                .count();
            let clones = group
                .iter()
                .filter(|(_, c)| matches!(c.kind, Kind::Clone))
                .count();
            let mut marks = Vec::new();
            if stale > 0 {
                marks.push(format!("{stale} stale"));
            }
            if orphan > 0 {
                marks.push(format!("{orphan} orphan"));
            }
            if *remote != "no remote" && clones > 1 {
                marks.push("copy".to_string());
            }
            let n = group.len();
            let unit = if n == 1 { "checkout" } else { "checkouts" };
            out::Row::new(
                *remote,
                format!("{n:>3} {unit}"),
                format!("{newest}  {}", marks.join(", ")),
            )
        })
        .collect();
    out::section("checkouts by remote, newest shown", items.len(), width);
    out::print(&items, false, width);
    println!();
    let count = |want: fn(&Scope) -> bool| hits.iter().filter(|h| want(&h.scope)).count();
    println!(
        "global, loaded in every session: {} files",
        count(|s| matches!(s, Scope::Global))
    );
    if let Some(h) = home {
        let fixtures = h.dir.join("cli/tests/fixtures");
        let (fixtures, files): (Vec<&Hit>, Vec<&Hit>) = hits
            .iter()
            .filter(|x| matches!(x.scope, Scope::Home))
            .partition(|x| x.path.starts_with(&fixtures));
        let unit = if files.len() == 1 { "file" } else { "files" };
        let mut line = format!("home {}: {} {unit}", tilde(&h.dir), files.len());
        match fixtures.len() {
            0 => {}
            1 => line.push_str(", 1 test fixture"),
            n => line.push_str(&format!(", {n} test fixtures")),
        }
        println!("{line}");
    }
    let claude = Claude::new();
    println!(
        "auto memory: {} indexes under {}",
        claude.auto_memory_dirs().len(),
        tilde(&claude.auto_memory_root())
    );
    println!("identical content: {} groups", identical(hits).len());
    println!(
        "tool settings files: {}, {} invalid",
        settings.len(),
        settings
            .iter()
            .filter(|(f, _)| matches!(f.state, State::Invalid(..)))
            .count()
    );
    println!();
    println!("every file: agents scan --long");
}

/// Checkouts of one remote found in more than one place, each group newest
/// last commit first.
fn same_remote<'a>(checkouts: &'a BTreeMap<&Path, Checkout>) -> Vec<(&'a str, Vec<&'a Path>)> {
    let mut remotes: BTreeMap<&str, Vec<&Path>> = BTreeMap::new();
    for (repo, c) in checkouts {
        if let Some(r) = &c.remote {
            remotes.entry(r).or_default().push(repo);
        }
    }
    remotes
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(r, mut v)| {
            v.sort_by(|a, b| checkouts[b].when.cmp(&checkouts[a].when));
            (r, v)
        })
        .collect()
}

/// How many files of one identical group sit in checkouts of each remote,
/// for remotes with more than one.
fn per_remote<'a>(
    group: &[&Hit],
    checkouts: &'a BTreeMap<&Path, Checkout>,
) -> Vec<(&'a str, usize)> {
    let mut per: BTreeMap<&str, std::collections::BTreeSet<&Path>> = BTreeMap::new();
    for h in group {
        if let Scope::Project(r) = &h.scope {
            if let Some(remote) = &checkouts[r.as_path()].remote {
                per.entry(remote).or_default().insert(r.as_path());
            }
        }
    }
    per.into_iter()
        .map(|(r, set)| (r, set.len()))
        .filter(|(_, n)| *n > 1)
        .collect()
}

/// The checkout under the root that still holds a record pointing back at
/// this orphan worktree.
fn orphan_source<'a>(repo: &Path, checkouts: &'a BTreeMap<&Path, Checkout>) -> Option<&'a Path> {
    let me = fs::canonicalize(repo.join(".git")).ok();
    checkouts.keys().copied().find(|s| {
        fs::read_dir(s.join(".git").join("worktrees"))
            .into_iter()
            .flatten()
            .flatten()
            .any(|w| {
                fs::read_to_string(w.path().join("gitdir"))
                    .ok()
                    .and_then(|g| fs::canonicalize(g.trim()).ok())
                    .is_some_and(|g| Some(g) == me)
            })
    })
}

/// What a project root is, read from local files and refs only. Nothing
/// here fetches: ahead and behind are against the upstream ref on disk.
struct Checkout {
    remote: Option<String>,
    kind: Kind,
    branch: Option<String>,
    /// Ahead and behind, or None when the branch has no upstream ref.
    ab: Option<(u64, u64)>,
    dirty: usize,
    /// Commit time in seconds, and the date as `YYYY-MM-DD`.
    when: Option<(u64, String)>,
}

enum Kind {
    Clone,
    Worktree(PathBuf),
    /// The `.git` file names a directory under the superproject's
    /// `.git/modules`.
    Submodule(PathBuf),
    /// The `.git` file names a directory that no longer exists.
    Orphan,
}

impl Checkout {
    fn read(dir: &Path) -> Checkout {
        let kind = match fs::read_to_string(dir.join(".git")) {
            Err(_) => Kind::Clone,
            Ok(text) => {
                let record = native(dir, text.trim().trim_start_matches("gitdir:").trim());
                match fs::read_to_string(record.join("commondir")) {
                    Ok(rel) => {
                        let common = native(&record, rel.trim());
                        Kind::Worktree(match common.file_name() {
                            Some(n) if n == ".git" => common.parent().unwrap_or(&common).into(),
                            _ => common,
                        })
                    }
                    Err(_) if !record.is_dir() => Kind::Orphan,
                    Err(_) => match superproject(&record) {
                        Some(root) => Kind::Submodule(root),
                        None => Kind::Clone,
                    },
                }
            }
        };
        let mut c = Checkout {
            remote: None,
            kind,
            branch: None,
            ab: None,
            dirty: 0,
            when: None,
        };
        if matches!(c.kind, Kind::Orphan) {
            return c;
        }
        c.remote = git_in(dir, &["config", "--get", "remote.origin.url"]).map(|u| owner_repo(&u));
        let status = git_in(
            dir,
            &[
                "--no-optional-locks",
                "status",
                "--porcelain=v2",
                "--branch",
            ],
        )
        .unwrap_or_default();
        for l in status.lines() {
            if let Some(b) = l.strip_prefix("# branch.head ") {
                c.branch = Some(b.to_string());
            } else if let Some(ab) = l.strip_prefix("# branch.ab ") {
                let mut n = ab.split(' ').map(|x| x[1..].parse().unwrap_or(0));
                c.ab = Some((n.next().unwrap_or(0), n.next().unwrap_or(0)));
            } else if !l.starts_with('#') {
                c.dirty += 1;
            }
        }
        c.when = git_in(dir, &["log", "-1", "--format=%ct %cs"]).and_then(|l| {
            let (t, d) = l.split_once(' ')?;
            Some((t.parse().ok()?, d.to_string()))
        });
        c
    }

    /// Older than 30 days and nothing to push. An orphan is always stale.
    fn stale(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        matches!(self.kind, Kind::Orphan)
            || (self.ab.is_some_and(|(a, _)| a == 0)
                && self
                    .when
                    .as_ref()
                    .is_some_and(|(t, _)| now.saturating_sub(*t) > 30 * 86400))
    }

    fn line(&self) -> String {
        let kind = match &self.kind {
            Kind::Clone => "clone".to_string(),
            Kind::Worktree(main) => format!("worktree of {}", tilde(main)),
            Kind::Submodule(root) => format!("submodule of {}", tilde(root)),
            Kind::Orphan => "orphan worktree".to_string(),
        };
        let mut parts = vec![
            format!("remote {}", self.remote.as_deref().unwrap_or("none")),
            kind,
        ];
        if !matches!(self.kind, Kind::Orphan) {
            parts.push(format!(
                "branch {}",
                self.branch.as_deref().unwrap_or("unknown")
            ));
            parts.push(match self.ab {
                Some((a, b)) => format!("{a} ahead {b} behind"),
                None => "no upstream".to_string(),
            });
            parts.push(format!("{} uncommitted", self.dirty));
            parts.push(match &self.when {
                Some((_, d)) => format!("last commit {d}"),
                None => "no commit".to_string(),
            });
        }
        if self.stale() {
            parts.push("stale".to_string());
        }
        format!("checkout: {}", parts.join(", "))
    }
}

/// A path git wrote, relative to `base` or absolute, in this system's form.
/// Git writes `/` on every system; the path is rebuilt from its parts so it
/// prints and compares like every path the walk found.
/// The checkout whose `.git/modules` holds this submodule's git directory.
fn superproject(record: &Path) -> Option<PathBuf> {
    record
        .ancestors()
        .find(|a| {
            a.file_name().is_some_and(|n| n == "modules")
                && a.parent()
                    .is_some_and(|p| p.file_name().is_some_and(|n| n == ".git"))
        })
        .and_then(|m| m.parent()?.parent())
        .map(Path::to_path_buf)
}

fn native(base: &Path, git_path: &str) -> PathBuf {
    let mut out = PathBuf::new();
    for part in base.join(git_path).components() {
        match part {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            p => out.push(p),
        }
    }
    out
}

/// `owner/repo` from a remote URL or path: its last two parts, without
/// `.git`.
fn owner_repo(url: &str) -> String {
    let url = url.trim_end_matches('/').trim_end_matches(".git");
    let parts: Vec<&str> = url.rsplit(['/', ':', '\\']).take(2).collect();
    match parts.as_slice() {
        [repo, owner] => format!("{owner}/{repo}"),
        _ => url.to_string(),
    }
}

/// Files with the same content, each group in path order.
fn identical(hits: &[Hit]) -> Vec<Vec<&Hit>> {
    let mut same: BTreeMap<u64, Vec<&Hit>> = BTreeMap::new();
    for h in hits.iter().filter(|h| h.bytes > 0 && h.link.is_none()) {
        same.entry(h.hash).or_default().push(h);
    }
    same.into_values().filter(|v| v.len() > 1).collect()
}

/// The same facts the text shows, as data.
fn data(
    roots: &[PathBuf],
    cfg: &Config,
    all: bool,
    seconds: f32,
    hits: &[Hit],
    checkouts: &BTreeMap<&Path, Checkout>,
    settings: &[(Setting, Scope)],
) -> Value {
    let project_of = |s: &Scope| match s {
        Scope::Project(r) => Some(tilde(r)),
        _ => None,
    };
    let files: Vec<Value> = hits
        .iter()
        .map(|h| {
            json!({
                "path": tilde(&h.path),
                "scope": scope_name(&h.scope),
                "project": project_of(&h.scope),
                "lines": h.lines,
                "bytes": h.bytes,
                "link": h.link.as_deref().map(tilde),
                "generated": h.generated,
            })
        })
        .collect();
    let projects: Vec<Value> = checkouts
        .iter()
        .map(|(r, c)| {
            let skills: serde_json::Map<String, Value> = skills::project_counts(r)
                .into_iter()
                .map(|(p, n)| (p.to_string(), json!(n)))
                .collect();
            let (kind, main) = match &c.kind {
                Kind::Clone => ("clone", None),
                Kind::Worktree(m) => ("worktree", Some(tilde(m))),
                Kind::Submodule(m) => ("submodule", Some(tilde(m))),
                Kind::Orphan => ("orphan worktree", None),
            };
            json!({
                "root": tilde(r),
                "skills": skills,
                "remote": c.remote,
                "kind": kind,
                "main_checkout": main,
                "branch": c.branch,
                "ahead": c.ab.map(|a| a.0),
                "behind": c.ab.map(|a| a.1),
                "uncommitted": c.dirty,
                "last_commit": c.when.as_ref().map(|w| &w.1),
                "stale": c.stale(),
                "prune": matches!(c.kind, Kind::Orphan)
                    .then(|| orphan_source(r, checkouts))
                    .flatten()
                    .map(|s| format!("git -C {} worktree prune", tilde(s))),
            })
        })
        .collect();
    let same_remote: Vec<Value> = same_remote(checkouts)
        .iter()
        .map(|(r, v)| json!({"remote": r, "checkouts": v.iter().map(|p| tilde(p)).collect::<Vec<_>>()}))
        .collect();
    let claude = Claude::new();
    let auto_memory: Vec<Value> = claude
        .auto_memory_dirs()
        .iter()
        .map(|d| {
            let index = d.join("MEMORY.md");
            let lines = fs::read_to_string(&index)
                .map(|t| t.lines().count())
                .unwrap_or(0);
            json!({"index": tilde(&index), "lines": lines})
        })
        .collect();
    let groups = identical(hits);
    let identical_remotes: Vec<Value> = groups
        .iter()
        .map(|g| {
            let per = per_remote(g, checkouts);
            json!(per
                .iter()
                .map(|(r, n)| json!({"remote": r, "checkouts": n}))
                .collect::<Vec<_>>())
        })
        .collect();
    let identical: Vec<Vec<String>> = groups
        .iter()
        .map(|g| g.iter().map(|h| tilde(&h.path)).collect())
        .collect();
    json!({
        "machine": config::machine(),
        "config": cfg.json(),
        "root": roots.first().map(|r| tilde(r)),
        "roots": roots.iter().map(|r| tilde(r)).collect::<Vec<_>>(),
        "seconds": (seconds * 10.0).round() / 10.0,
        "skipped": if all { Vec::new() } else { SKIP.to_vec() },
        "count": hits.len(),
        "files": files,
        "projects": projects,
        "auto_memory": {"root": tilde(&claude.auto_memory_root()), "indexes": auto_memory},
        "identical": identical,
        "identical_remotes": identical_remotes,
        "same_remote": same_remote,
        "settings": settings
            .iter()
            .map(|(f, scope)| {
                let mut v = f.json();
                v["scope"] = json!(scope_name(scope));
                v["project"] = json!(project_of(scope));
                v
            })
            .collect::<Vec<_>>(),
    })
}

fn scope_name(s: &Scope) -> &'static str {
    match s {
        Scope::Home => "home",
        Scope::Global => "global",
        Scope::Project(_) => "project",
        Scope::PluginCache => "plugin-cache",
        Scope::MovedAside => "moved-aside",
        Scope::Loose => "loose",
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

enum State {
    Valid(Value),
    /// The line of the parse error, and the error.
    Invalid(usize, String),
    Empty,
}

/// One tool settings file: what it is and whether it parses.
struct Setting {
    path: PathBuf,
    tool: &'static str,
    bytes: u64,
    state: State,
}

/// The entries of one settings file that change behavior.
#[derive(Default)]
struct Entries {
    /// Event, command, and true when a path in the command is gone.
    hooks: Vec<(String, String, bool)>,
    deny: Vec<String>,
    allow: Vec<String>,
    /// Allow entries that name a path that is gone.
    dead_allow: Vec<String>,
    env: Vec<String>,
    model: Option<String>,
    effort: Option<String>,
}

impl Setting {
    fn read(path: &Path) -> Setting {
        let text = fs::read_to_string(path).unwrap_or_default();
        let state = if text.trim().is_empty() {
            State::Empty
        } else if path.extension().is_some_and(|e| e == "toml") {
            match basic_toml::from_str::<Value>(&text) {
                Ok(v) => State::Valid(v),
                Err(e) => State::Invalid(e.line_col().map_or(0, |(l, _)| l + 1), e.to_string()),
            }
        } else {
            match serde_json::from_str::<Value>(&text) {
                Ok(v) => State::Valid(v),
                Err(e) => State::Invalid(e.line(), e.to_string()),
            }
        };
        Setting {
            path: path.to_path_buf(),
            tool: harness::settings_tool(path).unwrap_or("unknown"),
            bytes: fs::metadata(path).map_or(0, |m| m.len()),
            state,
        }
    }

    /// A file that does not parse changes nothing the scan can show.
    fn applies(&self) -> bool {
        !matches!(self.state, State::Invalid(..))
    }

    fn validity(&self) -> String {
        match &self.state {
            State::Valid(_) => "valid".to_string(),
            State::Invalid(line, _) => format!("invalid at line {line}"),
            State::Empty => "empty".to_string(),
        }
    }

    fn entries(&self) -> Entries {
        let State::Valid(v) = &self.state else {
            return Entries::default();
        };
        let keys = harness::settings_keys(&self.path);
        let at = |k: Option<&str>| k.and_then(|k| v.pointer(k));
        let strings = |k: Option<&str>| -> Vec<String> {
            at(k)
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default()
        };
        let scalar =
            |k: Option<&str>| at(k).map(|x| x.as_str().map_or(x.to_string(), str::to_string));
        let mut hooks = Vec::new();
        if let Some(Value::Object(events)) = at(keys.hooks) {
            for (event, x) in events {
                status::commands(x, &mut |cmd| {
                    hooks.push((event.clone(), cmd.to_string(), dead(cmd)));
                });
            }
        }
        let allow = strings(keys.allow);
        Entries {
            hooks,
            deny: strings(keys.deny),
            dead_allow: allow
                .iter()
                .filter(|a| {
                    let inner = a
                        .split_once('(')
                        .map_or("", |(_, r)| r.trim_end_matches(')'));
                    dead(inner)
                })
                .cloned()
                .collect(),
            allow,
            env: at(keys.env)
                .and_then(Value::as_object)
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default(),
            model: scalar(keys.model),
            effort: scalar(keys.effort),
        }
    }

    fn json(&self) -> Value {
        let (line, error) = match &self.state {
            State::Invalid(l, e) => (Some(*l), Some(e.as_str())),
            _ => (None, None),
        };
        let e = self.entries();
        json!({
            "path": tilde(&self.path),
            "tool": self.tool,
            "bytes": self.bytes,
            "validity": match self.state {
                State::Valid(_) => "valid",
                State::Invalid(..) => "invalid",
                State::Empty => "empty",
            },
            "error_line": line,
            "error": error,
            "hooks": e.hooks.iter().map(|(ev, c, d)| json!({"event": ev, "command": c, "dead": d})).collect::<Vec<_>>(),
            "deny": e.deny,
            "allow": e.allow.len(),
            "dead_allow": e.dead_allow,
            "env": e.env,
            "model": e.model,
            "effort": e.effort,
        })
    }
}

/// True when the command names an absolute path, or one under `~`, that
/// does not exist. A glob is not a path.
fn dead(cmd: &str) -> bool {
    cmd.split_whitespace().any(|t| {
        let t = t.trim_matches(['"', '\'']).trim_end_matches(":*");
        let p = expand_home(t);
        !t.contains('*') && p.has_root() && !p.exists()
    })
}

/// `agents scan --from <dir>`: for each tool, the settings files that
/// apply in a session started in `dir`, most specific last, what each one
/// changes, and a deny that blocks an allow in another file. Read only.
pub fn from(dir: &Path, json: bool, home: Option<&Home>, cfg: &Config) {
    let width = out::width();
    let root = dir
        .ancestors()
        .find(|d| d.join(".git").exists())
        .map(Path::to_path_buf);
    let main = root.as_ref().map(|r| match Checkout::read(r).kind {
        Kind::Worktree(m) => m,
        _ => r.clone(),
    });
    let orders = harness::settings_order(dir, root.as_deref(), main.as_deref());
    let mut tools = Vec::new();
    // Each tool's last file: the one that wins for that tool.
    let mut applying: Vec<PathBuf> = Vec::new();
    for o in &orders {
        // From the user home, a project path can be the user file itself.
        let mut seen = std::collections::BTreeSet::new();
        let files: Vec<(Setting, &str)> = o
            .files
            .iter()
            .filter(|(p, _)| p.is_file() && seen.insert(p))
            .map(|(p, scope)| (Setting::read(p), *scope))
            .collect();
        tools.push((o, files));
    }
    let mut conflicts: Vec<(String, PathBuf, PathBuf)> = Vec::new();
    for (_, files) in &tools {
        let on: Vec<&Setting> = files
            .iter()
            .map(|(f, _)| f)
            .filter(|f| f.applies())
            .collect();
        for d in &on {
            for a in &on {
                if d.path == a.path {
                    continue;
                }
                let allow = a.entries().allow;
                for rule in d.entries().deny {
                    if allow.contains(&rule) {
                        conflicts.push((rule, d.path.clone(), a.path.clone()));
                    }
                }
            }
        }
        applying.extend(on.last().map(|f| f.path.clone()));
    }
    // The deepest of those; among equals, the first tool's.
    let edit = applying
        .iter()
        .rev()
        .max_by_key(|p| p.components().count())
        .map(|p| format!("$EDITOR {}", tilde(p)));

    if json {
        let data = json!({
            "machine": config::machine(),
            "config": cfg.json(),
            "from": tilde(dir),
            "checkout": root.as_deref().map(tilde),
            "tools": tools.iter().map(|(o, files)| json!({
                "tool": o.tool,
                "order": if o.known { "documented" } else { "unknown" },
                "note": harness::settings_note(o.tool),
                "files": files.iter().map(|(f, scope)| {
                    let mut v = f.json();
                    v["scope"] = json!(scope);
                    v["applies"] = json!(f.applies());
                    v
                }).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "conflicts": conflicts.iter().map(|(r, d, a)| json!({"rule": r, "deny": tilde(d), "allow": tilde(a)})).collect::<Vec<_>>(),
            "edit": edit,
        });
        out::json("scan", home.map(|h| h.dir.as_path()), data);
        return;
    }

    println!(
        "settings that apply from {}, lowest precedence first:",
        tilde(dir)
    );
    for (o, files) in tools.iter().filter(|(_, f)| !f.is_empty()) {
        let title = if o.known {
            o.tool.to_string()
        } else {
            format!("{}, order unknown", o.tool)
        };
        out::section(&title, files.len(), width);
        for (f, scope) in files {
            let mark = if f.applies() { "applies" } else { "broken" };
            let line = format!("  {scope}  {}  {}  {mark}", f.validity(), tilde(&f.path));
            println!("{}", out::fit(&line, width));
            let e = f.entries();
            let mut lines: Vec<String> = Vec::new();
            for (event, cmd, gone) in &e.hooks {
                let tail = if *gone { "  dead" } else { "" };
                lines.push(format!("hook {event}: {cmd}{tail}"));
            }
            lines.extend(e.deny.iter().map(|d| format!("deny: {d}")));
            if !e.allow.is_empty() {
                lines.push(format!("allow: {} entries", e.allow.len()));
            }
            lines.extend(e.dead_allow.iter().map(|a| format!("allow: {a}  dead")));
            lines.extend(e.env.iter().map(|k| format!("env: {k}")));
            lines.extend(e.model.iter().map(|m| format!("model: {m}")));
            lines.extend(e.effort.iter().map(|m| format!("effort: {m}")));
            for l in lines {
                println!("{}", out::fit(&format!("    {l}"), width));
            }
        }
        if let Some(n) = harness::settings_note(o.tool) {
            out::say(n, 2, width);
        }
    }
    if !conflicts.is_empty() {
        out::section("conflict", conflicts.len(), width);
        for (rule, d, a) in &conflicts {
            out::say(
                &format!(
                    "conflict: {rule} is denied in {} and allowed in {}",
                    tilde(d),
                    tilde(a)
                ),
                2,
                width,
            );
        }
    }
    if let Some(e) = edit {
        println!();
        println!("{e}");
    }
}
