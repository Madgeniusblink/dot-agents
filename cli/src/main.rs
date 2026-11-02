//! Standalone `dot-agents` CLI — template is embedded at compile time.

mod readme;
mod upstream;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use include_dir::{include_dir, Dir, DirEntry};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../.agents");
const UPSTREAM_JSON: &str = include_str!("../../.agents/context/upstream.json");

#[derive(Parser)]
#[command(
    name = "dot-agents",
    version = VERSION,
    about = "Install the ~/.agents base template (standalone CLI)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Write embedded template to DOT_AGENTS_HOME (default ~/.agents)
    Install {
        /// Remove files in home that are not in the template (destructive)
        #[arg(long)]
        mirror: bool,
        /// Use a checkout's .agents/ instead of the embedded template (dev)
        #[arg(long, value_name = "PATH")]
        from: Option<PathBuf>,
    },
    /// Show install location and file count
    Status,
    /// Check foundation files exist
    Doctor,
    /// Update or verify generated README tree section
    Readme {
        #[command(subcommand)]
        command: ReadmeCommands,
    },
    /// Print canonical upstream repo links (from embedded upstream.json)
    Upstream {
        /// Output JSON instead of human text
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ReadmeCommands {
    /// Write generated tree into README.md (run from repo root)
    Sync,
    /// Exit with error if README tree is stale
    Check,
    /// Print generated tree to stdout
    Print,
}

fn agents_home() -> PathBuf {
    env::var("DOT_AGENTS_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            env::var("HOME")
                .map(|h| PathBuf::from(h).join(".agents"))
                .unwrap_or_else(|_| PathBuf::from(".agents"))
        })
}

fn resolve_filesystem_source(from: Option<PathBuf>) -> Result<Option<PathBuf>> {
    if let Some(p) = from {
        return Ok(Some(p));
    }
    if let Ok(p) = env::var("DOT_AGENTS_SOURCE") {
        return Ok(Some(PathBuf::from(p)));
    }
    let mut cwd = env::current_dir()?;
    if cwd.ends_with("cli") {
        cwd.pop();
    }
    let candidate = cwd.join(".agents");
    if candidate.is_dir() {
        return Ok(Some(candidate));
    }
    Ok(None)
}

fn skip_template_file(rel: &Path) -> bool {
    rel == Path::new("STRUCTURE.md")
}

fn install_from_dir(src: &Path, dest: &Path, mirror: bool) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in WalkDir::new(src).min_depth(1) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src)?;
        if skip_template_file(rel) {
            continue;
        }
        let target = dest.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)?;
        }
    }
    if mirror {
        prune_extra_filesystem(src, dest)?;
    }
    Ok(())
}

fn prune_extra_filesystem(src: &Path, dest: &Path) -> Result<()> {
    for entry in WalkDir::new(dest).min_depth(1) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(dest)?;
        if skip_template_file(rel) {
            continue;
        }
        if !src.join(rel).exists() {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn collect_template_paths(dir: &Dir<'_>, out: &mut Vec<PathBuf>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(d) => collect_template_paths(d, out),
            DirEntry::File(f) => {
                let rel = f.path().to_path_buf();
                if !skip_template_file(&rel) {
                    out.push(rel);
                }
            }
        }
    }
}

fn extract_dir(dir: &Dir<'_>, dest: &Path) -> Result<()> {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(d) => extract_dir(d, dest)?,
            DirEntry::File(f) => {
                let rel = f.path();
                if skip_template_file(rel) {
                    continue;
                }
                let target = dest.join(rel);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&target, f.contents())
                    .with_context(|| format!("write {}", target.display()))?;
            }
        }
    }
    Ok(())
}

fn install_embedded(dest: &Path, mirror: bool) -> Result<()> {
    fs::create_dir_all(dest)?;
    let mut template_paths: Vec<PathBuf> = Vec::new();
    collect_template_paths(&TEMPLATE, &mut template_paths);
    extract_dir(&TEMPLATE, dest)?;
    if mirror {
        for entry in WalkDir::new(dest).min_depth(1) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry.path().strip_prefix(dest)?;
            if skip_template_file(rel) {
                continue;
            }
            if !template_paths.iter().any(|p| p.as_path() == rel) {
                fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(())
}

fn cmd_install(mirror: bool, from: Option<PathBuf>) -> Result<()> {
    let dest = agents_home();
    if let Some(src) = resolve_filesystem_source(from)? {
        install_from_dir(&src, &dest, mirror)?;
        println!("dot-agents {VERSION}");
        println!("Installed from {}", src.display());
    } else {
        install_embedded(&dest, mirror)?;
        println!("dot-agents {VERSION}");
        println!("Installed from embedded template (v{VERSION})");
    }
    println!("Target: {}", dest.display());
    Ok(())
}

fn cmd_status() -> Result<()> {
    let dest = agents_home();
    if !dest.is_dir() {
        anyhow::bail!("Not installed: {}. Run: dot-agents install", dest.display());
    }
    let count = WalkDir::new(&dest)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();
    println!("dot-agents {VERSION}");
    println!("Home: {}", dest.display());
    println!("Files: {count}");
    Ok(())
}

fn cmd_doctor() -> Result<()> {
    let dest = agents_home();
    let required = [
        "AGENTS.md",
        "standards/README.md",
        "standards/extending.md",
        "context/writing-standard.md",
        "context/upstream.json",
    ];
    let mut ok = true;
    for f in required {
        let p = dest.join(f);
        if p.is_file() {
            println!("ok  {f}");
        } else {
            println!("missing  {f}");
            ok = false;
        }
    }
    if !ok {
        anyhow::bail!("Run: dot-agents install");
    }
    Ok(())
}

fn cmd_upstream(json: bool) -> Result<()> {
    let u = upstream::parse_upstream(UPSTREAM_JSON)?;
    if json {
        println!("{}", UPSTREAM_JSON.trim());
    } else {
        print!("{}", upstream::format_human(&u));
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Install { mirror, from } => cmd_install(mirror, from),
        Commands::Status => cmd_status(),
        Commands::Doctor => cmd_doctor(),
        Commands::Readme { command } => match command {
            ReadmeCommands::Sync => {
                let repo = readme::find_repo_root()?;
                readme::sync_readme(&repo)
            }
            ReadmeCommands::Check => {
                let repo = readme::find_repo_root()?;
                readme::check_readme(&repo)
            }
            ReadmeCommands::Print => {
                let repo = readme::find_repo_root()?;
                print!("{}", readme::generate_tree_markdown(&repo)?);
                Ok(())
            }
        },
        Commands::Upstream { json } => cmd_upstream(json),
    }
}
