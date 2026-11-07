//! `agents`: point every agent tool on this machine at one `~/.agents` home.
//!
//! The home is a git checkout and this binary embeds none of it. It owns the
//! wiring, the report, and the checks, and runs `git` for everything else.
//! Every command that writes prints its plan and does nothing without
//! `--apply`.

mod check;
mod git;
mod harness;
mod home;
mod plan;
mod reset;
mod scan;
mod skills;
mod status;
mod tree;
mod wire;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use home::Home;

#[derive(Parser)]
#[command(name = "agents", version, about, long_about = None)]
struct Cli {
    /// The agent home. Default: $AGENTS_HOME, then ~/.agents
    #[arg(long, global = true, value_name = "DIR")]
    home: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// What each tool loads, from where, and what is broken
    Status,

    /// The home, or any directory, as a tree: structure and sizes, not content
    Tree {
        /// Directory to show. Default: the home
        #[arg(long, value_name = "DIR")]
        root: Option<PathBuf>,
        /// How many levels to show
        #[arg(long, default_value_t = 4)]
        depth: usize,
        /// Show build output and vendored trees too
        #[arg(long)]
        all: bool,
    },

    /// Find every instruction file on this machine
    Scan {
        /// Directory to walk. Default: your home directory
        #[arg(long, value_name = "DIR")]
        root: Option<PathBuf>,
        /// Walk caches, vendored trees, and Library too
        #[arg(long)]
        all: bool,
    },

    /// Every skill each tool can load, with duplicates, and what to remove
    Skills {
        #[command(subcommand)]
        action: Option<SkillsAction>,
    },

    /// Point this machine's tools at the home
    Wire {
        /// Write the plan. Without it, print only
        #[arg(long)]
        apply: bool,
        /// Link the rules directory instead of writing a drop-in file
        #[arg(long)]
        link: bool,
    },

    /// Remove exactly what `wire` added, and nothing else
    Unwire {
        /// Write the plan. Without it, print only
        #[arg(long)]
        apply: bool,
    },

    /// Context budget and relative links inside the home
    Check,

    /// Clone your home, add the template as upstream, then wire
    Init {
        /// Your own copy of the template, the one you push to
        url: String,
        /// Write the plan. Without it, print only
        #[arg(long)]
        apply: bool,
    },

    /// Merge the template's main into the home, then report
    Update {
        /// Write the plan. Without it, print only
        #[arg(long)]
        apply: bool,
    },

    /// Update, wire, and report, in one step
    Sync {
        /// Write the plan. Without it, print only
        #[arg(long)]
        apply: bool,
    },

    /// Move each tool's own instruction files aside, then wire
    Reset {
        /// Write the plan. Without it, print only
        #[arg(long)]
        apply: bool,
        /// Move the tool's auto memory (its own per-repository notes) aside too
        #[arg(long)]
        memory: bool,
        /// Put back what an earlier reset moved aside
        #[arg(long, value_name = "DIR", conflicts_with = "memory")]
        restore: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SkillsAction {
    /// Move the named skills aside
    Remove {
        names: Vec<String>,
        /// Write the plan. Without it, print only
        #[arg(long)]
        apply: bool,
    },
    /// Move every skill aside except the named ones
    Keep {
        names: Vec<String>,
        /// Write the plan. Without it, print only
        #[arg(long)]
        apply: bool,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("agents: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let ok = ExitCode::SUCCESS;
    match cli.cmd {
        Cmd::Status => {
            status::status(&Home::locate(cli.home)?);
            Ok(ok)
        }
        Cmd::Tree { root, depth, all } => {
            let home = Home::locate(cli.home.clone()).ok();
            let root = match (root, &home) {
                (Some(r), _) => std::path::absolute(r)?,
                (None, Some(h)) => h.dir.clone(),
                (None, None) => std::env::current_dir()?,
            };
            tree::tree(&root, depth, all, home.as_ref());
            Ok(ok)
        }
        Cmd::Scan { root, all } => {
            let home = Home::locate(cli.home).ok();
            scan::scan(root.unwrap_or_else(home::user_home), all, home.as_ref());
            Ok(ok)
        }
        Cmd::Skills { action } => {
            let home = Home::locate(cli.home).ok();
            match action {
                None => skills::list(home.as_ref()),
                Some(SkillsAction::Remove { names, apply }) => {
                    plan::banner(apply);
                    skills::remove(&names, home.as_ref()).run(apply)?;
                }
                Some(SkillsAction::Keep { names, apply }) => {
                    plan::banner(apply);
                    skills::keep(&names, home.as_ref()).run(apply)?;
                }
            }
            Ok(ok)
        }
        Cmd::Wire { apply, link } => {
            let home = Home::locate(cli.home)?;
            plan::banner(apply);
            wire::wire(&home, link).run(apply)?;
            Ok(ok)
        }
        Cmd::Unwire { apply } => {
            let home = Home::locate(cli.home)?;
            plan::banner(apply);
            wire::unwire(&home).run(apply)?;
            Ok(ok)
        }
        Cmd::Check => {
            let passed = check::check(&Home::locate(cli.home)?);
            Ok(if passed { ok } else { ExitCode::FAILURE })
        }
        Cmd::Init { url, apply } => {
            git::init(cli.home, &url, apply)?;
            Ok(ok)
        }
        Cmd::Update { apply } => {
            git::update(&Home::locate(cli.home)?, apply)?;
            Ok(ok)
        }
        Cmd::Sync { apply } => {
            git::sync(&Home::locate(cli.home)?, apply)?;
            Ok(ok)
        }
        Cmd::Reset {
            apply,
            memory,
            restore,
        } => {
            let home = Home::locate(cli.home)?;
            plan::banner(apply);
            if let Some(dir) = restore {
                reset::restore(&dir).run(apply)?;
                return Ok(ok);
            }
            reset::reset(memory).run(apply)?;
            println!();
            if apply {
                // Planned after the moves, so a slot a move emptied can be linked.
                wire::wire(&home, false).run(true)?;
            } else {
                println!("then the wiring plan, planned after the moves, so a slot a move empties can be linked.");
            }
            Ok(ok)
        }
    }
}
