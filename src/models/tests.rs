#![allow(clippy::unwrap_used)]

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::*;

// ── Transaction ───────────────────────────────────────────────

fn make_txn(amount: Decimal) -> Transaction {
    Transaction {
        id: None,
        account_id: 1,
        date: "2024-01-15".into(),
        description: "Test".into(),
        original_description: "Test".into(),
        amount,
        category_id: None,
        notes: String::new(),
        is_transfer: false,
        import_hash: String::new(),
        created_at: String::new(),
    }
}

#[test]
fn test_income() {
    let txn = make_txn(dec!(100.00));
    assert!(txn.is_income());
    assert!(!txn.is_expense());
}

#[test]
fn test_expense() {
    let txn = make_txn(dec!(-50.00));
    assert!(!txn.is_income());
    assert!(txn.is_expense());
}

#[test]
fn test_zero_is_neither() {
    let txn = make_txn(Decimal::ZERO);
    assert!(!txn.is_income());
    assert!(!txn.is_expense());
}

#[test]
fn test_abs_amount() {
    assert_eq!(make_txn(dec!(-42.99)).abs_amount(), dec!(42.99));
    assert_eq!(make_txn(dec!(42.99)).abs_amount(), dec!(42.99));
    assert_eq!(make_txn(Decimal::ZERO).abs_amount(), Decimal::ZERO);
}

#[test]
fn test_small_amounts() {
    let txn = make_txn(dec!(0.01));
    assert!(txn.is_income());
    assert_eq!(txn.abs_amount(), dec!(0.01));

    let txn = make_txn(dec!(-0.01));
    assert!(txn.is_expense());
    assert_eq!(txn.abs_amount(), dec!(0.01));
}

// ── AccountType ───────────────────────────────────────────────

#[test]
fn test_account_type_parse() {
    assert_eq!(AccountType::parse("checking"), AccountType::Checking);
    assert_eq!(AccountType::parse("CHECKING"), AccountType::Checking);
    assert_eq!(AccountType::parse("savings"), AccountType::Savings);
    assert_eq!(AccountType::parse("credit card"), AccountType::CreditCard);
    assert_eq!(AccountType::parse("credit"), AccountType::CreditCard);
    assert_eq!(AccountType::parse("creditcard"), AccountType::CreditCard);
    assert_eq!(AccountType::parse("investment"), AccountType::Investment);
    assert_eq!(AccountType::parse("cash"), AccountType::Cash);
    assert_eq!(AccountType::parse("loan"), AccountType::Loan);
    assert_eq!(AccountType::parse("unknown"), AccountType::Other);
}

#[test]
fn test_account_type_as_str() {
    assert_eq!(AccountType::Checking.as_str(), "Checking");
    assert_eq!(AccountType::Savings.as_str(), "Savings");
    assert_eq!(AccountType::CreditCard.as_str(), "Credit Card");
    assert_eq!(AccountType::Investment.as_str(), "Investment");
    assert_eq!(AccountType::Cash.as_str(), "Cash");
    assert_eq!(AccountType::Loan.as_str(), "Loan");
    assert_eq!(AccountType::Other.as_str(), "Other");
}

#[test]
fn test_account_type_display() {
    assert_eq!(format!("{}", AccountType::Checking), "Checking");
    assert_eq!(format!("{}", AccountType::CreditCard), "Credit Card");
}

#[test]
fn test_account_type_all() {
    let all = AccountType::all();
    assert_eq!(all.len(), 7);
    assert!(all.contains(&AccountType::Checking));
    assert!(all.contains(&AccountType::Other));
}

#[test]
fn test_account_type_roundtrip() {
    // Every type should roundtrip through as_str -> parse
    for t in AccountType::all() {
        let s = t.as_str();
        let back = AccountType::parse(s);
        assert_eq!(*t, back, "Roundtrip failed for {s}");
    }
}

#[test]
fn test_account_new_defaults() {
    let account = Account::new("Test".into(), AccountType::Checking, "Bank".into());
    assert!(account.id.is_none());
    assert_eq!(account.name, "Test");
    assert_eq!(account.currency, "USD");
    assert_eq!(account.institution, "Bank");
    assert!(!account.created_at.is_empty());
}

// ── Category ──────────────────────────────────────────────────

#[test]
fn test_category_new() {
    let cat = Category::new("Food".into());
    assert!(cat.id.is_none());
    assert_eq!(cat.name, "Food");
    assert!(cat.parent_id.is_none());
    assert!(cat.icon.is_empty());
    assert!(cat.color.is_empty());
}

#[test]
fn test_category_display() {
    let cat = Category::new("Groceries".into());
    assert_eq!(format!("{cat}"), "Groceries");
}

// ── Budget ────────────────────────────────────────────────────

#[test]
fn test_budget_new() {
    let budget = Budget::new(1, "2024-01".into(), dec!(500));
    assert!(budget.id.is_none());
    assert_eq!(budget.category_id, 1);
    assert_eq!(budget.month, "2024-01");
    assert_eq!(budget.limit_amount, dec!(500));
}

// ── ImportRule ─────────────────────────────────────────────────

#[test]
fn test_import_rule_new_contains() {
    let rule = ImportRule::new_contains("coffee".into(), 1);
    assert!(rule.id.is_none());
    assert_eq!(rule.pattern, "coffee");
    assert_eq!(rule.category_id, 1);
    assert!(!rule.is_regex);
    assert_eq!(rule.priority, 0);
}

#[test]
fn test_import_rule_new_regex() {
    let rule = ImportRule::new_regex(r"^AMZN.*".into(), 2);
    assert!(rule.id.is_none());
    assert_eq!(rule.pattern, "^AMZN.*");
    assert_eq!(rule.category_id, 2);
    assert!(rule.is_regex);
    assert_eq!(rule.priority, 0);
}
