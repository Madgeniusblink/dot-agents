//! The Scan view: one row per checkout group, the checkouts of the selected
//! group in a detail pane, and a filter by kind. A group is every checkout of
//! one remote, as `agents scan` prints it.

use crate::out;
use serde_json::Value;
use std::collections::BTreeMap;

/// The kinds `scan` reports, with `All` first so the filter starts open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    All,
    Clone,
    Worktree,
    Submodule,
    Orphan,
}

impl Filter {
    const CYCLE: [Filter; 5] = [
        Filter::All,
        Filter::Clone,
        Filter::Worktree,
        Filter::Submodule,
        Filter::Orphan,
    ];

    /// The `kind` value `--json` prints, or None for every kind.
    fn kind(self) -> Option<&'static str> {
        match self {
            Filter::All => None,
            Filter::Clone => Some("clone"),
            Filter::Worktree => Some("worktree"),
            Filter::Submodule => Some("submodule"),
            Filter::Orphan => Some("orphan worktree"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Filter::All => "all kinds",
            Filter::Clone => "clones",
            Filter::Worktree => "worktrees",
            Filter::Submodule => "submodules",
            Filter::Orphan => "orphan worktrees",
        }
    }
}

/// One entry of `projects` in the scan document.
struct Checkout {
    root: String,
    kind: String,
    main: Option<String>,
    branch: Option<String>,
    ab: Option<(u64, u64)>,
    uncommitted: u64,
    last_commit: Option<String>,
    stale: bool,
    prune: Option<String>,
}

impl Checkout {
    fn read(v: &Value) -> Checkout {
        let text = |k: &str| v[k].as_str().map(str::to_string);
        Checkout {
            root: text("root").unwrap_or_default(),
            kind: text("kind").unwrap_or_default(),
            main: text("main_checkout"),
            branch: text("branch"),
            ab: v["ahead"].as_u64().zip(v["behind"].as_u64()),
            uncommitted: v["uncommitted"].as_u64().unwrap_or(0),
            last_commit: text("last_commit"),
            stale: v["stale"].as_bool().unwrap_or(false),
            prune: text("prune"),
        }
    }

    /// The facts after the root, in the order the text report prints them.
    fn facts(&self) -> String {
        let mut parts = vec![match &self.main {
            Some(m) => format!("{} of {m}", self.kind),
            None => self.kind.clone(),
        }];
        if self.kind != "orphan worktree" {
            parts.push(format!(
                "branch {}",
                self.branch.as_deref().unwrap_or("unknown")
            ));
            parts.push(match self.ab {
                Some((a, b)) => format!("{a} ahead {b} behind"),
                None => "no upstream".to_string(),
            });
            parts.push(format!("{} uncommitted", self.uncommitted));
            parts.push(match &self.last_commit {
                Some(d) => format!("last commit {d}"),
                None => "no commit".to_string(),
            });
        }
        if self.stale {
            parts.push("stale".to_string());
        }
        if let Some(p) = &self.prune {
            parts.push(format!("clear its record: {p}"));
        }
        parts.join(", ")
    }
}

/// Every checkout of one remote, in root order.
struct Group {
    remote: String,
    checkouts: Vec<Checkout>,
}

impl Group {
    /// The checkouts the filter keeps.
    fn kept(&self, filter: Filter) -> Vec<&Checkout> {
        self.checkouts
            .iter()
            .filter(|c| filter.kind().is_none_or(|k| c.kind == k))
            .collect()
    }

    /// The row `agents scan` prints for the group: how many checkouts, the
    /// newest, and `stale`, `orphan`, or `copy` where they apply.
    fn row(&self, kept: &[&Checkout]) -> out::Row {
        let newest = kept
            .iter()
            .max_by(|a, b| a.last_commit.cmp(&b.last_commit))
            .map_or("", |c| c.root.as_str());
        let stale = kept.iter().filter(|c| c.stale).count();
        let orphan = kept.iter().filter(|c| c.kind == "orphan worktree").count();
        let clones = kept.iter().filter(|c| c.kind == "clone").count();
        let mut marks = Vec::new();
        if stale > 0 {
            marks.push(format!("{stale} stale"));
        }
        if orphan > 0 {
            marks.push(format!("{orphan} orphan"));
        }
        if self.remote != "no remote" && clones > 1 {
            marks.push("copy".to_string());
        }
        let n = kept.len();
        let unit = if n == 1 { "checkout" } else { "checkouts" };
        out::Row::new(
            &self.remote,
            format!("{n:>3} {unit}"),
            format!("{newest}  {}", marks.join(", ")),
        )
    }
}

