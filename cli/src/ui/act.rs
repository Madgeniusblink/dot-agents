//! The actions of the view that write no file (decision 0011, item 5): copy
//! through OSC 52, open in `$EDITOR`, and OSC 8 links. Each sequence is
//! built here as text, so a test can read its bytes.

use crate::home::expand_home;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::{Command, Stdio};

/// The string terminator that ends an OSC sequence.
const ST: &str = "\x1b\\";

/// The bytes that put `text` on the terminal's clipboard. The terminal
/// does the copy, so it works over ssh and needs no clipboard tool.
pub fn copy(text: &str) -> String {
    format!("\x1b]52;c;{}{ST}", base64(text.as_bytes()))
}

/// The clipboard commands, first found on `PATH` wins. macOS Terminal.app
/// ignores OSC 52, so `pbcopy` is what makes `c` work there (#103).
const TOOLS: [(&str, &[&str]); 4] = [
    ("pbcopy", &[]),
    ("wl-copy", &[]),
    ("xclip", &["-selection", "clipboard"]),
    ("xsel", &["--clipboard", "--input"]),
];

/// Put `text` on the clipboard with the first command of `TOOLS` on `path`
/// (`PATH` when `None`) that takes it. A tool that fails, as `xclip` does
/// with no display, gives way to the next. The name of the command that
/// ran; `None` when no tool exists; the last failure when every tool failed.
pub fn copy_with_tool(text: &str, path: Option<&OsStr>) -> io::Result<Option<&'static str>> {
    use std::os::unix::fs::PermissionsExt;
    let path = path
        .map(OsStr::to_os_string)
        .or_else(|| std::env::var_os("PATH"));
    let dirs: Vec<_> = path.iter().flat_map(std::env::split_paths).collect();
    let executable = |b: &Path| {
        b.metadata()
            .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    };
    let mut failed = None;
    for (name, args) in TOOLS {
        let Some(bin) = dirs.iter().map(|d| d.join(name)).find(|b| executable(b)) else {
            continue;
        };
        match pipe_to(&bin, args, text) {
            Ok(()) => return Ok(Some(name)),
            Err(e) => failed = Some(io::Error::other(format!("{name}: {e}"))),
        }
    }
    failed.map_or(Ok(None), Err)
}

/// Run `bin` with `text` on its stdin, and fail unless it exits 0.
fn pipe_to(bin: &Path, args: &[&str], text: &str) -> io::Result<()> {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // A tool that quits early closes the pipe; its exit status says why.
    let _ = child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(text.as_bytes());
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(status.to_string()))
    }
}

/// `label` as a link to `path`, which a terminal opens on a click. A path
/// that is not absolute stays plain text.
pub fn link(path: &Path, label: &str) -> String {
    if !path.is_absolute() {
        return label.to_string();
    }
    format!("\x1b]8;;{}{ST}{label}\x1b]8;;{ST}", file_url(path))
}

/// `link` for a path as a report prints it, such as `~/.agents`.
pub fn link_shown(shown: &str) -> String {
    let path = if shown == "~" {
        crate::home::user_home()
    } else {
        expand_home(shown)
    };
    link(&path, shown)
}

/// `file://` and the path, each byte outside the unreserved set and `/` as
/// `%XX`. The host is empty, which names this machine.
pub fn file_url(path: &Path) -> String {
    let mut url = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            url.push(b as char);
        } else {
            url.push_str(&format!("%{b:02X}"));
        }
    }
    url
}

/// The command that opens `path` in `editor`, else in `vi`. The shell runs
/// the value as git runs `$EDITOR`, so `code -w` keeps its argument.
pub fn editor(editor: Option<&str>, path: &Path) -> Command {
    let e = editor
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .unwrap_or("vi");
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(format!("{e} \"$1\"")).arg("sh").arg(path);
    cmd
}

