//! `agents ui`: one full-screen view of the reports the other commands
//! print. Each view reads the same object that `--json` prints, so the view
//! and the report cannot disagree. Raw mode comes from `rustix` termios and
//! the drawing is ANSI written here, so no crate is added (decision 0011).
//! The view writes no file.

/// On Windows the view does not compile: `rustix` termios is Unix only.
#[cfg(not(unix))]
pub fn ui(
    _home: Option<std::path::PathBuf>,
    _cfg: &crate::config::Config,
) -> anyhow::Result<std::process::ExitCode> {
    eprintln!("agents ui is Unix only. Every other command works here.");
    Ok(std::process::ExitCode::FAILURE)
}

#[cfg(unix)]
pub use unix::ui;

#[cfg(unix)]
mod unix {
    use crate::config::Config;
    use crate::home::Home;
    use crate::out::{self, State};
    use crate::status;
    use anyhow::Result;
    use rustix::termios::{self, OptionalActions, SpecialCodeIndex, Termios};
    use serde_json::Value;
    use std::io::{self, Read, Write};
    use std::panic::{self, AssertUnwindSafe};
    use std::path::PathBuf;
    use std::process::ExitCode;
    use std::sync::{Mutex, PoisonError};

    /// The alternate screen, with the cursor hidden.
    const ENTER: &str = "\x1b[?1049h\x1b[?25l";
    /// The undo of `ENTER`: show the cursor, then the main screen again.
    const LEAVE: &str = "\x1b[?25h\x1b[?1049l";
    /// Reverse video, and the end of every attribute.
    const INVERT: &str = "\x1b[7m";
    const RESET: &str = "\x1b[0m";

    /// Open the view, or print one line and exit 1 when there is no terminal.
    pub fn ui(home: Option<PathBuf>, cfg: &Config) -> Result<ExitCode> {
        if !termios::isatty(io::stdin()) || !termios::isatty(io::stdout()) {
            eprintln!("agents ui needs a terminal on stdin and stdout.");
            return Ok(ExitCode::FAILURE);
        }
        let home = Home::locate(home)?;
        let mut tty = Tty;
        guarded(&mut tty, |_| run(&home, cfg))??;
        Ok(ExitCode::SUCCESS)
    }

    /// What the view takes from the terminal and gives back.
    pub(super) trait Term {
        fn enter(&mut self) -> io::Result<()>;
        fn leave(&mut self);
    }

    /// `f` between `enter` and `leave`. `leave` runs when `f` returns and
    /// when it panics; the panic then goes on.
    pub(super) fn guarded<T: Term, R>(term: &mut T, f: impl FnOnce(&mut T) -> R) -> io::Result<R> {
        term.enter()?;
        let result = panic::catch_unwind(AssertUnwindSafe(|| f(term)));
        term.leave();
        match result {
            Ok(v) => Ok(v),
            Err(p) => panic::resume_unwind(p),
        }
    }

    /// The settings the terminal had before `enter`. `None` once restored.
    static SAVED: Mutex<Option<Termios>> = Mutex::new(None);

    /// The real terminal: raw mode on stdin, the alternate screen on stdout.
    struct Tty;

    impl Tty {
        /// Put the terminal back once. A second call does nothing, so the
        /// panic hook and `leave` can both call it.
        fn restore() {
            let saved = SAVED.lock().unwrap_or_else(PoisonError::into_inner).take();
            let Some(saved) = saved else { return };
            let mut out = io::stdout().lock();
            let _ = out.write_all(LEAVE.as_bytes());
            let _ = out.flush();
            let _ = termios::tcsetattr(io::stdin(), OptionalActions::Flush, &saved);
        }
    }

    impl Term for Tty {
        fn enter(&mut self) -> io::Result<()> {
            let stdin = io::stdin();
            let saved = termios::tcgetattr(&stdin)?;
            let mut raw = saved.clone();
            raw.make_raw();
            // A read returns empty after 100 ms, so the loop sees a resize
            // and a lone Esc without a signal handler or a second thread.
            raw.special_codes[SpecialCodeIndex::VMIN] = 0;
            raw.special_codes[SpecialCodeIndex::VTIME] = 1;
            *SAVED.lock().unwrap_or_else(PoisonError::into_inner) = Some(saved);
            termios::tcsetattr(&stdin, OptionalActions::Flush, &raw)?;
            // Restore before the panic message prints, so it lands on the
            // main screen with line discipline on.
            let previous = panic::take_hook();
            panic::set_hook(Box::new(move |info| {
                Tty::restore();
                previous(info);
            }));
            let mut out = io::stdout().lock();
            out.write_all(ENTER.as_bytes())?;
            out.flush()
        }

