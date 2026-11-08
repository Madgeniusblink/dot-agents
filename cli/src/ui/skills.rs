//! The Skills view of `agents ui`: the counts per source, then one row per
//! skill that is yours, `BROKEN`, or a copy, and a detail pane that shows the
//! selected skill and each copy of it. The view draws lines; the frame owns
//! the terminal. It reads the same `Skill` values that `agents skills --json`
//! prints, so the view and the report cannot disagree. `r` and `K` build
//! the plan of `agents skills remove` and `agents skills keep` for the marked
//! names; `A` runs that same plan, as `--apply` does (decision 0011).

use crate::home::{tilde, Home};
use crate::out;
use crate::plan::Plan;
use crate::skills::{self, copy_note, Skill, Source, SOURCES};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The fewest lines the detail pane keeps when the terminal is short.
const DETAIL_MIN: usize = 5;

pub struct View<'a> {
    skills: &'a [Skill],
    project: Vec<&'a Skill>,
    root: Option<&'a Path>,
    /// The index into `skills` of each row the list shows, in listing order.
    rows: Vec<usize>,
    selected: usize,
    /// The first row on screen. It moves so the selected row stays visible.
    top: usize,
    /// The line of the first list row and the list height, from the last
    /// `render`, so a click lands on the row that was drawn there.
    list_at: usize,
    list_h: usize,
    /// The names `m` marked. The commands act on names, so every row with a
    /// marked name shows the mark.
    marked: BTreeSet<String>,
}

