//! The Tree view: a directory as `agents tree` prints it, with a size
//! column, a row a person can move, and a preview of what that row names.
//! Structure and sizes come from metadata; the preview reads the first
//! lines of one file. Nothing here writes.

use crate::home::{tilde, Home};
use crate::out::fit;
use crate::ui::files::preview;
use crate::ui::layout;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// What a path is, as the tree shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Dir,
    File,
    Link(PathBuf),
}

/// One path and, for a directory, the paths under it to the loaded depth.
#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub path: PathBuf,
    pub kind: Kind,
    /// A file's bytes, or the sum over a directory's loaded children.
    pub bytes: u64,
    /// A file's newlines, or the sum over a directory's loaded children.
    pub lines: usize,
    /// Seconds since the epoch, when the metadata carries a time.
    pub modified: Option<u64>,
    pub always_on: bool,
    pub children: Vec<Node>,
}

impl Node {
    /// `path` and everything under it to `depth` levels, in the order the
    /// tree prints: directories first, then by name. Build output and
    /// vendored trees are skipped unless `all`.
    pub fn load(path: &Path, depth: usize, all: bool, always_on: &[PathBuf]) -> Node {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let meta = fs::symlink_metadata(path).ok();
        let modified = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let mut node = Node {
            name,
            path: path.to_path_buf(),
            kind: Kind::File,
            bytes: 0,
            lines: 0,
            modified,
            always_on: false,
            children: Vec::new(),
        };
        if path.is_symlink() {
            node.kind = Kind::Link(fs::read_link(path).unwrap_or_default());
        } else if path.is_dir() {
            node.kind = Kind::Dir;
            if depth > 0 {
                node.children = children(path, all)
                    .iter()
                    .map(|p| Node::load(p, depth - 1, all, always_on))
                    .collect();
                node.bytes = node.children.iter().map(|c| c.bytes).sum();
                node.lines = node.children.iter().map(|c| c.lines).sum();
            }
        } else {
            node.bytes = meta.map_or(0, |m| m.len());
            node.lines = fs::read(path)
                .map(|b| b.iter().filter(|&&c| c == b'\n').count())
                .unwrap_or(0);
            node.always_on = always_on.contains(&node.path);
        }
        node
    }

    /// The name as a listing shows it: a directory ends in `/`, a link
    /// names its target.
    pub fn label(&self) -> String {
        match &self.kind {
            Kind::Dir => format!("{}/", self.name),
            Kind::File => self.name.clone(),
            Kind::Link(target) => format!("{} -> {}", self.name, tilde(target)),
        }
    }
}

/// The entries of `dir` in the order the tree prints them.
pub fn children(dir: &Path, all: bool) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    entries.retain(|p| {
        all || !p
            .file_name()
            .is_some_and(|n| crate::tree::SKIP.contains(&n.to_string_lossy().as_ref()))
    });
    entries.sort_by_key(|p| (!p.is_dir(), p.file_name().map(|n| n.to_os_string())));
    entries
}

/// Bytes as a file manager shows them, in at most five characters: the
/// number below 1024, then one decimal and a unit.
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["K", "M", "G", "T", "P", "E"];
    if bytes < 1024 {
        return bytes.to_string();
    }
    let mut n = bytes as f64 / 1024.0;
    let mut unit = 0;
    while n >= 1000.0 && unit + 1 < UNITS.len() {
        n /= 1024.0;
        unit += 1;
    }
    if n < 10.0 {
        format!("{n:.1}{}", UNITS[unit])
    } else {
        format!("{}{}", n.round() as u64, UNITS[unit])
    }
}

