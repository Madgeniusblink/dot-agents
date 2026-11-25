//! The Files tab: a tree from the home at depth [`TREE_DEPTH`], and a flat
//! listing when a person enters a directory. Columns for the name, size,
//! lines, and date, a preview of the selected entry, and backspace to
//! leave a drilled folder for the tree again. In flat mode, `t` shows a scoped
//! tree at the selected directory; escape returns to the flat list. Nothing
//! here writes.

use crate::home::{tilde, Home};
use crate::out::fit;
use crate::ui::layout;
use crate::ui::tree::{children, date, size, window, Kind, Node, TreeView};
use std::io::Read;
use std::path::{Path, PathBuf};

/// How many levels the default tree loads under the home.
pub const TREE_DEPTH: usize = 2;

/// The most of a file the preview reads.
const PREVIEW_BYTES: u64 = 64 * 1024;

/// The width the fixed columns take: the mark, size, lines, and date, with
/// two spaces before each.
const FIXED: usize = 2 + 2 + 5 + 2 + 5 + 2 + 10;

/// Tree at the home, or a flat list after enter on a directory in the tree.
pub struct BrowseView {
    pub tree: TreeView,
    flat: Option<DrillFlat>,
}

/// Flat listing opened from the tree; `anchor` is the folder enter opened.
struct DrillFlat {
    files: FilesView,
    anchor: PathBuf,
    tree_selected: usize,
    /// A scoped tree overlay; escape dismisses it and restores `flat_selected`.
    scoped: Option<ScopedTree>,
}

/// A tree rooted at one directory, shown over a flat listing.
struct ScopedTree {
    tree: TreeView,
    flat_selected: usize,
}

impl BrowseView {
    pub fn load(home_dir: &Path, all: bool, home: Option<&Home>) -> BrowseView {
        BrowseView {
            tree: TreeView::load(home_dir, TREE_DEPTH, all, home),
            flat: None,
        }
    }

    pub fn down(&mut self) {
        match &mut self.flat {
            Some(d) if d.scoped.is_some() => {
                d.scoped.as_mut().unwrap().tree.down();
            }
            Some(d) => d.files.down(),
            None => self.tree.down(),
        }
    }

    pub fn up(&mut self) {
        match &mut self.flat {
            Some(d) if d.scoped.is_some() => {
                d.scoped.as_mut().unwrap().tree.up();
            }
            Some(d) => d.files.up(),
            None => self.tree.up(),
        }
    }

    pub fn pick(&mut self, line: usize, height: usize) {
        match &mut self.flat {
            Some(d) if d.scoped.is_some() => {
                d.scoped.as_mut().unwrap().tree.pick(line, height);
            }
            Some(d) => d.files.pick(line, height),
            None => self.tree.pick(line, height),
        }
    }

    pub fn selected(&self) -> Option<&Node> {
        match &self.flat {
            Some(d) if let Some(s) = &d.scoped => s.tree.selected(),
            Some(d) => d.files.selected(),
            None => self.tree.selected(),
        }
    }

    /// Flat mode only: show a scoped tree at the selected directory.
    pub fn show_scoped_tree(&mut self, home: Option<&Home>) {
        let Some(d) = &mut self.flat else {
            return;
        };
        if d.scoped.is_some() {
            return;
        }
        let Some(n) = d.files.selected().filter(|n| n.kind == Kind::Dir) else {
            return;
        };
        let flat_selected = d.files.selected;
        let tree = TreeView::load(&n.path, TREE_DEPTH, d.files.all, home);
        d.scoped = Some(ScopedTree {
            tree,
            flat_selected,
        });
    }

    /// Leave the scoped tree overlay and restore the flat selection.
    pub fn dismiss_scoped_tree(&mut self) {
        let Some(d) = &mut self.flat else {
            return;
        };
        let Some(s) = d.scoped.take() else {
            return;
        };
        d.files.selected = s.flat_selected.min(d.files.entries.len().saturating_sub(1));
    }

    pub fn in_scoped_tree(&self) -> bool {
        self.flat.as_ref().is_some_and(|d| d.scoped.is_some())
    }

    /// In the tree, enter opens a flat list of the selected directory. In
    /// flat mode, enter follows a subdirectory as today.
    pub fn enter(&mut self, home: Option<&Home>) {
        if let Some(d) = &mut self.flat {
            if d.scoped.is_some() {
                return;
            }
            d.files.enter();
            return;
        }
        let Some(n) = self.tree.selected().filter(|n| n.kind == Kind::Dir) else {
            return;
        };
        let tree_selected = self.tree.selected;
        let anchor = n.path.clone();
        let files = FilesView::load(&anchor, self.tree.all, home);
        self.flat = Some(DrillFlat {
            files,
            anchor,
            tree_selected,
            scoped: None,
        });
    }

