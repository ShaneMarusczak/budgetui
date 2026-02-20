#![allow(clippy::unwrap_used)]

use super::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::io::Write;

fn make_csv_file(content: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

// ── parse_decimal ─────────────────────────────────────────────

#[test]
fn test_parse_decimal_basic() {
    assert_eq!(parse_decimal("100.50").unwrap(), dec!(100.50));
    assert_eq!(parse_decimal("-42.99").unwrap(), dec!(-42.99));
}

#[test]
fn test_parse_decimal_with_currency() {
    assert_eq!(parse_decimal("$1,234.56").unwrap(), dec!(1234.56));
    assert_eq!(parse_decimal("-$99.99").unwrap(), dec!(-99.99));
}

#[test]
fn test_parse_decimal_parentheses_negative() {
    assert_eq!(parse_decimal("(500.00)").unwrap(), dec!(-500.00));
}

#[test]
fn test_parse_decimal_empty() {
    assert_eq!(parse_decimal("").unwrap(), Decimal::ZERO);
    assert_eq!(parse_decimal("  ").unwrap(), Decimal::ZERO);
}

#[test]
fn test_parse_decimal_quoted() {
    assert_eq!(parse_decimal("\"100.00\"").unwrap(), dec!(100.00));
}

#[test]
fn test_parse_decimal_integer() {
    assert_eq!(parse_decimal("42").unwrap(), dec!(42));
}

#[test]
fn test_parse_decimal_large_with_commas() {
    assert_eq!(parse_decimal("$1,234,567.89").unwrap(), dec!(1234567.89));
}

#[test]
fn test_parse_decimal_invalid() {
    assert!(parse_decimal("not_a_number").is_err());
}

// ── parse_date ────────────────────────────────────────────────

#[test]
fn test_parse_date_us_format() {
    let d = parse_date("01/15/2024", "%m/%d/%Y").unwrap();
    assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
}

#[test]
fn test_parse_date_iso_format() {
    let d = parse_date("2024-01-15", "%Y-%m-%d").unwrap();
    assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
}

#[test]
fn test_parse_date_fallback() {
    // Pass wrong primary format, should fallback to try others
    let d = parse_date("2024-01-15", "%m/%d/%Y").unwrap();
    assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
}

#[test]
fn test_parse_date_two_digit_year() {
    let d = parse_date("01/15/24", "%m/%d/%y").unwrap();
    assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
}

#[test]
fn test_parse_date_dash_format() {
    let d = parse_date("01-15-2024", "%m-%d-%Y").unwrap();
    assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
}

#[test]
fn test_parse_date_invalid() {
    assert!(parse_date("not-a-date", "%m/%d/%Y").is_err());
}

#[test]
fn test_parse_date_empty() {
    assert!(parse_date("", "%m/%d/%Y").is_err());
}

// ── parse_amount ──────────────────────────────────────────────

#[test]
fn test_parse_amount_single_column() {
    let profile = CsvProfile::default();
    let row = vec!["01/15/2024".into(), "Coffee".into(), "-4.50".into()];
    assert_eq!(parse_amount(&row, &profile).unwrap(), dec!(-4.50));
}

#[test]
fn test_parse_amount_debit_credit_columns() {
    let profile = CsvProfile {
        amount_column: None,
        debit_column: Some(2),
        credit_column: Some(3),
        ..CsvProfile::default()
    };
    let debit_row = vec![
        "01/15/2024".into(),
        "Coffee".into(),
        "4.50".into(),
        "".into(),
    ];
    assert_eq!(parse_amount(&debit_row, &profile).unwrap(), dec!(-4.50));

    let credit_row = vec![
        "01/15/2024".into(),
        "Deposit".into(),
        "".into(),
        "1000.00".into(),
    ];
    assert_eq!(parse_amount(&credit_row, &profile).unwrap(), dec!(1000.00));
}

#[test]
fn test_parse_amount_both_empty_debit_credit() {
    let profile = CsvProfile {
        amount_column: None,
        debit_column: Some(2),
        credit_column: Some(3),
        ..CsvProfile::default()
    };
    let row = vec![
        "01/15/2024".into(),
        "Something".into(),
        "".into(),
        "".into(),
    ];
    assert_eq!(parse_amount(&row, &profile).unwrap(), Decimal::ZERO);
}

#[test]
fn test_parse_amount_negate() {
    let profile = CsvProfile {
        negate_amounts: true,
        ..CsvProfile::default()
    };
    let row = vec!["01/15/2024".into(), "Coffee".into(), "4.50".into()];
    assert_eq!(parse_amount(&row, &profile).unwrap(), dec!(-4.50));
}

// ── CsvImporter::preview ──────────────────────────────────────

#[test]
fn test_preview_with_headers() {
    let csv = "Date,Description,Amount\n01/15/2024,Coffee,-4.50\n01/16/2024,Lunch,-12.00\n";
    let file = make_csv_file(csv);
    let (headers, rows) = CsvImporter::preview(file.path()).unwrap();
    assert_eq!(headers, vec!["Date", "Description", "Amount"]);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][1], "Coffee");
}

