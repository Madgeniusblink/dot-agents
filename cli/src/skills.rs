//! Every skill each tool can load, where it comes from, and which ones are
//! the same skill twice. Removal moves a skill aside, never deletes it.

use crate::harness::{codex_dir, PROJECT_SKILL_DIRS};
use crate::home::{expand_home, rel, tilde, user_home, Home};
use crate::out;
use crate::plan::{Action, Plan};
use crate::reset::timestamp;
use serde_json::{json, Value};
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
    Project,
}

const SOURCES: [Source; 6] = [
    Source::Home,
    Source::User,
    Source::Codex,
    Source::Synced,
    Source::Plugin,
    Source::Project,
];

impl Source {
    /// A tool or a service owns these; the CLI lists them and leaves them.
    fn managed(self) -> bool {
        matches!(self, Source::Synced | Source::Plugin)
    }

    /// Who may move it: the user, a tool or service, or the project.
    fn scope(self) -> &'static str {
        match self {
            Source::Project => "project",
            s if s.managed() => "managed",
            _ => "yours",
        }
    }

    /// The `source` key of a JSON record.
    fn key(self) -> &'static str {
        match self {
            Source::Home => "home",
            Source::User => "claude-user",
            Source::Codex => "codex-user",
            Source::Synced => "claude-synced",
            Source::Plugin => "claude-plugin",
            Source::Project => "project",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Source::Home => "home",
            Source::User => "claude code, user skills",
            Source::Codex => "codex, user skills",
            Source::Synced => "claude code, skills synced from claude.ai",
            Source::Plugin => "claude code, plugin skills",
            Source::Project => "project",
        }
    }
}

pub struct Skill {
    pub name: String,
    pub description: String,
    pub dir: PathBuf,
    pub source: Source,
    pub lines: usize,
    /// The bytes of the body after the frontmatter.
    pub bytes: usize,
    pub hash: u64,
    pub front: Option<Frontmatter>,
    pub lint: Vec<Finding>,
    pub files: Files,
    pub refs: Refs,
}

impl Skill {
    pub fn broken(&self) -> bool {
        self.lint.iter().any(|f| f.level == Level::Error)
    }

    pub fn owner(&self) -> Option<&str> {
        self.front
            .as_ref()
            .and_then(|f| f.metadata.get("owner"))
            .map(String::as_str)
    }

    /// `metadata.source`: where a copy says it came from.
    pub fn origin(&self) -> Option<&str> {
        self.front
            .as_ref()
            .and_then(|f| f.metadata.get("source"))
            .map(String::as_str)
    }
}

/// The frontmatter fields the Agent Skills standard defines. Any other
/// top-level key is kept by name only.
#[derive(Default, Debug, PartialEq)]
pub struct Frontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub metadata: BTreeMap<String, String>,
    /// `key: value` for each metadata value that YAML does not read as a
    /// string, such as `version: 1.0`.
    pub not_strings: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub unknown_keys: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warn => "warn",
        }
    }
}

/// One violated rule of the standard. `text` names the value.
#[derive(Debug)]
pub struct Finding {
    pub rule: &'static str,
    pub level: Level,
    pub text: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}: {}", self.level.as_str(), self.rule, self.text)
    }
}

/// Files beside `SKILL.md`, by top directory, and their total size.
#[derive(Default, Debug, PartialEq)]
pub struct Files {
    pub scripts: usize,
    pub references: usize,
    pub assets: usize,
    pub other: usize,
    pub bytes: u64,
}

/// Relative file references in the body: how many resolve, and the rest.
#[derive(Default, Debug, PartialEq)]
pub struct Refs {
    pub ok: usize,
    pub broken: Vec<String>,
}

/// The body without fenced code blocks, inline code spans, and the text
/// part of Markdown links. Link targets are read separately.
fn strip_code_and_link_text(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut in_fence = false;
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut rest = line;
        while let Some(i) = rest.find('`') {
            out.push_str(&rest[..i]);
            let after = &rest[i + 1..];
            match after.find('`') {
                Some(j) => rest = &after[j + 1..],
                None => {
                    rest = "";
                }
            }
            out.push(' ');
        }
        out.push_str(rest);
        out.push('\n');
    }
    let mut cleaned = String::with_capacity(out.len());
    let mut rest = out.as_str();
    while let Some(i) = rest.find("](") {
        let head = &rest[..i];
        match head.rfind('[') {
            Some(k) => {
                cleaned.push_str(&head[..k]);
                cleaned.push(' ');
            }
            None => cleaned.push_str(head),
        }
        rest = &rest[i..];
        match rest.find(')') {
            Some(j) => rest = &rest[j + 1..],
            None => break,
        }
    }
    cleaned.push_str(rest);
    cleaned
}

/// `<plugin>@<marketplace>` keys that settings.json turns off.
fn disabled_plugins() -> std::collections::BTreeSet<String> {
    let path = user_home().join(".claude").join("settings.json");
    let Ok(text) = fs::read_to_string(path) else {
        return Default::default();
    };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get("enabledPlugins").cloned())
        .and_then(|v| v.as_object().cloned())
        .map(|m| {
            m.into_iter()
                .filter(|(_, v)| v.as_bool() == Some(false))
                .map(|(k, _)| k)
                .collect()
        })
        .unwrap_or_default()
}

