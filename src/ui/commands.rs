use std::collections::HashMap;
use std::sync::LazyLock;

use rust_decimal::Decimal;
use std::str::FromStr;

use super::app::{App, InputMode, PendingAction, Screen};
use crate::db::Database;
use crate::models::{Account, AccountType, Budget, Category, ImportRule};

pub(crate) struct Command {
    pub(crate) description: &'static str,
    pub(crate) run: fn(&str, &mut App, &mut Database) -> anyhow::Result<()>,
}

macro_rules! register_command {
    ($name:expr, $desc:expr, $func:expr, $registry:expr) => {{
        $registry.insert(
            $name,
            Command {
                description: $desc,
                run: $func,
            },
        );
    }};
}

pub(crate) static COMMANDS: LazyLock<HashMap<&str, Command>> = LazyLock::new(|| {
    let mut r: HashMap<&str, Command> = HashMap::new();

    register_command!("q", "Quit BudgeTUI", cmd_quit, r);
    register_command!("quit", "Quit BudgeTUI", cmd_quit, r);
    register_command!("d", "Go to Dashboard", cmd_dashboard, r);
    register_command!("dashboard", "Go to Dashboard", cmd_dashboard, r);
    register_command!("t", "Go to Transactions", cmd_transactions, r);
    register_command!("transactions", "Go to Transactions", cmd_transactions, r);
    register_command!("i", "Import CSV file", cmd_import, r);
    register_command!("import", "Import CSV file", cmd_import, r);
    register_command!("c", "Go to Categories", cmd_categories, r);
    register_command!("categories", "Go to Categories", cmd_categories, r);
    register_command!("b", "Go to Budgets", cmd_budgets, r);
    register_command!("budgets", "Go to Budgets", cmd_budgets, r);
    register_command!("help", "Show available commands", cmd_help, r);
    register_command!("h", "Show available commands", cmd_help, r);
    register_command!("month", "Set month (e.g. :month 2024-01)", cmd_month, r);
    register_command!("m", "Set month (e.g. :m 2024-01)", cmd_month, r);
    register_command!(
        "account",
        "Create account (e.g. :account Chase Checking)",
        cmd_account,
        r
    );
    register_command!(
        "a",
        "Create account (e.g. :a Chase Checking)",
        cmd_account,
        r
    );
    register_command!(
        "rule",
        "Add categorization rule (e.g. :rule amazon Shopping)",
        cmd_rule,
        r
    );
    register_command!(
        "r",
        "Add categorization rule (e.g. :r amazon Shopping)",
        cmd_rule,
        r
    );
    register_command!(
        "search",
        "Search transactions (e.g. :search coffee)",
        cmd_search,
        r
    );
    register_command!("s", "Search transactions (e.g. :s coffee)", cmd_search, r);
    register_command!(
        "budget",
        "Set budget (e.g. :budget Food & Dining 500)",
        cmd_budget,
        r
    );
    register_command!(
        "delete-budget",
        "Delete selected budget",
        cmd_delete_budget,
        r
    );
    register_command!(
        "category",
        "Create category (e.g. :category Subscriptions)",
        cmd_category,
        r
    );
    register_command!(
        "delete-rule",
        "Delete selected import rule",
        cmd_delete_rule,
        r
    );
    register_command!(
        "regex-rule",
        "Add regex rule (e.g. :regex-rule ^AMZ.* Shopping)",
        cmd_regex_rule,
        r
    );
    register_command!("rename", "Rename selected transaction", cmd_rename, r);
    register_command!("recat", "Re-categorize selected transaction", cmd_recat, r);
    register_command!("accounts", "Go to Accounts", cmd_accounts, r);
    register_command!(
        "add-txn",
        "Add manual transaction (e.g. :add-txn 2024-01-15 Coffee -4.50)",
        cmd_add_txn,
        r
    );
    register_command!(
        "delete-txn",
        "Delete selected transaction",
        cmd_delete_txn,
        r
    );
    register_command!(
        "export",
        "Export transactions to CSV (e.g. :export ~/budget.csv)",
        cmd_export,
        r
    );
    register_command!(
        "filter-account",
        "Filter transactions by account (e.g. :filter-account Chase)",
        cmd_filter_account,
        r
    );
    register_command!(
        "fa",
        "Filter transactions by account",
        cmd_filter_account,
        r
    );
    register_command!("next-month", "Go to next month", cmd_next_month, r);
    register_command!("prev-month", "Go to previous month", cmd_prev_month, r);
    register_command!("nav", "Open screen navigator", cmd_nav, r);
    register_command!(
        "delete-selected",
        "Delete all selected transactions",
        cmd_delete_selected,
        r
    );

    r
});