    /// In flat mode at the drill anchor, back returns to the tree with the
    /// prior row selected; otherwise flat mode goes up one directory.
    pub fn back(&mut self) {
        if self.in_scoped_tree() {
            return;
        }
        let Some(d) = &mut self.flat else {
            return;
        };
        if d.files.dir == d.anchor {
            self.tree.selected = d.tree_selected;
            self.flat = None;
            return;
        }
        d.files.parent();
    }

    pub fn render(&self, width: usize, height: usize) -> Vec<String> {
        match &self.flat {
            Some(d) if let Some(s) = &d.scoped => s.tree.render(width, height),
            Some(d) => d.files.render(width, height),
            None => self.tree.render(width, height),
        }
    }

    /// After a refresh, put the tree, any open flat list, and any scoped
    /// tree over it back, each with its selection.
    pub fn restore(&mut self, home_dir: &Path, all: bool, home: Option<&Home>, kept: Kept) {
        self.tree = TreeView::load(home_dir, TREE_DEPTH, all, home);
        self.flat = kept.flat.map(|f| {
            let mut files = FilesView::load(&f.dir, all, home);
            files.selected = f.selected.min(files.entries.len().saturating_sub(1));
            let scoped = f.scoped.map(|(root, selected)| {
                let mut tree = TreeView::load(&root, TREE_DEPTH, all, home);
                tree.selected = selected.min(tree.rows.len().saturating_sub(1));
                ScopedTree {
                    tree,
                    flat_selected: files.selected,
                }
            });
            DrillFlat {
                files,
                anchor: f.anchor,
                tree_selected: f.tree_selected,
                scoped,
            }
        });
        if self.flat.is_none() {
            self.tree.selected = kept
                .tree_selected
                .min(self.tree.rows.len().saturating_sub(1));
        }
    }

    /// The directory the Files view lists, for the live poll.
    pub fn watch_paths(&self) -> Vec<PathBuf> {
        match &self.flat {
            Some(d) if let Some(s) = &d.scoped => vec![s.tree.root.clone()],
            Some(d) => vec![d.files.dir.clone()],
            None => vec![self.tree.root.clone()],
        }
    }

    /// What a refresh keeps, for `restore`.
    pub fn snapshot(&self) -> Kept {
        Kept {
            flat: self.flat.as_ref().map(|d| KeptFlat {
                anchor: d.anchor.clone(),
                dir: d.files.dir.clone(),
                // Escape from a scoped tree goes back to this row.
                selected: d
                    .scoped
                    .as_ref()
                    .map_or(d.files.selected, |s| s.flat_selected),
                tree_selected: d.tree_selected,
                scoped: d
                    .scoped
                    .as_ref()
                    .map(|s| (s.tree.root.clone(), s.tree.selected)),
            }),
            tree_selected: self.tree.selected,
        }
    }
}

/// The places and selections of a `BrowseView`, across a refresh.
pub struct Kept {
    flat: Option<KeptFlat>,
    tree_selected: usize,
}

struct KeptFlat {
    anchor: PathBuf,
    dir: PathBuf,
    selected: usize,
    tree_selected: usize,
    /// The root and the selected row of a scoped tree over the flat list.
    scoped: Option<(PathBuf, usize)>,
}

/// One directory, with one entry selected.
pub struct FilesView {
    pub dir: PathBuf,
    pub entries: Vec<Node>,
    pub selected: usize,
    pub all: bool,
    always_on: Vec<PathBuf>,
}

impl FilesView {
    /// The entries of `dir`. Build output and vendored trees are skipped
    /// unless `all`. Always-on files are marked when the home is known.
    pub fn load(dir: &Path, all: bool, home: Option<&Home>) -> FilesView {
        let mut v = FilesView {
            dir: dir.to_path_buf(),
            entries: Vec::new(),
            selected: 0,
            all,
            always_on: home.map(Home::always_on).unwrap_or_default(),
        };
        v.reload();
        v
    }

    fn reload(&mut self) {
        self.entries = Node::load(&self.dir, 1, self.all, &self.always_on).children;
    }

    pub fn down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    pub fn up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn selected(&self) -> Option<&Node> {
        self.entries.get(self.selected)
    }