/// `<plugin>@<marketplace>` for a skill under plugins/cache/<marketplace>/<plugin>/...
fn plugin_key(cache: &Path, dir: &Path) -> Option<String> {
    let rel = dir.strip_prefix(cache).ok()?;
    let mut parts = rel.components().map(|c| c.as_os_str().to_string_lossy());
    let marketplace = parts.next()?;
    let plugin = parts.next()?;
    Some(format!("{plugin}@{marketplace}"))
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
    // Installed plugins live under plugins/cache; plugins/marketplaces is the
    // catalogue of everything on offer and loads nothing. A plugin that
    // settings.json turns off loads nothing either.
    let cache = claude.join("plugins").join("cache");
    let off = disabled_plugins();
    let mut plugins = Vec::new();
    collect(&cache, Source::Plugin, 6, &mut plugins);
    out.extend(
        plugins
            .into_iter()
            .filter(|s| plugin_key(&cache, &s.dir).is_none_or(|k| !off.contains(&k))),
    );
    // A checkout that is also the home, or holds a user directory, is
    // listed once, under the scope that lets the user move it.
    if let Some(root) = project_root() {
        let known: Vec<PathBuf> = out
            .iter()
            .filter_map(|s| fs::canonicalize(&s.dir).ok())
            .collect();
        out.extend(
            project_skills(&root)
                .into_iter()
                .filter(|s| fs::canonicalize(&s.dir).map_or(true, |d| !known.contains(&d))),
        );
    }
    out.sort_by(|a, b| (a.source, &a.dir).cmp(&(b.source, &b.dir)));
    out
}

/// The git checkout the command runs in, if any.
pub fn project_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    cwd.ancestors()
        .find(|d| d.join(".git").exists())
        .map(Path::to_path_buf)
}

/// The skills a checkout owns, two levels under each place a tool reads.
/// A place whose real path leaves the checkout is not read.
pub fn project_skills(root: &Path) -> Vec<Skill> {
    let mut out = Vec::new();
    let real = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let mut seen = Vec::new();
    for place in PROJECT_SKILL_DIRS {
        let Ok(dir) = fs::canonicalize(root.join(place)) else {
            continue;
        };
        if !dir.starts_with(&real) || seen.contains(&dir) {
            continue;
        }
        seen.push(dir);
        collect(&root.join(place), Source::Project, 2, &mut out);
    }
    out
}

/// How many skills sit in each place a tool reads inside the checkout.
pub fn project_counts(root: &Path) -> Vec<(&'static str, usize)> {
    let skills = project_skills(root);
    PROJECT_SKILL_DIRS
        .iter()
        .map(|&p| {
            let n = skills
                .iter()
                .filter(|s| s.dir.strip_prefix(root).is_ok_and(|r| r.starts_with(p)))
                .count();
            (p, n)
        })
        .collect()
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
        let dir_name = skill_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let (front, body) = match parse(&text) {
            Some((f, b)) => (Some(f), b),
            None => (None, text.as_str()),
        };
        let mut hasher = DefaultHasher::new();
        body.split_whitespace().for_each(|w| w.hash(&mut hasher));
        let field =
            |f: fn(&Frontmatter) -> &Option<String>| front.as_ref().and_then(|x| f(x).clone());
        out.push(Skill {
            name: field(|f| &f.name).unwrap_or_else(|| dir_name.clone()),
            description: field(|f| &f.description).unwrap_or_default(),
            lint: lint(front.as_ref(), &dir_name),
            files: files(&skill_dir),
            refs: refs(&skill_dir, body),
            front,
            dir: skill_dir,
            source,
            lines: text.lines().count(),
            bytes: body.len(),
            hash: hasher.finish(),
        });
    }
}

/// The frontmatter and the body after it, or `None` when the file does not
/// open with a `---` block. The YAML this reads is the subset the standard
/// uses: top-level scalars, one map, and one list.
pub fn parse(text: &str) -> Option<(Frontmatter, &str)> {
    let rest = text.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let (head, body) = rest.split_at(end);
    let lines: Vec<&str> = head
        .lines()
        .skip(1)
        .map(|l| l.trim_end_matches('\r'))
        .collect();
    let mut f = Frontmatter::default();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        i += 1;
        if line.starts_with([' ', '\t']) || line.trim_start().starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let start = i;
        while i < lines.len() && (lines[i].starts_with([' ', '\t']) || lines[i].trim().is_empty()) {
            i += 1;
        }
        let (key, value, block) = (key.trim(), value.trim(), &lines[start..i]);
        match key {
            "name" => f.name = Some(scalar(value, block).0),
            "description" => f.description = Some(scalar(value, block).0),
            "license" => f.license = Some(scalar(value, block).0),
            "compatibility" => f.compatibility = Some(scalar(value, block).0),
            "metadata" => metadata(value, block, &mut f),
            "allowed-tools" => f.allowed_tools = tools(value, block),
            _ => f.unknown_keys.push(key.to_string()),
        }
    }
    Some((f, &body[4..]))
}

