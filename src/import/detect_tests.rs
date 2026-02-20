#![allow(clippy::unwrap_used)]

use super::*;

fn h(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| s.to_string()).collect()
}

// ── Bank format detection ─────────────────────────────────────

#[test]
fn test_detect_wells_fargo() {
    let headers: Vec<String> = vec![];
    let first_row = h(&["01/15/2024", "-4.50", "*", "123", "COFFEE SHOP"]);
    let profile = detect_bank_format(&headers, &first_row).unwrap();
    assert_eq!(profile.name, "Wells Fargo");
    assert!(!profile.has_header);
    assert_eq!(profile.description_column, 4);
    assert_eq!(profile.amount_column, Some(1));
    assert!(!profile.is_credit_account);
}

#[test]
fn test_detect_amex() {
    let headers = h(&["Date", "Description", "Card Member", "Amount"]);
    let first_row = h(&["01/15/2024", "Coffee Shop", "JOHN DOE", "-4.50"]);
    let profile = detect_bank_format(&headers, &first_row).unwrap();
    assert_eq!(profile.name, "American Express");
    assert!(profile.negate_amounts);
    assert!(profile.is_credit_account);
}

#[test]
fn test_detect_boa_credit() {
    let headers = h(&[
        "Posted Date",
        "Reference Number",
        "Payee",
        "Address",
        "Amount",
    ]);
    let first_row = h(&["01/15/2024", "12345", "Coffee Shop", "123 Main St", "-4.50"]);
    let profile = detect_bank_format(&headers, &first_row).unwrap();
    assert_eq!(profile.name, "Bank of America Credit Card");
    assert!(profile.is_credit_account);
}

#[test]
fn test_detect_boa_checking() {
    let headers = h(&["Date", "Description", "Amount", "Running Bal."]);
    let first_row = h(&["01/15/2024", "Coffee Shop", "-4.50", "995.50"]);
    let profile = detect_bank_format(&headers, &first_row).unwrap();
    assert_eq!(profile.name, "Bank of America Checking");
    assert!(!profile.is_credit_account);
}

#[test]
fn test_detect_usaa() {
    let headers = h(&[
        "Date",
        "Description",
        "Original Description",
        "Category",
        "Amount",
    ]);
    let first_row = h(&["01/15/2024", "Coffee", "COFFEE SHOP #123", "Food", "-4.50"]);
    let profile = detect_bank_format(&headers, &first_row).unwrap();
    assert_eq!(profile.name, "USAA");
    assert!(!profile.is_credit_account);
}

#[test]
fn test_detect_citi() {
    let headers = h(&["Status", "Date", "Description", "Debit", "Credit"]);
    let first_row = h(&["Cleared", "01/15/2024", "Coffee", "4.50", ""]);
    let profile = detect_bank_format(&headers, &first_row).unwrap();
    assert_eq!(profile.name, "Citi");
    assert!(profile.amount_column.is_none());
    assert!(profile.debit_column.is_some());
    assert!(profile.credit_column.is_some());
    assert!(profile.is_credit_account);
}

#[test]
fn test_detect_capital_one_credit() {
    let headers = h(&[
        "Transaction Date",
        "Posted Date",
        "Card No.",
        "Description",
        "Category",
        "Debit",
        "Credit",
    ]);
    let first_row = h(&[
        "2024-01-15",
        "2024-01-16",
        "1234",
        "Coffee",
        "Food",
        "4.50",
        "",
    ]);
    let profile = detect_bank_format(&headers, &first_row).unwrap();
    assert_eq!(profile.name, "Capital One Credit Card");
    assert_eq!(profile.date_format, "%Y-%m-%d");
    assert!(profile.is_credit_account);
}

#[test]
fn test_detect_capital_one_checking() {
    let headers = h(&[
        "Account Number",
        "Transaction Date",
        "Transaction Amount",
        "Transaction Type",
        "Transaction Description",
        "Balance",
    ]);
    let first_row = h(&["1234", "01/15/2024", "-4.50", "Debit", "Coffee", "995.50"]);
    let profile = detect_bank_format(&headers, &first_row).unwrap();
    assert_eq!(profile.name, "Capital One Checking");
    assert!(!profile.is_credit_account);
}

#[test]
fn test_detect_discover() {
    let headers = h(&[
        "Trans. Date",
        "Post Date",
        "Description",
        "Amount",
        "Category",
    ]);
    let first_row = h(&["01/15/2024", "01/16/2024", "Coffee", "-4.50", "Food"]);
    let profile = detect_bank_format(&headers, &first_row).unwrap();
    assert_eq!(profile.name, "Discover");
    assert!(profile.is_credit_account);
}

#[test]
fn test_detect_chase_checking() {
    let headers = h(&[
        "Details",
        "Posting Date",
        "Description",
        "Amount",
        "Type",
        "Balance",
        "Check or Slip #",
    ]);
    let first_row = h(&[
        "DEBIT",
        "01/15/2024",
        "Coffee",
        "-4.50",
        "ACH",
        "995.50",
        "",
    ]);
    let profile = detect_bank_format(&headers, &first_row).unwrap();
    assert_eq!(profile.name, "Chase Checking");
    assert!(!profile.is_credit_account);
}

#[test]
fn test_detect_chase_credit() {
    let headers = h(&[
        "Transaction Date",
        "Post Date",
        "Description",
        "Category",
        "Type",
        "Amount",
        "Memo",
    ]);
    let first_row = h(&[
        "01/15/2024",
        "01/16/2024",
        "Coffee",
        "Food",
        "Sale",
        "-4.50",
        "",
    ]);
    let profile = detect_bank_format(&headers, &first_row).unwrap();
    assert_eq!(profile.name, "Chase Credit Card");
    assert!(profile.is_credit_account);
}

#[test]
fn test_detect_unknown_format() {
    let headers = h(&["Foo", "Bar", "Baz"]);
    let first_row = h(&["a", "b", "c"]);
    assert!(detect_bank_format(&headers, &first_row).is_none());
}

#[test]
fn test_detect_empty_headers() {
    let headers: Vec<String> = vec![];
    let first_row: Vec<String> = vec![];
    assert!(detect_bank_format(&headers, &first_row).is_none());
}

#[test]
fn test_detect_case_insensitive() {
    let headers = h(&["CARD MEMBER", "DATE", "DESCRIPTION", "AMOUNT"]);
    let first_row = h(&["JOHN DOE", "01/15/2024", "Coffee", "4.50"]);
    let profile = detect_bank_format(&headers, &first_row).unwrap();
    assert_eq!(profile.name, "American Express");
}

// ── Column index helper ───────────────────────────────────────

#[test]
fn test_wells_fargo_not_matched_wrong_column_count() {
    let headers: Vec<String> = vec![];
    // Only 3 columns instead of 5
    let first_row = h(&["01/15/2024", "-4.50", "*"]);
    assert!(detect_bank_format(&headers, &first_row).is_none());
}

#[test]
fn test_wells_fargo_not_matched_no_star() {
    let headers: Vec<String> = vec![];
    // 5 columns but no "*" in column 2
    let first_row = h(&["01/15/2024", "-4.50", "X", "123", "COFFEE SHOP"]);
    assert!(detect_bank_format(&headers, &first_row).is_none());
}