#[test]
fn test_preview_without_headers() {
    // Wells Fargo-style: no headers, starts with data
    let csv = "01/15/2024,-4.50,*,123,COFFEE SHOP\n01/16/2024,-12.00,*,456,RESTAURANT\n";
    let file = make_csv_file(csv);
    let (headers, rows) = CsvImporter::preview(file.path()).unwrap();
    assert!(headers[0].starts_with("Column"));
    assert_eq!(rows.len(), 2);
}

#[test]
fn test_preview_empty_file() {
    let file = make_csv_file("");
    assert!(CsvImporter::preview(file.path()).is_err());
}

#[test]
fn test_preview_single_row_with_header() {
    let csv = "Date,Description,Amount\n01/15/2024,Coffee,-4.50\n";
    let file = make_csv_file(csv);
    let (headers, rows) = CsvImporter::preview(file.path()).unwrap();
    assert_eq!(headers.len(), 3);
    assert_eq!(rows.len(), 1);
}

#[test]
fn test_preview_quoted_fields() {
    let csv = "Date,Description,Amount\n01/15/2024,\"Coffee, Shop\",-4.50\n";
    let file = make_csv_file(csv);
    let (_, rows) = CsvImporter::preview(file.path()).unwrap();
    assert_eq!(rows[0][1], "Coffee, Shop");
}

// ── CsvImporter::parse ────────────────────────────────────────

#[test]
fn test_parse_basic_rows() {
    let profile = CsvProfile::default();
    let rows = vec![
        vec!["01/15/2024".into(), "Coffee".into(), "-4.50".into()],
        vec!["01/16/2024".into(), "Lunch".into(), "-12.00".into()],
    ];
    let txns = CsvImporter::parse(&rows, &profile, 1).unwrap();
    assert_eq!(txns.len(), 2);
    assert_eq!(txns[0].date, "2024-01-15");
    assert_eq!(txns[0].description, "Coffee");
    assert_eq!(txns[0].amount, dec!(-4.50));
    assert_eq!(txns[0].account_id, 1);
}

#[test]
fn test_parse_skips_empty_dates() {
    let profile = CsvProfile::default();
    let rows = vec![
        vec!["01/15/2024".into(), "Coffee".into(), "-4.50".into()],
        vec!["".into(), "".into(), "".into()],
        vec!["01/16/2024".into(), "Lunch".into(), "-12.00".into()],
    ];
    let txns = CsvImporter::parse(&rows, &profile, 1).unwrap();
    assert_eq!(txns.len(), 2);
}

#[test]
fn test_parse_skip_rows() {
    let profile = CsvProfile {
        skip_rows: 1,
        ..CsvProfile::default()
    };
    let rows = vec![
        vec!["SKIP THIS ROW".into(), "ignore".into(), "0".into()],
        vec!["01/15/2024".into(), "Coffee".into(), "-4.50".into()],
    ];
    let txns = CsvImporter::parse(&rows, &profile, 1).unwrap();
    assert_eq!(txns.len(), 1);
    assert_eq!(txns[0].description, "Coffee");
}