/// A YAML scalar: plain, quoted, or a `|` or `>` block. The flag is false
/// when YAML reads the value as something other than a string.
fn scalar(value: &str, block: &[&str]) -> (String, bool) {
    if let Some(header) = value.strip_prefix(['|', '>']) {
        return (block_scalar(value.starts_with('>'), header, block), true);
    }
    let joined = std::iter::once(value)
        .chain(block.iter().map(|l| l.trim()))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if let Some(q) = joined.strip_prefix('"') {
        let mut out = String::new();
        let mut chars = q.chars();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some(other) => out.push(other),
                    None => {}
                },
                c => out.push(c),
            }
        }
        return (out, true);
    }
    if let Some(q) = joined.strip_prefix('\'') {
        let mut out = String::new();
        let mut chars = q.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\'' {
                if chars.peek() != Some(&'\'') {
                    break;
                }
                chars.next();
            }
            out.push(c);
        }
        return (out, true);
    }
    let plain = match joined.find(" #") {
        Some(j) => joined[..j].trim_end().to_string(),
        None => joined,
    };
    let string = !(plain.is_empty()
        || plain.starts_with(['[', '{'])
        || [
            "null", "Null", "NULL", "~", "true", "True", "TRUE", "false", "False", "FALSE",
        ]
        .contains(&plain.as_str())
        || [".inf", "-.inf", "+.inf", ".nan", ".NaN"].contains(&plain.as_str())
        || (plain.chars().any(|c| c.is_ascii_digit())
            && plain
                .chars()
                .all(|c| c.is_ascii_hexdigit() || "+-.xXoO".contains(c))
            && (plain.parse::<f64>().is_ok()
                || plain.starts_with("0x")
                || plain.starts_with("0o"))));
    (plain, string)
}

/// The text of a `|` (literal) or `>` (folded) block, with its chomping
/// indicator: `-` drops the final line break, `+` keeps every trailing one.
fn block_scalar(folded: bool, header: &str, block: &[&str]) -> String {
    let indent = block
        .iter()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(0);
    let mut lines: Vec<&str> = block
        .iter()
        .map(|l| l.get(indent..).unwrap_or("").trim_end())
        .collect();
    let mut trailing = 0;
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
        trailing += 1;
    }
    let mut out = String::new();
    if folded {
        let mut prev_text = false;
        for l in &lines {
            if l.is_empty() {
                out.push('\n');
                prev_text = false;
            } else {
                if prev_text {
                    out.push(' ');
                }
                out.push_str(l);
                prev_text = true;
            }
        }
    } else {
        out = lines.join("\n");
    }
    if lines.is_empty() {
        return out;
    }
    if header.contains('+') {
        out.push_str(&"\n".repeat(trailing + 1));
    } else if !header.contains('-') {
        out.push('\n');
    }
    out
}

/// `metadata` as a block map or a `{k: v}` flow map. A value YAML does not
/// read as a string, or a nested value, goes to `not_strings`.
fn metadata(value: &str, block: &[&str], f: &mut Frontmatter) {
    let mut entries: Vec<(String, String, Vec<&str>)> = Vec::new();
    if let Some(inner) = value.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
        for pair in inner.split(',').filter(|p| !p.trim().is_empty()) {
            let (k, v) = pair.split_once(':').unwrap_or((pair, ""));
            entries.push((k.trim().to_string(), v.trim().to_string(), Vec::new()));
        }
    } else if !value.is_empty() {
        f.not_strings.push(format!("metadata: {value}"));
        return;
    } else {
        let base = block
            .iter()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .unwrap_or(0);
        for l in block.iter().filter(|l| !l.trim().is_empty()) {
            let indent = l.len() - l.trim_start().len();
            if indent > base {
                if let Some(e) = entries.last_mut() {
                    e.2.push(l);
                }
                continue;
            }
            let (k, v) = l.trim().split_once(':').unwrap_or((l.trim(), ""));
            entries.push((k.trim().to_string(), v.trim().to_string(), Vec::new()));
        }
    }
    for (k, v, sub) in entries {
        let k = k.trim_matches(['"', '\'']).to_string();
        let nested = v.is_empty() && !sub.is_empty();
        let (text, string) = scalar(&v, &sub);
        if nested || !string {
            f.not_strings
                .push(format!("{k}: {}", if nested { "(nested)" } else { &v }));
        } else {
            f.metadata.insert(k, text);
        }
    }
}