pub(crate) fn handle_command(input: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    let trimmed = input.trim();
    let mut parts = trimmed.splitn(2, ' ');
    let cmd_name = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("").trim();

    if let Some(cmd) = COMMANDS.get(cmd_name) {
        (cmd.run)(args, app, db)?;
    } else {
        // Try fuzzy match
        let suggestion = find_closest(cmd_name);
        app.set_status(format!(
            "Unknown command: :{cmd_name}. Did you mean :{suggestion}?"
        ));
    }

    Ok(())
}

fn find_closest(input: &str) -> String {
    COMMANDS
        .keys()
        .filter(|k| k.len() > 1) // skip single-letter aliases for suggestions
        .min_by_key(|k| levenshtein(input, k))
        .unwrap_or(&"help")
        .to_string()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0; b.len() + 1];

    for i in 1..=a.len() {
        curr[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b.len()]
}

// ── Command implementations ──────────────────────────────────

fn cmd_quit(_args: &str, app: &mut App, _db: &mut Database) -> anyhow::Result<()> {
    app.running = false;
    Ok(())
}

fn cmd_dashboard(_args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    app.screen = Screen::Dashboard;
    app.refresh_dashboard(db)?;
    Ok(())
}

fn cmd_transactions(_args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    app.screen = Screen::Transactions;
    app.refresh_transactions(db)?;
    Ok(())
}

fn cmd_import(_args: &str, app: &mut App, _db: &mut Database) -> anyhow::Result<()> {
    app.screen = Screen::Import;
    app.import_step = super::app::ImportStep::SelectFile;
    app.refresh_file_browser();
    Ok(())
}

fn cmd_categories(_args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    app.screen = Screen::Categories;
    app.refresh_categories(db)?;
    Ok(())
}

fn cmd_budgets(_args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    app.screen = Screen::Budgets;
    app.refresh_budgets(db)?;
    Ok(())
}

fn cmd_help(_args: &str, app: &mut App, _db: &mut Database) -> anyhow::Result<()> {
    app.show_help = true;
    Ok(())
}

fn cmd_nav(_args: &str, app: &mut App, _db: &mut Database) -> anyhow::Result<()> {
    app.nav_index = Screen::all()
        .iter()
        .position(|s| *s == app.screen)
        .unwrap_or(0);
    app.show_nav = true;
    Ok(())
}

fn cmd_month(args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    if args.is_empty() {
        // No args → reset to all-time
        app.current_month = None;
        app.refresh_dashboard(db)?;
        app.refresh_budgets(db)?;
        app.refresh_accounts_tab(db)?;
        app.set_status("Showing all time");
        return Ok(());
    }

    // Accept formats like "2024-01", "2024-1", "01", "1"
    let month = if args.len() <= 2 {
        let year = app.current_month.as_ref().map_or_else(
            || chrono::Local::now().format("%Y").to_string(),
            |m| m[..4].to_string(),
        );
        format!("{year}-{args:0>2}")
    } else {
        args.to_string()
    };

    // Validate by parsing as an actual date
    if chrono::NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d").is_ok() {
        let m = month[..7].to_string();
        app.set_status(format!("Switched to month: {m}"));
        app.current_month = Some(m);
        app.refresh_dashboard(db)?;
        app.refresh_budgets(db)?;
        app.refresh_accounts_tab(db)?;
    } else {
        app.set_status("Invalid month format. Use YYYY-MM (e.g. 2024-01)");
    }

    Ok(())
}

fn cmd_account(args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    if args.is_empty() {
        let types: Vec<&str> = AccountType::all().iter().map(|t| t.as_str()).collect();
        app.set_status(format!(
            "Usage: :account <name> [type]. Types: {}",
            types.join(", ")
        ));
        return Ok(());
    }

    let parts: Vec<&str> = args.rsplitn(2, ' ').collect();
    let (name, account_type) = if parts.len() == 2 {
        let possible_type = parts[0].to_lowercase();
        if [
            "checking",
            "savings",
            "credit",
            "investment",
            "cash",
            "loan",
        ]
        .contains(&possible_type.as_str())
        {
            (parts[1].to_string(), AccountType::parse(&possible_type))
        } else {
            (args.to_string(), AccountType::Checking)
        }
    } else {
        (args.to_string(), AccountType::Checking)
    };

    let account = Account::new(name.to_string(), account_type, String::new());
    db.insert_account(&account)?;
    app.refresh_accounts(db)?;
    app.set_status(format!("Created account: {name}"));
    Ok(())
}