#[test]
fn test_parse_iso_dates() {
    let profile = CsvProfile {
        date_format: "%Y-%m-%d".into(),
        ..CsvProfile::default()
    };
    let rows = vec![vec!["2024-01-15".into(), "Coffee".into(), "-4.50".into()]];
    let txns = CsvImporter::parse(&rows, &profile, 1).unwrap();
    assert_eq!(txns[0].date, "2024-01-15");
}

#[test]
fn test_parse_generates_import_hash() {
    let profile = CsvProfile::default();
    let rows = vec![vec!["01/15/2024".into(), "Coffee".into(), "-4.50".into()]];
    let txns = CsvImporter::parse(&rows, &profile, 1).unwrap();
    assert!(!txns[0].import_hash.is_empty());
}

#[test]
fn test_parse_sets_account_id() {
    let profile = CsvProfile::default();
    let rows = vec![vec!["01/15/2024".into(), "Coffee".into(), "-4.50".into()]];
    let txns = CsvImporter::parse(&rows, &profile, 42).unwrap();
    assert_eq!(txns[0].account_id, 42);
}

#[test]
fn test_parse_empty_rows() {
    let profile = CsvProfile::default();
    let rows: Vec<Vec<String>> = vec![];
    let txns = CsvImporter::parse(&rows, &profile, 1).unwrap();
    assert!(txns.is_empty());
}

// ── compute_hash ──────────────────────────────────────────────

#[test]
fn test_hash_deterministic() {
    let h1 = compute_hash(1, 0, "2024-01-15", "Coffee", &dec!(-4.50));
    let h2 = compute_hash(1, 0, "2024-01-15", "Coffee", &dec!(-4.50));
    assert_eq!(h1, h2);
}

#[test]
fn test_hash_different_inputs() {
    let h1 = compute_hash(1, 0, "2024-01-15", "Coffee", &dec!(-4.50));
    let h2 = compute_hash(1, 0, "2024-01-15", "Tea", &dec!(-4.50));
    let h3 = compute_hash(1, 0, "2024-01-16", "Coffee", &dec!(-4.50));
    let h4 = compute_hash(1, 0, "2024-01-15", "Coffee", &dec!(-5.00));
    assert_ne!(h1, h2);
    assert_ne!(h1, h3);
    assert_ne!(h1, h4);
}

#[test]
fn test_hash_different_rows_same_data() {
    // Two identical transactions at different row positions should get different hashes
    let h1 = compute_hash(1, 0, "2024-01-15", "Coffee", &dec!(-4.50));
    let h2 = compute_hash(1, 1, "2024-01-15", "Coffee", &dec!(-4.50));
    assert_ne!(h1, h2);
}

#[test]
fn test_hash_different_accounts_same_data() {
    let h1 = compute_hash(1, 0, "2024-01-15", "Coffee", &dec!(-4.50));
    let h2 = compute_hash(2, 0, "2024-01-15", "Coffee", &dec!(-4.50));
    assert_ne!(h1, h2);
}

#[test]
fn test_hash_format() {
    let h = compute_hash(1, 0, "2024-01-15", "Coffee", &dec!(-4.50));
    // Should be 16 hex chars
    assert_eq!(h.len(), 16);
    assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
}

// ── fnv1a ─────────────────────────────────────────────────────

#[test]
fn test_fnv1a_empty() {
    // FNV-1a offset basis
    assert_eq!(fnv1a(b""), 0xcbf29ce484222325);
}

#[test]
fn test_fnv1a_consistency() {
    assert_eq!(fnv1a(b"hello"), fnv1a(b"hello"));
    assert_ne!(fnv1a(b"hello"), fnv1a(b"world"));
}

#[test]
fn test_fnv1a_single_byte_changes() {
    // Even single-byte changes should produce different hashes
    assert_ne!(fnv1a(b"a"), fnv1a(b"b"));
    assert_ne!(fnv1a(b"aa"), fnv1a(b"ab"));
}