/// `allowed-tools` as a space-separated string, a flow list, or a block
/// list. A separator inside parentheses belongs to the tool, as in
/// `Bash(git log:*)`. Commas also separate, since tools write them.
fn tools(value: &str, block: &[&str]) -> Vec<String> {
    let text = if value.is_empty() {
        block
            .iter()
            .filter_map(|l| l.trim().strip_prefix('-'))
            .map(|item| scalar(item.trim(), &[]).0)
            .collect::<Vec<_>>()
            .join(",")
    } else if let Some(inner) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        inner
            .split(',')
            .map(|item| scalar(item.trim(), &[]).0)
            .collect::<Vec<_>>()
            .join(",")
    } else {
        scalar(value, block).0
    };
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0 && (c.is_whitespace() || c == ',') {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Every rule of the Agent Skills standard. A rule on a required field is
/// an error, because a tool may refuse the skill; the rest warn.
pub fn lint(front: Option<&Frontmatter>, dir_name: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut add = |rule, level, text: String| out.push(Finding { rule, level, text });
    let Some(f) = front else {
        add(
            "frontmatter",
            Level::Error,
            "SKILL.md does not open with a block between --- lines".into(),
        );
        return out;
    };
    match &f.name {
        None => add("name-required", Level::Error, "no name".into()),
        Some(name) => {
            let n = name.chars().count();
            if !(1..=64).contains(&n) {
                add(
                    "name-length",
                    Level::Error,
                    format!("name has {n} characters, allowed 1 to 64: {name:?}"),
                );
            }
            if name
                .chars()
                .any(|c| c != '-' && !(c.is_alphanumeric() && !c.is_uppercase()))
            {
                add(
                    "name-chars",
                    Level::Error,
                    format!("name has a character that is not a lowercase letter, digit, or hyphen: {name:?}"),
                );
            }
            if name.starts_with('-') || name.ends_with('-') || name.contains("--") {
                add(
                    "name-hyphens",
                    Level::Error,
                    format!("name starts or ends with a hyphen, or has two together: {name:?}"),
                );
            }
            if name != dir_name {
                add(
                    "name-matches-dir",
                    Level::Error,
                    format!("name is {name}, directory is {dir_name}"),
                );
            }
        }
    }
    match &f.description {
        None => add(
            "description-required",
            Level::Error,
            "no description".into(),
        ),
        Some(d) => {
            let n = d.chars().count();
            if !(1..=1024).contains(&n) {
                add(
                    "description-length",
                    Level::Error,
                    format!(
                        "description has {n} characters, allowed 1 to 1024: {:?}",
                        out::fit(d, 40)
                    ),
                );
            }
        }
    }
    if let Some(c) = &f.compatibility {
        let n = c.chars().count();
        if !(1..=500).contains(&n) {
            add(
                "compatibility-length",
                Level::Warn,
                format!(
                    "compatibility has {n} characters, allowed 1 to 500: {:?}",
                    out::fit(c, 40)
                ),
            );
        }
    }
    for kv in &f.not_strings {
        add(
            "metadata-strings",
            Level::Warn,
            format!("metadata value is not a string: {kv}"),
        );
    }
    out
}

/// Count every file under the skill directory except `SKILL.md`, by its
/// top directory. A subdirectory with its own `SKILL.md` is another skill.
fn files(dir: &Path) -> Files {
    let mut f = Files::default();
    let walk = WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            e.depth() == 0 || !(e.file_type().is_dir() && e.path().join("SKILL.md").is_file())
        });
    for e in walk.flatten().filter(|e| e.file_type().is_file()) {
        let rel = e.path().strip_prefix(dir).unwrap_or(e.path());
        if rel == Path::new("SKILL.md") {
            continue;
        }
        let top = rel.components().next().map(|c| c.as_os_str());
        match top.and_then(|t| t.to_str()) {
            Some("scripts") if rel.components().count() > 1 => f.scripts += 1,
            Some("references") if rel.components().count() > 1 => f.references += 1,
            Some("assets") if rel.components().count() > 1 => f.assets += 1,
            _ => f.other += 1,
        }
        f.bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
    }
    f
}

/// Resolve each relative file reference in the body against the skill
/// directory. A reference is a Markdown link target, or a bare path with a
/// slash whose last part has an extension, such as `scripts/run.py`.
fn refs(dir: &Path, body: &str) -> Refs {
    let mut found = std::collections::BTreeSet::new();
    let mut rest = body;
    while let Some(i) = rest.find("](") {
        let after = &rest[i + 2..];
        let Some(j) = after.find(')') else { break };
        let target = after[..j].split_whitespace().next().unwrap_or("");
        let target = target
            .trim_matches(['<', '>'])
            .split('#')
            .next()
            .unwrap_or("");
        if !target.is_empty()
            && !target.contains("://")
            && !target.starts_with(['/', '~', '$'])
            && !target.starts_with("mailto:")
        {
            found.insert(target.to_string());
        }
        rest = &after[j + 1..];
    }
    // A path inside a code span is a mention, not a reference, and a link's
    // text is not its target, so both are removed before the bare-word scan.
    let prose = strip_code_and_link_text(body);
    for word in prose.split(|c: char| c.is_whitespace() || "`\"'()[]<>,;|*".contains(c)) {
        let word = word.trim_end_matches(['.', ':', '!', '?']);
        let path_chars = word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-/+@".contains(c));
        let mut parts = word.split('/');
        let first = parts.next().unwrap_or("");
        let last = word.rsplit('/').next().unwrap_or("");
        if path_chars
            && word.contains('/')
            && !word.starts_with('/')
            && (first == "." || first == ".." || !first.contains('.'))
            && last.trim_start_matches('.').contains('.')
        {
            found.insert(word.to_string());
        }
    }
    let mut r = Refs::default();
    for target in found {
        if dir.join(&target).exists() {
            r.ok += 1;
        } else {
            r.broken.push(target);
        }
    }
    r
}