        fn leave(&mut self) {
            Tty::restore();
        }
    }

    /// The loop: draw, wait for a key, apply it, until `q`.
    fn run(home: &Home, cfg: &Config) -> Result<()> {
        let mut frame = Frame::default();
        let mut body = frame.view.lines(home, cfg);
        let mut stdin = io::stdin().lock();
        let mut out = io::stdout().lock();
        loop {
            let (width, height) = size();
            let lines = screen(&frame, &body, width, height);
            draw(&mut out, &lines)?;
            let mut buf = [0u8; 16];
            let n = stdin.read(&mut buf)?;
            let Some(key) = key(&buf[..n]) else { continue };
            let before = frame.view;
            if !frame.press(key, body.len(), height.saturating_sub(2)) {
                return Ok(());
            }
            if frame.view != before {
                body = frame.view.lines(home, cfg);
            }
        }
    }

    /// Columns and rows, from the terminal, else 80 by 24.
    fn size() -> (usize, usize) {
        termios::tcgetwinsize(io::stdout())
            .ok()
            .filter(|w| w.ws_col > 0 && w.ws_row > 0)
            .map(|w| (w.ws_col as usize, w.ws_row as usize))
            .unwrap_or((80, 24))
    }

    /// Write every line from the top, each cleared to its end.
    fn draw(out: &mut impl Write, lines: &[String]) -> io::Result<()> {
        let mut text = String::from("\x1b[H");
        for (i, l) in lines.iter().enumerate() {
            if i > 0 {
                text.push_str("\r\n");
            }
            text.push_str(l);
            text.push_str("\x1b[K");
        }
        out.write_all(text.as_bytes())?;
        out.flush()
    }

