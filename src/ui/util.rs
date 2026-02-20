/// Truncate a string to `max` visible characters, appending "…" if truncated.
/// The result is guaranteed to be at most `max` characters (counting "…" as one).
/// Safe for multi-byte UTF-8 characters.
pub(crate) fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let char_count = s.chars().count();
    if char_count <= max {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{truncated}…")
}

/// Move a list cursor down by one, adjusting scroll to keep cursor visible.
pub(crate) fn scroll_down(index: &mut usize, scroll: &mut usize, len: usize, page: usize) {
    if *index + 1 < len {
        *index += 1;
        if *index >= *scroll + page {
            *scroll = index.saturating_sub(page - 1);
        }
    }
}

/// Move a list cursor up by one, adjusting scroll to keep cursor visible.
pub(crate) fn scroll_up(index: &mut usize, scroll: &mut usize) {
    *index = index.saturating_sub(1);
    if *index < *scroll {
        *scroll = *index;
    }
}

/// Jump cursor to the top of a list.
pub(crate) fn scroll_to_top(index: &mut usize, scroll: &mut usize) {
    *index = 0;
    *scroll = 0;
}

/// Jump cursor to the bottom of a list.
pub(crate) fn scroll_to_bottom(index: &mut usize, scroll: &mut usize, len: usize, page: usize) {
    if len > 0 {
        *index = len - 1;
        *scroll = index.saturating_sub(page.saturating_sub(1));
    }
}
