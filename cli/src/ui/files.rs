//! The Files view: one directory as a file manager shows it. Columns for
//! the name, size, lines, and date, a preview of the selected entry, and
//! a way into a directory and back up. Nothing here writes.

use crate::home::{tilde, Home};
use crate::out::fit;
use crate::ui::tree::{children, date, pane_width, size, window, Kind, Node};
use std::io::Read;
use std::path::{Path, PathBuf};

/// The most of a file the preview reads.
const PREVIEW_BYTES: u64 = 64 * 1024;

/// The width the fixed columns take: the mark, size, lines, and date, with
/// two spaces before each.
const FIXED: usize = 2 + 2 + 5 + 2 + 5 + 2 + 10;

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
    /// totals, the column names, then the rows around the selection beside
    /// a preview of it.
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
        let left_w = pane_width(width);
        let name_w = left_w.saturating_sub(FIXED).max(8);
        let body_h = height.saturating_sub(2);
        let start = window(self.selected, body_h);
        let mut left = vec![format!(
            "  {:<name_w$}  {:>5}  {:>5}  {:<10}",
            "name", "size", "lines", "modified"
        )];
        if self.entries.is_empty() {
            left.push("  (empty)".to_string());
        }
        left.extend(
            self.entries
                .iter()
                .enumerate()
                .skip(start)
                .take(body_h)
                .map(|(i, n)| {
                    let mark = if i == self.selected { "> " } else { "  " };
                    let (bytes, lines) = match n.kind {
                        Kind::File => (size(n.bytes), n.lines.to_string()),
                        _ => ("-".to_string(), "-".to_string()),
                    };
                    // The name gives way before the mark, so a cut row
                    // still says what the file is.
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
        let right = match self.selected() {
            Some(n) if left_w < width => {
                let w = width.saturating_sub(left_w + 3);
                let mut r = vec![fit(&n.label(), w)];
                r.extend(preview(n, self.all, w, body_h));
                r
            }
            _ => Vec::new(),
        };
        out.extend(split(
            &left,
            &right,
            left_w,
            width,
            height.saturating_sub(1),
        ));
        out
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

/// Two panes side by side, `height` lines, each at most `width` wide. The
/// left pane takes `left_w`; the rest holds the right pane after a bar.
/// Without room for a right pane, the left pane takes the whole width.
pub fn split(
    left: &[String],
    right: &[String],
    left_w: usize,
    width: usize,
    height: usize,
) -> Vec<String> {
    let right_w = width.saturating_sub(left_w + 3);
    (0..height)
        .map(|i| {
            let l = left.get(i).map(String::as_str).unwrap_or("");
            if right_w == 0 {
                return fit(l, width).trim_end().to_string();
            }
            let r = right.get(i).map(String::as_str).unwrap_or("");
            format!("{:<left_w$} │ {}", fit(l, left_w), fit(r, right_w))
                .trim_end()
                .to_string()
        })
        .collect()
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
        assert!(lines[0].ends_with("  5 items, 2.0K"), "{}", lines[0]);
        assert_eq!(
            lines[1],
            format!("  {:<20}   size  lines  modified   │ rules/", "name")
        );
        assert!(lines[2].starts_with("> rules/"), "{}", lines[2]);
        assert!(lines[2].ends_with("│ boundaries.md"), "{}", lines[2]);
        let today = date(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        assert_eq!(
            lines[4],
            format!("  {:<20}      9      1  {today} │", "AGENTS.md")
        );
        assert!(
            lines[5].contains("blob.bin") && lines[5].contains("  2.0K      0  "),
            "{}",
            lines[5]
        );
        assert!(
            lines[6].starts_with("  principl…  always-on     35      2  "),
            "{}",
            lines[6]
        );
        for _ in 0..4 {
            v.down();
        }
        assert_eq!(v.selected().unwrap().name, "principles.md");
        let lines = v.render(80, 10);
        assert!(lines[1].ends_with("│ principles.md"), "{}", lines[1]);
        assert!(lines[2].ends_with("│ # principles"), "{}", lines[2]);
        assert!(
            lines[3].ends_with("│ 1. One idea per file."),
            "{}",
            lines[3]
        );
        v.up();
        let lines = v.render(80, 10);
        assert!(lines[2].ends_with("│ binary, 2.0K"), "{}", lines[2]);
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
    fn preview_cuts_long_files_and_split_never_passes_the_width() {
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
        let s = split(
            &["left".to_string(), "x".repeat(50)],
            &["right".to_string()],
            20,
            40,
            3,
        );
        assert_eq!(
            s,
            [
                "left                 │ right",
                "xxxxxxxxxxxxxxxxxxx… │",
                "                     │"
            ]
        );
        assert_eq!(
            split(&["abc".to_string()], &["r".to_string()], 30, 30, 1),
            ["abc"]
        );
    }
}