fn cmd_rule(args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    if args.is_empty() {
        app.set_status("Usage: :rule <pattern> <category_name>");
        return Ok(());
    }

    let parts: Vec<&str> = args.rsplitn(2, ' ').collect();
    if parts.len() < 2 {
        app.set_status("Usage: :rule <pattern> <category_name>");
        return Ok(());
    }

    let category_name = parts[0];
    let pattern = parts[1].to_lowercase();

    let categories = db.get_categories()?;
    if let Some(cat) = Category::find_by_name(&categories, category_name) {
        let cat_id = match cat.id {
            Some(id) => id,
            None => {
                app.set_status("Category has no ID (this shouldn't happen)");
                return Ok(());
            }
        };
        let rule = ImportRule::new_contains(pattern.clone(), cat_id);
        db.insert_import_rule(&rule)?;
        app.refresh_categories(db)?;
        app.set_status(format!("Added rule: '{pattern}' -> {}", cat.name));
    } else {
        app.set_status(format!("Category '{category_name}' not found"));
    }

    Ok(())
}

fn cmd_search(args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    app.search_input = args.to_string();
    app.screen = Screen::Transactions;
    app.refresh_transactions(db)?;

    if args.is_empty() {
        app.set_status("Search cleared");
    } else {
        app.set_status(format!("Searching: {args}"));
    }

    Ok(())
}

fn cmd_budget(args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    if args.is_empty() {
        app.set_status(
            "Usage: :budget <category_name> <amount>. Example: :budget Food & Dining 500",
        );
        return Ok(());
    }

    // Last token is the amount, everything before is the category name
    let parts: Vec<&str> = args.rsplitn(2, ' ').collect();
    if parts.len() < 2 {
        app.set_status("Usage: :budget <category_name> <amount>");
        return Ok(());
    }

    let amount_str = parts[0];
    let category_name = parts[1];

    let amount = match Decimal::from_str(amount_str) {
        Ok(a) => a,
        Err(_) => {
            app.set_status(format!("Invalid amount: {amount_str}"));
            return Ok(());
        }
    };

    let categories = db.get_categories()?;
    if let Some(cat) = Category::find_by_name(&categories, category_name) {
        let cat_id = match cat.id {
            Some(id) => id,
            None => {
                app.set_status("Category has no ID (this shouldn't happen)");
                return Ok(());
            }
        };
        let budget_month = app.current_month.clone().unwrap_or_else(|| {
            chrono::Local::now().format("%Y-%m").to_string()
        });
        let budget = Budget::new(cat_id, budget_month.clone(), amount);
        db.upsert_budget(&budget)?;
        app.refresh_budgets(db)?;
        app.screen = Screen::Budgets;
        app.set_status(format!(
            "Budget set: {} = ${amount} for {budget_month}",
            cat.name
        ));
    } else {
        app.set_status(format!("Category '{category_name}' not found"));
    }

    Ok(())
}

fn cmd_delete_budget(_args: &str, app: &mut App, _db: &mut Database) -> anyhow::Result<()> {
    if app.budgets.is_empty() {
        app.set_status("No budgets to delete");
        return Ok(());
    }

    if let Some(budget) = app.budgets.get(app.budget_index) {
        if let Some(id) = budget.id {
            let cat_name = Category::find_by_id(&app.categories, budget.category_id)
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown");
            app.confirm_message = format!("Delete budget for '{cat_name}'?");
            app.pending_action = Some(PendingAction::DeleteBudget {
                id,
                name: cat_name.to_string(),
            });
            app.input_mode = InputMode::Confirm;
        }
    }

    Ok(())
}

fn cmd_category(args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    if args.is_empty() {
        app.set_status("Usage: :category <name>. Creates a new top-level category");
        return Ok(());
    }

    let cat = Category::new(args.to_string());
    db.insert_category(&cat)?;
    app.refresh_categories(db)?;
    app.set_status(format!("Created category: {args}"));
    Ok(())
}

fn cmd_delete_rule(_args: &str, app: &mut App, _db: &mut Database) -> anyhow::Result<()> {
    if app.import_rules.is_empty() {
        app.set_status("No rules to delete");
        return Ok(());
    }

    if let Some(rule) = app.import_rules.get(app.rule_index) {
        if let Some(id) = rule.id {
            let pattern = rule.pattern.clone();
            app.confirm_message = format!("Delete rule '{pattern}'?");
            app.pending_action = Some(PendingAction::DeleteRule { id, pattern });
            app.input_mode = InputMode::Confirm;
        }
    }

    Ok(())
}

