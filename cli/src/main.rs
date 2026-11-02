//! Install and inspect ~/.agents from a dot-agents checkout.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "dot-agents", version = VERSION, about = "Install ~/.agents template")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Copy .agents/ into DOT_AGENTS_HOME (default ~/.agents)
    Install {
        /// Remove files in home that are not in source (destructive)
        #[arg(long)]
        mirror: bool,
    },
    /// Show install location and file count
    Status,
    /// Check foundation files exist
    Doctor,
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

fn agents_source() -> Result<PathBuf> {
    if let Ok(s) = env::var("DOT_AGENTS_SOURCE") {
        return Ok(PathBuf::from(s));
    }
    let mut cwd = env::current_dir()?;
    if cwd.ends_with("cli") {
        cwd.pop();
    }
    let candidate = cwd.join(".agents");
    if candidate.is_dir() {
        return Ok(candidate);
    }
    anyhow::bail!(
        "Set DOT_AGENTS_SOURCE to the repo .agents/ path, or run from the dot-agents repo root"
    )
}

fn copy_dir(src: &Path, dest: &Path, mirror: bool) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in WalkDir::new(src).min_depth(1) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src)?;
        if rel == Path::new("STRUCTURE.md") {
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
        for entry in WalkDir::new(dest).min_depth(1) {
            let entry = entry?;
            if entry.file_type().is_dir() {
                continue;
            }
            let rel = entry.path().strip_prefix(dest)?;
            if rel == Path::new("STRUCTURE.md") {
                continue;
            }
            if !src.join(rel).exists() {
                fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(())
}

fn cmd_install(mirror: bool) -> Result<()> {
    let src = agents_source()?;
    let dest = agents_home();
    copy_dir(&src, &dest, mirror)?;
    println!("Installed from {}", src.display());
    println!("Target: {}", dest.display());
    Ok(())
}

fn cmd_status() -> Result<()> {
    let dest = agents_home();
    if !dest.is_dir() {
        anyhow::bail!("Not installed: {}", dest.display());
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
        "context/writing-standard.md",
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Install { mirror } => cmd_install(mirror),
        Commands::Status => cmd_status(),
        Commands::Doctor => cmd_doctor(),
    }
}
