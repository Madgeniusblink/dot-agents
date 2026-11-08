//! Send text to a tmux pane that runs an agent tool. The text is typed with
//! no Enter, so the person sends it from the tool. No file is written.

use crate::harness;
use std::ffi::OsString;
use std::io;
use std::process::Command;

/// A pane, by its tmux id, and the tool it runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pane {
    pub id: String,
    pub label: &'static str,
}

/// How to reach tmux. `path` replaces `PATH` for the child, so a test can
/// put a fake `tmux` first. `here` is the pane the view runs in.
#[derive(Debug, Default)]
pub struct Tmux {
    pub path: Option<OsString>,
    pub here: Option<String>,
}

impl Tmux {
    pub fn from_env() -> Tmux {
        Tmux {
            path: None,
            here: std::env::var("TMUX_PANE").ok(),
        }
    }

    fn command(&self) -> Command {
        let mut c = Command::new("tmux");
        if let Some(p) = &self.path {
            c.env("PATH", p);
        }
        c
    }

    /// Every pane that runs an agent tool, except the view's own. Empty
    /// when tmux is missing or has no server.
    pub fn panes(&self) -> Vec<Pane> {
        let out = self
            .command()
            .args([
                "list-panes",
                "-a",
                "-F",
                "#{pane_id}\t#{pane_current_command}",
            ])
            .output();
        let Ok(out) = out else { return Vec::new() };
        if !out.status.success() {
            return Vec::new();
        }
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let (id, command) = l.split_once('\t')?;
                let label = harness::pane_label(command)?;
                (self.here.as_deref() != Some(id)).then(|| Pane {
                    id: id.to_string(),
                    label,
                })
            })
            .collect()
    }

    /// Type `text` into `pane` as literal keys. `--` keeps a text that
    /// starts with `-` from being read as a flag.
    pub fn send(&self, pane: &Pane, text: &str) -> io::Result<()> {
        let status = self
            .command()
            .args(["send-keys", "-t", &pane.id, "-l", "--", text])
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("tmux send-keys: {status}")))
        }
    }
}
