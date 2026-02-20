#![allow(clippy::unwrap_used)]

use super::*;
use crate::models::{ImportRule, Transaction};
use rust_decimal_macros::dec;

fn make_rule(pattern: &str, cat_id: i64) -> ImportRule {
    ImportRule::new_contains(pattern.to_string(), cat_id)
}

fn make_regex_rule(pattern: &str, cat_id: i64) -> ImportRule {
    ImportRule::new_regex(pattern.to_string(), cat_id)
}

fn make_txn(desc: &str) -> Transaction {
    Transaction {
        id: None,
        account_id: 1,
        date: "2024-01-15".into(),
        description: desc.into(),
        original_description: desc.into(),
        amount: dec!(-10.00),
        category_id: None,
        notes: String::new(),
        is_transfer: false,
        import_hash: String::new(),
        created_at: String::new(),
    }
}

// ── Categorizer ───────────────────────────────────────────────

#[test]
fn test_categorize_contains_match() {
    let rules = vec![make_rule("coffee", 1), make_rule("amazon", 2)];
    let cat = Categorizer::new(&rules);
    assert_eq!(cat.categorize("STARBUCKS COFFEE #123"), Some(1));
    assert_eq!(cat.categorize("AMAZON.COM PURCHASE"), Some(2));
}

#[test]
fn test_categorize_case_insensitive() {
    let rules = vec![make_rule("coffee", 1)];
    let cat = Categorizer::new(&rules);
    assert_eq!(cat.categorize("Coffee Shop"), Some(1));
    assert_eq!(cat.categorize("COFFEE SHOP"), Some(1));
    assert_eq!(cat.categorize("coffee shop"), Some(1));
}

#[test]
fn test_categorize_no_match() {
    let rules = vec![make_rule("coffee", 1)];
    let cat = Categorizer::new(&rules);
    assert_eq!(cat.categorize("GROCERY STORE"), None);
}

#[test]
fn test_categorize_first_match_wins() {
    let rules = vec![make_rule("shop", 1), make_rule("coffee shop", 2)];
    let cat = Categorizer::new(&rules);
    // "shop" matches first
    assert_eq!(cat.categorize("Coffee Shop"), Some(1));
}

#[test]
fn test_categorize_regex() {
    let rules = vec![make_regex_rule(r"^AMZN.*MKTP", 1)];
    let cat = Categorizer::new(&rules);
    assert_eq!(cat.categorize("AMZN MKTP US*2A1B3C"), Some(1));
    assert_eq!(cat.categorize("AMAZON.COM"), None);
}

#[test]
fn test_categorize_regex_case_insensitive() {
    // Regex matching is case-insensitive (consistent with contains rules)
    let rules = vec![make_regex_rule(r"STARBUCKS", 1)];
    let cat = Categorizer::new(&rules);
    assert_eq!(cat.categorize("STARBUCKS COFFEE"), Some(1));
    assert_eq!(cat.categorize("starbucks coffee"), Some(1));
    assert_eq!(cat.categorize("Starbucks Coffee"), Some(1));
}

#[test]
fn test_categorize_regex_pattern_match() {
    // Test regex patterns with quantifiers and anchors
    let rules = vec![make_regex_rule(r"^SQ \*", 1)];
    let cat = Categorizer::new(&rules);
    assert_eq!(cat.categorize("SQ *COFFEE SHOP"), Some(1));
    assert_eq!(cat.categorize("NOT SQ *COFFEE"), None);
}

#[test]
fn test_categorize_invalid_regex_skipped() {
    let rules = vec![make_regex_rule(r"[invalid", 1)];
    let cat = Categorizer::new(&rules);
    // Invalid regex compiles to None, match returns false
    assert_eq!(cat.categorize("anything"), None);
}

#[test]
fn test_categorize_empty_rules() {
    let rules: Vec<ImportRule> = vec![];
    let cat = Categorizer::new(&rules);
    assert_eq!(cat.categorize("anything"), None);
}