/// A skill outside the project that is a copy of one inside it: its
/// `metadata.source` names that skill, or its body is the same. An absolute
/// source path that names no file is reported as missing. A URL or a
/// repository-relative path needs its repository, so it is not checked.
fn copy_note(s: &Skill, project: &[&Skill], root: Option<&Path>) -> Option<String> {
    if s.source == Source::Project {
        return None;
    }
    // Relative to the checkout, whose root heads the project section.
    let rel = |p: &Skill| {
        root.and_then(|r| p.dir.strip_prefix(r).ok())
            .map(|r| r.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| tilde(&p.dir))
    };
    if let Some(o) = s.origin() {
        let o = o.trim_end_matches('/').trim_end_matches("/SKILL.md");
        let named = project.iter().find(|p| {
            let r = rel(p);
            o == r || o.ends_with(&format!("/{r}")) || Path::new(o) == p.dir
        });
        if let Some(p) = named {
            return Some(format!("copy of {} (declared)", rel(p)));
        }
    }
    if let Some(p) = project.iter().find(|p| p.hash == s.hash) {
        return Some(format!(
            "copy of {} ({})",
            rel(p),
            out::state(out::State::Identical)
        ));
    }
    let o = s.origin()?;
    let local = o.starts_with('/') || o.starts_with("~/") || o.starts_with("$HOME/");
    (local && !expand_home(o).exists()).then(|| "source missing".to_string())
}

/// No `long`: counts per source and rows only for skills that are yours,
/// BROKEN, or a copy. Once: every skill. Twice: each skill in full.
pub fn list(home: Option<&Home>, long: u8, json: bool) {
    let width = out::width();
    let skills = find(home);
    let links = fs::read_dir(user_home().join(".claude").join("skills"))
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_symlink())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let root = project_root();
    let project: Vec<&Skill> = skills
        .iter()
        .filter(|s| s.source == Source::Project)
        .collect();
    if json {
        let data = data(&skills, &links, &project, root.as_deref());
        out::json("skills", home.map(|h| h.dir.as_path()), data);
        return;
    }
    if long == 0 {
        brief(&skills, &project, root.as_deref(), width);
        return;
    }
    let long = long >= 2;

    println!("tools can load {} skills.", skills.len());
    for source in SOURCES {
        let group: Vec<&Skill> = skills.iter().filter(|s| s.source == source).collect();
        if group.is_empty() && !(source == Source::User && !links.is_empty()) {
            continue;
        }
        let title = match (source, &root) {
            (Source::Project, Some(r)) => format!("project {}", tilde(r)),
            _ => source.label().to_string(),
        };
        out::section(&title, group.len(), width);
        let mut items = Vec::new();
        if source == Source::User {
            for l in &links {
                items.push(out::Row::new(
                    rel(l),
                    "link",
                    format!("-> {}", tilde(&fs::read_link(l).unwrap_or_default())),
                ));
            }
        }
        for s in &group {
            let mark = if s.broken() {
                format!(" {}", out::state(out::State::Broken))
            } else {
                String::new()
            };
            let desc = if long {
                details(s)
            } else {
                s.description
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            let text = match copy_note(s, &project, root.as_deref()) {
                Some(n) if long => format!("{n}\n{desc}"),
                Some(n) => format!("{n}  {desc}"),
                None => desc,
            };
            items.push(out::Row::new(
                &s.name,
                format!("{:>4} lines{mark}", s.lines),
                text,
            ));
        }
        out::print(&items, long, width);
        if source == Source::Project {
            out::say(
                "The project owns these skills. The CLI lists them and never writes inside the checkout.",
                2,
                width,
            );
        }
        if source == Source::User && !links.is_empty() {
            out::say(
                "A link points to another source. Its skills show under that source.",
                2,
                width,
            );
        }
        if source == Source::Synced {
            out::say(
                "claude.ai manages these skills. To remove one, remove it there.",
                2,
                width,
            );
        }
        if source == Source::Plugin {
            out::say(
                "A plugin manages these skills. To remove one, disable its plugin.",
                2,
                width,
            );
        }
    }

    let (same_name, same_body) = duplicates(&skills);
    if same_name.is_empty() && same_body.is_empty() {
        println!();
        println!("no duplicates: every name and every body appears once.");
    }
    if !same_name.is_empty() {
        out::section("same name in more than one place", same_name.len(), width);
        let sep = if long { "\n" } else { ", " };
        let items: Vec<out::Row> = same_name
            .iter()
            .map(|group| {
                let identical = group.iter().all(|s| s.hash == group[0].hash);
                let places: Vec<String> = group
                    .iter()
                    .map(|s| format!("{} {}", s.source.scope(), tilde(&s.dir)))
                    .collect();
                out::Row::new(
                    &group[0].name,
                    if identical {
                        out::state(out::State::Identical)
                    } else {
                        format!("{} bodies", out::state(out::State::Different))
                    },
                    places.join(sep),
                )
            })
            .collect();
        out::print(&items, long, width);
        out::say(
            "The tool picks one of each. You did not choose which.",
            2,
            width,
        );
    }
    if !same_body.is_empty() {
        out::section("same body under different names", same_body.len(), width);
        let sep = if long { "\n" } else { " = " };
        let items: Vec<out::Row> = same_body
            .iter()
            .map(|group| {
                let names: Vec<&str> = group.iter().map(|s| s.name.as_str()).collect();
                let places: Vec<String> = group.iter().map(|s| tilde(&s.dir)).collect();
                out::Row::new(names.join(", "), "", places.join(sep))
            })
            .collect();
        out::print(&items, long, width);
    }
    let mut pairs = vec![
        ("remove some", "agents skills remove <name>... --apply"),
        ("keep only some", "agents skills keep <name>... --apply"),
    ];
    if !long {
        pairs.push(("each in full", "agents skills --long --long"));
    }
    out::steps(
        &pairs,
        &["Both commands move the skill to ~/.agents-reset/<timestamp>/. They never delete."],
        width,
    );
}

