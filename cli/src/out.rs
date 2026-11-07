//! How a listing prints: sections with a count, rows aligned by name, and
//! each row cut to the terminal width so it never wraps. A state is a word;
//! on a terminal it also gets a glyph and a colour. The text never carries a
//! fact that the words do not carry, so a pipe reads the same as a screen.
//! `--json` prints one document instead.

use crate::home::tilde;
use crate::reset::rfc3339;
use serde_json::{json, Value};
use std::io::IsTerminal;
use std::path::Path;

/// The widest the name column gets. A longer name is cut.
pub const NAME_CAP: usize = 28;

/// `COLUMNS`, then no limit when stdout is not a terminal, so a reader in a
/// pipe gets every row whole. Then the terminal, then 80.
pub fn width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .or_else(|| (!std::io::stdout().is_terminal()).then_some(usize::MAX))
        .or_else(|| terminal_size::terminal_size().map(|(w, _)| w.0 as usize))
        .filter(|&n| n > 0)
        .unwrap_or(80)
}

/// A state a line names. The word always prints. On a terminal with
/// `NO_COLOR` unset, a glyph and a colour go with it; a pipe gets the word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Ok,
    AlwaysOn,
    Identical,
    Different,
    Broken,
    Stale,
}

impl State {
    /// The word, the one part that prints everywhere.
    pub fn word(self) -> &'static str {
        match self {
            State::Ok => "ok",
            State::AlwaysOn => "always-on",
            State::Identical => "identical",
            State::Different => "different",
            State::Broken => "BROKEN",
            State::Stale => "STALE",
        }
    }

    /// The glyph before the word on a terminal.
    fn glyph(self) -> Option<&'static str> {
        match self {
            State::Ok => Some("✓"),
            State::AlwaysOn => None,
            State::Identical => Some("="),
            State::Different => Some("≠"),
            State::Broken => Some("✗"),
            State::Stale => Some("!"),
        }
    }

    /// The SGR parameter that colours the token on a terminal.
    fn colour(self) -> &'static str {
        match self {
            State::Ok => "32",
            State::AlwaysOn => "36",
            State::Identical => "2",
            State::Different => "33",
            State::Broken => "31",
            State::Stale => "33",
        }
    }
}

/// Whether tokens get glyphs and colour: stdout is a terminal and `NO_COLOR`
/// is unset or empty.
pub fn styled() -> bool {
    !std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) && std::io::stdout().is_terminal()
}

/// The token for `s`: the word alone, or the glyph and the word in colour.
pub fn state(s: State) -> String {
    render(s, styled())
}

fn render(s: State, styled: bool) -> String {
    if !styled {
        return s.word().to_string();
    }
    let glyph = s.glyph().map(|g| format!("{g} ")).unwrap_or_default();
    format!("\x1b[{}m{glyph}{}\x1b[0m", s.colour(), s.word())
}

/// The characters a terminal shows: every one outside an escape sequence.
pub fn visible(s: &str) -> usize {
    let mut n = 0;
    let mut esc = false;
    for c in s.chars() {
        if esc {
            esc = c != 'm';
        } else if c == '\x1b' {
            esc = true;
        } else {
            n += 1;
        }
    }
    n
}

/// `s` in at most `max` characters. A cut ends in one `…`.
pub fn fit(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

/// `fit`, but keep the end: a path is known by its last part.
pub fn fit_start(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut out = String::from("…");
    out.extend(s.chars().skip(n - (max - 1)));
    out
}

/// Words in lines of at most `width` characters, each line indented.
/// A word longer than a line gets a line of its own.
pub fn wrap(text: &str, indent: usize, width: usize) -> Vec<String> {
    let pad = " ".repeat(indent);
    let room = width.saturating_sub(indent).max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let len = line.chars().count();
        if len > 0 && len + 1 + word.chars().count() > room {
            lines.push(format!("{pad}{line}"));
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(format!("{pad}{line}"));
    }
    lines
}

/// Print `text` wrapped to the width, indented.
pub fn say(text: &str, indent: usize, width: usize) {
    for l in wrap(text, indent, width) {
        println!("{l}");
    }
}

/// One item in a listing: its name, a short fixed field such as a line
/// count, and free text that is cut at the terminal edge.
pub struct Row {
    pub name: String,
    pub meta: String,
    pub text: String,
}

impl Row {
    pub fn new(name: impl Into<String>, meta: impl Into<String>, text: impl Into<String>) -> Row {
        Row {
            name: name.into(),
            meta: meta.into(),
            text: text.into(),
        }
    }
}

/// Rows indented two spaces, the name column as wide as the widest name up
/// to `NAME_CAP`, and no line longer than `width`. An empty column takes no
/// space.
pub fn rows(rows: &[Row], width: usize) -> Vec<String> {
    let name_w = rows
        .iter()
        .map(|r| r.name.chars().count())
        .max()
        .unwrap_or(0)
        .min(NAME_CAP);
    let meta_w = rows.iter().map(|r| visible(&r.meta)).max().unwrap_or(0);
    rows.iter()
        .map(|r| {
            let mut line = String::from("  ");
            if name_w > 0 {
                line.push_str(&format!("{:<name_w$}  ", fit(&r.name, name_w)));
            }
            if meta_w > 0 {
                line.push_str(&r.meta);
                line.push_str(&" ".repeat(meta_w - visible(&r.meta) + 2));
            }
            let room = width.saturating_sub(visible(&line));
            line.push_str(&fit(&r.text, room));
            let line = line.trim_end();
            if visible(line) <= width {
                line.to_string()
            } else {
                fit(line, width)
            }
        })
        .collect()
}

/// A section header: a blank line, then the name and the count.
pub fn section(title: &str, count: usize, width: usize) {
    let tail = format!(" ({count}):");
    println!();
    println!(
        "{}{tail}",
        fit_start(title, width.saturating_sub(tail.chars().count()))
    );
}

/// The rows of a section. With `long`, each item is a block: the name, then
/// the meta and every paragraph of the text in full, wrapped.
pub fn print(items: &[Row], long: bool, width: usize) {
    if !long {
        for l in rows(items, width) {
            println!("{l}");
        }
        return;
    }
    for (i, r) in items.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("  {}", r.name);
        for part in std::iter::once(r.meta.trim()).chain(r.text.lines()) {
            if !part.trim().is_empty() {
                say(part, 4, width);
            }
        }
    }
}

