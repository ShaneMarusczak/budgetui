#![allow(clippy::unwrap_used)]

use rust_decimal_macros::dec;

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

// ── format_amount ──────────────────────────────────────────

#[test]
fn test_format_amount_basic() {
    assert_eq!(format_amount(dec!(1234.56)), "$1,234.56");
}

#[test]
fn test_format_amount_no_commas() {
    assert_eq!(format_amount(dec!(999.99)), "$999.99");
}

#[test]
fn test_format_amount_zero() {
    assert_eq!(format_amount(dec!(0)), "$0.00");
}

#[test]
fn test_format_amount_negative() {
    assert_eq!(format_amount(dec!(-42.50)), "-$42.50");
}

#[test]
fn test_format_amount_large() {
    assert_eq!(format_amount(dec!(1234567.89)), "$1,234,567.89");
}

#[test]
fn test_format_amount_millions() {
    assert_eq!(format_amount(dec!(10000000.00)), "$10,000,000.00");
}

#[test]
fn test_format_amount_rounds_to_two_decimals() {
    assert_eq!(format_amount(dec!(1.5)), "$1.50");
}

#[test]
fn test_format_amount_negative_large() {
    assert_eq!(format_amount(dec!(-99999.01)), "-$99,999.01");
}

#[test]
fn test_format_amount_single_digit() {
    assert_eq!(format_amount(dec!(5)), "$5.00");
}