fn brief(skills: &[Skill], project: &[&Skill], root: Option<&Path>, width: usize) {
    println!("tools can load {} skills.", skills.len());
    let counts: Vec<out::Row> = SOURCES
        .iter()
        .filter_map(|&source| {
            let n = skills.iter().filter(|s| s.source == source).count();
            let label = match (source, root) {
                (Source::Project, Some(r)) => format!("project {}", tilde(r)),
                _ => source.label().to_string(),
            };
            (n > 0).then(|| {
                out::Row::new(
                    "",
                    format!("{n:>4}"),
                    format!("{label}, {}", source.scope()),
                )
            })
        })
        .collect();
    out::print(&counts, false, width);
    let shown: Vec<out::Row> = skills
        .iter()
        .filter_map(|s| {
            let copy = copy_note(s, project, root).filter(|n| n.starts_with("copy of"));
            if s.source.scope() != "yours" && !s.broken() && copy.is_none() {
                return None;
            }
            let mark = if s.broken() {
                format!(" {}", out::state(out::State::Broken))
            } else {
                String::new()
            };
            let desc = s
                .description
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let text = match copy {
                Some(n) => format!("{n}  {desc}"),
                None => desc,
            };
            Some(out::Row::new(
                &s.name,
                format!("{:>4} lines{mark}", s.lines),
                text,
            ))
        })
        .collect();
    out::section("yours, BROKEN, or a copy", shown.len(), width);
    out::print(&shown, false, width);
    let (same_name, same_body) = duplicates(skills);
    println!();
    if same_name.is_empty() && same_body.is_empty() {
        println!("no duplicates: every name and every body appears once.");
    } else {
        println!(
            "duplicates: {} names in more than one place, {} bodies under different names.",
            same_name.len(),
            same_body.len()
        );
    }
    out::steps(
        &[
            ("remove some", "agents skills remove <name>... --apply"),
            ("every skill", "agents skills --long"),
            ("each in full", "agents skills --long --long"),
        ],
        &[],
        width,
    );
}

/// One name in more than one place, and one body under more than one name.
type Groups<'a> = Vec<Vec<&'a Skill>>;

fn duplicates(skills: &[Skill]) -> (Groups<'_>, Groups<'_>) {
    let mut by_name: BTreeMap<&str, Vec<&Skill>> = BTreeMap::new();
    let mut by_hash: BTreeMap<u64, Vec<&Skill>> = BTreeMap::new();
    for s in skills {
        by_name.entry(&s.name).or_default().push(s);
        by_hash.entry(s.hash).or_default().push(s);
    }
    let same_name = by_name.into_values().filter(|v| v.len() > 1).collect();
    let same_body = by_hash
        .into_values()
        .filter(|v| {
            v.len() > 1
                && v.iter()
                    .map(|s| &s.name)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    > 1
        })
        .collect();
    (same_name, same_body)
}

/// One skill in the record shape of the spec, plus `copy`, the note the
/// text shows on a copy of a project skill.
fn record(s: &Skill, project: &[&Skill], root: Option<&Path>) -> Value {
    let front = s.front.as_ref().map(|f| {
        json!({
            "name": f.name,
            "description": f.description,
            "license": f.license,
            "compatibility": f.compatibility,
            "metadata": f.metadata,
            "allowed_tools": f.allowed_tools,
            "unknown_keys": f.unknown_keys,
        })
    });
    let lint: Vec<Value> = s
        .lint
        .iter()
        .map(|f| json!({"rule": f.rule, "level": f.level.as_str(), "text": f.text}))
        .collect();
    let f = &s.files;
    json!({
        "name": s.name,
        "dir": tilde(&s.dir),
        "scope": s.source.scope(),
        "source": s.source.key(),
        "project": root.filter(|_| s.source == Source::Project).map(tilde),
        "owner": s.owner(),
        "frontmatter": front,
        "lint": lint,
        "body": {"lines": s.lines, "bytes": s.bytes, "hash": format!("{:016x}", s.hash)},
        "files": {
            "scripts": f.scripts,
            "references": f.references,
            "assets": f.assets,
            "other": f.other,
            "bytes": f.bytes,
        },
        "refs": {"ok": s.refs.ok, "broken": s.refs.broken},
        "copy": copy_note(s, project, root),
    })
}