/// The view's state: the groups of the scan document, the filter, and the
/// selected row among the groups the filter keeps.
pub struct Scan {
    groups: Vec<Group>,
    filter: Filter,
    selected: usize,
}

impl Scan {
    /// The groups of a scan document, by remote in byte order, `no remote`
    /// for a checkout without one.
    pub fn from(report: &Value) -> Scan {
        let mut groups: BTreeMap<String, Vec<Checkout>> = BTreeMap::new();
        for p in report["projects"].as_array().into_iter().flatten() {
            let remote = p["remote"].as_str().unwrap_or("no remote").to_string();
            groups.entry(remote).or_default().push(Checkout::read(p));
        }
        Scan {
            groups: groups
                .into_iter()
                .map(|(remote, checkouts)| Group { remote, checkouts })
                .collect(),
            filter: Filter::All,
            selected: 0,
        }
    }

    pub fn filter(&self) -> Filter {
        self.filter
    }

    /// The next kind in the cycle, back to every kind after orphans. The
    /// selection returns to the first row.
    pub fn next_filter(&mut self) {
        let i = Filter::CYCLE.iter().position(|f| *f == self.filter);
        self.filter = Filter::CYCLE[(i.unwrap_or(0) + 1) % Filter::CYCLE.len()];
        self.selected = 0;
    }

    pub fn down(&mut self) {
        if self.selected + 1 < self.shown().len() {
            self.selected += 1;
        }
    }

    pub fn up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Each group with a checkout the filter keeps, and those checkouts.
    fn shown(&self) -> Vec<(&Group, Vec<&Checkout>)> {
        self.groups
            .iter()
            .map(|g| (g, g.kept(self.filter)))
            .filter(|(_, kept)| !kept.is_empty())
            .collect()
    }

    /// The pane as at most `height` lines of at most `width` characters:
    /// the filter, the rows with `>` at the selected one, then the selected
    /// group's checkouts. A list too long for its room ends in `… N more`.
    pub fn render(&self, width: usize, height: usize) -> Vec<String> {
        let shown = self.shown();
        let mut lines = vec![out::fit(
            &format!(
                "filter: {}  ({} of {} groups)",
                self.filter.label(),
                shown.len(),
                self.groups.len()
            ),
            width,
        )];
        if shown.is_empty() {
            lines.push(out::fit("  no checkout of this kind", width));
            lines.truncate(height);
            return lines;
        }
        let rows: Vec<out::Row> = shown.iter().map(|(g, kept)| g.row(kept)).collect();
        let rows = out::rows(&rows, width.saturating_sub(2));
        // The filter line, a blank, and the detail header take three lines.
        // The rows get up to half of the rest, and the detail the remainder.
        let room = height.saturating_sub(3);
        let row_room = rows.len().min(room.div_ceil(2)).max(1);
        let first = self.selected.saturating_sub(row_room - 1);
        let marked = rows
            .iter()
            .enumerate()
            .skip(first)
            .take(row_room)
            .map(|(i, r)| {
                let mark = if i == self.selected { '>' } else { ' ' };
                out::fit(&format!("{mark} {}", r.trim_start()), width)
            });
        lines.extend(cut(marked.collect(), rows.len() - first, width));
        let (group, kept) = &shown[self.selected.min(shown.len() - 1)];
        lines.push(String::new());
        let n = kept.len();
        let unit = if n == 1 { "checkout" } else { "checkouts" };
        lines.push(out::fit(&format!("{}, {n} {unit}", group.remote), width));
        // The root whole, since the path is the point of the pane.
        let detail_room = height.saturating_sub(lines.len());
        let detail = kept
            .iter()
            .take(detail_room)
            .map(|c| out::fit(&format!("  {}  {}", c.root, c.facts()), width));
        lines.extend(cut(detail.collect(), kept.len(), width));
        lines.truncate(height);
        lines
    }
}