/// Seconds since the epoch as `YYYY-MM-DD`.
pub fn date(secs: u64) -> String {
    let (y, m, d) = crate::reset::civil_from_days((secs / 86400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The row of a tree line: its drawn text and the node it names.
#[derive(Clone, Debug)]
pub struct TreeRow {
    /// The branch characters and the label, as `agents tree` prints them.
    pub text: String,
    pub node: Node,
}

/// The tree of one directory, with one row selected.
pub struct TreeView {
    pub root: PathBuf,
    pub rows: Vec<TreeRow>,
    pub selected: usize,
    pub all: bool,
}

impl TreeView {
    /// The tree under `root` to `depth` levels. Always-on files are marked
    /// when `root` is in the home.
    pub fn load(root: &Path, depth: usize, all: bool, home: Option<&Home>) -> TreeView {
        let always_on: Vec<PathBuf> = home
            .filter(|h| root.starts_with(&h.dir))
            .map(Home::always_on)
            .unwrap_or_default();
        let node = Node::load(root, depth, all, &always_on);
        let mut rows = Vec::new();
        flatten(&node, "", &mut rows);
        TreeView {
            root: root.to_path_buf(),
            rows,
            selected: 0,
            all,
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

    pub fn selected(&self) -> Option<&Node> {
        self.rows.get(self.selected).map(|r| &r.node)
    }

    /// Select the row on `line` of a `render` of `height` lines. A line
    /// outside the list selects nothing.
    pub fn pick(&mut self, line: usize, height: usize) {
        let (list_h, _) = layout::heights(height, 1, 1, self.detail_len());
        let start = window(self.selected, list_h);
        if let Some(i) = line
            .checked_sub(1)
            .filter(|r| *r < list_h)
            .map(|r| start + r)
            .filter(|i| *i < self.rows.len())
        {
            self.selected = i;
        }
    }

    fn detail_len(&self) -> usize {
        let mut n = 1;
        if let Some(node) = self.selected() {
            n += 1;
            n += preview(node, self.all, usize::MAX, usize::MAX).len();
        }
        n
    }

    fn detail_lines(&self, width: usize, preview_h: usize) -> Vec<String> {
        let mut lines = vec![layout::rule(width)];
        if let Some(n) = self.selected() {
            lines.push(fit(&n.label(), width));
            lines.extend(preview(n, self.all, width, preview_h));
        }
        lines
    }

    /// `height` lines of at most `width` characters: the root, the rows
    /// around the selection, a rule, a preview below, then the counts.
    pub fn render(&self, width: usize, height: usize) -> Vec<String> {
        let (list_h, detail_h) = layout::heights(height, 1, 1, self.detail_len());
        let detail = self.detail_lines(width, detail_h);
        let start = window(self.selected, list_h);
        let name_w = width.saturating_sub(2 + 2 + 5 + 2 + 5);
        let mut out = vec![fit(&tilde(&self.root), width)];
        out.extend(
            self.rows
                .iter()
                .enumerate()
                .skip(start)
                .take(list_h)
                .map(|(i, r)| {
                    let mark = if i == self.selected { "> " } else { "  " };
                    let n = &r.node;
                    let (bytes, lines) = match n.kind {
                        Kind::Link(_) => ("-".to_string(), "-".to_string()),
                        _ => (size(n.bytes), n.lines.to_string()),
                    };
                    let mut text = r.text.clone();
                    if n.always_on {
                        text.push_str("  always-on");
                    }
                    format!(
                        "{mark}{:<name_w$}  {bytes:>5}  {lines:>5}",
                        fit(&text, name_w)
                    )
                }),
        );
        out.extend(detail.into_iter().take(detail_h));
        let dirs = self
            .rows
            .iter()
            .filter(|r| r.node.kind == Kind::Dir)
            .count();
        let files = self.rows.len() - dirs;
        let lines: usize = self
            .rows
            .iter()
            .filter(|r| r.node.kind != Kind::Dir)
            .map(|r| r.node.lines)
            .sum();
        let bytes: u64 = self
            .rows
            .iter()
            .filter(|r| r.node.kind != Kind::Dir)
            .map(|r| r.node.bytes)
            .sum();
        out.resize(height.saturating_sub(1), String::new());
        out.push(fit(
            &format!(
                "{dirs} directories, {files} files, {lines} lines, {}",
                size(bytes)
            ),
            width,
        ));
        out.into_iter().map(|l| fit(&l, width)).collect()
    }
}

fn flatten(node: &Node, prefix: &str, out: &mut Vec<TreeRow>) {
    let n = node.children.len();
    for (i, c) in node.children.iter().enumerate() {
        let last = i + 1 == n;
        let branch = if last { "└── " } else { "├── " };
        out.push(TreeRow {
            text: format!("{prefix}{branch}{}", c.label()),
            node: c.clone(),
        });
        if c.kind == Kind::Dir {
            let next = format!("{prefix}{}", if last { "    " } else { "│   " });
            flatten(c, &next, out);
        }
    }
}

/// The first row shown, so the selected row is always in view.
pub fn window(selected: usize, height: usize) -> usize {
    if height == 0 {
        return selected;
    }
    (selected + 1).saturating_sub(height)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::out::visible;

    /// A home with the always-on files, a rule, a skill, and one binary.
    pub(crate) fn fixture() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        let h = t.path();
        let write = |p: &str, s: &[u8]| {
            let p = h.join(p);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, s).unwrap();
        };
        write("AGENTS.md", b"# router\n");
        write("principles.md", b"# principles\n1. One idea per file.\n");
        write(
            "rules/boundaries.md",
            b"- Never push to main.\n- Ask first.\n",
        );
        write("skills/demo/SKILL.md", b"---\nname: demo\n---\nBody.\n");
        write("blob.bin", &[0u8; 2048]);
        fs::create_dir_all(h.join("target/debug")).unwrap();
        t
    }

    #[test]
    fn size_fits_five_characters() {
        assert_eq!(size(0), "0");
        assert_eq!(size(1023), "1023");
        assert_eq!(size(1536), "1.5K");
        assert_eq!(size(2048), "2.0K");
        assert_eq!(size(10 * 1024), "10K");
        assert_eq!(size(1_048_576), "1.0M");
        assert_eq!(size(999 * 1024), "999K");
        assert_eq!(size(1000 * 1024), "1.0M");
        for b in [0, 1023, 1536, 1_048_576, u64::MAX] {
            assert!(size(b).chars().count() <= 5, "{}", size(b));
        }
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(86400 * 365), "1971-01-01");
    }

    #[test]
    fn t5_tree_view_renders_rows_sizes_and_a_preview() {
        let t = fixture();
        let home = Home {
            dir: t.path().to_path_buf(),
        };
        let mut v = TreeView::load(t.path(), 4, false, Some(&home));
        let texts: Vec<&str> = v.rows.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "├── rules/",
                "│   └── boundaries.md",
                "├── skills/",
                "│   └── demo/",
                "│       └── SKILL.md",
                "├── AGENTS.md",
                "├── blob.bin",
                "└── principles.md",
            ]
        );
        let lines = v.render(80, 12);
        assert_eq!(lines.len(), 12);
        assert!(lines.iter().all(|l| visible(l) <= 80), "{lines:#?}");
        assert!(lines[0].ends_with(t.path().file_name().unwrap().to_str().unwrap()));
        assert!(lines[1].starts_with("> ├── rules/"), "{}", lines[1]);
        assert!(lines
            .iter()
            .any(|l| l.contains("blob.bin") && l.contains("2.0K")));
        assert_eq!(lines[11], "3 directories, 5 files, 9 lines, 2.1K");
        let rule = lines.iter().position(|l| l.starts_with('─')).unwrap();
        assert!(
            lines[rule + 2..]
                .iter()
                .any(|l| l.contains("boundaries.md")),
            "{lines:#?}"
        );
        for _ in 0..7 {
            v.down();
        }
        assert_eq!(v.selected().unwrap().name, "principles.md");
        let lines = v.render(80, 12);
        assert!(
            lines.iter().any(|l| l.starts_with("> └── principles.md")),
            "{lines:#?}"
        );
        let rule = lines.iter().position(|l| l.starts_with('─')).unwrap();
        assert!(
            lines[rule + 1..].iter().any(|l| l.contains("# principles")),
            "{lines:#?}"
        );
        v.down();
        assert_eq!(v.selected, 7, "down stops at the last row");
        v.up();
        assert_eq!(v.selected().unwrap().name, "blob.bin");
        let lines = v.render(80, 12);
        let rule = lines.iter().position(|l| l.starts_with('─')).unwrap();
        assert!(
            lines[rule + 1..].iter().any(|l| l.contains("binary, 2.0K")),
            "{lines:#?}"
        );
    }

    #[test]
    fn tree_view_scrolls_to_keep_the_selection_and_drops_the_preview_when_narrow() {
        let t = fixture();
        let mut v = TreeView::load(t.path(), 4, true, None);
        assert!(v.rows.iter().any(|r| r.text.ends_with("target/")));
        v.selected = v.rows.len() - 1;
        let lines = v.render(40, 5);
        assert_eq!(lines.len(), 5);
        assert!(lines.iter().all(|l| visible(l) <= 40), "{lines:#?}");
        assert!(lines.iter().any(|l| l.starts_with("> ")), "{lines:#?}");
        assert!(lines.iter().any(|l| l.starts_with('─')));
        assert_eq!(window(0, 10), 0);
        assert_eq!(window(9, 10), 0);
        assert_eq!(window(10, 10), 1);
        assert_eq!(window(3, 0), 3);
    }

    #[test]
    fn t9_a_click_on_a_row_selects_that_node() {
        let t = fixture();
        let mut v = TreeView::load(t.path(), 4, false, None);
        let lines = v.render(80, 10);
        assert!(lines[2].contains("boundaries.md"), "{}", lines[2]);
        v.pick(2, 10);
        assert_eq!(v.selected().unwrap().name, "boundaries.md");
        for outside in [0, 9, 40] {
            v.pick(outside, 10);
            assert_eq!(v.selected, 1, "line {outside}");
        }
    }
}