/// The same facts the text shows, as data.
fn data(skills: &[Skill], links: &[PathBuf], project: &[&Skill], root: Option<&Path>) -> Value {
    let (same_name, same_body) = duplicates(skills);
    let same_name: Vec<Value> = same_name
        .iter()
        .map(|g| {
            let places: Vec<Value> = g
                .iter()
                .map(|s| json!({"scope": s.source.scope(), "dir": tilde(&s.dir)}))
                .collect();
            json!({
                "name": g[0].name,
                "identical": g.iter().all(|s| s.hash == g[0].hash),
                "places": places,
            })
        })
        .collect();
    let same_body: Vec<Value> = same_body
        .iter()
        .map(|g| {
            let names: Vec<&str> = g.iter().map(|s| s.name.as_str()).collect();
            let dirs: Vec<String> = g.iter().map(|s| tilde(&s.dir)).collect();
            json!({"names": names, "dirs": dirs})
        })
        .collect();
    let links: Vec<Value> = links
        .iter()
        .map(|l| json!({"path": tilde(l), "target": tilde(&fs::read_link(l).unwrap_or_default())}))
        .collect();
    let records: Vec<Value> = skills.iter().map(|s| record(s, project, root)).collect();
    json!({
        "count": skills.len(),
        "skills": records,
        "links": links,
        "duplicates": {"same_name": same_name, "same_body": same_body},
    })
}

