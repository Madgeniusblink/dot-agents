//! Point the tools on this machine at the home, or take it back out.
//!
//! Never edit a file the tool owns. Where the tool expands imports, drop one
//! file of ours into its directory holding pointers; where it follows a link,
//! link; where it does neither, write one file of ours holding the content,
//! rewritten by every `wire` and reported stale by `status`. Never a bare
//! copy: a copy nothing rewrites and nothing checks goes stale in silence.
//! `unwire` removes exactly what `wire` added, so a home can be taken out
//! without stranding the half that pointed at it.

use crate::config::Config;
use crate::harness::{
    codex_dir, cursor_dir, cursor_hook_command, cursor_hooks, grok_dir, grok_dropin, Claude,
    CURSOR_HOOK_SCRIPT,
};
use crate::home::{rel, tilde, Home};
use crate::plan::{Action, Plan};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// The mark every file the home writes carries on its first line.
pub const OWNED: &str = "owns this file";

/// The drop-in for a tool that expands imports: one header, then one import
/// line per always-on file. `scripts/wire.sh` writes the same bytes.
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

/// The file for a tool that follows neither links nor imports: one header,
/// then every always-on file in full, each under a comment naming it.
/// `scripts/wire.sh` writes the same bytes.
pub fn generated_content(home: &Home) -> String {
    let h = tilde(&home.dir);
    let mut s = format!(
        "<!-- {h} owns this file. wire writes it from principles.md and rules/*.md \
         because this tool follows no import; unwire deletes it. Edit those files, \
         then run wire again; status reports when this is stale. -->\n"
    );
    for f in home.always_on() {
        s.push('\n');
        s.push_str(&format!("<!-- {} -->\n", tilde(&f)));
        let text = fs::read_to_string(&f).unwrap_or_default();
        s.push_str(&text);
        if !text.ends_with('\n') {
            s.push('\n');
        }
    }
    s
}

/// True when the home wrote this file.
pub fn is_ours(path: &Path) -> bool {
    fs::read_to_string(path)
        .ok()
        .and_then(|t| t.lines().next().map(|l| l.contains(OWNED)))
        .unwrap_or(false)
}

fn canonical(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
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

/// Write the generated file into a slot. A link the home made earlier, to
/// the router this tool cannot follow, is replaced. A file that is not the
/// home's is left alone.
fn write_generated(plan: &mut Plan, home: &Home, path: &Path) {
    let into_home = path.is_symlink() && canonical(path).starts_with(canonical(&home.dir));
    if into_home {
        plan.act(
            Action::Unlink(path.to_path_buf()),
            format!(
                "unlink {} (a link to the router, which this tool cannot follow)",
                rel(path)
            ),
        );
    } else if (path.exists() || path.is_symlink()) && !is_ours(path) {
        plan.say(format!(
            "SKIP {} exists and is not the home's. Move your rules into the home first.",
            rel(path)
        ));
        return;
    }
    let want = generated_content(home);
    if !into_home
        && path.is_file()
        && fs::read_to_string(path).ok().as_deref() == Some(want.as_str())
    {
        plan.say(format!("present: {}", rel(path)));
    } else {
        let n = home.always_on().len();
        plan.act(
            Action::Write {
                path: path.to_path_buf(),
                content: want,
            },
            format!("write {} ({n} files)", rel(path)),
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
                matches: Box::new(is_legacy_import),
            },
            format!("drop {n} import line(s) from {}", rel(path)),
        );
    }
}

