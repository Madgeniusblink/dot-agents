//! The tool's auto memory: notes it writes for itself, one directory per
//! repository, machine-local, never synced. The index loads at the start of
//! every session in that repository. This lists them, says which ones the
//! home already covers so the copy here is a near-duplicate, and prunes by
//! moving aside, never deleting.

use crate::harness::Claude;
use crate::home::{rel, tilde, user_home, Home};
use crate::out;
use crate::plan::{Action, Plan};
use crate::reset::timestamp;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Covered: the home says the same thing. Similar: worth a look.
const COVERED: f32 = 0.5;
const SIMILAR: f32 = 0.3;

const STOP: &[&str] = &[
    "that", "this", "with", "from", "when", "what", "have", "which", "their", "there", "then",
    "than", "they", "them", "into", "only", "also", "never", "every", "after", "before", "about",
    "your", "other", "does", "done", "each", "over", "under", "will", "would", "should", "must",
    "more", "most", "some", "same", "such", "like", "just", "here", "where", "while", "these",
    "those", "been", "being", "were", "through", "because", "without", "instead", "always",
    "related", "apply", "user", "cris", "claude", "session", "sessions",
];

pub struct Entry {
    pub dir: PathBuf,
    pub file: PathBuf,
    pub name: String,
    pub kind: String,
    pub description: String,
    pub lines: usize,
    words: BTreeSet<String>,
}

struct Unit {
    place: String,
    words: BTreeSet<String>,
}

/// Distinctive words: four letters or more, not a stop word.
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(str::to_ascii_lowercase)
        .filter(|w| w.len() >= 4 && !STOP.contains(&w.as_str()))
        .collect()
}

/// Overlap coefficient: shared words over the smaller set, so a one-line
/// rule can match a long note.
fn overlap(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f32 {
    let m = a.len().min(b.len());
    if m == 0 {
        0.0
    } else {
        a.intersection(b).count() as f32 / m as f32
    }
}

/// `key: value` from frontmatter, at any indent.
fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let head = text.strip_prefix("---")?;
    let end = head.find("\n---")?;
    head[..end]
        .lines()
        .find_map(|l| l.trim_start().strip_prefix(key)?.strip_prefix(':'))
        .map(|v| v.trim().trim_matches(['"', '\'']))
}

fn body(text: &str) -> &str {
    text.strip_prefix("---")
        .and_then(|h| h.find("\n---").map(|i| &h[i + 4..]))
        .unwrap_or(text)
        .trim_start()
}

pub fn entries(claude: &Claude) -> Vec<Entry> {
    let mut out = Vec::new();
    for dir in claude.auto_memory_dirs() {
        let mut files: Vec<PathBuf> = fs::read_dir(&dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path())
                    .filter(|p| {
                        p.is_file()
                            && p.extension().is_some_and(|x| x == "md")
                            && p.file_name().is_some_and(|n| n != "MEMORY.md")
                    })
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        for file in files {
            let text = fs::read_to_string(&file).unwrap_or_default();
            let stem = file
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let name = field(&text, "name").unwrap_or(&stem).to_string();
            let description = field(&text, "description").unwrap_or("");
            let first = body(&text).split("\n\n").next().unwrap_or("");
            out.push(Entry {
                dir: dir.clone(),
                words: words(&format!("{name} {description} {first}")),
                name,
                kind: field(&text, "type").unwrap_or("").to_string(),
                description: description.to_string(),
                lines: text.lines().count(),
                file,
            });
        }
    }
    out
}

/// What the home says, one unit per rule line and per memory file.
fn home_units(home: &Home) -> Vec<Unit> {
    let mut units = Vec::new();
    let mut rules: Vec<PathBuf> = fs::read_dir(home.dir.join("rules"))
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    rules.sort();
    for f in rules
        .iter()
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
    {
        let name = f.file_name().unwrap_or_default().to_string_lossy();
        for (i, line) in fs::read_to_string(f)
            .unwrap_or_default()
            .lines()
            .enumerate()
        {
            if line.starts_with("- ") {
                units.push(Unit {
                    place: format!("rules/{name}:{}", i + 1),
                    words: words(line),
                });
            }
        }
    }
    let mut memory: Vec<PathBuf> = fs::read_dir(home.dir.join("memory"))
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    memory.sort();
    for f in memory
        .iter()
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
    {
        units.push(Unit {
            place: format!(
                "memory/{}",
                f.file_name().unwrap_or_default().to_string_lossy()
            ),
            words: words(&fs::read_to_string(f).unwrap_or_default()),
        });
    }
    units
}

fn best_match<'a>(e: &Entry, units: &'a [Unit]) -> Option<(f32, &'a str)> {
    units
        .iter()
        .map(|u| (overlap(&e.words, &u.words), u.place.as_str()))
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .filter(|(s, _)| *s >= SIMILAR)
}

