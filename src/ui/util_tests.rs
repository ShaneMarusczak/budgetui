#![allow(clippy::unwrap_used)]

use super::util::*;

// ── truncate ──────────────────────────────────────────────────

#[test]
fn test_truncate_short_string() {
    assert_eq!(truncate("hello", 10), "hello");
}

#[test]
fn test_truncate_exact_length() {
    assert_eq!(truncate("hello", 5), "hello");
}

#[test]
fn test_truncate_long_string() {
    assert_eq!(truncate("hello world", 5), "hell…");
}

#[test]
fn test_truncate_empty() {
    assert_eq!(truncate("", 5), "");
}

#[test]
fn test_truncate_zero_max() {
    assert_eq!(truncate("hello", 0), "");
}

#[test]
fn test_truncate_unicode() {
    // Japanese characters are multi-byte UTF-8
    assert_eq!(truncate("日本語テスト", 4), "日本語…");
}

#[test]
fn test_truncate_emoji() {
    assert_eq!(truncate("🎉🎊🎈🎁", 3), "🎉🎊…");
}

#[test]
fn test_truncate_one_char() {
    assert_eq!(truncate("hello", 1), "…");
}

#[test]
fn test_truncate_mixed_unicode() {
    assert_eq!(truncate("café résumé", 5), "café…");
}

#[test]
fn test_truncate_two_chars() {
    assert_eq!(truncate("hello", 2), "h…");
}

#[test]
fn test_truncate_single_char_string() {
    assert_eq!(truncate("a", 1), "a");
    assert_eq!(truncate("a", 5), "a");
}

#[test]
fn test_truncate_max_one_with_long_string() {
    // max=1 should always produce "…" for strings longer than 1
    assert_eq!(truncate("ab", 1), "…");
    assert_eq!(truncate("abc", 1), "…");
}