    /// One key, after decoding. Arrow and page keys arrive as CSI sequences.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) enum Key {
        Quit,
        Help,
        Next,
        Prev,
        View(usize),
        Down,
        Up,
        PageDown,
        PageUp,
        Top,
        Bottom,
        Esc,
        Other,
    }

    /// The key the bytes of one read name; `None` for an empty read.
    pub(super) fn key(bytes: &[u8]) -> Option<Key> {
        Some(match bytes {
            b"" => return None,
            b"q" | b"\x03" => Key::Quit,
            b"?" => Key::Help,
            b"\t" | b"l" | b"\x1b[C" => Key::Next,
            b"\x1b[Z" | b"h" | b"\x1b[D" => Key::Prev,
            [d @ b'1'..=b'9'] => Key::View((d - b'1') as usize),
            b"j" | b"\x1b[B" => Key::Down,
            b"k" | b"\x1b[A" => Key::Up,
            b" " | b"\x06" | b"\x1b[6~" => Key::PageDown,
            b"b" | b"\x02" | b"\x1b[5~" => Key::PageUp,
            b"g" | b"\x1b[H" => Key::Top,
            b"G" | b"\x1b[F" => Key::Bottom,
            b"\x1b" => Key::Esc,
            _ => Key::Other,
        })
    }

    /// A tab. Each one reads the object its command prints with `--json`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) enum View {
        Status,
    }

    impl View {
        pub(super) const ALL: [View; 1] = [View::Status];

        pub(super) fn name(self) -> &'static str {
            match self {
                View::Status => "Status",
            }
        }

        /// The body, read fresh each time the view is opened.
        fn lines(self, home: &Home, cfg: &Config) -> Vec<String> {
            match self {
                View::Status => status_lines(&out::document(
                    "status",
                    Some(&home.dir),
                    status::data(home, cfg),
                )),
            }
        }
    }

    /// Which tab shows, whether help covers it, and the first body row.
    #[derive(Debug)]
    pub(super) struct Frame {
        pub(super) view: View,
        pub(super) help: bool,
        pub(super) top: usize,
    }

    impl Default for Frame {
        fn default() -> Frame {
            Frame {
                view: View::Status,
                help: false,
                top: 0,
            }
        }
    }

    impl Frame {
        /// Apply one key. `false` means quit. `rows` is the body height.
        pub(super) fn press(&mut self, key: Key, len: usize, rows: usize) -> bool {
            let last = len.saturating_sub(rows);
            let i = View::ALL.iter().position(|v| *v == self.view).unwrap_or(0);
            match key {
                Key::Quit => return false,
                Key::Help => self.help = !self.help,
                Key::Esc => self.help = false,
                Key::Next => self.switch(View::ALL[(i + 1) % View::ALL.len()]),
                Key::Prev => self.switch(View::ALL[(i + View::ALL.len() - 1) % View::ALL.len()]),
                Key::View(n) => {
                    if let Some(v) = View::ALL.get(n) {
                        self.switch(*v);
                    }
                }
                Key::Down => self.top = (self.top + 1).min(last),
                Key::Up => self.top = self.top.saturating_sub(1),
                Key::PageDown => self.top = (self.top + rows).min(last),
                Key::PageUp => self.top = self.top.saturating_sub(rows),
                Key::Top => self.top = 0,
                Key::Bottom => self.top = last,
                Key::Other => {}
            }
            true
        }

        fn switch(&mut self, view: View) {
            if self.view != view {
                self.view = view;
                self.top = 0;
            }
        }
    }

    /// Every row of one screen: the tab bar, the body or the help, and the
    /// key bar. Exactly `height` rows, none wider than `width`.
    pub(super) fn screen(
        frame: &Frame,
        body: &[String],
        width: usize,
        height: usize,
    ) -> Vec<String> {
        let mut rows = Vec::with_capacity(height);
        if height == 0 {
            return rows;
        }
        rows.push(tab_bar(frame.view, width));
        if height > 2 {
            let room = height - 2;
            let help = HELP.iter().map(|l| l.to_string()).collect::<Vec<_>>();
            let (lines, top) = if frame.help {
                (&help[..], 0)
            } else {
                (body, frame.top.min(body.len().saturating_sub(room)))
            };
            for i in 0..room {
                rows.push(
                    lines
                        .get(top + i)
                        .map(|l| cut(l, width))
                        .unwrap_or_default(),
                );
            }
        }
        if height > 1 {
            rows.push(key_bar(frame.help, width));
        }
        rows
    }

    /// Each tab by number and name, the open one in reverse video, then the
    /// version at the right edge when it fits.
    fn tab_bar(open: View, width: usize) -> String {
        let mut bar = String::new();
        for (i, v) in View::ALL.iter().enumerate() {
            let tab = format!(" {} {} ", i + 1, v.name());
            if *v == open {
                bar.push_str(&format!("{INVERT}{tab}{RESET}"));
            } else {
                bar.push_str(&tab);
            }
        }
        let title = format!("agents {}", crate::version::VERSION);
        let used = out::visible(&bar);
        if used + 2 + title.chars().count() <= width {
            bar.push_str(&" ".repeat(width - used - title.chars().count()));
            bar.push_str(&title);
        }
        cut(&bar, width)
    }

    /// The keys a person needs next, each key in reverse video.
    fn key_bar(help: bool, width: usize) -> String {
        let keys: &[(&str, &str)] = if help {
            &[("esc", "close"), ("q", "quit")]
        } else {
            &[
                ("?", "help"),
                ("j/k", "scroll"),
                ("tab", "view"),
                ("q", "quit"),
            ]
        };
        let bar = keys
            .iter()
            .map(|(k, what)| format!("{INVERT} {k} {RESET} {what}"))
            .collect::<Vec<_>>()
            .join("  ");
        cut(&bar, width)
    }

    const HELP: [&str; 14] = [
        "Keys",
        "",
        "  ?            this help; again to close",
        "  q, ctrl-c    quit",
        "  tab, l, →    next view        shift-tab, h, ←   previous view",
        "  1 .. 9       the view by number",
        "  j, k, ↓, ↑   one line down, up",
        "  space, b     one page down, up",
        "  g, G         top, bottom",
        "  esc          close this help",
        "",
        "Each view reads what its command prints with --json, so the view and",
        "the report say the same thing. The view writes no file.",
        "",
    ];

    /// `s` in at most `width` visible characters. Escape sequences take no
    /// width and a cut ends in `…` and a reset, so colour never leaks.
    pub(super) fn cut(s: &str, width: usize) -> String {
        if out::visible(s) <= width {
            return s.to_string();
        }
        if width == 0 {
            return String::new();
        }
        let mut text = String::new();
        let mut shown = 0;
        let mut esc = false;
        for c in s.chars() {
            if esc {
                text.push(c);
                esc = c != 'm';
            } else if c == '\x1b' {
                text.push(c);
                esc = true;
            } else if shown + 1 < width {
                text.push(c);
                shown += 1;
            } else {
                break;
            }
        }
        text.push('…');
        text.push_str(RESET);
        text
    }

    fn s<'a>(v: &'a Value, key: &str) -> &'a str {
        v[key].as_str().unwrap_or_default()
    }

    fn n(v: &Value, key: &str) -> u64 {
        v[key].as_u64().unwrap_or(0)
    }

    /// The Status view: every fact of `agents status --json`, with the
    /// detail `--long` adds under each tool.
    pub(super) fn status_lines(doc: &Value) -> Vec<String> {
        let mut lines = Vec::new();
        let config = &doc["config"];
        let file = if config["exists"].as_bool() == Some(true) {
            s(config, "path").to_string()
        } else {
            format!("{} (no file, so defaults)", s(config, "path"))
        };
        lines.push(format!("machine: {}, config: {file}", s(doc, "machine")));
        let mut home = format!("home: {}", s(doc, "home"));
        match &doc["git"] {
            Value::Null => home.push_str(", NOT a git checkout"),
            git => {
                home.push_str(&format!(
                    ", {} at {}, {} uncommitted",
                    s(git, "branch"),
                    s(git, "sha"),
                    n(git, "uncommitted")
                ));
                if let Some(b) = git["behind_upstream"].as_u64() {
                    home.push_str(&format!(", behind upstream by {b} commits"));
                }
            }
        }
        lines.push(home);
        if doc["merge_ours_unset"].as_bool() == Some(true) {
            lines.push(format!(
                "merge.ours.driver is not set. Run: git -C {} config merge.ours.driver true",
                s(doc, "home")
            ));
        }
        lines.push(String::new());

        let always = &doc["always_on"];
        let (total, cap) = (n(always, "bytes"), n(always, "cap"));
        let over = if total > cap { ", OVER the cap" } else { "" };
        lines.push(format!(
            "{}: {total} of {cap} bytes ({}){over}",
            out::state(State::AlwaysOn),
            s(always, "cap_source")
        ));
        for f in always["files"].as_array().into_iter().flatten() {
            lines.push(format!("  {:>7} bytes  {}", n(f, "bytes"), s(f, "name")));
        }
        let on_demand: Vec<String> = doc["on_demand"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|d| format!("{} {}", s(d, "dir"), n(d, "files")))
            .collect();
        if !on_demand.is_empty() {
            lines.push(format!("on demand: {}", on_demand.join(", ")));
        }
        lines.push(String::new());

        for h in doc["harnesses"].as_array().into_iter().flatten() {
            lines.push(format!("{}: {}", s(h, "label"), harness_state(h)));
            let link = match h["link"].as_str() {
                Some(t) => format!(", a link to {t}"),
                None => String::new(),
            };
            lines.push(format!("  {}{link}", s(h, "path")));
            if h["exists"].as_bool() == Some(true) {
                let imports = &h["imports"];
                let broken = imports["broken"].as_array().map(Vec::len).unwrap_or(0);
                lines.push(format!(
                    "  {} lines, {} bytes, imports: {} home, {} elsewhere, {broken} broken",
                    n(h, "lines"),
                    n(h, "bytes"),
                    n(imports, "home"),
                    n(imports, "elsewhere")
                ));
                for b in imports["broken"].as_array().into_iter().flatten() {
                    lines.push(format!(
                        "    {} {}",
                        out::state(State::Broken),
                        b.as_str().unwrap_or_default()
                    ));
                }
                if let Some(c) = h["over_cap"].as_object() {
                    lines.push(format!(
                        "  {} bytes passes the cap of {}",
                        c["bytes"].as_u64().unwrap_or(0),
                        c["cap"].as_u64().unwrap_or(0)
                    ));
                }
                if let Some(own) = h["own_rule_lines"].as_u64() {
                    lines.push(format!(
                        "  {own} rule lines in the file itself, a second source"
                    ));
                }
                for g in h["generator_lines"].as_array().into_iter().flatten() {
                    lines.push(format!("  generator: {}", g.as_str().unwrap_or_default()));
                }
            }
            lines.push(String::new());
        }
        if let Some(m) = doc["manual"].as_object() {
            lines.push(format!(
                "{}: paste the pointer line by hand:",
                m["label"].as_str().unwrap_or_default()
            ));
            lines.push(format!("  {}", m["line"].as_str().unwrap_or_default()));
            lines.push(String::new());
        }

        if let Some(hooks) = doc["hooks"].as_object() {
            let settings = hooks["settings"].as_str().unwrap_or_default();
            if hooks["valid_json"].as_bool() == Some(true) {
                let missing = hooks["missing_paths"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                lines.push(format!(
                    "hooks: {settings}, {} name a missing path",
                    missing.len()
                ));
                for p in &missing {
                    lines.push(format!(
                        "  {} {}",
                        out::state(State::Broken),
                        p.as_str().unwrap_or_default()
                    ));
                }
            } else {
                lines.push(format!(
                    "hooks: {settings} is not valid JSON. The harness ignores it."
                ));
            }
        }
        let links = doc["links"].as_array().cloned().unwrap_or_default();
        let broken = links
            .iter()
            .filter(|l| l["broken"].as_bool() == Some(true))
            .count();
        if links.is_empty() {
            lines.push("links: none".to_string());
        } else {
            lines.push(format!("links: {}, {broken} broken", links.len()));
        }
        for l in &links {
            let state = if l["broken"].as_bool() == Some(true) {
                out::state(State::Broken)
            } else {
                out::state(State::Ok)
            };
            lines.push(format!("  {state} {} -> {}", s(l, "path"), s(l, "target")));
        }
        lines
    }

    /// The state the brief prints for one harness file, from its object.
    fn harness_state(h: &Value) -> String {
        let wire = "Run: agents wire --apply";
        if h["exists"].as_bool() != Some(true) {
            format!("not wired. {wire}")
        } else if h["ours"].as_bool() == Some(false) {
            "the file there is not the home's".to_string()
        } else if h["stale"].as_bool() == Some(true) {
            format!("{}. {wire}", out::state(State::Stale))
        } else {
            out::state(State::Ok)
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::unix::*;
    use serde_json::json;
    use std::io;

    /// The words alone: a token's colour and glyph depend on the terminal
    /// the test runs on, and the facts are in the words.
    fn words(s: &str) -> String {
        let mut text = String::new();
        let mut esc = false;
        for c in s.chars() {
            if esc {
                esc = c != 'm';
            } else if c == '\x1b' {
                esc = true;
            } else {
                text.push(c);
            }
        }
        ["✓ ", "✗ ", "! ", "= ", "≠ "]
            .iter()
            .fold(text, |t, g| t.replace(g, ""))
    }

    /// A terminal that records what it was told.
    #[derive(Default)]
    struct Fake {
        raw: bool,
        entered: usize,
        left: usize,
    }

    impl Term for Fake {
        fn enter(&mut self) -> io::Result<()> {
            self.raw = true;
            self.entered += 1;
            Ok(())
        }
        fn leave(&mut self) {
            self.raw = false;
            self.left += 1;
        }
    }

    #[test]
    fn t2_restore_runs_on_exit() {
        let mut fake = Fake::default();
        let got = guarded(&mut fake, |t| {
            assert!(t.raw, "the view runs in raw mode");
            7
        })
        .unwrap();
        assert_eq!(got, 7);
        assert!(!fake.raw);
        assert_eq!((fake.entered, fake.left), (1, 1));
    }

    #[test]
    fn t2_restore_runs_on_panic_and_the_panic_goes_on() {
        let mut fake = Fake::default();
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            guarded(&mut fake, |_| {
                panic!("a view bug");
            })
        }));
        std::panic::set_hook(hook);
        let payload = result.expect_err("the panic must go on after the restore");
        assert_eq!(payload.downcast_ref::<&str>(), Some(&"a view bug"));
        assert!(!fake.raw, "raw mode must end when the view panics");
        assert_eq!((fake.entered, fake.left), (1, 1));
    }

    #[test]
    fn t2_q_quits_and_question_mark_toggles_help() {
        let mut f = Frame::default();
        assert!(f.press(Key::Help, 10, 5));
        assert!(f.help);
        assert!(f.press(Key::Esc, 10, 5));
        assert!(!f.help);
        assert!(f.press(Key::Help, 10, 5) && f.help);
        assert!(f.press(Key::Help, 10, 5) && !f.help);
        assert!(!f.press(Key::Quit, 10, 5));
        assert_eq!(key(b"q"), Some(Key::Quit));
        assert_eq!(key(b"\x03"), Some(Key::Quit));
        assert_eq!(key(b"?"), Some(Key::Help));
        assert_eq!(key(b""), None);
    }

    #[test]
    fn t2_tabs_cycle_and_scroll_stays_in_the_body() {
        let mut f = Frame::default();
        for k in [Key::Next, Key::Prev, Key::View(0), Key::View(8)] {
            assert!(f.press(k, 10, 5));
            assert_eq!(f.view, View::Status);
        }
        assert_eq!(key(b"\t"), Some(Key::Next));
        assert_eq!(key(b"\x1b[Z"), Some(Key::Prev));
        assert_eq!(key(b"3"), Some(Key::View(2)));
        assert_eq!(key(b"\x1b[B"), Some(Key::Down));
        assert_eq!(key(b"\x1b[A"), Some(Key::Up));
        assert_eq!(key(b"\x1b"), Some(Key::Esc));
        assert_eq!(key(b"z"), Some(Key::Other));
        // 10 body lines in 4 rows: the top row can reach 6 and no further.
        f.press(Key::Bottom, 10, 4);
        assert_eq!(f.top, 6);
        f.press(Key::Down, 10, 4);
        assert_eq!(f.top, 6);
        f.press(Key::PageUp, 10, 4);
        assert_eq!(f.top, 2);
        f.press(Key::Up, 10, 4);
        f.press(Key::Up, 10, 4);
        f.press(Key::Up, 10, 4);
        assert_eq!(f.top, 0);
        f.press(Key::PageDown, 3, 4);
        assert_eq!(f.top, 0, "a body that fits does not scroll");
    }

    #[test]
    fn t2_screen_is_tab_bar_body_key_bar_within_the_size() {
        let body: Vec<String> = (0..30)
            .map(|i| format!("line {i} {}", "x".repeat(50)))
            .collect();
        let mut f = Frame {
            view: View::Status,
            help: false,
            top: 3,
        };
        let rows = screen(&f, &body, 50, 10);
        assert_eq!(rows.len(), 10);
        assert!(rows[0].contains(" 1 Status "), "{}", rows[0]);
        assert!(rows[0].contains("\x1b[7m"), "the open tab is inverted");
        assert!(rows[1].starts_with("line 3 "), "{}", rows[1]);
        assert!(
            rows[1].ends_with("…\x1b[0m"),
            "a long line is cut: {}",
            rows[1]
        );
        assert!(rows[9].contains("? \x1b[0m help") && rows[9].contains("q \x1b[0m quit"));
        assert!(
            rows.iter().all(|r| crate::out::visible(r) <= 50),
            "{rows:#?}"
        );
        let narrow = screen(&f, &body, 20, 10);
        assert!(
            narrow.iter().all(|r| crate::out::visible(r) <= 20),
            "{narrow:#?}"
        );
        f.help = true;
        let rows = screen(&f, &body, 80, 24);
        assert_eq!(rows[1], "Keys");
        assert!(rows[23].contains("esc"));
        assert_eq!(screen(&f, &body, 80, 1).len(), 1);
        assert_eq!(screen(&f, &body, 80, 0).len(), 0);
    }

    #[test]
    fn t2_cut_keeps_escapes_and_ends_in_a_reset() {
        assert_eq!(cut("abc", 3), "abc");
        assert_eq!(cut("abcdef", 4), "abc…\x1b[0m");
        assert_eq!(cut("\x1b[32mabcdef\x1b[0m", 4), "\x1b[32mabc…\x1b[0m");
        assert_eq!(cut("abc", 0), "");
    }

    #[test]
    fn t2_status_view_holds_the_facts_of_the_json() {
        let doc = json!({
            "home": "~/.agents",
            "machine": "box",
            "config": {"path": "~/.config/agents/config.toml", "exists": false},
            "git": {"branch": "main", "sha": "abc1234", "uncommitted": 2, "behind_upstream": 3},
            "merge_ours_unset": true,
            "always_on": {
                "files": [{"name": "AGENTS.md", "bytes": 100}, {"name": "principles.md", "bytes": 200}],
                "bytes": 300, "cap": 250, "cap_source": "default cap"
            },
            "on_demand": [{"dir": "skills/", "files": 4}],
            "harnesses": [
                {"label": "claude code, the home's drop-in", "path": "~/.claude/rules/x.md", "exists": true,
                 "link": null, "lines": 3, "bytes": 40, "imports": {"home": 3, "elsewhere": 0, "broken": ["@~/gone.md"]},
                 "over_cap": null, "stale": true, "ours": true, "own_rule_lines": 2, "generator_lines": []},
                {"label": "codex, the home's generated file", "path": "~/.codex/AGENTS.md", "exists": false,
                 "link": null, "lines": 0, "bytes": 0, "imports": {"home": 0, "elsewhere": 0, "broken": []},
                 "over_cap": null, "stale": false, "ours": true, "own_rule_lines": null, "generator_lines": []},
                {"label": "grok, the home's generated file", "path": "~/.grok/rules/x.md", "exists": true,
                 "link": null, "lines": 9, "bytes": 90, "imports": {"home": 0, "elsewhere": 0, "broken": []},
                 "over_cap": {"bytes": 90, "cap": 50}, "stale": false, "ours": false, "own_rule_lines": null, "generator_lines": []}
            ],
            "manual": {"label": "cursor", "line": "@~/.agents/AGENTS.md"},
            "hooks": {"settings": "~/.claude/settings.json", "valid_json": true, "missing_paths": ["~/bin/gone"]},
            "links": [{"path": "~/.claude/skills", "target": "~/.agents/skills", "broken": true}]
        });
        let text = words(&status_lines(&doc).join("\n"));
        for want in [
            "machine: box, config: ~/.config/agents/config.toml (no file, so defaults)",
            "home: ~/.agents, main at abc1234, 2 uncommitted, behind upstream by 3 commits",
            "merge.ours.driver is not set",
            "always-on: 300 of 250 bytes (default cap), OVER the cap",
            "    100 bytes  AGENTS.md",
            "on demand: skills/ 4",
            "claude code, the home's drop-in: STALE. Run: agents wire --apply",
            "  3 lines, 40 bytes, imports: 3 home, 0 elsewhere, 1 broken",
            "    BROKEN @~/gone.md",
            "  2 rule lines in the file itself",
            "codex, the home's generated file: not wired. Run: agents wire --apply",
            "grok, the home's generated file: the file there is not the home's",
            "  90 bytes passes the cap of 50",
            "cursor: paste the pointer line by hand:",
            "hooks: ~/.claude/settings.json, 1 name a missing path",
            "  BROKEN ~/bin/gone",
            "links: 1, 1 broken",
            "  BROKEN ~/.claude/skills -> ~/.agents/skills",
        ] {
            assert!(text.contains(want), "missing {want:?} in:\n{text}");
        }
        let ok = json!({"git": null, "config": {}, "always_on": {}, "harnesses": [
            {"label": "codex", "path": "p", "exists": true, "imports": {}, "stale": false, "ours": true}
        ], "links": []});
        let text = words(&status_lines(&ok).join("\n"));
        assert!(text.contains("NOT a git checkout"), "{text}");
        assert!(text.contains("codex: ok"), "{text}");
        assert!(text.contains("links: none"), "{text}");
    }
}