fn cmd_regex_rule(args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    if args.is_empty() {
        app.set_status("Usage: :regex-rule <pattern> <category_name>");
        return Ok(());
    }

    let parts: Vec<&str> = args.rsplitn(2, ' ').collect();
    if parts.len() < 2 {
        app.set_status("Usage: :regex-rule <pattern> <category_name>");
        return Ok(());
    }

    let category_name = parts[0];
    let pattern = parts[1].to_string();

    // Validate regex
    if regex::Regex::new(&pattern).is_err() {
        app.set_status(format!("Invalid regex: {pattern}"));
        return Ok(());
    }

    let categories = db.get_categories()?;
    if let Some(cat) = Category::find_by_name(&categories, category_name) {
        let cat_id = match cat.id {
            Some(id) => id,
            None => {
                app.set_status("Category has no ID (this shouldn't happen)");
                return Ok(());
            }
        };
        let rule = ImportRule::new_regex(pattern.clone(), cat_id);
        db.insert_import_rule(&rule)?;
        app.refresh_categories(db)?;
        app.set_status(format!("Added regex rule: /{pattern}/ -> {}", cat.name));
    } else {
        app.set_status(format!("Category '{category_name}' not found"));
    }

    Ok(())
}

fn cmd_rename(args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    if app.screen != Screen::Transactions || app.transactions.is_empty() {
        app.set_status("Navigate to Transactions and select one first");
        return Ok(());
    }

    if args.is_empty() {
        // Enter editing mode for inline rename
        if let Some(txn) = app.transactions.get(app.transaction_index) {
            app.command_input = txn.description.clone();
            app.input_mode = InputMode::Editing;
            app.set_status("Type new name, press Enter to confirm");
        }
        return Ok(());
    }

    if let Some(txn) = app.transactions.get(app.transaction_index) {
        if let Some(id) = txn.id {
            db.update_transaction_description(id, args)?;
            app.refresh_transactions(db)?;
            app.set_status(format!("Renamed transaction to: {args}"));
        }
    }

    Ok(())
}

fn cmd_recat(args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    if app.screen != Screen::Transactions || app.transactions.is_empty() {
        app.set_status("Navigate to Transactions and select one first");
        return Ok(());
    }

    if args.is_empty() {
        app.set_status("Usage: :recat <category_name>");
        return Ok(());
    }

    let categories = db.get_categories()?;

    // Try name match first, then ID match
    let cat = Category::find_by_name(&categories, args).or_else(|| {
        args.parse::<i64>()
            .ok()
            .and_then(|id| Category::find_by_id(&categories, id))
    });

    if let Some(cat) = cat {
        if let Some(txn) = app.transactions.get(app.transaction_index) {
            if let Some(txn_id) = txn.id {
                db.update_transaction_category(txn_id, cat.id)?;
                app.refresh_transactions(db)?;
                app.set_status(format!("Categorized as: {}", cat.name));
            }
        }
    } else {
        app.set_status(format!("Category '{args}' not found"));
    }

    Ok(())
}

fn cmd_accounts(_args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    app.screen = Screen::Accounts;
    app.refresh_accounts_tab(db)?;
    Ok(())
}

