//! Point the tools on this machine at the home, or take it back out.
//!
//! Never edit a file the tool owns. Where the tool scans a directory, drop one
//! file of ours into it; where it follows a link, link. Never a copy: a copy is
//! a snapshot and goes stale in silence. `unwire` removes exactly what `wire`
//! added, so a home can be taken out without stranding the half that pointed
//! at it.

use crate::harness::{codex_dir, cursor_dir, Claude, CURSOR_LINE};
use crate::home::{rel, tilde, Home};
use crate::plan::{Action, Plan};
use std::fs;
use std::path::Path;

/// The drop-in file's content: one header, then one import line per
/// always-on file. `scripts/wire.sh` writes the same bytes.
pub fn dropin_content(home: &Home) -> String {
    let h = tilde(&home.dir);
    let mut s = format!(
        "<!-- {h} owns this file. wire writes it and unwire deletes it. \
         Add a rule as a file in {h}/rules/, then run wire again. -->\n"
    );
    for f in home.always_on() {
        s.push('@');
        s.push_str(&tilde(&f));
        s.push('\n');
    }
    s
}

/// An import line an earlier version appended to the harness's own file.
pub fn is_legacy_import(line: &str) -> bool {
    line.starts_with('@') && line.contains("/.agents/")
}

fn add_link(plan: &mut Plan, src: &Path, dst: &Path) {
    if dst.is_symlink() {
        plan.say(format!("linked: {}", rel(dst)));
    } else if dst.exists() {
        plan.say(format!("SKIP {} exists and is not a link", rel(dst)));
    } else {
        plan.act(
            Action::Link {
                src: src.to_path_buf(),
                dst: dst.to_path_buf(),
            },
            format!("link {} -> {}", rel(dst), rel(src)),
        );
    }
}

fn write_dropin(plan: &mut Plan, home: &Home, claude: &Claude) {
    let path = claude.dropin();
    let want = dropin_content(home);
    if path.is_file() && fs::read_to_string(&path).ok().as_deref() == Some(want.as_str()) {
        plan.say(format!("present: {}", rel(&path)));
    } else {
        let n = home.always_on().len();
        plan.act(
            Action::Write {
                path: path.clone(),
                content: want,
            },
            format!("write {} ({n} import lines)", rel(&path)),
        );
    }
}

fn drop_legacy_imports(plan: &mut Plan, path: &Path) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    let n = text.lines().filter(|l| is_legacy_import(l)).count();
    if n > 0 {
        plan.act(
            Action::DropLines {
                path: path.to_path_buf(),
                matches: is_legacy_import,
            },
            format!("drop {n} import line(s) from {}", rel(path)),
        );
    }
}

pub fn wire(home: &Home, link: bool) -> Plan {
    let mut plan = Plan::new();
    let mut found = false;

    let claude = Claude::new();
    if claude.exists() {
        found = true;
        plan.header("claude code:");
        if link {
            add_link(&mut plan, &home.dir.join("rules"), &claude.rules_link());
        } else {
            write_dropin(&mut plan, home, &claude);
        }
        drop_legacy_imports(&mut plan, &claude.own_file());
        // Skills have no import syntax. A link is the only live transport.
        add_link(&mut plan, &home.dir.join("skills"), &claude.skills_link());
    }

    let codex = codex_dir();
    if codex.is_dir() {
        found = true;
        plan.header("codex:");
        let dst = codex.join("AGENTS.md");
        add_link(&mut plan, &home.dir.join("AGENTS.md"), &dst);
        if dst.exists() && !dst.is_symlink() {
            plan.say("it already has its own AGENTS.md. Move your rules into the home first.");
        }
    }

    if cursor_dir().is_dir() {
        found = true;
        plan.header("cursor:");
        plan.say("user rules live in the app settings, not a file, so nothing here can");
        plan.say("wire them. Paste this into Settings -> Rules by hand:");
        plan.say(format!("  {CURSOR_LINE}"));
    }

    if !found {
        plan.say("no known tool directory in $HOME. see docs/setup.md.");
    }
    plan.blank();
    plan.text("verify: agents check");
    plan
}

pub fn unwire(home: &Home) -> Plan {
    let mut plan = Plan::new();
    let claude = Claude::new();
    let dropin = claude.dropin();
    if dropin.is_file() {
        plan.act(
            Action::Delete(dropin.clone()),
            format!("delete {}", rel(&dropin)),
        );
    } else {
        plan.say(format!("no file at {}", rel(&dropin)));
    }
    drop_legacy_imports(&mut plan, &claude.own_file());
    for l in [
        claude.rules_link(),
        claude.skills_link(),
        codex_dir().join("AGENTS.md"),
    ] {
        if l.is_symlink() {
            plan.act(Action::Unlink(l.clone()), format!("unlink {}", rel(&l)));
        }
    }
    plan.blank();
    plan.text(format!(
        "the home itself is untouched. remove it with: rm -rf {}",
        home.dir.display()
    ));
    plan
}