/// The next steps, last in the output: a blank line, each label and its
/// command aligned, then the notes.
pub fn steps(pairs: &[(&str, &str)], notes: &[&str], width: usize) {
    let w = pairs.iter().map(|(l, _)| l.len() + 1).max().unwrap_or(0);
    println!();
    for (label, cmd) in pairs {
        println!("{:<w$} {cmd}", format!("{label}:"));
    }
    for n in notes {
        say(n, 0, width);
    }
}

/// The one JSON document a `--json` listing prints: the envelope every
/// command shares, then the keys of `data`, which must be an object.
pub fn document(command: &str, home: Option<&Path>, data: Value) -> Value {
    let mut doc = json!({
        "agents": env!("CARGO_PKG_VERSION"),
        "build": crate::version::json(),
        "command": command,
        "generated": rfc3339(),
        "home": home.map(tilde),
    });
    if let (Some(d), Value::Object(m)) = (doc.as_object_mut(), data) {
        d.extend(m);
    }
    doc
}

/// Print `document` on stdout and nothing else.
pub fn json(command: &str, home: Option<&Path>, data: Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(&document(command, home, data)).unwrap_or_default()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_has_the_envelope_then_the_data() {
        let d = document("skills", None, json!({"skills": []}));
        let mut keys: Vec<&str> = d.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(
            keys,
            ["agents", "build", "command", "generated", "home", "skills"]
        );
        assert_eq!(d["build"]["commit"], env!("AGENTS_COMMIT"));
        assert_eq!(d["command"], "skills");
        assert_eq!(d["agents"], env!("CARGO_PKG_VERSION"));
        assert_eq!(d["home"], Value::Null);
        let g = d["generated"].as_str().unwrap();
        assert!(
            g.len() == 20 && g.ends_with('Z') && &g[10..11] == "T",
            "{g}"
        );
    }

    #[test]
    fn fit_cuts_with_one_ellipsis() {
        assert_eq!(fit("abcdef", 6), "abcdef");
        assert_eq!(fit("abcdef", 4), "abc…");
        assert_eq!(fit("abcdef", 1), "…");
        assert_eq!(fit("abcdef", 0), "");
        assert_eq!(fit("ééé", 2), "é…");
        assert_eq!(fit_start("a/b/c.md", 5), "…c.md");
    }

    #[test]
    fn rows_align_and_never_pass_the_width() {
        let long_name = "n".repeat(40);
        let r = rows(
            &[
                Row::new("ab", " 12 lines", "short"),
                Row::new("abcd", "  3 lines", "x".repeat(200)),
                Row::new(long_name, "  1 lines", ""),
            ],
            60,
        );
        assert_eq!(r[0], format!("  ab{}   12 lines  short", " ".repeat(26)));
        let text_col = r[0].find("short").unwrap();
        assert_eq!(r[1].char_indices().nth(text_col).unwrap().1, 'x');
        assert!(r.iter().all(|l| l.chars().count() <= 60));
        assert!(r[1].ends_with('…'));
        assert!(r[2].starts_with(&format!("  {}…", "n".repeat(NAME_CAP - 1))));
        assert!(!r[2].ends_with(' '));
    }

    #[test]
    fn empty_columns_take_no_space() {
        let r = rows(&[Row::new("", "  5 lines", "CLAUDE.md")], 80);
        assert_eq!(r[0], "    5 lines  CLAUDE.md");
    }

    #[test]
    fn o3_a_state_is_its_word_alone_without_a_styled_terminal() {
        for s in [
            State::Ok,
            State::AlwaysOn,
            State::Identical,
            State::Different,
            State::Broken,
            State::Stale,
        ] {
            let plain = render(s, false);
            assert_eq!(plain, s.word());
            assert!(!plain.contains('\x1b'));
            let styled = render(s, true);
            assert!(
                styled.starts_with("\x1b[") && styled.ends_with("\x1b[0m"),
                "{styled:?}"
            );
            assert!(styled.contains(s.word()), "{styled:?}");
        }
        assert_eq!(render(State::Broken, true), "\x1b[31m✗ BROKEN\x1b[0m");
        assert_eq!(
            visible(&render(State::Broken, true)),
            "✗ BROKEN".chars().count()
        );
        std::env::set_var("NO_COLOR", "1");
        assert!(!styled());
        assert_eq!(state(State::Stale), "STALE");
    }

    #[test]
    fn rows_align_on_what_a_terminal_shows() {
        let r = rows(
            &[
                Row::new("a", render(State::Broken, true), "x"),
                Row::new("b", "plain", "y"),
            ],
            80,
        );
        assert_eq!(visible(&r[0]), visible(&r[1]));
        assert_eq!(r[0].find('x').map(|i| visible(&r[0][..i])), r[1].find('y'));
    }

    #[test]
    fn wrap_keeps_every_word() {
        let w = wrap("one two three four", 2, 11);
        assert_eq!(w, ["  one two", "  three", "  four"]);
    }
}
