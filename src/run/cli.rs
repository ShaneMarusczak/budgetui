use anyhow::Result;
use std::path::Path;

use crate::db::Database;

pub(crate) fn as_cli(args: &[String], db: &mut Database) -> Result<()> {
    match args[1].as_str() {
        "import" => cli_import(&args[2..], db),
        "export" => cli_export(&args[2..], db),
        "summary" | "s" => cli_summary(&args[2..], db),
        "accounts" => cli_accounts(db),
        "--help" | "-h" | "help" => {
            print_usage();
            Ok(())
        }
        "--version" | "-V" | "version" => {
            println!("budgetui {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        other => {
            print_usage();
            anyhow::bail!("Unknown command: {other}");
        }
    }
}

fn print_usage() {
    println!("BudgeTUI — local-only personal finance tracker");
    println!();
    println!("Usage: budgetui [command]");
    println!();
    println!("Commands:");
    println!("  (none)                        Launch interactive TUI");
    println!("  import <file.csv>             Import a CSV file (auto-detects bank format)");
    println!("    --account <name>            Account to import into (default: first account)");
    println!("  export [path]                 Export transactions to CSV");
    println!("    --month <YYYY-MM>           Month to export (default: current)");
    println!("  summary [YYYY-MM]             Print monthly financial summary");
    println!("  accounts                      List all accounts");
    println!("  --help, -h                    Show this help");
    println!("  --version, -V                 Show version");
}

fn cli_import(args: &[String], db: &mut Database) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("Usage: budgetui import <file.csv> [--account <name>]");
    }

    let file_path = &args[0];
    let path = Path::new(file_path);
    if !path.exists() {
        anyhow::bail!("File not found: {file_path}");
    }

    // Parse --account flag
    let account_name = args
        .windows(2)
        .find(|w| w[0] == "--account")
        .map(|w| w[1].as_str());

    // Load and parse CSV
    let (headers, rows) = crate::import::CsvImporter::preview(path)?;
    let first_row = rows.first().cloned().unwrap_or_default();

    let profile = if let Some(detected) = crate::import::detect_bank_format(&headers, &first_row) {
        println!("Detected format: {}", detected.name);
        detected
    } else {
        println!("Using default CSV profile (date=0, desc=1, amount=2)");
        crate::import::CsvProfile::default()
    };

    let account_id = if let Some(name) = account_name {
        let accounts = db.get_accounts()?;
        accounts
            .iter()
            .find(|a| a.name.to_lowercase() == name.to_lowercase())
            .and_then(|a| a.id)
            .ok_or_else(|| anyhow::anyhow!("Account '{name}' not found"))?
    } else {
        let accounts = db.get_accounts()?;
        if accounts.is_empty() {
            anyhow::bail!("No accounts found. Create one first, or use --account <name>");
        } else if accounts.len() == 1 {
            // Only one account — use it automatically
            accounts[0]
                .id
                .ok_or_else(|| anyhow::anyhow!("Account has no ID"))?
        } else {
            // Multiple accounts — user must specify
            let names: Vec<String> = accounts
                .iter()
                .map(|a| format!("  --account \"{}\"  ({})", a.name, a.account_type))
                .collect();
            anyhow::bail!(
                "Multiple accounts found. Use --account <name> to specify:\n{}",
                names.join("\n")
            );
        }
    };

    let mut txns = crate::import::CsvImporter::parse(&rows, &profile, account_id)?;
    println!("Parsed {} transactions", txns.len());

    // Auto-categorize
    let rules = db.get_import_rules()?;
    if !rules.is_empty() {
        let (categorizer, bad_patterns) = crate::categorize::Categorizer::new(&rules);
        if !bad_patterns.is_empty() {
            eprintln!(
                "Warning: invalid regex rule(s): {}",
                bad_patterns.join(", ")
            );
        }
        categorizer.categorize_batch(&mut txns);
        let categorized = txns.iter().filter(|t| t.category_id.is_some()).count();
        println!("Auto-categorized {categorized}/{} transactions", txns.len());
    }

    // Insert
    let count = db.insert_transactions_batch(&txns)?;
    let dupes = txns.len() - count;
    println!("Imported {count} new transactions ({dupes} duplicates skipped)");

    Ok(())
}

fn cli_export(args: &[String], db: &mut Database) -> Result<()> {
    // Parse --month flag
    let month = args
        .windows(2)
        .find(|w| w[0] == "--month")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m").to_string());

    // Output path is the first non-flag argument
    let output_path = args
        .first()
        .filter(|a| !a.starts_with('-'))
        .map(|a| shellexpand(a))
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            format!("{home}/budgetui-export-{month}.csv")
        });

    let count = db.export_to_csv(&output_path, Some(&month))?;
    if count == 0 {
        println!("No transactions for {month}");
    } else {
        println!("Exported {count} transactions to {output_path}");
    }
    Ok(())
}

fn cli_summary(args: &[String], db: &mut Database) -> Result<()> {
    let month = args
        .first()
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m").to_string());

    let (income, expenses) = db.get_monthly_totals(Some(&month))?;
    let net = income + expenses;
    let net_worth = db.get_net_worth()?;
    let spending = db.get_spending_by_category(Some(&month))?;
    let txn_count = db.get_transaction_count()?;

    println!("BudgeTUI — {month}");
    println!("{}", "─".repeat(40));
    println!("  Income:     ${:.2}", income);
    println!("  Expenses:   ${:.2}", expenses.abs());
    println!("  Net:        ${:.2}", net);
    println!("  Net Worth:  ${:.2}", net_worth);
    println!("  Total Txns: {txn_count}");

    if !spending.is_empty() {
        println!();
        println!("Spending by Category:");
        for (name, amount) in &spending {
            println!("  {name:<24} ${:.2}", amount.abs());
        }
    }

    Ok(())
}

fn cli_accounts(db: &mut Database) -> Result<()> {
    let accounts = db.get_accounts()?;
    if accounts.is_empty() {
        println!("No accounts");
        return Ok(());
    }

    println!("{:<4} {:<20} {:<15} Institution", "ID", "Name", "Type");
    println!("{}", "─".repeat(55));
    for acct in &accounts {
        println!(
            "{:<4} {:<20} {:<15} {}",
            acct.id.unwrap_or(0),
            acct.name,
            acct.account_type,
            acct.institution,
        );
    }
    Ok(())
}

pub(crate) fn shellexpand(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/{rest}")
    } else {
        path.to_string()
    }
}