fn cmd_add_txn(args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    if args.is_empty() {
        app.set_status("Usage: :add-txn <date> <description> <amount>. Example: :add-txn 2024-01-15 Coffee -4.50");
        return Ok(());
    }

    let parts: Vec<&str> = args.splitn(3, ' ').collect();
    if parts.len() < 3 {
        app.set_status("Usage: :add-txn <date> <description> <amount>");
        return Ok(());
    }

    let date = parts[0];
    // The last token is the amount, middle is description
    let rest = parts[1..].join(" ");
    let rest_parts: Vec<&str> = rest.rsplitn(2, ' ').collect();
    if rest_parts.len() < 2 {
        app.set_status("Usage: :add-txn <date> <description> <amount>");
        return Ok(());
    }

    let amount_str = rest_parts[0];
    let description = rest_parts[1];

    let amount = match Decimal::from_str(amount_str) {
        Ok(a) => a,
        Err(_) => {
            app.set_status(format!("Invalid amount: {amount_str}"));
            return Ok(());
        }
    };

    let account_id = match app.accounts.get(app.account_index).and_then(|a| a.id) {
        Some(id) => id,
        None => {
            app.set_status("No account found. Create one with :account <name>");
            return Ok(());
        }
    };

    let account = db.get_account_by_id(account_id)?;
    let account_name = account.map(|a| a.name).unwrap_or_else(|| "Unknown".into());

    let txn = crate::models::Transaction {
        id: None,
        account_id,
        date: date.to_string(),
        description: description.to_string(),
        original_description: description.to_string(),
        amount,
        category_id: None,
        notes: String::new(),
        is_transfer: false,
        import_hash: format!("manual-{}-{}-{}", date, description, amount),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    db.insert_transaction(&txn)?;
    app.refresh_transactions(db)?;
    app.refresh_dashboard(db)?;
    app.set_status(format!(
        "Added transaction: {description} ${amount} to {account_name}"
    ));
    Ok(())
}

fn cmd_delete_txn(_args: &str, app: &mut App, _db: &mut Database) -> anyhow::Result<()> {
    if app.screen != Screen::Transactions || app.transactions.is_empty() {
        app.set_status("Navigate to Transactions and select one first");
        return Ok(());
    }

    if let Some(txn) = app.transactions.get(app.transaction_index) {
        if let Some(id) = txn.id {
            let desc = txn.description.clone();
            app.confirm_message = format!("Delete '{desc}'?");
            app.pending_action = Some(PendingAction::DeleteTransaction {
                id,
                description: desc,
            });
            app.input_mode = InputMode::Confirm;
        }
    }

    Ok(())
}

fn cmd_export(args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    let path = if args.is_empty() {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let suffix = app
            .current_month
            .as_deref()
            .unwrap_or("all");
        format!("{home}/budgetui-export-{suffix}.csv")
    } else {
        crate::run::shellexpand(args)
    };

    let count = db.export_to_csv(&path, app.current_month.as_deref())?;
    if count == 0 {
        app.set_status("No transactions to export");
    } else {
        app.set_status(format!("Exported {count} transactions to {path}"));
    }
    Ok(())
}

fn cmd_filter_account(args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    if args.is_empty() {
        // Clear filter
        app.transaction_filter_account = None;
        app.screen = Screen::Transactions;
        app.refresh_transactions(db)?;
        app.set_status("Account filter cleared - showing all transactions");
        return Ok(());
    }

    let accounts = db.get_accounts()?;
    let found = accounts
        .iter()
        .find(|a| a.name.to_lowercase() == args.to_lowercase());

    if let Some(acct) = found {
        app.transaction_filter_account = acct.id;
        app.screen = Screen::Transactions;
        app.transaction_index = 0;
        app.transaction_scroll = 0;
        app.refresh_transactions(db)?;
        app.set_status(format!("Filtering by account: {}", acct.name));
    } else {
        let names: Vec<&str> = accounts.iter().map(|a| a.name.as_str()).collect();
        app.set_status(format!(
            "Account not found. Available: {}",
            names.join(", ")
        ));
    }

    Ok(())
}

fn cmd_next_month(_args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    advance_month(app, db, 1)
}

fn cmd_prev_month(_args: &str, app: &mut App, db: &mut Database) -> anyhow::Result<()> {
    advance_month(app, db, -1)
}

fn cmd_delete_selected(_args: &str, app: &mut App, _db: &mut Database) -> anyhow::Result<()> {
    if app.screen != Screen::Transactions {
        app.set_status("Navigate to Transactions first");
        return Ok(());
    }

    if app.selected_transactions.is_empty() {
        app.set_status("No transactions selected. Use Space to select");
        return Ok(());
    }

    let ids: Vec<i64> = app.selected_transactions.iter().copied().collect();
    let count = ids.len();
    app.confirm_message = format!("Delete {count} selected transactions?");
    app.pending_action = Some(PendingAction::DeleteTransactions { ids, count });
    app.input_mode = InputMode::Confirm;

    Ok(())
}

fn advance_month(app: &mut App, db: &mut Database, delta: i32) -> anyhow::Result<()> {
    let base = app.current_month.as_ref().map_or_else(
        || chrono::Local::now().format("%Y-%m").to_string(),
        |m| m.clone(),
    );
    if let Ok(date) = chrono::NaiveDate::parse_from_str(&format!("{base}-01"), "%Y-%m-%d") {
        let new_date = if delta > 0 {
            date.checked_add_months(chrono::Months::new(1))
        } else {
            date.checked_sub_months(chrono::Months::new(1))
        };

        if let Some(d) = new_date {
            let m = d.format("%Y-%m").to_string();
            app.set_status(format!("Month: {m}"));
            app.current_month = Some(m);
            app.clear_selections();
            app.refresh_dashboard(db)?;
            app.refresh_budgets(db)?;
            app.refresh_accounts_tab(db)?;
        }
    }

    Ok(())
}