/// The `--long` text of a skill: its description, then each fact the
/// listing found about it, one per line.
fn details(s: &Skill) -> String {
    let mut lines = vec![s.description.trim().to_string()];
    if let Some(owner) = s.owner() {
        lines.push(format!("owner: {owner}"));
    }
    let f = &s.files;
    if f.scripts + f.references + f.assets + f.other > 0 {
        lines.push(format!(
            "files: {} scripts, {} references, {} assets, {} other, {} bytes",
            f.scripts, f.references, f.assets, f.other, f.bytes
        ));
    }
    for target in &s.refs.broken {
        lines.push(format!("broken reference: {target}"));
    }
    lines.extend(s.lint.iter().map(Finding::to_string));
    lines.join("\n")
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
        if s.source == Source::Project {
            plan.say(format!("SKIP {}: the project owns it.", tilde(&s.dir)));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn front(head: &str) -> Frontmatter {
        parse(&format!("---\n{head}\n---\nbody\n")).unwrap().0
    }

    fn rules(head: &str, dir: &str) -> Vec<(&'static str, Level, String)> {
        lint(Some(&front(head)), dir)
            .into_iter()
            .map(|f| (f.rule, f.level, f.text))
            .collect()
    }

    #[test]
    fn reads_every_field_in_each_form() {
        let f = front(concat!(
            "name: pdf-tools\n",
            "description: \"Reads \\\"quoted\\\" PDFs\"\n",
            "license: 'Apache-2.0, see ''LICENSE'''\n",
            "compatibility: Needs python3 # a comment\n",
            "allowed-tools: Bash(git log:*) Read\n",
            "metadata:\n  owner: example-team\n  version: \"1.0\"\n",
            "model: fast",
        ));
        assert_eq!(f.name.as_deref(), Some("pdf-tools"));
        assert_eq!(f.description.as_deref(), Some("Reads \"quoted\" PDFs"));
        assert_eq!(f.license.as_deref(), Some("Apache-2.0, see 'LICENSE'"));
        assert_eq!(f.compatibility.as_deref(), Some("Needs python3"));
        assert_eq!(f.allowed_tools, ["Bash(git log:*)", "Read"]);
        assert_eq!(f.metadata["owner"], "example-team");
        assert_eq!(f.metadata["version"], "1.0");
        assert!(f.not_strings.is_empty());
        assert_eq!(f.unknown_keys, ["model"]);
    }

    #[test]
    fn reads_block_scalars_in_full() {
        let folded = front("description: >\n  Line one\n  line two.\n\n  New paragraph.\nname: x");
        assert_eq!(
            folded.description.as_deref(),
            Some("Line one line two.\nNew paragraph.\n")
        );
        assert_eq!(folded.name.as_deref(), Some("x"));
        let literal = front("description: |-\n  one\n  two");
        assert_eq!(literal.description.as_deref(), Some("one\ntwo"));
        let plain = front("description: starts here\n  and goes on");
        assert_eq!(
            plain.description.as_deref(),
            Some("starts here and goes on")
        );
    }

    #[test]
    fn reads_tool_lists() {
        assert_eq!(
            front("allowed-tools: [Read, \"Bash(ls:*)\"]").allowed_tools,
            ["Read", "Bash(ls:*)"]
        );
        assert_eq!(
            front("allowed-tools:\n  - Read\n  - Grep").allowed_tools,
            ["Read", "Grep"]
        );
        assert_eq!(
            front("allowed-tools: Read, Grep").allowed_tools,
            ["Read", "Grep"]
        );
    }

    #[test]
    fn a_valid_skill_has_no_findings() {
        assert!(rules("name: pdf-tools\ndescription: Reads PDFs.", "pdf-tools").is_empty());
    }

    #[test]
    fn each_rule_fails_on_its_own_fixture() {
        let cases: [(&str, &str, &str, Level, &str); 13] = [
            (
                "description: d",
                "x",
                "name-required",
                Level::Error,
                "no name",
            ),
            (
                "name: ''\ndescription: d",
                "",
                "name-length",
                Level::Error,
                "0 characters",
            ),
            (
                &format!("name: {}\ndescription: d", "a".repeat(65)),
                &"a".repeat(65),
                "name-length",
                Level::Error,
                "65 characters",
            ),
            (
                "name: Pdf\ndescription: d",
                "Pdf",
                "name-chars",
                Level::Error,
                "\"Pdf\"",
            ),
            (
                "name: pdf_x\ndescription: d",
                "pdf_x",
                "name-chars",
                Level::Error,
                "\"pdf_x\"",
            ),
            (
                "name: -pdf\ndescription: d",
                "-pdf",
                "name-hyphens",
                Level::Error,
                "\"-pdf\"",
            ),
            (
                "name: pdf-\ndescription: d",
                "pdf-",
                "name-hyphens",
                Level::Error,
                "\"pdf-\"",
            ),
            (
                "name: pdf--x\ndescription: d",
                "pdf--x",
                "name-hyphens",
                Level::Error,
                "\"pdf--x\"",
            ),
            (
                "name: data-query\ndescription: d",
                "data_query",
                "name-matches-dir",
                Level::Error,
                "name is data-query, directory is data_query",
            ),
            (
                "name: x",
                "x",
                "description-required",
                Level::Error,
                "no description",
            ),
            (
                &format!("name: x\ndescription: {}", "d".repeat(1025)),
                "x",
                "description-length",
                Level::Error,
                "1025 characters",
            ),
            (
                &format!(
                    "name: x\ndescription: d\ncompatibility: {}",
                    "c".repeat(501)
                ),
                "x",
                "compatibility-length",
                Level::Warn,
                "501 characters",
            ),
            (
                "name: x\ndescription: d\nmetadata:\n  version: 1.0",
                "x",
                "metadata-strings",
                Level::Warn,
                "version: 1.0",
            ),
        ];
        for (head, dir, rule, level, value) in cases {
            let found = rules(head, dir);
            assert!(
                found
                    .iter()
                    .any(|(r, l, t)| *r == rule && *l == level && t.contains(value)),
                "{rule} on {head:?}: {found:?}"
            );
        }
        let missing = lint(None, "x");
        assert_eq!(missing[0].rule, "frontmatter");
        assert_eq!(missing[0].level, Level::Error);
    }

    #[test]
    fn ignores_code_spans_and_link_text() {
        let tmp = tempfile::tempdir().unwrap();
        let skill = tmp.path().join("skills").join("demo");
        fs::create_dir_all(skill.join("references")).unwrap();
        fs::write(skill.join("references").join("REAL.md"), "x").unwrap();
        let body = "Send `SiteAudit/1.0` as the agent. See `docs/setup.md` later.\n\
                    Read [rules/corrections.md](references/REAL.md) first.\n\
                    ```sh\nls scripts/missing.sh\n```\n";
        let r = refs(&skill, body);
        assert_eq!(r.ok, 1, "the link target resolves");
        assert!(
            r.broken.is_empty(),
            "nothing in a code span or link text is a reference: {:?}",
            r.broken
        );
    }

    #[test]
    fn counts_files_and_resolves_references() {
        let t = tempfile::tempdir().unwrap();
        let dir = t.path().join("pdf-tools");
        fs::create_dir_all(dir.join("scripts")).unwrap();
        fs::create_dir_all(dir.join("references")).unwrap();
        fs::write(dir.join("scripts").join("run.py"), "print()\n").unwrap();
        fs::write(dir.join("references").join("GOOD.md"), "ok\n").unwrap();
        fs::write(dir.join("notes.txt"), "n\n").unwrap();
        let body = "See [the guide](references/GOOD.md).\nRun scripts/run.py, \
                    then read references/MISSING.md and/or https://example.com/a.md.\n";
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: pdf-tools\ndescription: d\nmetadata:\n  owner: example-team\n---\n{body}"),
        )
        .unwrap();
        let mut found = Vec::new();
        collect(t.path(), Source::Home, 2, &mut found);
        let s = &found[0];
        assert_eq!(
            s.files,
            Files {
                scripts: 1,
                references: 1,
                assets: 0,
                other: 1,
                bytes: 13
            }
        );
        assert_eq!(
            s.refs,
            Refs {
                ok: 2,
                broken: vec!["references/MISSING.md".into()]
            }
        );
        assert_eq!(s.owner(), Some("example-team"));
        assert!(details(s).contains("owner: example-team"));
        assert!(details(s).contains("broken reference: references/MISSING.md"));
    }
}