/// No `long`: counts per repository and the notes the home covers or
/// resembles. Once: every note. Twice: each note in full.
pub fn list(home: Option<&Home>, long: u8, json: bool) {
    let width = out::width();
    let claude = Claude::new();
    let entries = entries(&claude);
    if json {
        out::json(
            "memory",
            home.map(|h| h.dir.as_path()),
            data(home, &claude, &entries),
        );
        return;
    }
    if entries.is_empty() {
        println!(
            "no auto memory under {}.",
            tilde(&claude.auto_memory_root())
        );
        return;
    }
    let units = home.map(home_units).unwrap_or_default();
    let repos: BTreeSet<&Path> = entries.iter().map(|e| e.dir.as_path()).collect();
    if long == 0 {
        brief(&entries, &units, &repos, width);
        return;
    }
    let long = long >= 2;
    out::say(
        &format!(
            "claude code auto memory: {} repositories, {} notes. The notes stay on this machine. Each MEMORY.md loads in every session in its repository.",
            repos.len(),
            entries.len()
        ),
        0,
        width,
    );
    if units.is_empty() {
        println!("no home found, so there is nothing to compare against.");
    }
    for dir in repos {
        let group: Vec<&Entry> = entries.iter().filter(|e| e.dir == dir).collect();
        out::section(&tilde(dir), group.len(), width);
        let kind_w = group.iter().map(|e| e.kind.len()).max().unwrap_or(0);
        let items: Vec<out::Row> = group
            .iter()
            .map(|e| {
                let verdict = match best_match(e, &units) {
                    Some((s, place)) if s >= COVERED => format!("covered by {place} ({s:.2})"),
                    Some((s, place)) => format!("similar to {place} ({s:.2})"),
                    None => String::new(),
                };
                if long {
                    out::Row::new(
                        &e.name,
                        format!("{}, {} lines", e.kind, e.lines),
                        format!("{verdict}\n{}", e.description),
                    )
                } else {
                    out::Row::new(
                        &e.name,
                        format!("{:>4} lines", e.lines),
                        format!("{:<kind_w$}  {verdict}", e.kind),
                    )
                }
            })
            .collect();
        out::print(&items, long, width);
    }

    let dup = same_file(&entries);
    if !dup.is_empty() {
        out::section(
            "the same note in more than one repository",
            dup.len(),
            width,
        );
        let sep = if long { "\n" } else { ", " };
        let items: Vec<out::Row> = dup
            .iter()
            .map(|(f, v)| {
                let places: Vec<String> = v.iter().map(|e| tilde(&e.dir)).collect();
                out::Row::new(f.as_str(), "", format!("in {}", places.join(sep)))
            })
            .collect();
        out::print(&items, long, width);
    }

    let covered = entries
        .iter()
        .filter(|e| best_match(e, &units).is_some_and(|(s, _)| s >= COVERED))
        .count();
    println!();
    out::say(
        &format!(
            "the home covers {covered} of these notes. A note that the home already states is a near-duplicate, and that kind hurts most."
        ),
        0,
        width,
    );
    let mut pairs = vec![
        ("prune some", "agents memory prune <name>... --apply"),
        ("prune covered", "agents memory prune --covered --apply"),
    ];
    if !long {
        pairs.push(("each in full", "agents memory --long --long"));
    }
    out::steps(
        &pairs,
        &["Both commands move the note to ~/.agents-reset/<timestamp>/ and drop its line from MEMORY.md."],
        width,
    );
}

fn brief(entries: &[Entry], units: &[Unit], repos: &BTreeSet<&Path>, width: usize) {
    out::say(
        &format!(
            "claude code auto memory: {} repositories, {} notes. Each MEMORY.md loads in every session in its repository.",
            repos.len(),
            entries.len()
        ),
        0,
        width,
    );
    if units.is_empty() {
        println!("no home found, so there is nothing to compare against.");
    }
    let verdicts: Vec<(&Entry, Option<(f32, &str)>)> =
        entries.iter().map(|e| (e, best_match(e, units))).collect();
    out::section("repositories", repos.len(), width);
    let items: Vec<out::Row> = repos
        .iter()
        .map(|dir| {
            let mine: Vec<f32> = verdicts
                .iter()
                .filter(|(e, _)| e.dir == *dir)
                .map(|(_, v)| v.map_or(0.0, |(s, _)| s))
                .collect();
            let covered = mine.iter().filter(|s| **s >= COVERED).count();
            let similar = mine.iter().filter(|s| **s >= SIMILAR).count() - covered;
            out::Row::new(
                "",
                format!("{:>4} notes", mine.len()),
                format!("{covered} covered, {similar} similar  {}", tilde(dir)),
            )
        })
        .collect();
    out::print(&items, false, width);
    let flagged: Vec<out::Row> = verdicts
        .iter()
        .filter_map(|(e, v)| {
            let (s, place) = (*v)?;
            let verdict = if s >= COVERED {
                "covered by"
            } else {
                "similar to"
            };
            Some(out::Row::new(
                &e.name,
                format!("{:>4} lines", e.lines),
                format!("{verdict} {place} ({s:.2})  in {}", tilde(&e.dir)),
            ))
        })
        .collect();
    out::section("covered or similar", flagged.len(), width);
    out::print(&flagged, false, width);
    out::steps(
        &[
            ("prune covered", "agents memory prune --covered --apply"),
            ("every note", "agents memory --long"),
            ("each in full", "agents memory --long --long"),
        ],
        &[],
        width,
    );
}