#[test]
fn test_categorize_empty_description() {
    let rules = vec![make_rule("", 1)];
    let cat = Categorizer::new(&rules);
    // Empty pattern matches everything (contains "")
    assert_eq!(cat.categorize("anything"), Some(1));
}

#[test]
fn test_categorize_mixed_rules() {
    // Mix of contains and regex rules
    let rules = vec![
        make_rule("walmart", 1),
        make_regex_rule(r"^AMZN", 2),
        make_rule("target", 3),
    ];
    let cat = Categorizer::new(&rules);
    assert_eq!(cat.categorize("WALMART SUPERCENTER"), Some(1));
    assert_eq!(cat.categorize("AMZN MKTP US"), Some(2));
    assert_eq!(cat.categorize("TARGET STORE #123"), Some(3));
    assert_eq!(cat.categorize("COSTCO WHOLESALE"), None);
}

// ── Batch categorization ──────────────────────────────────────

#[test]
fn test_categorize_batch() {
    let rules = vec![make_rule("coffee", 1), make_rule("grocery", 2)];
    let cat = Categorizer::new(&rules);
    let mut txns = vec![
        make_txn("COFFEE SHOP"),
        make_txn("GROCERY STORE"),
        make_txn("UNKNOWN MERCHANT"),
    ];
    cat.categorize_batch(&mut txns);
    assert_eq!(txns[0].category_id, Some(1));
    assert_eq!(txns[1].category_id, Some(2));
    assert_eq!(txns[2].category_id, None);
}

#[test]
fn test_categorize_batch_preserves_existing() {
    let rules = vec![make_rule("coffee", 1)];
    let cat = Categorizer::new(&rules);
    let mut txns = vec![make_txn("COFFEE SHOP")];
    txns[0].category_id = Some(99); // Already categorized
    cat.categorize_batch(&mut txns);
    assert_eq!(txns[0].category_id, Some(99)); // Not overwritten
}

#[test]
fn test_categorize_batch_empty() {
    let rules = vec![make_rule("coffee", 1)];
    let cat = Categorizer::new(&rules);
    let mut txns: Vec<Transaction> = vec![];
    cat.categorize_batch(&mut txns); // Should not panic
    assert!(txns.is_empty());
}

#[test]
fn test_categorize_batch_uses_original_description() {
    let rules = vec![make_rule("starbucks", 1)];
    let cat = Categorizer::new(&rules);
    let mut txns = vec![Transaction {
        id: None,
        account_id: 1,
        date: "2024-01-15".into(),
        description: "Coffee Shop".into(),
        original_description: "STARBUCKS #123".into(),
        amount: dec!(-5.00),
        category_id: None,
        notes: String::new(),
        is_transfer: false,
        import_hash: String::new(),
        created_at: String::new(),
    }];
    cat.categorize_batch(&mut txns);
    // Should match on original_description
    assert_eq!(txns[0].category_id, Some(1));
}

// ── suggest_rule ──────────────────────────────────────────────

#[test]
fn test_suggest_rule_basic() {
    let s = suggest_rule("STARBUCKS COFFEE #123").unwrap();
    assert_eq!(s, "starbucks coffee");
}

#[test]
fn test_suggest_rule_strips_numbers() {
    let s = suggest_rule("AMZ*AMAZON 12345").unwrap();
    // Numbers stripped, * becomes space
    assert!(s.contains("amz"));
}

#[test]
fn test_suggest_rule_single_word() {
    let s = suggest_rule("NETFLIX").unwrap();
    assert_eq!(s, "netflix");
}

#[test]
fn test_suggest_rule_empty() {
    let s = suggest_rule("12345 #").unwrap();
    // After stripping digits and #, should still return something
    assert!(!s.is_empty());
}

#[test]
fn test_suggest_rule_lowercase() {
    let s = suggest_rule("WHOLE FOODS MARKET").unwrap();
    assert_eq!(s, "whole foods");
    // All lowercase
    assert_eq!(s, s.to_lowercase());
}