impl<'a> View<'a> {
    /// `skills` in the order `skills::find` returns them; `root` is the
    /// checkout the command runs in, if any.
    pub fn new(skills: &'a [Skill], root: Option<&'a Path>) -> View<'a> {
        let project: Vec<&Skill> = skills
            .iter()
            .filter(|s| s.source == Source::Project)
            .collect();
        let rows = skills
            .iter()
            .enumerate()
            .filter(|(_, s)| shown(s, &project, root))
            .map(|(i, _)| i)
            .collect();
        View {
            skills,
            project,
            root,
            rows,
            selected: 0,
            top: 0,
            list_at: 0,
            list_h: 0,
            marked: BTreeSet::new(),
        }
    }

    pub fn down(&mut self) {
        if self.selected + 1 < self.rows.len() {
            self.selected += 1;
        }
    }

    pub fn up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn selected(&self) -> Option<&'a Skill> {
        self.rows.get(self.selected).map(|&i| &self.skills[i])
    }

    /// Select the row drawn on `line` of the last `render`. A line outside
    /// the list selects nothing.
    pub fn pick(&mut self, line: usize) {
        let Some(at) = line.checked_sub(self.list_at).filter(|r| *r < self.list_h) else {
            return;
        };
        if self.top + at < self.rows.len() {
            self.selected = self.top + at;
        }
    }

    /// Mark the name of the selected row, or unmark it.
    pub fn mark(&mut self) {
        if let Some(s) = self.selected() {
            if !self.marked.remove(&s.name) {
                self.marked.insert(s.name.clone());
            }
        }
    }

    /// The selected skill's directory and the marked names, so a refresh
    /// keeps what the person chose.
    pub fn state(&self) -> (Option<PathBuf>, BTreeSet<String>) {
        (self.selected().map(|s| s.dir.clone()), self.marked.clone())
    }

    /// `state` from the view before a refresh. A mark on a name that is
    /// gone is dropped; a selected skill that is gone leaves the first row.
    pub fn restore(&mut self, (dir, marked): (Option<PathBuf>, BTreeSet<String>)) {
        if let Some(i) = self
            .rows
            .iter()
            .position(|&i| Some(&self.skills[i].dir) == dir.as_ref())
        {
            self.selected = i;
        }
        self.marked = marked
            .into_iter()
            .filter(|n| self.skills.iter().any(|s| &s.name == n))
            .collect();
    }

    /// The names `r` and `K` act on: the marked ones, else the selected one.
    pub fn names(&self) -> Vec<String> {
        if self.marked.is_empty() {
            self.selected()
                .map(|s| vec![s.name.clone()])
                .unwrap_or_default()
        } else {
            self.marked.iter().cloned().collect()
        }
    }

    /// Exactly `height` lines, none wider than `width` on a terminal. The
    /// counts and the list take the top, the detail pane the bottom, with a
    /// rule between. The selected row starts with `>`. On a short terminal
    /// the counts go first, so one row and the detail pane stay.
    pub fn render(&mut self, width: usize, height: usize) -> Vec<String> {
        let mut lines = vec![format!("tools can load {} skills.", self.skills.len())];
        let counts = out::rows(&self.counts(), width);
        if height >= lines.len() + counts.len() + 3 + DETAIL_MIN {
            lines.extend(counts);
            lines.push(String::new());
        }
        lines.push(format!("yours, BROKEN, or a copy ({}):", self.rows.len()));

        let detail = self.detail(width);
        let room = height.saturating_sub(lines.len() + 1);
        let detail_h = detail.len().min((height / 2).max(DETAIL_MIN)).min(room);
        let list_h = height.saturating_sub(lines.len() + detail_h);

        let rows = out::rows(&self.list_rows(), width);
        if list_h == 0 || self.selected < self.top {
            self.top = self.selected;
        } else if self.selected >= self.top + list_h {
            self.top = self.selected + 1 - list_h;
        }
        (self.list_at, self.list_h) = (lines.len(), list_h);
        for (i, row) in rows.iter().enumerate().skip(self.top).take(list_h) {
            let cursor = if i == self.selected { ">" } else { " " };
            let marked = self.marked.contains(&self.skills[self.rows[i]].name);
            let mark = if marked { "*" } else { " " };
            lines.push(format!("{cursor}{mark}{}", &row[2..]));
        }

        lines.extend(detail.into_iter().take(detail_h));
        lines.resize(height, String::new());
        lines.into_iter().map(|l| cut(&l, width)).collect()
    }

    /// One row per source that has a skill, as the brief listing prints it.
    fn counts(&self) -> Vec<out::Row> {
        SOURCES
            .iter()
            .filter_map(|&source| {
                let n = self.skills.iter().filter(|s| s.source == source).count();
                let label = match (source, self.root) {
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
            .collect()
    }

    /// The name, the line count with `BROKEN`, then the copy note and the
    /// description, for each skill the list shows.
    fn list_rows(&self) -> Vec<out::Row> {
        self.rows
            .iter()
            .map(|&i| {
                let s = &self.skills[i];
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
                let text = match copy_note(s, &self.project, self.root) {
                    Some(n) if n.starts_with("copy of") => format!("{n}  {desc}"),
                    _ => desc,
                };
                out::Row::new(&s.name, format!("{:>4} lines{mark}", s.lines), text)
            })
            .collect()
    }

    /// A rule, then the selected skill: where it is, its size, its
    /// description, its copy note, each copy of it with `identical` or
    /// `different`, and each lint finding.
    fn detail(&self, width: usize) -> Vec<String> {
        let mut lines = vec!["─".repeat(width)];
        let Some(s) = self.selected() else {
            lines.push("  nothing is yours, BROKEN, or a copy.".to_string());
            return lines;
        };
        lines.push(format!(
            "  {}  {}, {}",
            s.name,
            s.source.label(),
            s.source.scope()
        ));
        lines.push(format!("  {}", tilde(&s.dir)));
        let mut size = format!("  {} lines, {} bytes", s.lines, s.bytes);
        if let Some(owner) = s.owner() {
            size.push_str(&format!(", owner {owner}"));
        }
        lines.push(size);
        lines.extend(out::wrap(&s.description, 2, width));
        if let Some(note) = copy_note(s, &self.project, self.root) {
            lines.push(format!("  {note}"));
        }

        let copies: Vec<out::Row> = self
            .skills
            .iter()
            .filter(|t| !std::ptr::eq(*t, s) && (t.name == s.name || t.hash == s.hash))
            .map(|t| {
                let state = if t.hash == s.hash {
                    out::State::Identical
                } else {
                    out::State::Different
                };
                out::Row::new(t.source.scope(), out::state(state), tilde(&t.dir))
            })
            .collect();
        if copies.is_empty() {
            lines.push("  no other copy.".to_string());
        } else {
            lines.push(format!("  copies ({}):", copies.len()));
            lines.extend(out::rows(&copies, width));
        }

        for target in &s.refs.broken {
            lines.push(format!("  broken reference: {target}"));
        }
        lines.extend(s.lint.iter().map(|f| format!("  {f}")));
        lines
    }
}

/// A plan that `r` or `K` built, and the lines the view shows for it.
pub struct Pending {
    plan: Plan,
    pub lines: Vec<String>,
    pub done: bool,
}

impl Pending {
    /// The dry run of `agents skills remove` or, with `keep`, of
    /// `agents skills keep`, for `names`. Nothing is written.
    pub fn new(keep: bool, names: &[String], home: Option<&Home>) -> Pending {
        let plan = if keep {
            skills::keep(names, home)
        } else {
            skills::remove(names, home)
        };
        let mut lines = vec![format!(
            "dry run of agents skills {} {}. nothing is written. A applies it, esc cancels.",
            if keep { "keep" } else { "remove" },
            names.join(" ")
        )];
        lines.extend(text(&plan, false).0);
        Pending {
            plan,
            lines,
            done: false,
        }
    }

    /// Run the plan, as `--apply` runs it. The lines then show what ran, or
    /// the step that failed.
    pub fn apply(&mut self) {
        let (lines, ok) = text(&self.plan, true);
        self.lines = vec![if ok {
            "applied. esc goes back to the list.".to_string()
        } else {
            "FAILED. the steps above the error ran. esc goes back to the list.".to_string()
        }];
        self.lines.extend(lines);
        self.done = true;
    }
}

/// The lines `Plan::run_to` prints, then the error if a step failed, and
/// whether every step ran.
fn text(plan: &Plan, apply: bool) -> (Vec<String>, bool) {
    let mut buf = Vec::new();
    let result = plan.run_to(apply, &mut buf);
    let mut lines: Vec<String> = String::from_utf8_lossy(&buf)
        .lines()
        .map(str::to_string)
        .collect();
    if let Err(e) = &result {
        lines.push(format!("{e:#}"));
    }
    (lines, result.is_ok())
}

/// Whether the list shows `s`: it is yours, it is BROKEN, or it is a copy of
/// a project skill. The same rule as the brief `agents skills` listing.
fn shown(s: &Skill, project: &[&Skill], root: Option<&Path>) -> bool {
    s.source.scope() == "yours"
        || s.broken()
        || copy_note(s, project, root).is_some_and(|n| n.starts_with("copy of"))
}

/// `line` in at most `width` visible characters.
fn cut(line: &str, width: usize) -> String {
    if out::visible(line) <= width {
        line.to_string()
    } else {
        out::fit(line, width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::{Files, Finding, Level, Refs};
    use std::path::PathBuf;

    fn skill(name: &str, source: Source, dir: &str, hash: u64, broken: bool) -> Skill {
        let lint = if broken {
            vec![Finding {
                rule: "name",
                level: Level::Error,
                text: "no name".to_string(),
            }]
        } else {
            Vec::new()
        };
        Skill {
            name: name.to_string(),
            description: format!("What {name} does,\n  on two lines."),
            dir: PathBuf::from(dir),
            source,
            lines: 12,
            bytes: 340,
            hash,
            front: None,
            lint,
            files: Files::default(),
            refs: Refs::default(),
        }
    }

    /// Seven skills in `find` order. Five rows show: two `deploy` that are
    /// yours, `pdf` that is yours, `pdf` from a plugin that is BROKEN, and
    /// `lint` from a plugin that is a copy of the project's. The synced
    /// `notes` and the project's own `lint` stay out of the list.
    fn fixture() -> Vec<Skill> {
        vec![
            skill("deploy", Source::Home, "/h/.agents/skills/deploy", 1, false),
            skill("deploy", Source::User, "/h/.claude/skills/deploy", 1, false),
            skill("pdf", Source::User, "/h/.claude/skills/pdf", 5, false),
            skill(
                "notes",
                Source::Synced,
                "/h/.claude/skills/synced/a/notes",
                2,
                false,
            ),
            skill(
                "lint",
                Source::Plugin,
                "/h/.claude/plugins/cache/m/p/skills/lint",
                4,
                false,
            ),
            skill(
                "pdf",
                Source::Plugin,
                "/h/.claude/plugins/cache/m/p/skills/pdf",
                3,
                true,
            ),
            skill("lint", Source::Project, "/p/skills/lint", 4, false),
        ]
    }

    fn has(lines: &[String], text: &str) -> bool {
        lines.iter().any(|l| l.contains(text))
    }

    fn selected_line(lines: &[String]) -> &str {
        lines
            .iter()
            .find(|l| l.starts_with("> "))
            .expect("one row is selected")
    }

    #[test]
    fn t4_counts_per_source_then_rows_that_are_yours_broken_or_a_copy() {
        let skills = fixture();
        let root = PathBuf::from("/p");
        let mut view = View::new(&skills, Some(&root));
        let lines = view.render(100, 30);

        assert_eq!(lines[0], "tools can load 7 skills.");
        let counts = &lines[1..6];
        assert_eq!(counts[0].trim(), "1  home, yours");
        assert_eq!(counts[1].trim(), "2  claude code, user skills, yours");
        assert_eq!(
            counts[2].trim(),
            "1  claude code, skills synced from claude.ai, managed"
        );
        assert_eq!(counts[3].trim(), "2  claude code, plugin skills, managed");
        assert_eq!(counts[4].trim(), "1  project /p, project");
        assert_eq!(lines[6], "");
        assert_eq!(lines[7], "yours, BROKEN, or a copy (5):");

        let rows = &lines[8..13];
        assert!(rows[0].starts_with("> deploy"), "{}", rows[0]);
        assert!(rows[1].starts_with("  deploy"), "{}", rows[1]);
        assert!(rows[2].starts_with("  pdf"), "{}", rows[2]);
        assert!(
            rows[3].contains("lint") && rows[3].contains("copy of skills/lint (identical)"),
            "{}",
            rows[3]
        );
        assert!(
            rows[4].contains("pdf") && rows[4].contains("BROKEN"),
            "{}",
            rows[4]
        );
        assert!(
            !has(&lines, "notes"),
            "a managed skill that is clean stays out"
        );
        assert!(rows.iter().all(|r| r.contains("12 lines")));
        assert!(rows
            .iter()
            .all(|r| r.contains("What ") && r.contains("on two lines.")));
    }

    #[test]
    fn t4_detail_pane_shows_the_selected_skill_and_each_copy() {
        let skills = fixture();
        let root = PathBuf::from("/p");
        let mut view = View::new(&skills, Some(&root));

        let lines = view.render(100, 30);
        let rule = lines.iter().position(|l| l.starts_with('─')).unwrap();
        let detail = &lines[rule..];
        assert_eq!(detail[1], "  deploy  home, yours");
        assert_eq!(detail[2], "  /h/.agents/skills/deploy");
        assert_eq!(detail[3], "  12 lines, 340 bytes");
        assert_eq!(detail[4], "  What deploy does, on two lines.");
        assert_eq!(detail[5], "  copies (1):");
        assert_eq!(
            detail[6].trim(),
            "yours  identical  /h/.claude/skills/deploy"
        );

        for _ in 0..3 {
            view.down();
        }
        let lines = view.render(100, 30);
        assert!(selected_line(&lines).starts_with("> lint"));
        assert!(has(&lines, "  copy of skills/lint (identical)"));
        assert!(has(&lines, "copies (1):"));
        assert!(has(&lines, "project  identical  /p/skills/lint"));

        view.down();
        let lines = view.render(100, 30);
        assert!(selected_line(&lines).contains("BROKEN"));
        assert!(has(&lines, "  pdf  claude code, plugin skills, managed"));
        assert!(has(&lines, "yours  different  /h/.claude/skills/pdf"));
        assert!(has(&lines, "  error name: no name"));

        for _ in 0..10 {
            view.up();
        }
        let lines = view.render(100, 30);
        assert!(selected_line(&lines).starts_with("> deploy"));
    }

    #[test]
    fn t4_render_fits_the_width_and_fills_the_height() {
        let skills = fixture();
        let root = PathBuf::from("/p");
        for (width, height) in [(100, 30), (60, 16), (40, 10), (24, 8)] {
            let mut view = View::new(&skills, Some(&root));
            for _ in 0..4 {
                view.down();
            }
            let lines = view.render(width, height);
            assert_eq!(lines.len(), height, "{width}x{height}");
            for l in &lines {
                assert!(out::visible(l) <= width, "{width}x{height}: {l:?}");
            }
            assert!(
                selected_line(&lines).starts_with("> pdf"),
                "{width}x{height}: the selected row scrolls into view"
            );
            // One title, five counts, a blank, a section line, one row, and
            // the five-line detail pane: 14 lines keep the counts.
            assert_eq!(
                has(&lines, "home, yours"),
                height >= 14,
                "{width}x{height}: the counts go first on a short terminal"
            );
            assert!(
                has(&lines, "─"),
                "{width}x{height}: the detail pane keeps its rule"
            );
        }
    }

    #[test]
    fn t4_an_empty_list_says_so() {
        let skills = vec![skill(
            "notes",
            Source::Synced,
            "/h/.claude/skills/synced/a/notes",
            2,
            false,
        )];
        let mut view = View::new(&skills, None);
        assert!(view.selected().is_none());
        let lines = view.render(80, 12);
        assert!(has(&lines, "yours, BROKEN, or a copy (0):"));
        assert!(has(&lines, "  nothing is yours, BROKEN, or a copy."));
        assert!(!lines.iter().any(|l| l.starts_with("> ")));
    }

    #[test]
    fn t9_a_click_on_a_row_selects_that_skill() {
        let skills = fixture();
        let root = PathBuf::from("/p");
        let mut view = View::new(&skills, Some(&root));
        let lines = view.render(100, 30);
        assert!(lines[10].starts_with("  pdf"), "{}", lines[10]);
        view.pick(10);
        let lines = view.render(100, 30);
        assert!(lines[10].starts_with("> pdf"), "{}", lines[10]);
        for outside in [0, 7, 29] {
            view.pick(outside);
            assert_eq!(view.selected, 2, "line {outside}");
        }
    }
}