    /// Select the entry on `line` of a `render` of `height` lines. A line
    /// outside the list selects nothing.
    pub fn pick(&mut self, line: usize, height: usize) {
        let (list_h, _) = layout::heights(height, 2, 0, self.detail_len());
        let start = window(self.selected, list_h);
        if let Some(i) = line
            .checked_sub(2)
            .filter(|r| *r < list_h)
            .map(|r| start + r)
            .filter(|i| *i < self.entries.len())
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

    /// Into the selected directory. A file or a link stays where it is.
    pub fn enter(&mut self) {
        if let Some(n) = self.selected().filter(|n| n.kind == Kind::Dir) {
            self.dir = n.path.clone();
            self.selected = 0;
            self.reload();
        }
    }

    /// Up to the parent, with the directory just left selected.
    pub fn parent(&mut self) {
        let Some(parent) = self.dir.parent().map(Path::to_path_buf) else {
            return;
        };
        let from = std::mem::replace(&mut self.dir, parent);
        self.reload();
        self.selected = self
            .entries
            .iter()
            .position(|e| e.path == from)
            .unwrap_or(0);
    }

    /// `height` lines of at most `width` characters: the directory and its
    /// totals, the column names, the rows around the selection, a rule, then
    /// a preview of the selection below.
    pub fn render(&self, width: usize, height: usize) -> Vec<String> {
        let files: Vec<&Node> = self
            .entries
            .iter()
            .filter(|n| n.kind == Kind::File)
            .collect();
        let total: u64 = files.iter().map(|n| n.bytes).sum();
        let mut out = vec![fit(
            &format!(
                "{}  {} items, {}",
                tilde(&self.dir),
                self.entries.len(),
                size(total)
            ),
            width,
        )];
        let name_w = width.saturating_sub(FIXED).max(8);
        let (list_h, detail_h) = layout::heights(height, 2, 0, self.detail_len());
        let detail = self.detail_lines(width, detail_h);
        let start = window(self.selected, list_h);
        out.push(format!(
            "  {:<name_w$}  {:>5}  {:>5}  {:<10}",
            "name", "size", "lines", "modified"
        ));
        if self.entries.is_empty() {
            out.push("  (empty)".to_string());
        }
        out.extend(
            self.entries
                .iter()
                .enumerate()
                .skip(start)
                .take(list_h)
                .map(|(i, n)| {
                    let mark = if i == self.selected { "> " } else { "  " };
                    let (bytes, lines) = match n.kind {
                        Kind::File => (size(n.bytes), n.lines.to_string()),
                        _ => ("-".to_string(), "-".to_string()),
                    };
                    let name = if n.always_on {
                        format!(
                            "{}  {}",
                            fit(&n.label(), name_w.saturating_sub(11).max(1)),
                            "always-on"
                        )
                    } else {
                        n.label()
                    };
                    format!(
                        "{mark}{:<name_w$}  {bytes:>5}  {lines:>5}  {:<10}",
                        fit(&name, name_w),
                        n.modified.map(date).unwrap_or_default()
                    )
                }),
        );
        out.extend(detail.into_iter().take(detail_h));
        out.resize(height, String::new());
        out.into_iter().map(|l| fit(&l, width)).collect()
    }
}

/// What the selected entry holds, in at most `height` lines of `width`: a
/// directory's entries, a link's target, or the first lines of a file.
pub fn preview(node: &Node, all: bool, width: usize, height: usize) -> Vec<String> {
    let lines: Vec<String> = match &node.kind {
        Kind::Link(target) => vec![format!("-> {}", tilde(target))],
        Kind::Dir => children(&node.path, all)
            .iter()
            .map(|p| {
                let name = p.file_name().unwrap_or_default().to_string_lossy();
                if p.is_dir() {
                    format!("{name}/")
                } else {
                    name.into_owned()
                }
            })
            .collect(),
        Kind::File => {
            let mut bytes = Vec::new();
            if let Ok(f) = std::fs::File::open(&node.path) {
                let _ = f.take(PREVIEW_BYTES).read_to_end(&mut bytes);
            }
            if bytes.contains(&0) {
                vec![format!("binary, {}", size(node.bytes))]
            } else {
                String::from_utf8_lossy(&bytes)
                    .lines()
                    .map(|l| l.replace('\t', "    "))
                    .collect()
            }
        }
    };
    if lines.is_empty() {
        return vec![fit("(empty)", width)];
    }
    let mut out: Vec<String> = lines.iter().take(height).map(|l| fit(l, width)).collect();
    if lines.len() > height && height > 0 {
        let rest = lines.len() + 1 - height;
        out[height - 1] = fit(&format!("… {rest} more lines"), width);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::out::visible;
    use crate::ui::tree::tests::fixture;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn t5_files_view_renders_columns_sizes_and_a_preview() {
        let t = fixture();
        let home = Home {
            dir: t.path().to_path_buf(),
        };
        let mut v = FilesView::load(t.path(), false, Some(&home));
        let names: Vec<&str> = v.entries.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(
            names,
            ["rules", "skills", "AGENTS.md", "blob.bin", "principles.md"]
        );
        let lines = v.render(80, 10);
        assert_eq!(lines.len(), 10);
        assert!(lines.iter().all(|l| visible(l) <= 80), "{lines:#?}");
        assert!(lines.iter().all(|l| !l.contains(" │ ")));
        assert!(lines[0].ends_with("  5 items, 2.0K"), "{}", lines[0]);
        assert!(lines[1].starts_with("  name"), "{}", lines[1]);
        assert!(lines[2].starts_with("> rules/"), "{}", lines[2]);
        let rule = lines.iter().position(|l| l.starts_with('─')).unwrap();
        assert!(lines[rule + 1].contains("rules/"), "{}", lines[rule + 1]);
        assert!(
            lines[rule + 2..]
                .iter()
                .any(|l| l.contains("boundaries.md")),
            "{lines:#?}"
        );
        let today = date(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        assert!(lines
            .iter()
            .any(|l| l.contains("AGENTS.md") && l.contains(&today)));
        for _ in 0..4 {
            v.down();
        }
        assert_eq!(v.selected().unwrap().name, "principles.md");
        let lines = v.render(80, 10);
        let rule = lines.iter().position(|l| l.starts_with('─')).unwrap();
        assert!(
            lines[rule + 1].contains("principles.md"),
            "{}",
            lines[rule + 1]
        );
        assert!(
            lines.iter().any(|l| l.contains("# principles")),
            "{lines:#?}"
        );
        v.up();
        let lines = v.render(80, 10);
        let rule = lines.iter().position(|l| l.starts_with('─')).unwrap();
        assert!(
            lines[rule + 1..].iter().any(|l| l.contains("binary, 2.0K")),
            "{lines:#?}"
        );
    }

    #[test]
    fn files_view_goes_into_a_directory_and_back_to_it() {
        let t = fixture();
        let mut v = FilesView::load(t.path(), true, None);
        v.down();
        assert_eq!(v.selected().unwrap().name, "skills");
        v.enter();
        assert_eq!(v.dir, t.path().join("skills"));
        assert_eq!(v.entries.len(), 1);
        v.enter();
        assert!(v.dir.ends_with("skills/demo"));
        v.enter();
        assert!(v.dir.ends_with("skills/demo"), "a file is not entered");
        v.parent();
        assert_eq!(v.selected().unwrap().name, "demo");
        v.parent();
        assert_eq!(v.dir, t.path());
        assert_eq!(v.selected().unwrap().name, "skills");
        let target = v.entries.iter().position(|n| n.name == "target").unwrap();
        v.selected = target;
        v.enter();
        v.enter();
        assert!(v.entries.is_empty());
        let lines = v.render(80, 6);
        assert_eq!(lines.len(), 6);
        assert!(lines[2].starts_with("  (empty)"), "{}", lines[2]);
    }

    #[test]
    fn preview_cuts_long_files_and_files_render_has_no_side_pane() {
        let t = fixture();
        let p = t.path().join("long.txt");
        std::fs::write(&p, "a\tb\n".repeat(20)).unwrap();
        let n = Node::load(&p, 0, false, &[]);
        let out = preview(&n, false, 10, 5);
        assert_eq!(out.len(), 5);
        assert_eq!(out[0], "a    b");
        assert_eq!(out[4], "… 16 more…");
        let out = preview(&n, false, 40, 30);
        assert_eq!(out.len(), 20);
        let empty = Node::load(&t.path().join("target/debug"), 1, true, &[]);
        assert_eq!(preview(&empty, true, 20, 3), ["(empty)"]);
        let v = FilesView::load(t.path(), false, None);
        let lines = v.render(80, 10);
        assert!(lines.iter().all(|l| !l.contains(" │ ")));
        assert!(lines.iter().any(|l| l.starts_with('─')));
    }

    #[test]
    fn browse_enters_a_directory_from_the_tree_and_bksp_returns() {
        let t = fixture();
        let home = Home {
            dir: t.path().to_path_buf(),
        };
        let mut b = BrowseView::load(t.path(), false, Some(&home));
        let skills = b
            .tree
            .rows
            .iter()
            .position(|r| r.node.name == "skills")
            .unwrap();
        b.tree.selected = skills;
        b.enter(Some(&home));
        assert_eq!(b.selected().unwrap().name, "demo");
        let flat_header = b.render(80, 10)[0].clone();
        assert!(flat_header.contains("items"), "flat listing: {flat_header}");
        b.back();
        assert_eq!(b.tree.selected, skills);
        let tree_header = b.render(80, 10)[0].clone();
        assert!(!tree_header.contains("items"), "tree again: {tree_header}");
    }

    #[test]
    fn browse_tree_renders_within_width_at_80_and_120() {
        let t = fixture();
        let home = Home {
            dir: t.path().to_path_buf(),
        };
        let b = BrowseView::load(t.path(), false, Some(&home));
        for width in [80, 120] {
            let lines = b.render(width, 24);
            assert_eq!(lines.len(), 24, "{width}x24");
            assert!(lines.iter().all(|l| visible(l) <= width), "{lines:#?}");
        }
    }

    #[test]
    fn browse_flat_scoped_tree_and_esc_returns() {
        let t = fixture();
        let home = Home {
            dir: t.path().to_path_buf(),
        };
        let mut b = BrowseView::load(t.path(), false, Some(&home));
        let skills = b
            .tree
            .rows
            .iter()
            .position(|r| r.node.name == "skills")
            .unwrap();
        b.tree.selected = skills;
        b.enter(Some(&home));
        assert!(!b.in_scoped_tree());
        b.show_scoped_tree(Some(&home));
        assert!(b.in_scoped_tree());
        let tree_lines = b.render(80, 24);
        assert_eq!(tree_lines.len(), 24);
        assert!(tree_lines.iter().all(|l| visible(l) <= 80));
        assert!(
            tree_lines[0].contains("skills"),
            "scoped root: {}",
            tree_lines[0]
        );
        assert!(
            tree_lines.iter().any(|l| l.contains("demo")),
            "scoped tree shows subtree: {tree_lines:#?}"
        );
        b.dismiss_scoped_tree();
        assert!(!b.in_scoped_tree());
        let flat_lines = b.render(80, 24);
        assert!(
            flat_lines[0].contains("items"),
            "flat again: {}",
            flat_lines[0]
        );
        for width in [80, 120] {
            b.show_scoped_tree(Some(&home));
            let lines = b.render(width, 24);
            assert_eq!(lines.len(), 24, "{width}x24");
            assert!(lines.iter().all(|l| visible(l) <= width), "{lines:#?}");
            b.dismiss_scoped_tree();
        }
    }

    #[test]
    fn t156_a_refresh_keeps_an_open_scoped_tree_and_its_rows() {
        let t = fixture();
        let home = Home {
            dir: t.path().to_path_buf(),
        };
        let mut b = BrowseView::load(t.path(), false, Some(&home));
        b.tree.selected = b
            .tree
            .rows
            .iter()
            .position(|r| r.node.name == "skills")
            .unwrap();
        b.enter(Some(&home));
        b.show_scoped_tree(Some(&home));
        b.down();
        let (root, row) = (b.watch_paths(), b.selected().map(|n| n.path.clone()));
        b.restore(t.path(), false, Some(&home), b.snapshot());
        assert!(b.in_scoped_tree(), "{:#?}", b.render(80, 24));
        assert_eq!(b.watch_paths(), root);
        assert_eq!(b.selected().map(|n| n.path.clone()), row);
        b.dismiss_scoped_tree();
        assert_eq!(b.selected().unwrap().name, "demo");
    }

    #[test]
    fn browse_tree_at_home_uses_depth_two() {
        let t = fixture();
        let home = Home {
            dir: t.path().to_path_buf(),
        };
        let b = BrowseView::load(t.path(), false, Some(&home));
        assert!(
            !b.tree.rows.iter().any(|r| r.text.contains("SKILL.md")),
            "depth 2 does not load SKILL.md under skills/demo"
        );
        assert!(
            b.tree.rows.iter().any(|r| r.text.contains("demo/")),
            "depth 2 still shows demo/"
        );
    }

    #[test]
    fn t9_a_click_on_a_row_selects_that_entry() {
        let t = fixture();
        let mut v = FilesView::load(t.path(), false, None);
        let lines = v.render(80, 10);
        assert!(lines[4].starts_with("  AGENTS.md"), "{}", lines[4]);
        v.pick(4, 10);
        assert_eq!(v.selected().unwrap().name, "AGENTS.md");
        for outside in [0, 1, 7, 9, 40] {
            v.pick(outside, 10);
            assert_eq!(v.selected().unwrap().name, "AGENTS.md", "line {outside}");
        }
        v.pick(2, 10);
        assert_eq!(v.selected().unwrap().name, "rules");
    }
}