/// Each note file name found in more than one repository.
fn same_file(entries: &[Entry]) -> Vec<(String, Vec<&Entry>)> {
    let mut by_file: BTreeMap<String, Vec<&Entry>> = BTreeMap::new();
    for e in entries {
        let f = e
            .file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        by_file.entry(f).or_default().push(e);
    }
    by_file.into_iter().filter(|(_, v)| v.len() > 1).collect()
}

/// The same facts the text shows, as data.
fn data(home: Option<&Home>, claude: &Claude, entries: &[Entry]) -> Value {
    let units = home.map(home_units).unwrap_or_default();
    let repos: BTreeSet<&Path> = entries.iter().map(|e| e.dir.as_path()).collect();
    let notes: Vec<Value> = entries
        .iter()
        .map(|e| {
            let (verdict, place, score) = match best_match(e, &units) {
                Some((s, place)) => (
                    if s >= COVERED { "covered" } else { "similar" },
                    Some(place),
                    Some((s * 100.0).round() / 100.0),
                ),
                None => ("none", None, None),
            };
            json!({
                "name": e.name,
                "repository": tilde(&e.dir),
                "file": tilde(&e.file),
                "type": e.kind,
                "lines": e.lines,
                "description": e.description,
                "verdict": verdict,
                "match": place,
                "score": score,
            })
        })
        .collect();
    let same: Vec<Value> = same_file(entries)
        .iter()
        .map(|(f, v)| {
            let places: Vec<String> = v.iter().map(|e| tilde(&e.dir)).collect();
            json!({"file": f, "repositories": places})
        })
        .collect();
    let covered = entries
        .iter()
        .filter(|e| best_match(e, &units).is_some_and(|(s, _)| s >= COVERED))
        .count();
    json!({
        "root": tilde(&claude.auto_memory_root()),
        "repositories": repos.len(),
        "count": entries.len(),
        "covered": covered,
        "notes": notes,
        "same_note": same,
    })
}

pub fn prune(names: &[String], covered: bool, home: Option<&Home>) -> Plan {
    let mut plan = Plan::new();
    let claude = Claude::new();
    let entries = entries(&claude);
    let units = home.map(home_units).unwrap_or_default();
    let user = user_home();
    let base = user.join(".agents-reset").join(timestamp());
    let mut manifest = String::new();
    let mut by_index: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();

    plan.header("move aside (nothing is deleted):");
    for e in &entries {
        let stem = e
            .file
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let named = names.iter().any(|n| *n == e.name || *n == stem);
        let is_covered = covered && best_match(e, &units).is_some_and(|(s, _)| s >= COVERED);
        if !named && !is_covered {
            continue;
        }
        let to = base.join(e.file.strip_prefix(&user).unwrap_or(&e.file));
        manifest.push_str(&format!("{}\t{}\n", e.file.display(), to.display()));
        plan.act(
            Action::Move {
                from: e.file.clone(),
                to: to.clone(),
            },
            format!("move {} -> {}", tilde(&e.file), rel(&to)),
        );
        by_index.entry(e.dir.join("MEMORY.md")).or_default().push(
            e.file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        );
    }
    for n in names {
        if !entries
            .iter()
            .any(|e| *n == e.name || e.file.file_stem().is_some_and(|s| s == n.as_str()))
        {
            plan.say(format!("no note named {n}"));
        }
    }
    if manifest.is_empty() {
        plan.say("nothing selected.");
        return plan;
    }
    for (index, files) in by_index {
        let n = files.len();
        plan.act(
            Action::DropLines {
                path: index.clone(),
                matches: Box::new(move |line: &str| {
                    files.iter().any(|f| line.contains(f.as_str()))
                }),
            },
            format!("drop {n} index line(s) from {}", tilde(&index)),
        );
    }
    plan.act(
        Action::Write {
            path: base.join("manifest.txt"),
            content: manifest,
        },
        format!("write {}", rel(&base.join("manifest.txt"))),
    );
    plan.say(format!(
        "put the files back with: agents reset --restore {} --apply  (index lines are not restored)",
        tilde(&base)
    ));
    plan
}
