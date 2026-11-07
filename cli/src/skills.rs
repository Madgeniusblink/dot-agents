//! Every skill each tool can load, where it comes from, and which ones are
//! the same skill twice. Removal moves a skill aside, never deletes it.

use crate::harness::codex_dir;
use crate::home::{rel, tilde, user_home, Home};
use crate::plan::{Action, Plan};
use crate::reset::timestamp;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    Home,
    User,
    Codex,
    Synced,
    Plugin,
}

const SOURCES: [Source; 5] = [
    Source::Home,
    Source::User,
    Source::Codex,
    Source::Synced,
    Source::Plugin,
];

impl Source {
    /// A tool or a service owns these; the CLI lists them and leaves them.
    fn managed(self) -> bool {
        matches!(self, Source::Synced | Source::Plugin)
    }

    fn label(self) -> &'static str {
        match self {
            Source::Home => "home",
            Source::User => "claude code, user skills",
            Source::Codex => "codex, user skills",
            Source::Synced => {
                "claude code, skills synced from claude.ai (managed there; remove them there)"
            }
            Source::Plugin => {
                "claude code, plugin skills (managed by the plugin; disable it instead)"
            }
        }
    }
}

pub struct Skill {
    pub name: String,
    pub description: String,
    pub dir: PathBuf,
    pub source: Source,
    pub lines: usize,
    pub hash: u64,
}

pub fn find(home: Option<&Home>) -> Vec<Skill> {
    let mut out = Vec::new();
    if let Some(h) = home {
        collect(&h.dir.join("skills"), Source::Home, 2, &mut out);
    }
    let claude = user_home().join(".claude");
    collect(&claude.join("skills"), Source::User, 2, &mut out);
    // Codex reads $CODEX_HOME/skills/<name>/SKILL.md, the same shape.
    collect(&codex_dir().join("skills"), Source::Codex, 2, &mut out);
    // claude.ai syncs skills to skills/synced/<id>/<name>/SKILL.md, one level
    // deeper than a user skill, so the user walk above does not reach them.
    collect(
        &claude.join("skills").join("synced"),
        Source::Synced,
        3,
        &mut out,
    );
    collect(&claude.join("plugins"), Source::Plugin, 8, &mut out);
    out.sort_by(|a, b| (a.source, &a.dir).cmp(&(b.source, &b.dir)));
    out
}

/// `<dir>/**/skills/<name>/SKILL.md`, not following links, so a link into
/// the home is reported once, as the home.
fn collect(dir: &Path, source: Source, depth: usize, out: &mut Vec<Skill>) {
    if !dir.is_dir() {
        return;
    }
    for e in WalkDir::new(dir)
        .max_depth(depth)
        .follow_links(false)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file() && e.file_name() == "SKILL.md")
    {
        let skill_dir = e.path().parent().unwrap_or(dir).to_path_buf();
        let text = fs::read_to_string(e.path()).unwrap_or_default();
        let (name, description, body) = frontmatter(&text);
        let mut hasher = DefaultHasher::new();
        body.split_whitespace().for_each(|w| w.hash(&mut hasher));
        out.push(Skill {
            name: name.unwrap_or_else(|| {
                skill_dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            }),
            description: description.unwrap_or_default(),
            dir: skill_dir,
            source,
            lines: text.lines().count(),
            hash: hasher.finish(),
        });
    }
}

/// `name`, `description`, and the body after the frontmatter.
fn frontmatter(text: &str) -> (Option<String>, Option<String>, &str) {
    let Some(rest) = text.strip_prefix("---") else {
        return (None, None, text);
    };
    let Some(end) = rest.find("\n---") else {
        return (None, None, text);
    };
    let (head, body) = rest.split_at(end);
    let field = |key: &str| {
        head.lines().find_map(|l| {
            l.strip_prefix(key)
                .and_then(|v| v.strip_prefix(':'))
                .map(|v| v.trim().trim_matches(['"', '\'']).replace("\\\"", "\""))
        })
    };
    (field("name"), field("description"), &body[4..])
}

