//! List on top, detail below: shared heights and rule for Skills, Scan,
//! Files, and Tree.

/// The fewest lines the detail pane keeps when the terminal is short.
pub const DETAIL_MIN: usize = 5;

/// A horizontal rule between the list and the detail pane.
pub fn rule(width: usize) -> String {
    "─".repeat(width)
}

/// How many lines the list and detail panes get after `header` header lines
/// and `footer` footer lines. `detail_len` is the natural height of the
/// detail content before truncation.
pub fn heights(height: usize, header: usize, footer: usize, detail_len: usize) -> (usize, usize) {
    let fixed = header + footer;
    let room = height.saturating_sub(fixed + 1);
    let detail_h = detail_len.min((height / 2).max(DETAIL_MIN)).min(room);
    let list_h = height.saturating_sub(fixed + detail_h);
    (list_h, detail_h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_reserves_detail_min_on_a_short_terminal() {
        let (list, detail) = heights(10, 2, 0, 20);
        assert_eq!(detail, 5);
        assert_eq!(list, 3);
    }

    #[test]
    fn heights_respects_footer_lines() {
        let (list, detail) = heights(12, 1, 1, 20);
        assert_eq!(list + detail + 1 + 1, 12);
        assert!(detail >= DETAIL_MIN);
    }
}