/// `lines`, the first `lines.len()` of `total`. When some are missing, the
/// last line says how many the pane hides, that line included. A window of
/// one line keeps its item, since the header above it carries the count.
fn cut(mut lines: Vec<String>, total: usize, width: usize) -> Vec<String> {
    if lines.len() < total && lines.len() >= 2 {
        lines.pop();
        let hidden = total - lines.len();
        lines.push(out::fit(&format!("  … {hidden} more"), width));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// `git`, each commit dated 2020 so every checkout is stale.
    fn git(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(args)
            .env("GIT_AUTHOR_DATE", "2020-01-02T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2020-01-02T00:00:00Z")
            .output()
            .expect("git");
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// A root with two clones of one remote, a worktree of the first, a
    /// submodule inside the second, a checkout with no remote, and a
    /// worktree whose source directory was moved.
    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        // git reports resolved paths: macOS /var is a link.
        let root = tmp.path().canonicalize().unwrap();
        let s = |p: &Path| p.to_string_lossy().into_owned();
        let seed = root.join("seed");
        fs::create_dir_all(&seed).unwrap();
        git(&seed, &["init", "-q", "-b", "main"]);
        fs::write(seed.join("AGENTS.md"), "# app\n").unwrap();
        git(&seed, &["add", "-A"]);
        git(&seed, &["commit", "-qm", "init"]);
        let remote = root.join("remotes/team/app.git");
        git(&root, &["clone", "-q", "--bare", &s(&seed), &s(&remote)]);
        fs::remove_dir_all(&seed).unwrap();
        for name in ["a", "b"] {
            git(&root, &["clone", "-q", &s(&remote), name]);
        }
        let a = root.join("a");
        git(
            &a,
            &["worktree", "add", "-q", "-b", "side", &s(&root.join("wt"))],
        );
        let b = root.join("b");
        git(
            &b,
            &[
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "add",
                "-q",
                &s(&remote),
                "sub",
            ],
        );
        git(&b, &["commit", "-qm", "sub"]);
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        git(&src, &["init", "-q", "-b", "main"]);
        fs::write(src.join("AGENTS.md"), "# src\n").unwrap();
        git(&src, &["add", "-A"]);
        git(&src, &["commit", "-qm", "init"]);
        git(
            &src,
            &["worktree", "add", "-q", "-b", "o", &s(&root.join("orphan"))],
        );
        fs::rename(&src, root.join("src-moved")).unwrap();
        (tmp, root)
    }

    fn view(root: &Path) -> Scan {
        let report = crate::scan::report(&[root.to_path_buf()], false, None, &Config::default());
        assert_eq!(report["projects"].as_array().unwrap().len(), 6, "{report}");
        Scan::from(&report)
    }

    /// The row lines: after the filter line, up to the blank.
    fn rows(lines: &[String]) -> Vec<&String> {
        lines[1..].iter().take_while(|l| !l.is_empty()).collect()
    }

    /// The detail lines: after the blank.
    fn detail(lines: &[String]) -> Vec<&String> {
        lines.iter().skip_while(|l| !l.is_empty()).skip(1).collect()
    }

    #[test]
    fn t3_one_row_per_checkout_group() {
        let (_tmp, root) = fixture();
        let lines = view(&root).render(160, 40);
        let rows = rows(&lines);
        assert_eq!(rows.len(), 2, "{lines:#?}");
        assert!(rows[0].starts_with("> no remote"), "{}", rows[0]);
        assert!(rows[0].contains("  2 checkouts"), "{}", rows[0]);
        assert!(rows[0].contains("1 stale, 1 orphan"), "{}", rows[0]);
        assert!(rows[1].starts_with("  team/app"), "{}", rows[1]);
        assert!(rows[1].contains("  4 checkouts"), "{}", rows[1]);
        assert!(rows[1].ends_with("stale, copy"), "{}", rows[1]);
        assert!(lines.iter().all(|l| l.chars().count() <= 160), "{lines:#?}");
        assert!(lines.len() <= 40);
    }

    #[test]
    fn t3_filter_by_kind() {
        let (_tmp, root) = fixture();
        let mut v = view(&root);
        v.next_filter();
        assert_eq!(v.filter(), Filter::Clone);
        let lines = v.render(160, 40);
        assert_eq!(lines[0], "filter: clones  (2 of 2 groups)");
        let rows_all = rows(&lines);
        assert!(rows_all[0].contains("  1 checkout "), "{}", rows_all[0]);
        assert!(rows_all[1].contains("  2 checkouts"), "{}", rows_all[1]);
        v.next_filter();
        v.next_filter();
        v.next_filter();
        assert_eq!(v.filter(), Filter::Orphan);
        let lines = v.render(160, 40);
        assert_eq!(lines[0], "filter: orphan worktrees  (1 of 2 groups)");
        let rows_orphan = rows(&lines);
        assert_eq!(rows_orphan.len(), 1, "{lines:#?}");
        assert!(
            rows_orphan[0].starts_with("> no remote"),
            "{}",
            rows_orphan[0]
        );
        let d = detail(&lines);
        assert_eq!(d[0], "no remote, 1 checkout");
        assert_eq!(d.len(), 2, "{d:#?}");
        assert!(
            d[1].contains("orphan worktree, stale, clear its record: git -C"),
            "{}",
            d[1]
        );
        v.next_filter();
        assert_eq!(v.filter(), Filter::All);
    }

    #[test]
    fn t3_detail_pane_lists_the_selected_group() {
        let (_tmp, root) = fixture();
        let mut v = view(&root);
        v.down();
        // Wide enough for two temp paths on one line.
        let lines = v.render(240, 40);
        assert!(rows(&lines)[1].starts_with("> team/app"), "{lines:#?}");
        let d = detail(&lines);
        assert_eq!(d[0], "team/app, 4 checkouts");
        let facts: Vec<&str> = d[1..].iter().map(|l| l.trim_start()).collect();
        assert_eq!(facts.len(), 4, "{d:#?}");
        let find = |name: &str| {
            let prefix = format!("{}/{name} ", root.display());
            facts
                .iter()
                .find(|f| f.starts_with(&prefix))
                .unwrap_or_else(|| panic!("{name} in {facts:#?}"))
        };
        assert!(
            find("a").ends_with(
                "clone, branch main, 0 ahead 0 behind, 0 uncommitted, last commit 2020-01-02, stale"
            ),
            "{}",
            find("a")
        );
        assert!(
            find("b").contains("clone, branch main, 1 ahead 0 behind"),
            "{}",
            find("b")
        );
        assert!(
            find("wt").contains(&format!("worktree of {}/a, branch side", root.display())),
            "{}",
            find("wt")
        );
        assert!(
            find("b/sub").contains(&format!("submodule of {}/b, ", root.display())),
            "{}",
            find("b/sub")
        );
        assert!(lines.iter().all(|l| l.chars().count() <= 240), "{lines:#?}");
    }

    #[test]
    fn t3_a_short_pane_keeps_the_selected_row_and_counts_what_it_hides() {
        let (_tmp, root) = fixture();
        let mut v = view(&root);
        v.down();
        // Five lines: the filter, one row, a blank, the header, one item.
        let lines = v.render(160, 5);
        assert_eq!(lines.len(), 5, "{lines:#?}");
        assert!(lines[1].starts_with("> team/app"), "{lines:#?}");
        assert_eq!(lines[3], "team/app, 4 checkouts");
        assert!(lines[4].contains("/a  clone,"), "{lines:#?}");
        // Seven lines: both rows, then two of four items and the count.
        let lines = v.render(160, 7);
        assert_eq!(lines.len(), 7, "{lines:#?}");
        assert!(lines[1].starts_with("  no remote"), "{lines:#?}");
        assert!(lines[2].starts_with("> team/app"), "{lines:#?}");
        assert_eq!(lines[6], "  … 3 more", "{lines:#?}");
        v.up();
        let lines = v.render(40, 7);
        assert!(lines[1].starts_with("> no remote"), "{lines:#?}");
        assert!(lines.iter().all(|l| l.chars().count() <= 40), "{lines:#?}");
    }
}
