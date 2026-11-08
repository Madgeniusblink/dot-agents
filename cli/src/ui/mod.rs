//! `agents ui`: one full-screen view of the reports the other commands
//! print. Each view reads the same object that `--json` prints, so the view
//! and the report cannot disagree. Raw mode comes from `rustix` termios and
//! the drawing is ANSI written here, so no crate is added (decision 0011).
//! The view writes a file only through a `Plan` from `cli/src/plan.rs`: the
//! Skills view shows the plan of `skills keep` or `skills remove`, and `A`
//! runs it as `--apply` does. `c` copies through the terminal and `o` opens
//! an editor, which writes only what the person saves. The frame owns the
//! terminal, the tabs, and the keys; each view below draws its own lines.

#[cfg(unix)]
pub mod act;
// The tabs reach these views in later items of #82. Until then only their
// tests call them.
#[allow(dead_code)]
pub mod files;
#[cfg(unix)]
pub mod live;
#[allow(dead_code)]
pub mod scan;
#[cfg(unix)]
pub mod send;
// Only the Unix frame draws the Skills view.
#[cfg_attr(not(unix), allow(dead_code))]
pub mod skills;
#[allow(dead_code)]
pub mod tree;

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
    use super::act;
    use super::live;
    use super::send::{Pane, Tmux};
    use crate::config::Config;
    use crate::home::{tilde, Home};
    use crate::out::{self, State};
    use crate::status;
    use crate::ui::skills;
    use anyhow::Result;
    use rustix::termios::{self, OptionalActions, SpecialCodeIndex, Termios};
    use serde_json::Value;
    use std::io::{self, Read, Write};
    use std::panic::{self, AssertUnwindSafe};
    use std::path::PathBuf;
    use std::process::{Command, ExitCode};
    use std::sync::{Mutex, Once, PoisonError};
    use std::time::Instant;

    /// The alternate screen, with the cursor hidden.
    const ENTER: &str = "\x1b[?1049h\x1b[?25l";
    /// The undo of `ENTER`: show the cursor, then the main screen again.
    const LEAVE: &str = "\x1b[?25h\x1b[?1049l";
    /// Reverse video, and the end of every attribute.
    const INVERT: &str = "\x1b[7m";
    const RESET: &str = "\x1b[0m";
    /// The end of an OSC 8 link.
    const LINK_END: &str = "\x1b]8;;\x1b\\";

    /// Open the view, or print one line and exit 1 when there is no terminal.
    pub fn ui(home: Option<PathBuf>, cfg: &Config) -> Result<ExitCode> {
        if !termios::isatty(io::stdin()) || !termios::isatty(io::stdout()) {
            eprintln!("agents ui needs a terminal on stdin and stdout.");
            return Ok(ExitCode::FAILURE);
        }
        let home = Home::locate(home)?;
        let mut tty = Tty;
        guarded(&mut tty, |t| run(t, &home, cfg))??;
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

    /// `cmd` on the main screen with line discipline on, then the view
    /// again. The person's own editor needs the terminal as it was.
    pub(super) fn open<T: Term>(term: &mut T, mut cmd: Command) -> io::Result<()> {
        term.leave();
        let status = cmd.status();
        term.enter()?;
        status.map(drop)
    }

    /// The settings the terminal had before `enter`. `None` once restored.
    static SAVED: Mutex<Option<Termios>> = Mutex::new(None);
    /// `enter` runs again after `o`; the panic hook goes in once.
    static HOOK: Once = Once::new();

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
            HOOK.call_once(|| {
                let previous = panic::take_hook();
                panic::set_hook(Box::new(move |info| {
                    Tty::restore();
                    previous(info);
                }));
            });
            let mut out = io::stdout().lock();
            out.write_all(ENTER.as_bytes())?;
            out.flush()
        }

        fn leave(&mut self) {
            Tty::restore();
        }
    }

    /// The loop: draw, wait for a key, apply it, until `q`. The outer loop
    /// reads the skills again after a plan ran, after `R`, and when a file
    /// time changed. A refresh keeps the scroll, the selection, and the marks.
    fn run<T: Term>(term: &mut T, home: &Home, cfg: &Config) -> Result<()> {
        let root = crate::skills::project_root();
        let tmux = Tmux::from_env();
        let mut frame = Frame::default();
        let mut stdin = io::stdin().lock();
        let mut out = io::stdout().lock();
        let mut kept = None;
        loop {
            let found = crate::skills::find(Some(home));
            let mut list = skills::View::new(&found, root.as_deref());
            if let Some(state) = kept.take() {
                list.restore(state);
            }
            let roots = live::roots(&home.dir, &found);
            let seen = live::stamp(&roots);
            let mut checked = Instant::now();
            let mut body = frame.view.lines(home, cfg);
            let mut pending: Option<skills::Pending> = None;
            let refresh = loop {
                let (width, height) = size();
                let rows = height.saturating_sub(2);
                let (shown, keys) = match (&pending, frame.view) {
                    (Some(p), _) if p.done => (p.lines.clone(), Some(DONE_KEYS)),
                    (Some(p), _) => (p.lines.clone(), Some(PLAN_KEYS)),
                    (None, View::Skills) => (list.render(width, rows), Some(SKILLS_KEYS)),
                    (None, View::Status) => (body.clone(), None),
                };
                draw(&mut out, &screen_with(&frame, &shown, keys, width, height))?;
                let mut buf = [0u8; 16];
                let n = stdin.read(&mut buf)?;
                let Some(key) = key(&buf[..n]) else {
                    // A plan on screen is not read again under the person.
                    if pending.is_none() && checked.elapsed() >= live::EVERY {
                        checked = Instant::now();
                        if live::stamp(&roots) != seen {
                            break true;
                        }
                    }
                    continue;
                };
                if sending(&mut frame, key, &shown, &tmux) {
                    continue;
                }
                if key == Key::Refresh && pending.is_none() {
                    break true;
                }
                frame.note.clear();
                if let Some(p) = pending.as_mut().filter(|_| !frame.help) {
                    match key {
                        Key::Apply if !p.done => {
                            p.apply();
                            frame.top = 0;
                            continue;
                        }
                        Key::Esc if p.done => break false,
                        Key::Esc => {
                            pending = None;
                            frame.top = 0;
                            continue;
                        }
                        Key::Next | Key::Prev | Key::View(_) => continue,
                        _ => {}
                    }
                } else if pending.is_none() && frame.view == View::Skills && !frame.help {
                    match key {
                        Key::Down => list.down(),
                        Key::Up => list.up(),
                        Key::Mark => list.mark(),
                        Key::Remove | Key::Keep => {
                            let names = list.names();
                            if !names.is_empty() {
                                let keep = key == Key::Keep;
                                pending = Some(skills::Pending::new(keep, &names, Some(home)));
                                frame.top = 0;
                            }
                        }
                        _ => {}
                    }
                    if matches!(
                        key,
                        Key::Down | Key::Up | Key::Mark | Key::Remove | Key::Keep
                    ) {
                        continue;
                    }
                }
                // On the Skills list, `c` and `o` take the selected skill.
                let target = match (frame.view, &pending, list.selected()) {
                    (View::Skills, None, Some(s)) => s.dir.clone(),
                    _ => frame.view.target(home),
                };
                match key {
                    Key::Copy => {
                        out.write_all(act::copy(&tilde(&target)).as_bytes())?;
                        out.flush()?;
                        continue;
                    }
                    Key::Open => {
                        let editor = std::env::var("EDITOR").ok();
                        open(term, act::editor(editor.as_deref(), &target))?;
                        continue;
                    }
                    _ => {}
                }
                let before = frame.view;
                if !frame.press(key, shown.len(), rows) {
                    return Ok(());
                }
                if frame.view != before {
                    body = frame.view.lines(home, cfg);
                }
            };
            if refresh {
                kept = Some(list.state());
            } else {
                frame.top = 0;
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
        /// The Skills view: mark a row, then plan a remove or a keep of the
        /// marked names, then apply the plan.
        Mark,
        Remove,
        Keep,
        Apply,
        /// Read every view again now. `r` is taken by remove.
        Refresh,
        Send,
        Yes,
        Copy,
        Open,
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
            b"m" => Key::Mark,
            b"r" => Key::Remove,
            b"K" => Key::Keep,
            b"A" => Key::Apply,
            b"R" => Key::Refresh,
            b"s" => Key::Send,
            b"y" => Key::Yes,
            b"c" => Key::Copy,
            b"o" => Key::Open,
            _ => Key::Other,
        })
    }

    /// A tab. Each one reads the object its command prints with `--json`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) enum View {
        Status,
        Skills,
    }

    impl View {
        pub(super) const ALL: [View; 2] = [View::Status, View::Skills];

        pub(super) fn name(self) -> &'static str {
            match self {
                View::Status => "Status",
                View::Skills => "Skills",
            }
        }

        /// The path `c` copies and `o` opens. Status reports on the home.
        pub(super) fn target(self, home: &Home) -> PathBuf {
            match self {
                View::Status => home.dir.clone(),
                View::Skills => home.dir.join("skills"),
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
                // The Skills view keeps a selection, so the loop draws it.
                View::Skills => Vec::new(),
            }
        }
    }

    /// Text the person must confirm before it goes to `pane`.
    #[derive(Debug)]
    pub(super) struct Ask {
        pub(super) text: String,
        pub(super) pane: Pane,
    }

    /// Which tab shows, whether help covers it, the first body row, a send
    /// that waits for its answer, and the result of the last action.
    #[derive(Debug)]
    pub(super) struct Frame {
        pub(super) view: View,
        pub(super) help: bool,
        pub(super) top: usize,
        pub(super) ask: Option<Ask>,
        pub(super) note: String,
    }

    impl Default for Frame {
        fn default() -> Frame {
            Frame {
                view: View::Status,
                help: false,
                top: 0,
                ask: None,
                note: String::new(),
            }
        }
    }

    /// `s` and its answer; `false` when the key is not theirs. `s` shows the
    /// top body line and the first agent pane. Only `y` then sends it; any
    /// other key cancels.
    pub(super) fn sending(frame: &mut Frame, key: Key, body: &[String], tmux: &Tmux) -> bool {
        if let Some(ask) = frame.ask.take() {
            frame.note = match key {
                Key::Yes => match tmux.send(&ask.pane, &ask.text) {
                    Ok(()) => format!("sent to the {} pane {}", ask.pane.label, ask.pane.id),
                    Err(e) => format!("not sent: {e}"),
                },
                _ => "not sent".to_string(),
            };
            return true;
        }
        if key != Key::Send || frame.help {
            return false;
        }
        let text = body
            .get(frame.top)
            .map(|l| plain(l).trim().to_string())
            .unwrap_or_default();
        frame.note.clear();
        if text.is_empty() {
            frame.note = "nothing to send on the top line".to_string();
        } else if let Some(pane) = tmux.panes().into_iter().next() {
            frame.ask = Some(Ask { text, pane });
        } else {
            frame.note = "no agent pane in tmux".to_string();
        }
        true
    }

    /// `s` with every escape sequence taken out.
    fn plain(s: &str) -> String {
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
        text
    }

    /// The rows that ask before a send: the pane, then the whole text,
    /// wrapped and never cut, so the person sees all of what goes.
    fn ask_lines(ask: &Ask, width: usize) -> Vec<String> {
        let mut lines = vec![
            format!(
                "Send this text to the {} pane {}?",
                ask.pane.label, ask.pane.id
            ),
            String::new(),
        ];
        let chars: Vec<char> = ask.text.chars().collect();
        for chunk in chars.chunks(width.saturating_sub(2).max(1)) {
            lines.push(format!("  {}", chunk.iter().collect::<String>()));
        }
        lines.push(String::new());
        lines.push("It is typed with no Enter. y sends it; any other key cancels.".to_string());
        lines
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
                Key::Mark
                | Key::Remove
                | Key::Keep
                | Key::Apply
                | Key::Refresh
                | Key::Send
                | Key::Yes
                | Key::Copy
                | Key::Open
                | Key::Other => {}
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
    #[cfg(test)]
    pub(super) fn screen(
        frame: &Frame,
        body: &[String],
        width: usize,
        height: usize,
    ) -> Vec<String> {
        screen_with(frame, body, None, width, height)
    }

    /// `screen`, with the key bar `keys` when help is closed.
    fn screen_with(
        frame: &Frame,
        body: &[String],
        keys: Option<&[(&str, &str)]>,
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
            let asking = frame.ask.as_ref().map(|a| ask_lines(a, width));
            let (lines, top) = if let Some(a) = &asking {
                (&a[..], 0)
            } else if frame.help {
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
            rows.push(key_bar(frame, keys, width));
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
    fn key_bar(frame: &Frame, keys: Option<&[(&str, &str)]>, width: usize) -> String {
        let keys: &[(&str, &str)] = if frame.ask.is_some() {
            &[("y", "send"), ("any key", "cancel")]
        } else if frame.help {
            &[("esc", "close"), ("q", "quit")]
        } else if let Some(keys) = keys {
            keys
        } else {
            &[
                ("?", "help"),
                ("j/k", "scroll"),
                ("tab", "view"),
                ("q", "quit"),
                ("s", "send"),
            ]
        };
        let mut bar = keys
            .iter()
            .map(|(k, what)| format!("{INVERT} {k} {RESET} {what}"))
            .collect::<Vec<_>>()
            .join("  ");
        // The result of the last action comes first, so a narrow bar keeps it.
        if !frame.note.is_empty() {
            bar = format!("{}  {bar}", frame.note);
        }
        cut(&bar, width)
    }

    const SKILLS_KEYS: &[(&str, &str)] = &[
        ("?", "help"),
        ("j/k", "select"),
        ("m", "mark"),
        ("r", "remove"),
        ("K", "keep only"),
        ("tab", "view"),
        ("q", "quit"),
        ("s", "send"),
    ];
    const PLAN_KEYS: &[(&str, &str)] = &[
        ("A", "apply"),
        ("esc", "cancel"),
        ("j/k", "scroll"),
        ("q", "quit"),
    ];
    const DONE_KEYS: &[(&str, &str)] = &[("esc", "back"), ("j/k", "scroll"), ("q", "quit")];

    const HELP: [&str; 26] = [
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
        "  s            send the top line to an agent pane in tmux, after y",
        "  c            copy the view's path; the terminal holds the copy",
        "  o            open the view's path in $EDITOR",
        "  R            read every view again now; a changed file does it in 2 s",
        "",
        "Skills",
        "  m            mark the selected skill, or unmark it",
        "  r            plan agents skills remove for the marked, else the selected",
        "  K            plan agents skills keep for the marked, else the selected",
        "  A            apply the plan shown, as --apply does; esc cancels it",
        "  c, o         copy or open the selected skill's directory",
        "",
        "Each view reads what its command prints with --json, so the view and",
        "the report say the same thing. Each path is a link in a terminal that",
        "shows links. The view writes a file only after A, through the same",
        "plan the command prints. Files move to ~/.agents-reset/.",
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
        let mut link = false;
        let mut rest = s;
        while let Some(c) = rest.chars().next() {
            if c == '\x1b' {
                let (esc, after) = rest.split_at(out::escape_len(rest));
                if let Some(body) = esc.strip_prefix("\x1b]8;") {
                    // An empty URI ends a link; any other opens one.
                    let body = body.trim_end_matches(['\x07', '\\']);
                    link = !body.trim_end_matches('\x1b').ends_with(';');
                }
                text.push_str(esc);
                rest = after;
            } else if shown + 1 < width {
                text.push(c);
                shown += 1;
                rest = &rest[c.len_utf8()..];
            } else {
                break;
            }
        }
        text.push('…');
        if link {
            text.push_str(LINK_END);
        }
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
        let path = act::link_shown(s(config, "path"));
        let file = if config["exists"].as_bool() == Some(true) {
            path
        } else {
            format!("{path} (no file, so defaults)")
        };
        lines.push(format!("machine: {}, config: {file}", s(doc, "machine")));
        let mut home = format!("home: {}", act::link_shown(s(doc, "home")));
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
            lines.push(format!("  {}{link}", act::link_shown(s(h, "path"))));
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
            let settings = act::link_shown(hooks["settings"].as_str().unwrap_or_default());
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
            lines.push(format!(
                "  {state} {} -> {}",
                act::link_shown(s(l, "path")),
                act::link_shown(s(l, "target"))
            ));
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
        let mut rest = s;
        while let Some(c) = rest.chars().next() {
            if c == '\x1b' {
                rest = &rest[crate::out::escape_len(rest)..];
            } else {
                text.push(c);
                rest = &rest[c.len_utf8()..];
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
        for (k, view) in [
            (Key::Next, View::Skills),
            (Key::Next, View::Status),
            (Key::Prev, View::Skills),
            (Key::View(0), View::Status),
            (Key::View(1), View::Skills),
            (Key::View(8), View::Skills),
        ] {
            assert!(f.press(k, 10, 5));
            assert_eq!(f.view, view, "{k:?}");
        }
        f.press(Key::View(0), 10, 5);
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
            ..Frame::default()
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

    /// A fake `tmux` alone on `PATH`. It logs each call, one bracket per
    /// argument, and lists four panes: a shell, two agent tools, an editor.
    fn fake_tmux() -> (tempfile::TempDir, std::path::PathBuf, super::send::Tmux) {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("log");
        let bin = dir.path().join("tmux");
        let script = format!(
            "#!/bin/sh\n\
             for a in \"$@\"; do printf '[%s]' \"$a\"; done >> '{log}'\n\
             echo >> '{log}'\n\
             case \"$1\" in list-panes) printf '%%1\\tzsh\\n%%2\\tclaude\\n%%3\\tcodex\\n%%4\\tvim\\n';; esac\n",
            log = log.display()
        );
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let tmux = super::send::Tmux {
            path: Some(dir.path().into()),
            here: Some("%2".to_string()),
        };
        (dir, log, tmux)
    }

    fn calls(log: &std::path::Path) -> String {
        std::fs::read_to_string(log).unwrap_or_default()
    }

    #[test]
    fn t7_s_shows_the_text_and_sends_it_to_the_pane_only_after_y() {
        let (_dir, log, tmux) = fake_tmux();
        let body = vec!["\x1b[32m-n review\x1b[0m this file ".to_string()];
        let mut f = Frame::default();
        assert_eq!(key(b"s"), Some(Key::Send));
        assert_eq!(key(b"y"), Some(Key::Yes));

        // The own pane %2 is skipped; %3 runs an agent tool, labelled by harness.rs.
        assert!(sending(&mut f, Key::Send, &body, &tmux));
        let ask = f.ask.as_ref().expect("s asks before it sends");
        assert_eq!((ask.pane.id.as_str(), ask.pane.label), ("%3", "codex"));
        let rows = screen(&f, &body, 80, 12);
        let shown = rows.join("\n");
        assert!(
            shown.contains("Send this text to the codex pane %3?"),
            "{shown}"
        );
        assert!(shown.contains("  -n review this file"), "{shown}");
        assert!(rows[11].contains("y \x1b[0m send"), "{}", rows[11]);
        assert!(!calls(&log).contains("send-keys"), "nothing goes before y");

        // Any other key cancels.
        assert!(sending(&mut f, Key::Other, &body, &tmux));
        assert!(f.ask.is_none());
        assert_eq!(f.note, "not sent");
        assert!(!calls(&log).contains("send-keys"));

        // y sends the shown text, literal, with no Enter.
        assert!(sending(&mut f, Key::Send, &body, &tmux));
        assert!(sending(&mut f, Key::Yes, &body, &tmux));
        assert_eq!(f.note, "sent to the codex pane %3");
        let log_text = calls(&log);
        let sent: Vec<&str> = log_text
            .lines()
            .filter(|l| l.starts_with("[send-keys]"))
            .collect();
        assert_eq!(sent, ["[send-keys][-t][%3][-l][--][-n review this file]"]);
        assert!(screen(&f, &body, 80, 12)[11].contains("sent to the codex pane %3"));
    }

    #[test]
    fn t7_s_with_no_agent_pane_or_no_tmux_sends_nothing() {
        let (dir, log, mut tmux) = fake_tmux();
        let body = vec!["text".to_string()];
        let mut f = Frame::default();
        tmux.here = Some("%3".to_string());
        assert_eq!(tmux.panes()[0].label, "claude code");
        tmux.path = Some(dir.path().join("empty").into());
        assert!(sending(&mut f, Key::Send, &body, &tmux));
        assert!(f.ask.is_none());
        assert_eq!(f.note, "no agent pane in tmux");
        assert!(
            !sending(&mut f, Key::Down, &body, &tmux),
            "other keys go to the frame"
        );
        assert!(sending(&mut f, Key::Send, &[String::new()], &tmux));
        assert_eq!(f.note, "nothing to send on the top line");
        assert!(!calls(&log).contains("send-keys"));
    }

    #[test]
    fn t10_shift_r_refreshes_and_r_stays_remove() {
        assert_eq!(key(b"R"), Some(Key::Refresh));
        assert_eq!(key(b"r"), Some(Key::Remove));
        let mut f = Frame {
            top: 2,
            ..Frame::default()
        };
        assert!(f.press(Key::Refresh, 10, 5));
        assert_eq!((f.view, f.help, f.top), (View::Status, false, 2));
    }

    #[test]
    fn t2_cut_keeps_escapes_and_ends_in_a_reset() {
        assert_eq!(cut("abc", 3), "abc");
        assert_eq!(cut("abcdef", 4), "abc…\x1b[0m");
        assert_eq!(cut("\x1b[32mabcdef\x1b[0m", 4), "\x1b[32mabc…\x1b[0m");
        assert_eq!(cut("abc", 0), "");
    }

    #[test]
    fn t6_c_and_o_are_keys_that_leave_the_frame_as_it_was() {
        assert_eq!(key(b"c"), Some(Key::Copy));
        assert_eq!(key(b"o"), Some(Key::Open));
        let mut f = Frame {
            top: 2,
            ..Frame::default()
        };
        assert!(f.press(Key::Copy, 10, 5) && f.press(Key::Open, 10, 5));
        assert_eq!((f.view, f.help, f.top), (View::Status, false, 2));
        let home = crate::home::Home {
            dir: std::path::PathBuf::from("/h/.agents"),
        };
        assert_eq!(View::Status.target(&home), home.dir);
    }

    #[test]
    fn t6_o_runs_the_editor_outside_raw_mode_then_the_view_comes_back() {
        use std::os::unix::fs::PermissionsExt;
        let t = tempfile::tempdir().unwrap();
        let script = t.path().join("ed");
        std::fs::write(&script, "#!/bin/sh\nprintf %s \"$1\" > \"$SEEN\"\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let seen = t.path().join("seen");
        let target = t.path().join("a file.md");
        let mut cmd = super::act::editor(Some(script.to_str().unwrap()), &target);
        cmd.env("SEEN", &seen);
        let mut fake = Fake::default();
        fake.enter().unwrap();
        open(&mut fake, cmd).unwrap();
        assert_eq!(
            std::fs::read_to_string(&seen).unwrap(),
            target.to_str().unwrap()
        );
        assert!(fake.raw, "the view is back in raw mode");
        assert_eq!((fake.entered, fake.left), (2, 1));
        let missing = super::act::editor(Some("/no/such/editor"), &target);
        open(&mut fake, missing).unwrap();
        assert!(fake.raw, "a failed editor still gives the view back");
    }

    #[test]
    fn t6_cut_ends_a_link_it_cuts_and_a_link_takes_no_width() {
        let link = super::act::link(std::path::Path::new("/a/long-name.md"), "long-name.md");
        assert_eq!(cut(&link, 12), link);
        let cut5 = cut(&link, 5);
        assert_eq!(
            cut5,
            "\x1b]8;;file:///a/long-name.md\x1b\\long…\x1b]8;;\x1b\\\x1b[0m"
        );
        assert_eq!(crate::out::visible(&cut5), 5);
        let after = format!("{link} and more text");
        assert_eq!(cut(&after, 15), format!("{link} a…\x1b[0m"));
    }

    #[test]
    fn t6_status_view_paths_are_links() {
        let doc = json!({
            "home": "~/.agents", "machine": "box",
            "config": {"path": "~/.config/agents/config.toml", "exists": true},
            "always_on": {},
            "harnesses": [{"label": "codex", "path": "~/.codex/AGENTS.md", "exists": false}],
            "hooks": {"settings": "~/.claude/settings.json", "valid_json": true, "missing_paths": []},
            "links": [{"path": "~/.claude/skills", "target": "~/.agents/skills", "broken": false}]
        });
        let text = status_lines(&doc).join("\n");
        for p in [
            "~/.agents",
            "~/.config/agents/config.toml",
            "~/.codex/AGENTS.md",
            "~/.claude/settings.json",
            "~/.claude/skills",
            "~/.agents/skills",
        ] {
            let l = super::act::link_shown(p);
            assert!(l.starts_with("\x1b]8;;file:///"), "{l:?}");
            assert!(text.contains(&l), "no link for {p} in:\n{text:?}");
        }
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
