use super::CsvProfile;

/// Known bank CSV fingerprints for auto-detection.
/// Returns a CsvProfile if the format is recognized, None otherwise.
pub(crate) fn detect_bank_format(headers: &[String], first_row: &[String]) -> Option<CsvProfile> {
    let h: Vec<String> = headers
        .iter()
        .map(|s| s.to_lowercase().trim().to_string())
        .collect();

    // Wells Fargo: no headers, 5 columns, col[2] == "*"
    if headers.is_empty() && first_row.len() == 5 && first_row.get(2).map(|s| s.trim()) == Some("*")
    {
        return Some(CsvProfile {
            name: "Wells Fargo".into(),
            date_column: 0,
            description_column: 4,
            amount_column: Some(1),
            debit_column: None,
            credit_column: None,
            date_format: "%m/%d/%Y".into(),
            has_header: false,
            skip_rows: 0,
            negate_amounts: false,
            is_credit_account: false,
        });
    }

    // American Express: "Card Member" header
    if h.contains(&"card member".into()) {
        return Some(CsvProfile {
            name: "American Express".into(),
            date_column: col_index(&h, "date").unwrap_or(0),
            description_column: col_index(&h, "description").unwrap_or(1),
            amount_column: col_index(&h, "amount"),
            debit_column: None,
            credit_column: None,
            date_format: "%m/%d/%Y".into(),
            has_header: true,
            skip_rows: 0,
            negate_amounts: true, // AmEx inverts: charges positive, payments negative
            is_credit_account: true,
        });
    }

    // Bank of America Credit Card: "Reference Number" + "Address"
    if h.contains(&"reference number".into()) && h.contains(&"address".into()) {
        return Some(CsvProfile {
            name: "Bank of America Credit Card".into(),
            date_column: col_index(&h, "posted date").unwrap_or(0),
            description_column: col_index(&h, "payee").unwrap_or(2),
            amount_column: col_index(&h, "amount"),
            debit_column: None,
            credit_column: None,
            date_format: "%m/%d/%Y".into(),
            has_header: true,
            skip_rows: 0,
            negate_amounts: false,
            is_credit_account: true,
        });
    }

    // Bank of America Checking: "Running Bal."
    if h.iter().any(|s| s.contains("running bal")) {
        return Some(CsvProfile {
            name: "Bank of America Checking".into(),
            date_column: col_index(&h, "date").unwrap_or(0),
            description_column: col_index(&h, "description").unwrap_or(1),
            amount_column: col_index(&h, "amount"),
            debit_column: None,
            credit_column: None,
            date_format: "%m/%d/%Y".into(),
            has_header: true,
            skip_rows: 0,
            negate_amounts: false,
            is_credit_account: false,
        });
    }

    // USAA: "Original Description"
    if h.contains(&"original description".into()) {
        return Some(CsvProfile {
            name: "USAA".into(),
            date_column: col_index(&h, "date").unwrap_or(0),
            description_column: col_index(&h, "description").unwrap_or(1),
            amount_column: col_index(&h, "amount"),
            debit_column: None,
            credit_column: None,
            date_format: "%m/%d/%Y".into(),
            has_header: true,
            skip_rows: 0,
            negate_amounts: false,
            is_credit_account: false,
        });
    }

    // Citi: starts with "Status" + has Debit/Credit columns
    if h.first().map(|s| s.as_str()) == Some("status")
        && h.contains(&"debit".into())
        && h.contains(&"credit".into())
    {
        return Some(CsvProfile {
            name: "Citi".into(),
            date_column: col_index(&h, "date").unwrap_or(1),
            description_column: col_index(&h, "description").unwrap_or(2),
            amount_column: None,
            debit_column: col_index(&h, "debit"),
            credit_column: col_index(&h, "credit"),
            date_format: "%m/%d/%Y".into(),
            has_header: true,
            skip_rows: 0,
            negate_amounts: false,
            is_credit_account: true,
        });
    }

    // Capital One Credit Card: "Card No." + ISO dates
    if h.contains(&"card no.".into()) {
        return Some(CsvProfile {
            name: "Capital One Credit Card".into(),
            date_column: col_index(&h, "transaction date").unwrap_or(0),
            description_column: col_index(&h, "description").unwrap_or(3),
            amount_column: None,
            debit_column: col_index(&h, "debit"),
            credit_column: col_index(&h, "credit"),
            date_format: "%Y-%m-%d".into(),
            has_header: true,
            skip_rows: 0,
            negate_amounts: false,
            is_credit_account: true,
        });
    }

    // Capital One Checking: "Account Number" first + "Transaction Amount"
    if h.first().map(|s| s.as_str()) == Some("account number")
        && h.contains(&"transaction amount".into())
    {
        return Some(CsvProfile {
            name: "Capital One Checking".into(),
            date_column: col_index(&h, "transaction date").unwrap_or(1),
            description_column: col_index(&h, "transaction description").unwrap_or(4),
            amount_column: col_index(&h, "transaction amount"),
            debit_column: None,
            credit_column: None,
            date_format: "%m/%d/%Y".into(),
            has_header: true,
            skip_rows: 0,
            negate_amounts: false,
            is_credit_account: false,
        });
    }

    // Discover: "Trans. Date" (with period)
    if h.iter()
        .any(|s| s.contains("trans. date") || s.contains("trans.date"))
    {
        return Some(CsvProfile {
            name: "Discover".into(),
            date_column: 0,
            description_column: col_index(&h, "description").unwrap_or(2),
            amount_column: col_index(&h, "amount"),
            debit_column: None,
            credit_column: None,
            date_format: "%m/%d/%Y".into(),
            has_header: true,
            skip_rows: 0,
            negate_amounts: false,
            is_credit_account: true,
        });
    }

    // Chase Checking: "Details" + "Check or Slip #"
    if h.contains(&"details".into()) && h.iter().any(|s| s.contains("check or slip")) {
        return Some(CsvProfile {
            name: "Chase Checking".into(),
            date_column: col_index(&h, "posting date").unwrap_or(1),
            description_column: col_index(&h, "description").unwrap_or(2),
            amount_column: col_index(&h, "amount"),
            debit_column: None,
            credit_column: None,
            date_format: "%m/%d/%Y".into(),
            has_header: true,
            skip_rows: 0,
            negate_amounts: false,
            is_credit_account: false,
        });
    }

    // Chase Credit Card: "Transaction Date" + "Post Date" + "Type"
    if h.contains(&"transaction date".into())
        && h.contains(&"post date".into())
        && h.contains(&"type".into())
    {
        return Some(CsvProfile {
            name: "Chase Credit Card".into(),
            date_column: col_index(&h, "transaction date").unwrap_or(0),
            description_column: col_index(&h, "description").unwrap_or(2),
            amount_column: col_index(&h, "amount"),
            debit_column: None,
            credit_column: None,
            date_format: "%m/%d/%Y".into(),
            has_header: true,
            skip_rows: 0,
            negate_amounts: false,
            is_credit_account: true,
        });
    }

    None
}

fn col_index(headers: &[String], name: &str) -> Option<usize> {
    headers.iter().position(|h| h == name)
}

#[cfg(test)]
#[path = "detect_tests.rs"]
mod tests;
