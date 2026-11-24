//! `agents`: point every agent tool on this machine at one `~/.agents` home.
//!
//! The home is a git checkout and this binary embeds none of it. It owns the
//! wiring, the report, and the checks, and runs `git` for everything else.
//! Every command that writes prints its plan and does nothing without
//! `--apply`.

mod check;
mod config;
mod git;
mod harness;
mod home;
mod memory;
mod out;
mod owner_decision;
mod plan;
mod reset;
mod scan;
mod skills;
mod status;
mod tree;
mod ui;
mod version;
mod wire;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use home::Home;

#[derive(Parser)]
#[command(name = "agents", version = version::VERSION, about, long_about = None)]
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
    Status {
        /// Print the full report
        #[arg(long)]
        long: bool,
        /// Print one JSON document instead of the text
        #[arg(long)]
        json: bool,
    },

    /// A full-screen view of the reports. Unix only
    Ui,

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
        /// Print every file. Twice: each file in full, one block per file
        #[arg(long, action = clap::ArgAction::Count)]
        long: u8,
        /// Print one JSON document instead of the text
        #[arg(long)]
        json: bool,
        /// The settings files each tool reads in a session started here, in order
        #[arg(long, value_name = "DIR")]
        from: Option<PathBuf>,
    },

    /// The tool's auto memory per repository, what the home already covers, and what to prune
    Memory {
        /// Print every note. Twice: each note in full, one block per note
        #[arg(long, action = clap::ArgAction::Count)]
        long: u8,
        /// Print one JSON document instead of the text
        #[arg(long)]
        json: bool,
        #[command(subcommand)]
        action: Option<MemoryAction>,
    },

    /// Owner Decision Queue backed by a Slack List (`ask`, `read`, `close`)
    OwnerDecision {
        #[command(subcommand)]
        action: OwnerDecisionAction,
    },

    /// Every skill each tool can load, with duplicates, and what to remove
    Skills {
        /// Print every skill. Twice: each skill in full, with lint and files
        #[arg(long, action = clap::ArgAction::Count)]
        long: u8,
        /// Print one JSON document instead of the text
        #[arg(long)]
        json: bool,
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

    /// Every rule standards/ writes, the always-on budget, and links
    Check {
        /// Also hold the home to the template budget: at most 3 skills,
        /// nothing in memory/ or decisions/ beyond a README
        #[arg(long)]
        template: bool,
        /// Also compare the home with this git ref: fail on a budget in
        /// context/upstream.json that is higher, or a rule's dated test
        /// that is gone and not retired
        #[arg(long, value_name = "REF")]
        base: Option<String>,
        /// Print one JSON document instead of the text
        #[arg(long)]
        json: bool,
    },

    /// Clone your home, add the template as upstream, then wire
    ///
    /// A clone with the template's layout, the home under home/, or an
    /// empty repository first gets one commit whose tree is home/, so its
    /// root is the home.
    Init {
        /// Your own copy of the template, the one you push to
        url: String,
        /// Write the plan. Without it, print only
        #[arg(long)]
        apply: bool,
    },

    /// Merge the template's main into the home, then report
    ///
    /// Adds -X subtree=home when the template keeps the home under home/,
    /// and --allow-unrelated-histories when the histories share no commit.
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
enum OwnerDecisionAction {
    /// Add a row to the Owner Decision Queue list
    Ask {
        /// Short title of what needs a decision
        #[arg(long)]
        decision: String,
        /// Agent recommendation
        #[arg(long, default_value = "")]
        recommended: String,
        /// Evidence link(s); must include an http(s) URL
        #[arg(long)]
        checked: String,
        /// Why the owner must decide
        #[arg(long = "why-you")]
        why_you: String,
        /// Project or tracker key
        #[arg(long, default_value = "")]
        project: String,
        /// Session correlation id (default: $OWNER_DECISION_SESSION or generated)
        #[arg(long)]
        session: Option<String>,
        /// Print one JSON document instead of the text
        #[arg(long)]
        json: bool,
    },
    /// Read the Answer plus row column fields (no list-item comment thread; Lists API has no read method for thread text)
    Read {
        /// Session correlation id
        #[arg(long)]
        session: Option<String>,
        /// Slack List record id (skips session lookup)
        #[arg(long = "record-id")]
        record_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Mark a row Done and append a proof link to Checked
    Close {
        #[arg(long)]
        session: Option<String>,
        #[arg(long = "record-id")]
        record_id: Option<String>,
        /// Proof link (http(s)); merged into Checked
        #[arg(long = "proof-link")]
        proof_link: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum MemoryAction {
    /// Move the named notes aside and drop their index lines
    Prune {
        names: Vec<String>,
        /// Select every note the home already covers
        #[arg(long)]
        covered: bool,
        /// Write the plan. Without it, print only
        #[arg(long)]
        apply: bool,
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
    // Rust starts with SIGPIPE ignored, so a print after the reader has closed
    // the pipe fails and panics. The default action ends the process quietly,
    // which is what `agents status | head` expects.
    #[cfg(unix)]
    // SAFETY: one call, before any other thread exists, with constant arguments.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    let code = match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("agents: {e:#}");
            ExitCode::from(2)
        }
    };
    version::warn();
    code
}

fn owner_decision_cmd(action: OwnerDecisionAction) -> Result<ExitCode> {
    let resolve_session = |opt: &Option<String>| {
        opt.clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(owner_decision::session_id)
    };
    match action {
        OwnerDecisionAction::Ask {
            decision,
            recommended,
            checked,
            why_you,
            project,
            session,
            json,
        } => {
            owner_decision::ask(
                &decision,
                &recommended,
                &checked,
                &why_you,
                &project,
                &resolve_session(&session),
                json,
            )?;
        }
        OwnerDecisionAction::Read {
            session,
            record_id,
            json,
        } => {
            owner_decision::read(&resolve_session(&session), record_id.as_deref(), json)?;
        }
        OwnerDecisionAction::Close {
            session,
            record_id,
            proof_link,
            json,
        } => {
            owner_decision::close(
                &resolve_session(&session),
                record_id.as_deref(),
                &proof_link,
                json,
            )?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let cfg = config::Config::load()?;
    let ok = ExitCode::SUCCESS;
    match cli.cmd {
        Cmd::Status { long, json } => {
            status::status(&Home::locate(cli.home)?, &cfg, long, json);
            Ok(ok)
        }
        Cmd::Ui => ui::ui(cli.home, &cfg),
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
        Cmd::Scan {
            root,
            all,
            long,
            json,
            from,
        } => {
            let home = Home::locate(cli.home).ok();
            if let Some(dir) = from {
                scan::from(&std::path::absolute(dir)?, json, home.as_ref(), &cfg);
                return Ok(ok);
            }
            let roots = match root {
                Some(r) => vec![r],
                None if !cfg.roots.is_empty() => cfg.roots(),
                None => vec![home::user_home()],
            };
            scan::scan(roots, all, long, json, home.as_ref(), &cfg);
            Ok(ok)
        }
        Cmd::OwnerDecision { action } => owner_decision_cmd(action),
        Cmd::Memory { long, json, action } => {
            let home = Home::locate(cli.home).ok();
            match action {
                None => memory::list(home.as_ref(), long, json),
                Some(MemoryAction::Prune {
                    names,
                    covered,
                    apply,
                }) => {
                    plan::banner(apply);
                    memory::prune(&names, covered, home.as_ref()).run(apply)?;
                }
            }
            Ok(ok)
        }
        Cmd::Skills { long, json, action } => {
            let home = Home::locate(cli.home).ok();
            match action {
                None => skills::list(home.as_ref(), long, json),
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
            wire::wire(&home, &cfg, link).run(apply)?;
            Ok(ok)
        }
        Cmd::Unwire { apply } => {
            let home = Home::locate(cli.home)?;
            plan::banner(apply);
            wire::unwire(&home).run(apply)?;
            Ok(ok)
        }
        Cmd::Check {
            template,
            base,
            json,
        } => {
            let passed = check::check(&Home::locate(cli.home)?, template, base.as_deref(), json)?;
            Ok(if passed { ok } else { ExitCode::FAILURE })
        }
        Cmd::Init { url, apply } => {
            git::init(cli.home, &cfg, &url, apply)?;
            Ok(ok)
        }
        Cmd::Update { apply } => {
            git::update(&Home::locate(cli.home)?, &cfg, apply)?;
            Ok(ok)
        }
        Cmd::Sync { apply } => {
            git::sync(&Home::locate(cli.home)?, &cfg, apply)?;
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
                wire::wire(&home, &cfg, false).run(true)?;
            } else {
                println!("then the wiring plan, planned after the moves, so a slot a move empties can be linked.");
            }
            Ok(ok)
        }
    }
}
