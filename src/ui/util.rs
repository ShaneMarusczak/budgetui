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
