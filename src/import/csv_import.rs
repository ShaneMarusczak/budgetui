use anyhow::{Context, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::path::Path;
use std::str::FromStr;

use crate::models::Transaction;

#[derive(Debug, Clone)]
pub(crate) struct CsvProfile {
    pub(crate) name: String,
    pub(crate) date_column: usize,
    pub(crate) description_column: usize,
    pub(crate) amount_column: Option<usize>,
    pub(crate) debit_column: Option<usize>,
    pub(crate) credit_column: Option<usize>,
    pub(crate) date_format: String,
    pub(crate) has_header: bool,
    pub(crate) skip_rows: usize,
    pub(crate) negate_amounts: bool,
}

impl Default for CsvProfile {
    fn default() -> Self {
        Self {
            name: "Custom".into(),
            date_column: 0,
            description_column: 1,
            amount_column: Some(2),
            debit_column: None,
            credit_column: None,
            date_format: "%m/%d/%Y".into(),
            has_header: true,
            skip_rows: 0,
            negate_amounts: false,
        }
    }
}

pub(crate) struct CsvImporter;

impl CsvImporter {
    /// Read the CSV and return headers + all rows as strings for preview.
    pub(crate) fn preview(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        let mut rdr = csv::ReaderBuilder::new()
            .flexible(true)
            .has_headers(false)
            .from_path(path)
            .context("Failed to open CSV file")?;

        let mut all_rows: Vec<Vec<String>> = Vec::new();
        for result in rdr.records() {
            let record = result.context("Failed to read CSV record")?;
            all_rows.push(record.iter().map(|s| s.to_string()).collect());
        }

        if all_rows.is_empty() {
            anyhow::bail!("CSV file is empty");
        }

        // Try to detect if first row is a header
        let first_row = &all_rows[0];
        let looks_like_header = first_row.iter().all(|field| {
            let trimmed = field.trim();
            // Headers typically don't parse as dates or numbers
            Decimal::from_str(trimmed.replace(['$', ','], "").trim()).is_err()
                && NaiveDate::parse_from_str(trimmed, "%m/%d/%Y").is_err()
                && NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").is_err()
        });

        if looks_like_header {
            let headers = all_rows.remove(0);
            Ok((headers, all_rows))
        } else {
            // Generate generic column names
            let headers: Vec<String> = (0..first_row.len())
                .map(|i| format!("Column {}", i + 1))
                .collect();
            Ok((headers, all_rows))
        }
    }

    /// Parse rows into Transactions using the given profile.
    pub(crate) fn parse(
        rows: &[Vec<String>],
        profile: &CsvProfile,
        account_id: i64,
    ) -> Result<Vec<Transaction>> {
        let mut transactions = Vec::new();
        let now = chrono::Utc::now().to_rfc3339();

        for (i, row) in rows.iter().enumerate().skip(profile.skip_rows) {
            let date_str = row
                .get(profile.date_column)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            if date_str.is_empty() {
                continue;
            }

            let date = parse_date(&date_str, &profile.date_format)
                .with_context(|| format!("Row {}: failed to parse date '{}'", i + 1, date_str))?;

            let description = row
                .get(profile.description_column)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            let amount = parse_amount(row, profile)
                .with_context(|| format!("Row {}: failed to parse amount", i + 1))?;

            let hash = compute_hash(&date_str, &description, &amount);

            transactions.push(Transaction {
                id: None,
                account_id,
                date: date.format("%Y-%m-%d").to_string(),
                description: description.clone(),
                original_description: description,
                amount,
                category_id: None,
                notes: String::new(),
                is_transfer: false,
                import_hash: hash,
                created_at: now.clone(),
            });
        }

        Ok(transactions)
    }
}

fn parse_date(s: &str, fmt: &str) -> Result<NaiveDate> {
    // Try the specified format first
    if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
        return Ok(d);
    }
    // Fallback: try common formats
    for fallback in &["%m/%d/%Y", "%Y-%m-%d", "%m-%d-%Y", "%m/%d/%y", "%d/%m/%Y"] {
        if let Ok(d) = NaiveDate::parse_from_str(s, fallback) {
            return Ok(d);
        }
    }
    anyhow::bail!("Could not parse date: {}", s)
}

fn parse_amount(row: &[String], profile: &CsvProfile) -> Result<Decimal> {
    let amount = if let Some(amt_col) = profile.amount_column {
        let raw = row
            .get(amt_col)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        parse_decimal(&raw)?
    } else {
        // Separate debit/credit columns
        let debit = profile
            .debit_column
            .and_then(|c| row.get(c))
            .map(|s| s.trim())
            .unwrap_or("");
        let credit = profile
            .credit_column
            .and_then(|c| row.get(c))
            .map(|s| s.trim())
            .unwrap_or("");

        if !debit.is_empty() {
            -parse_decimal(debit)?.abs()
        } else if !credit.is_empty() {
            parse_decimal(credit)?.abs()
        } else {
            Decimal::ZERO
        }
    };

    if profile.negate_amounts {
        Ok(-amount)
    } else {
        Ok(amount)
    }
}

fn parse_decimal(s: &str) -> Result<Decimal> {
    let cleaned = s
        .replace(['$', ','], "")
        .replace('(', "-")
        .replace(')', "")
        .trim()
        .to_string();
    if cleaned.is_empty() {
        return Ok(Decimal::ZERO);
    }
    Decimal::from_str(&cleaned)
        .or_else(|_| Decimal::from_str(&cleaned.replace('"', "")))
        .context(format!("Failed to parse '{}' as decimal", s))
}

/// Compute a stable, deterministic hash for deduplication.
/// Uses FNV-1a (32-bit) which is simple, fast, and stable across Rust versions,
/// unlike DefaultHasher which can change between releases.
fn compute_hash(date: &str, description: &str, amount: &Decimal) -> String {
    let input = format!("{date}|{description}|{amount}");
    let hash = fnv1a(input.as_bytes());
    format!("{hash:016x}")
}

fn fnv1a(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
#[path = "csv_import_tests.rs"]
mod tests;