/// `~/.cursor/hooks.json` as an object, or why it cannot be edited.
fn cursor_hooks_doc() -> Result<Value, String> {
    let path = cursor_hooks();
    if !path.exists() {
        return Ok(json!({"hooks": {}, "version": 1}));
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let doc: Value = serde_json::from_str(&text).map_err(|_| "not valid JSON".to_string())?;
    if !doc.is_object() {
        return Err("not a JSON object".to_string());
    }
    Ok(doc)
}

fn is_our_hook(entry: &Value, command: &str) -> bool {
    entry["command"].as_str() == Some(command)
}

/// One `sessionStart` entry of ours in Cursor's user-level hooks file. Other
/// hooks in the file stay as they are. An entry from a home at another path
/// is replaced, not joined. `scripts/wire.sh` writes the same bytes when it
/// creates the file.
fn write_cursor_hook(plan: &mut Plan, home: &Home) {
    let path = cursor_hooks();
    let command = cursor_hook_command(&home.dir);
    let mut doc = match cursor_hooks_doc() {
        Ok(doc) => doc,
        Err(why) => {
            plan.say(format!("SKIP {}: {why}", rel(&path)));
            return;
        }
    };
    let obj = doc.as_object_mut().unwrap();
    obj.entry("version").or_insert(json!(1));
    let hooks = obj.entry("hooks").or_insert(json!({}));
    if !hooks.is_object() {
        plan.say(format!("SKIP {}: \"hooks\" is not an object", rel(&path)));
        return;
    }
    let start = hooks["sessionStart"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if start.iter().any(|e| is_our_hook(e, &command)) {
        plan.say(format!("present: {}", rel(&path)));
        return;
    }
    let mut kept: Vec<Value> = start
        .into_iter()
        .filter(|e| {
            !e["command"]
                .as_str()
                .is_some_and(|c| c.ends_with(CURSOR_HOOK_SCRIPT))
        })
        .collect();
    kept.push(json!({"command": command}));
    hooks["sessionStart"] = Value::Array(kept);
    let content = serde_json::to_string_pretty(&doc).unwrap() + "\n";
    plan.act(
        Action::Write {
            path: path.clone(),
            content,
        },
        format!("write {} (sessionStart hook)", rel(&path)),
    );
}

/// Take our entry out again. When nothing else is left in the file, delete
/// it; a file with other hooks is rewritten without ours.
fn remove_cursor_hook(plan: &mut Plan, home: &Home) -> bool {
    let path = cursor_hooks();
    if !path.is_file() {
        return false;
    }
    let command = cursor_hook_command(&home.dir);
    let mut doc = match cursor_hooks_doc() {
        Ok(doc) => doc,
        Err(_) => return false,
    };
    let Some(start) = doc["hooks"]["sessionStart"].as_array().cloned() else {
        return false;
    };
    let kept: Vec<Value> = start
        .iter()
        .filter(|e| !is_our_hook(e, &command))
        .cloned()
        .collect();
    if kept.len() == start.len() {
        return false;
    }
    let hooks = doc["hooks"].as_object_mut().unwrap();
    if kept.is_empty() {
        hooks.remove("sessionStart");
    } else {
        hooks.insert("sessionStart".into(), Value::Array(kept));
    }
    let obj = doc.as_object();
    let bare = obj.is_some_and(|o| {
        o.keys().all(|k| k == "hooks" || k == "version")
            && o["hooks"].as_object().is_some_and(|h| h.is_empty())
    });
    if bare {
        plan.act(
            Action::Delete(path.clone()),
            format!("delete {}", rel(&path)),
        );
    } else {
        let content = serde_json::to_string_pretty(&doc).unwrap() + "\n";
        plan.act(
            Action::Write {
                path: path.clone(),
                content,
            },
            format!("write {} without the sessionStart hook", rel(&path)),
        );
    }
    true
}

/// Only the tools `[tools] present` allows, when it names any.
pub fn wire(home: &Home, cfg: &Config, link: bool) -> Plan {
    let mut plan = Plan::new();
    let mut found = false;

    let claude = Claude::new();
    if claude.exists() && cfg.tool("claude") {
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

    // Codex reads one global file and expands no import, so that slot gets
    // the generated file.
    let codex = codex_dir();
    if codex.is_dir() && cfg.tool("codex") {
        found = true;
        plan.header("codex:");
        write_generated(&mut plan, home, &codex.join("AGENTS.md"));
    }

    // Grok Build scans ~/.grok/rules/ in every project and loads each file
    // in full, with no import, so its drop-in holds the content.
    if grok_dir().is_dir() && cfg.tool("grok") {
        found = true;
        plan.header("grok:");
        write_generated(&mut plan, home, &grok_dropin());
    }

    if cursor_dir().is_dir() && cfg.tool("cursor") {
        found = true;
        plan.header("cursor:");
        write_cursor_hook(&mut plan, home);
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
    let mut any = false;
    for f in [
        claude.dropin(),
        codex_dir().join("AGENTS.md"),
        grok_dropin(),
    ] {
        if f.is_file() && !f.is_symlink() && is_ours(&f) {
            any = true;
            plan.act(Action::Delete(f.clone()), format!("delete {}", rel(&f)));
        }
    }
    drop_legacy_imports(&mut plan, &claude.own_file());
    any |= remove_cursor_hook(&mut plan, home);
    for l in [
        claude.rules_link(),
        claude.skills_link(),
        codex_dir().join("AGENTS.md"),
    ] {
        if l.is_symlink() {
            any = true;
            plan.act(Action::Unlink(l.clone()), format!("unlink {}", rel(&l)));
        }
    }
    if !any {
        plan.say("nothing of the home's is wired.");
    }
    plan.blank();
    plan.text(format!(
        "the home itself is untouched. remove it with: rm -rf {}",
        home.dir.display()
    ));
    plan
}