pub fn list(home: Option<&Home>) {
    let skills = find(home);
    let links = fs::read_dir(user_home().join(".claude").join("skills"))
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_symlink())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for source in SOURCES {
        let group: Vec<&Skill> = skills.iter().filter(|s| s.source == source).collect();
        if group.is_empty() && !(source == Source::User && !links.is_empty()) {
            continue;
        }
        println!();
        println!("{} ({}):", source.label(), group.len());
        if source == Source::User {
            for l in &links {
                println!(
                    "  {} -> {}  (a link; those skills are listed under their own source)",
                    rel(l),
                    tilde(&fs::read_link(l).unwrap_or_default())
                );
            }
        }
        let width = group.iter().map(|s| s.name.len()).max().unwrap_or(0);
        for s in group {
            let desc: String = s.description.chars().take(64).collect();
            let more = if s.description.chars().count() > 64 {
                "…"
            } else {
                ""
            };
            println!(
                "  {:<width$}  {:>4} lines  {desc}{more}",
                s.name,
                s.lines,
                width = width
            );
        }
    }

    let mut by_name: BTreeMap<&str, Vec<&Skill>> = BTreeMap::new();
    let mut by_hash: BTreeMap<u64, Vec<&Skill>> = BTreeMap::new();
    for s in &skills {
        by_name.entry(&s.name).or_default().push(s);
        by_hash.entry(s.hash).or_default().push(s);
    }
    let same_name: Vec<_> = by_name.values().filter(|v| v.len() > 1).collect();
    let same_body: Vec<_> = by_hash
        .values()
        .filter(|v| {
            v.len() > 1
                && v.iter()
                    .map(|s| &s.name)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    > 1
        })
        .collect();
    if same_name.is_empty() && same_body.is_empty() {
        println!();
        println!("no duplicates: every name and every body appears once.");
    }
    if !same_name.is_empty() {
        println!();
        println!(
            "same name in more than one place (the tool picks one; you did not choose which):"
        );
        for group in same_name {
            let identical = group.iter().all(|s| s.hash == group[0].hash);
            let places: Vec<String> = group.iter().map(|s| tilde(&s.dir)).collect();
            println!(
                "  {}  {}  {}",
                group[0].name,
                if identical {
                    "identical"
                } else {
                    "DIFFERENT bodies"
                },
                places.join(", ")
            );
        }
    }
    if !same_body.is_empty() {
        println!();
        println!("same body under different names:");
        for group in same_body {
            let places: Vec<String> = group.iter().map(|s| tilde(&s.dir)).collect();
            println!("  {}", places.join(" = "));
        }
    }
    println!();
    println!("remove some:   agents skills remove <name>... --apply");
    println!("keep only some: agents skills keep <name>... --apply");
    println!("both move the skill to ~/.agents-reset/<timestamp>/ and never delete.");
}

fn aside(names: &[String], keep: bool, home: Option<&Home>) -> Plan {
    let mut plan = Plan::new();
    let skills = find(home);
    let user = user_home();
    let stamp = timestamp();
    let base = user.join(".agents-reset").join(&stamp);
    let mut manifest = String::new();
    plan.header(if keep {
        "move aside every skill not named (nothing is deleted):"
    } else {
        "move aside (nothing is deleted):"
    });
    let mut matched = 0;
    for s in &skills {
        let named = names.iter().any(|n| n == &s.name);
        if named {
            matched += 1;
        }
        if named == keep {
            continue;
        }
        if s.source.managed() {
            plan.say(format!(
                "SKIP {}: {}.",
                tilde(&s.dir),
                if s.source == Source::Plugin {
                    "a plugin owns it. Disable the plugin instead"
                } else {
                    "claude.ai owns it. Remove it there"
                }
            ));
            continue;
        }
        let to = base.join(s.dir.strip_prefix(&user).unwrap_or(&s.dir));
        manifest.push_str(&format!("{}\t{}\n", s.dir.display(), to.display()));
        plan.act(
            Action::Move {
                from: s.dir.clone(),
                to: to.clone(),
            },
            format!("move {} -> {}", tilde(&s.dir), rel(&to)),
        );
    }
    for n in names {
        if !skills.iter().any(|s| &s.name == n) {
            plan.say(format!("no skill named {n}"));
        }
    }
    if matched == 0 && keep {
        plan.say("none of the names given exist, so nothing moves.");
        return Plan::new();
    }
    if !manifest.is_empty() {
        plan.act(
            Action::Write {
                path: base.join("manifest.txt"),
                content: manifest,
            },
            format!("write {}", rel(&base.join("manifest.txt"))),
        );
        plan.say(format!(
            "put it back with: agents reset --restore {} --apply",
            tilde(&base)
        ));
    }
    plan
}

pub fn remove(names: &[String], home: Option<&Home>) -> Plan {
    aside(names, false, home)
}

pub fn keep(names: &[String], home: Option<&Home>) -> Plan {
    aside(names, true, home)
}
