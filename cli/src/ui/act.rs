//! The actions of the view that write no file (decision 0011, item 5): copy
//! through OSC 52, open in `$EDITOR`, and OSC 8 links. Each sequence is
//! built here as text, so a test can read its bytes.

use crate::home::expand_home;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::Command;

/// The string terminator that ends an OSC sequence.
const ST: &str = "\x1b\\";

/// The bytes that put `text` on the terminal's clipboard. The terminal
/// does the copy, so it works over ssh and needs no clipboard tool.
pub fn copy(text: &str) -> String {
    format!("\x1b]52;c;{}{ST}", base64(text.as_bytes()))
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