/// Standard base64, with padding.
pub fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | (b as u32) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ABC[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t6_c_copies_with_osc_52() {
        assert_eq!(copy("~/.agents"), "\x1b]52;c;fi8uYWdlbnRz\x1b\\");
        assert_eq!(copy("").as_bytes(), b"\x1b]52;c;\x1b\\");
        for (text, want) in [
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
            ("→", "4oaS"),
        ] {
            assert_eq!(base64(text.as_bytes()), want, "{text}");
        }
    }

    /// A fake clipboard command named `name` alone on a `PATH`. It writes
    /// its stdin to `log`.
    fn fake_tool(name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("log");
        let bin = dir.path().join(name);
        std::fs::write(&bin, format!("#!/bin/sh\ncat > '{}'\n", log.display())).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        (dir, log)
    }

    #[test]
    fn c_also_copies_with_a_clipboard_command_on_path() {
        let (dir, log) = fake_tool("pbcopy");
        let tool = copy_with_tool("~/.agents", Some(dir.path().as_os_str())).unwrap();
        assert_eq!(tool, Some("pbcopy"));
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "~/.agents");

        // The first tool found wins; xclip gets the clipboard selection.
        let (dir, log) = fake_tool("xclip");
        let tool = copy_with_tool("x", Some(dir.path().as_os_str())).unwrap();
        assert_eq!(tool, Some("xclip"));
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "x");
    }

    #[test]
    fn c_moves_to_the_next_tool_when_one_fails_or_is_not_executable() {
        use std::os::unix::fs::PermissionsExt;
        // pbcopy is a plain file, wl-copy quits with 1 before it reads, and
        // xclip takes the text.
        let (dir, log) = fake_tool("xclip");
        std::fs::write(dir.path().join("pbcopy"), "not a program").unwrap();
        let bad = dir.path().join("wl-copy");
        std::fs::write(&bad, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o755)).unwrap();
        let tool = copy_with_tool("x", Some(dir.path().as_os_str())).unwrap();
        assert_eq!(tool, Some("xclip"));
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "x");

        // Every tool failed: the last failure, with the tool's name.
        std::fs::remove_file(dir.path().join("xclip")).unwrap();
        let err = copy_with_tool("x", Some(dir.path().as_os_str())).unwrap_err();
        assert_eq!(err.to_string(), "wl-copy: exit status: 1");
    }

    #[test]
    fn c_without_a_clipboard_command_is_terminal_only() {
        let empty = tempfile::tempdir().unwrap();
        let tool = copy_with_tool("x", Some(empty.path().as_os_str())).unwrap();
        assert_eq!(tool, None);
        // OSC 52 still goes to the terminal; that part does not depend on PATH.
        assert!(copy("x").starts_with("\x1b]52;c;"));
    }

    #[test]
    fn t6_paths_are_osc_8_links() {
        assert_eq!(
            link(Path::new("/a/b c/ü.md"), "b c/ü.md").as_bytes(),
            b"\x1b]8;;file:///a/b%20c/%C3%BC.md\x1b\\b c/\xc3\xbc.md\x1b]8;;\x1b\\"
        );
        assert_eq!(link(Path::new("rel/x"), "rel/x"), "rel/x");
        let home = crate::home::user_home();
        assert_eq!(
            link_shown("~/.agents"),
            format!(
                "\x1b]8;;{}{ST}~/.agents\x1b]8;;{ST}",
                file_url(&home.join(".agents"))
            )
        );
        assert_eq!(link_shown("~"), link(&home, "~"));
    }

    #[test]
    fn t6_o_runs_editor_through_the_shell_with_the_path() {
        let p = Path::new("/x/it's here.md");
        let args = |c: &Command| {
            c.get_args()
                .map(|a| a.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        };
        let c = editor(Some("code -w"), p);
        assert_eq!(c.get_program(), "sh");
        assert_eq!(args(&c), ["-c", "code -w \"$1\"", "sh", "/x/it's here.md"]);
        assert_eq!(args(&editor(None, p))[1], "vi \"$1\"");
        assert_eq!(args(&editor(Some("  "), p))[1], "vi \"$1\"");
    }
}
