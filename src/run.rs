use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::Path;

use crate::db::Database;
use crate::models::{Account, AccountType};
use crate::ui::app::{App, ImportStep, InputMode, Screen};
use crate::ui::commands;

// ── TUI mode ─────────────────────────────────────────────────

pub(crate) fn as_tui(db: &mut Database) -> Result<()> {
    let mut app = App::new();
    app.refresh_all(db)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app, db);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(ref e) = result {
        eprintln!("Error: {e:?}");
    }

    result
}

// ── CLI mode ─────────────────────────────────────────────────

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
            eprintln!("Unknown command: {other}");
            eprintln!();
            print_usage();
            std::process::exit(1);
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
        eprintln!("Usage: budgetui import <file.csv> [--account <name>]");
        std::process::exit(1);
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
            eprintln!("Multiple accounts found. Use --account <name> to specify:");
            for acct in &accounts {
                eprintln!("  --account \"{}\"  ({})", acct.name, acct.account_type);
            }
            std::process::exit(1);
        }
    };

    let mut txns = crate::import::CsvImporter::parse(&rows, &profile, account_id)?;
    println!("Parsed {} transactions", txns.len());

    // Auto-categorize
    let rules = db.get_import_rules()?;
    if !rules.is_empty() {
        let categorizer = crate::categorize::Categorizer::new(&rules);
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

    let txns = db.get_all_transactions_for_export(Some(&month))?;
    if txns.is_empty() {
        println!("No transactions for {month}");
        return Ok(());
    }

    let categories = db.get_categories()?;
    let accounts = db.get_accounts()?;

    let mut wtr = csv::Writer::from_path(&output_path).context("Failed to create export file")?;
    wtr.write_record([
        "Date",
        "Description",
        "Amount",
        "Category",
        "Account",
        "Notes",
    ])?;

    for txn in &txns {
        let cat_name = txn
            .category_id
            .and_then(|cid| categories.iter().find(|c| c.id == Some(cid)))
            .map(|c| c.name.as_str())
            .unwrap_or("");
        let acct_name = accounts
            .iter()
            .find(|a| a.id == Some(txn.account_id))
            .map(|a| a.name.as_str())
            .unwrap_or("");
        wtr.write_record([
            &txn.date,
            &txn.description,
            &txn.amount.to_string(),
            cat_name,
            acct_name,
            &txn.notes,
        ])?;
    }

    wtr.flush()?;
    println!("Exported {} transactions to {output_path}", txns.len());
    Ok(())
}

fn cli_summary(args: &[String], db: &mut Database) -> Result<()> {
    let month = args
        .first()
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m").to_string());

    let (income, expenses) = db.get_monthly_totals(&month)?;
    let net = income + expenses;
    let net_worth = db.get_net_worth()?;
    let spending = db.get_spending_by_category(&month)?;
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

fn shellexpand(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/{rest}")
    } else {
        path.to_string()
    }
}

// ── TUI event loop ───────────────────────────────────────────

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    db: &mut Database,
) -> Result<()> {
    while app.running {
        terminal.draw(|f| {
            // Update visible rows based on terminal height (subtract outer chrome only)
            let content_height = f.area().height.saturating_sub(3) as usize; // 1 tab + 1 status + 1 cmd
            app.visible_rows = content_height.max(1);
            crate::ui::render::render(f, app);
        })?;

        if let Event::Key(key) = event::read()? {
            if app.show_help {
                app.show_help = false;
                continue;
            }
            if app.show_nav {
                handle_nav_input(key, app, db)?;
                continue;
            }
            match app.input_mode {
                InputMode::Normal => handle_normal_input(key, app, db)?,
                InputMode::Command => handle_command_input(key, app, db)?,
                InputMode::Search => handle_search_input(key, app, db)?,
                InputMode::Editing => handle_editing_input(key, app, db)?,
                InputMode::Confirm => handle_confirm_input(key, app, db)?,
            }
        }
    }
    Ok(())
}

// ── Input handlers ───────────────────────────────────────────

fn handle_normal_input(key: event::KeyEvent, app: &mut App, db: &mut Database) -> Result<()> {
    // File browser path input has its own key handling
    if app.screen == Screen::Import
        && app.import_step == ImportStep::SelectFile
        && app.file_browser_input_focused
    {
        return handle_file_browser_input(key, app);
    }

    // Categorize step has its own key handling (category picker + new category input)
    if app.screen == Screen::Import && app.import_step == ImportStep::Categorize {
        return handle_categorize_input(key, app, db);
    }

    // Account picker step has its own key handling (list + new account form)
    if app.screen == Screen::Import && app.import_step == ImportStep::SelectAccount {
        return handle_select_account_input(key, app, db);
    }

    match key.code {
        KeyCode::Char(':') => {
            app.input_mode = InputMode::Command;
            app.command_input.clear();
        }
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
            app.search_input.clear();
        }
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.running = false;
        }
        KeyCode::Char('j') | KeyCode::Down => handle_move_down(app),
        KeyCode::Char('k') | KeyCode::Up => handle_move_up(app),
        KeyCode::Char('1') => switch_screen(app, db, Screen::Dashboard)?,
        KeyCode::Char('2') => switch_screen(app, db, Screen::Accounts)?,
        KeyCode::Char('3') => switch_screen(app, db, Screen::Transactions)?,
        KeyCode::Char('4') => switch_screen(app, db, Screen::Import)?,
        KeyCode::Char('5') => switch_screen(app, db, Screen::Categories)?,
        KeyCode::Char('6') => switch_screen(app, db, Screen::Budgets)?,
        KeyCode::Tab
            if app.screen == Screen::Import && app.import_step == ImportStep::SelectFile =>
        {
            app.file_browser_input_focused = true;
        }
        KeyCode::Tab => {
            let screens = Screen::all();
            let idx = screens.iter().position(|s| *s == app.screen).unwrap_or(0);
            let next = (idx + 1) % screens.len();
            switch_screen(app, db, screens[next])?;
        }
        KeyCode::BackTab => {
            let screens = Screen::all();
            let idx = screens.iter().position(|s| *s == app.screen).unwrap_or(0);
            let prev = if idx == 0 { screens.len() - 1 } else { idx - 1 };
            switch_screen(app, db, screens[prev])?;
        }
        KeyCode::Enter => handle_enter(app, db)?,
        KeyCode::Esc => handle_escape(app),
        KeyCode::Char('+') | KeyCode::Char('=') => handle_adjust_field(app, 1),
        KeyCode::Char('-') => handle_adjust_field(app, -1),
        KeyCode::Char('.')
            if app.screen == Screen::Import && app.import_step == ImportStep::SelectFile =>
        {
            app.file_browser_show_hidden = !app.file_browser_show_hidden;
            app.refresh_file_browser();
        }
        KeyCode::Char('g') => handle_goto_top(app),
        KeyCode::Char('G') => handle_goto_bottom(app),
        KeyCode::Char('?') => {
            app.show_help = true;
        }
        KeyCode::Char('r') if app.screen == Screen::Categories => {
            app.category_view_rules = !app.category_view_rules;
        }
        KeyCode::Char('n') if app.screen == Screen::Dashboard => {
            if !app.accounts.is_empty() {
                app.account_index = (app.account_index + 1) % app.accounts.len();
                let name = &app.accounts[app.account_index].name;
                app.set_status(format!("Active account: {name}"));
            }
        }
        KeyCode::Char('p') if app.screen == Screen::Dashboard => {
            if !app.accounts.is_empty() {
                app.account_index = if app.account_index == 0 {
                    app.accounts.len() - 1
                } else {
                    app.account_index - 1
                };
                let name = &app.accounts[app.account_index].name;
                app.set_status(format!("Active account: {name}"));
            }
        }
        KeyCode::Char('H') => {
            commands::handle_command("prev-month", app, db)?;
        }
        KeyCode::Char('L') => {
            commands::handle_command("next-month", app, db)?;
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let half_page = app.visible_rows / 2;
            for _ in 0..half_page {
                handle_move_down(app);
            }
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let half_page = app.visible_rows / 2;
            for _ in 0..half_page {
                handle_move_up(app);
            }
        }
        KeyCode::Char('D') if app.screen == Screen::Transactions => {
            commands::handle_command("delete-txn", app, db)?;
        }
        KeyCode::Char('i')
            if app.screen == Screen::Import && app.import_step == ImportStep::Complete =>
        {
            app.import_step = ImportStep::SelectFile;
            app.refresh_file_browser();
        }
        _ => {}
    }
    Ok(())
}

fn handle_file_browser_input(key: event::KeyEvent, app: &mut App) -> Result<()> {
    match key.code {
        KeyCode::Char(c) => {
            app.file_browser_filter.push(c);
            app.file_browser_index = 0;
            app.file_browser_scroll = 0;
        }
        KeyCode::Backspace => {
            if app.file_browser_filter.pop().is_none() {
                // Filter was already empty — go to parent directory
                if let Some(parent) = app.file_browser_path.parent().map(|p| p.to_path_buf()) {
                    app.file_browser_path = parent;
                    app.refresh_file_browser();
                }
            }
            app.file_browser_index = 0;
            app.file_browser_scroll = 0;
        }
        KeyCode::Down | KeyCode::Tab => {
            app.file_browser_input_focused = false;
        }
        KeyCode::Esc => {
            if !app.file_browser_filter.is_empty() {
                app.file_browser_filter.clear();
                app.file_browser_index = 0;
                app.file_browser_scroll = 0;
            } else {
                app.file_browser_input_focused = false;
            }
        }
        KeyCode::Enter => {
            let filtered = app.file_browser_filtered();
            if filtered.len() == 1 {
                let path = app.file_browser_entries[filtered[0]].clone();
                if path.is_dir() {
                    app.file_browser_path = path;
                    app.refresh_file_browser();
                } else {
                    app.import_path = path.display().to_string();
                    if let Err(e) = app.load_import_file() {
                        app.set_status(format!("Error loading file: {e}"));
                    }
                }
            } else {
                // Multiple matches — switch to list to pick one
                app.file_browser_input_focused = false;
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_categorize_input(key: event::KeyEvent, app: &mut App, db: &mut Database) -> Result<()> {
    if app.import_cat_creating {
        // Typing a new category name
        match key.code {
            KeyCode::Char(c) => {
                app.import_cat_new_name.push(c);
            }
            KeyCode::Backspace => {
                if app.import_cat_new_name.pop().is_none() {
                    app.import_cat_creating = false;
                }
            }
            KeyCode::Esc => {
                app.import_cat_creating = false;
                app.import_cat_new_name.clear();
            }
            KeyCode::Enter => {
                let name = app.import_cat_new_name.trim().to_string();
                if !name.is_empty() {
                    // Create the category in DB
                    let cat = crate::models::Category::new(name.clone());
                    let cat_id = db.insert_category(&cat)?;

                    // Generate and save a rule for this description
                    if let Some((desc, _)) = app.import_cat_descriptions.get(app.import_cat_index) {
                        if let Ok(pattern) = crate::categorize::suggest_rule(desc) {
                            let rule = crate::models::ImportRule::new_contains(pattern, cat_id);
                            db.insert_import_rule(&rule)?;
                        }
                    }

                    // Apply category to all matching transactions
                    app.apply_category_to_current(cat_id);
                    app.refresh_categories(db)?;

                    let count = app
                        .import_cat_descriptions
                        .get(app.import_cat_index)
                        .map(|(_, c)| *c)
                        .unwrap_or(0);
                    app.set_status(format!(
                        "Created '{name}' and categorized {count} transaction{}",
                        if count == 1 { "" } else { "s" }
                    ));

                    app.import_cat_creating = false;
                    app.import_cat_new_name.clear();

                    if !app.advance_categorize() {
                        commit_import(app, db)?;
                    }
                }
            }
            _ => {}
        }
        return Ok(());
    }

    // Normal category picker navigation
    let page = app.categorize_visible_rows();
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.import_cat_selected + 1 < app.categories.len() {
                app.import_cat_selected += 1;
                if app.import_cat_selected >= app.import_cat_scroll + page {
                    app.import_cat_scroll = app.import_cat_selected.saturating_sub(page - 1);
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.import_cat_selected = app.import_cat_selected.saturating_sub(1);
            if app.import_cat_selected < app.import_cat_scroll {
                app.import_cat_scroll = app.import_cat_selected;
            }
        }
        KeyCode::Char('g') => {
            app.import_cat_selected = 0;
            app.import_cat_scroll = 0;
        }
        KeyCode::Char('G') => {
            if !app.categories.is_empty() {
                app.import_cat_selected = app.categories.len() - 1;
                app.import_cat_scroll = app.import_cat_selected.saturating_sub(page - 1);
            }
        }
        KeyCode::Char('s') => {
            // Skip this description
            if !app.advance_categorize() {
                commit_import(app, db)?;
            } else {
                app.set_status("Skipped — moving to next");
            }
        }
        KeyCode::Char('S') => {
            // Skip all remaining
            commit_import(app, db)?;
        }
        KeyCode::Char('n') => {
            // Start typing a new category name
            app.import_cat_creating = true;
            app.import_cat_new_name.clear();
        }
        KeyCode::Enter => {
            // Assign the selected category
            if let Some(cat) = app.categories.get(app.import_cat_selected) {
                if let Some(cat_id) = cat.id {
                    let cat_name = cat.name.clone();

                    // Generate and save a rule
                    if let Some((desc, _)) = app.import_cat_descriptions.get(app.import_cat_index) {
                        if let Ok(pattern) = crate::categorize::suggest_rule(desc) {
                            let rule =
                                crate::models::ImportRule::new_contains(pattern.clone(), cat_id);
                            db.insert_import_rule(&rule)?;
                            app.refresh_categories(db)?;
                        }
                    }

                    // Apply to all matching transactions
                    app.apply_category_to_current(cat_id);

                    let count = app
                        .import_cat_descriptions
                        .get(app.import_cat_index)
                        .map(|(_, c)| *c)
                        .unwrap_or(0);
                    app.set_status(format!(
                        "Categorized {count} transaction{} as '{cat_name}'",
                        if count == 1 { "" } else { "s" }
                    ));

                    if !app.advance_categorize() {
                        commit_import(app, db)?;
                    }
                }
            }
        }
        KeyCode::Esc => {
            // Go back to preview step (abandon categorization, keep any changes already made)
            app.import_step = ImportStep::Preview;
            app.set_status("Back to preview — categories already assigned will be kept");
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let half_page = page / 2;
            for _ in 0..half_page {
                if app.import_cat_selected + 1 < app.categories.len() {
                    app.import_cat_selected += 1;
                }
            }
            if app.import_cat_selected >= app.import_cat_scroll + page {
                app.import_cat_scroll = app.import_cat_selected.saturating_sub(page - 1);
            }
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let half_page = page / 2;
            for _ in 0..half_page {
                app.import_cat_selected = app.import_cat_selected.saturating_sub(1);
            }
            if app.import_cat_selected < app.import_cat_scroll {
                app.import_cat_scroll = app.import_cat_selected;
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_select_account_input(
    key: event::KeyEvent,
    app: &mut App,
    db: &mut Database,
) -> Result<()> {
    if app.import_creating_account {
        // New account form: typing name, cycling type, or confirming
        match key.code {
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let types = AccountType::all();
                app.import_new_account_type = (app.import_new_account_type + 1) % types.len();
            }
            KeyCode::Char('-') => {
                let types = AccountType::all();
                app.import_new_account_type = if app.import_new_account_type == 0 {
                    types.len() - 1
                } else {
                    app.import_new_account_type - 1
                };
            }
            KeyCode::Tab => {
                // Tab cycles type forward (same as +)
                let types = AccountType::all();
                app.import_new_account_type = (app.import_new_account_type + 1) % types.len();
            }
            KeyCode::Char(c) => {
                app.import_new_account_name.push(c);
            }
            KeyCode::Backspace => {
                if app.import_new_account_name.pop().is_none() {
                    app.import_creating_account = false;
                }
            }
            KeyCode::Esc => {
                app.import_creating_account = false;
                app.import_new_account_name.clear();
            }
            KeyCode::Enter => {
                let name = app.import_new_account_name.trim().to_string();
                if !name.is_empty() {
                    let acct_type = AccountType::all()
                        .get(app.import_new_account_type)
                        .cloned()
                        .unwrap_or(AccountType::Checking);
                    let is_credit =
                        matches!(acct_type, AccountType::CreditCard | AccountType::Loan);
                    let acct = Account::new(name.clone(), acct_type, String::new());
                    let id = db.insert_account(&acct)?;
                    app.import_account_id = Some(id);
                    app.refresh_accounts(db)?;

                    // Derive is_credit_account from new account type
                    app.import_profile.is_credit_account = is_credit;
                    if app.import_detected_bank.is_none() {
                        app.import_profile.negate_amounts = is_credit;
                    }

                    app.import_creating_account = false;
                    app.import_new_account_name.clear();
                    app.set_status(format!("Created account: {name}"));

                    if let Err(e) = app.generate_import_preview() {
                        app.set_status(format!("Error generating preview: {e}"));
                    }
                }
            }
            _ => {}
        }
        return Ok(());
    }

    // Normal account list navigation
    let page = app.import_account_page();
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.import_account_index + 1 < app.accounts.len() {
                app.import_account_index += 1;
                if app.import_account_index >= app.import_account_scroll + page {
                    app.import_account_scroll = app.import_account_index.saturating_sub(page - 1);
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.import_account_index = app.import_account_index.saturating_sub(1);
            if app.import_account_index < app.import_account_scroll {
                app.import_account_scroll = app.import_account_index;
            }
        }
        KeyCode::Char('g') => {
            app.import_account_index = 0;
            app.import_account_scroll = 0;
        }
        KeyCode::Char('G') => {
            if !app.accounts.is_empty() {
                app.import_account_index = app.accounts.len() - 1;
                app.import_account_scroll = app
                    .import_account_index
                    .saturating_sub(page.saturating_sub(1));
            }
        }
        KeyCode::Char('n') => {
            app.import_creating_account = true;
            app.import_new_account_name.clear();
        }
        KeyCode::Enter => {
            if let Some(acct) = app.accounts.get(app.import_account_index) {
                app.import_account_id = acct.id;
                let is_credit = matches!(
                    acct.account_type,
                    AccountType::CreditCard | AccountType::Loan
                );
                app.import_profile.is_credit_account = is_credit;
                if app.import_detected_bank.is_none() {
                    app.import_profile.negate_amounts = is_credit;
                }
                let name = acct.name.clone();
                app.set_status(format!("Using account: {name}"));
                if let Err(e) = app.generate_import_preview() {
                    app.set_status(format!("Error generating preview: {e}"));
                }
            } else if app.accounts.is_empty() {
                // No accounts — prompt to create one
                app.import_creating_account = true;
                app.import_new_account_name.clear();
            }
        }
        KeyCode::Esc => {
            app.import_step = ImportStep::MapColumns;
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let half = page / 2;
            for _ in 0..half {
                if app.import_account_index + 1 < app.accounts.len() {
                    app.import_account_index += 1;
                }
            }
            if app.import_account_index >= app.import_account_scroll + page {
                app.import_account_scroll = app.import_account_index.saturating_sub(page - 1);
            }
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let half = page / 2;
            for _ in 0..half {
                app.import_account_index = app.import_account_index.saturating_sub(1);
            }
            if app.import_account_index < app.import_account_scroll {
                app.import_account_scroll = app.import_account_index;
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_command_input(key: event::KeyEvent, app: &mut App, db: &mut Database) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            let input = app.command_input.clone();
            app.input_mode = InputMode::Normal;
            app.command_input.clear();
            commands::handle_command(&input, app, db)?;
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.command_input.clear();
        }
        KeyCode::Backspace => {
            app.command_input.pop();
            if app.command_input.is_empty() {
                app.input_mode = InputMode::Normal;
            }
        }
        KeyCode::Char(c) => {
            app.command_input.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_search_input(key: event::KeyEvent, app: &mut App, db: &mut Database) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
            app.screen = Screen::Transactions;
            app.refresh_transactions(db)?;
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.search_input.clear();
            app.refresh_transactions(db)?;
        }
        KeyCode::Backspace => {
            app.search_input.pop();
            // Live search: filter as you type
            app.screen = Screen::Transactions;
            app.transaction_index = 0;
            app.transaction_scroll = 0;
            app.refresh_transactions(db)?;
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
            // Live search: filter as you type
            app.screen = Screen::Transactions;
            app.transaction_index = 0;
            app.transaction_scroll = 0;
            app.refresh_transactions(db)?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_editing_input(key: event::KeyEvent, app: &mut App, db: &mut Database) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            let new_name = app.command_input.clone();
            if !new_name.is_empty() {
                if let Some(txn) = app.transactions.get(app.transaction_index) {
                    if let Some(id) = txn.id {
                        db.update_transaction_description(id, &new_name)?;
                        app.refresh_transactions(db)?;
                        app.set_status(format!("Renamed to: {new_name}"));
                    }
                }
            }
            app.command_input.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Esc => {
            app.command_input.clear();
            app.input_mode = InputMode::Normal;
            app.set_status("Edit cancelled");
        }
        KeyCode::Backspace => {
            app.command_input.pop();
        }
        KeyCode::Char(c) => {
            app.command_input.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_nav_input(key: event::KeyEvent, app: &mut App, db: &mut Database) -> Result<()> {
    let screens = Screen::all();
    match key.code {
        KeyCode::Char(c @ '1'..='9') => {
            let idx = (c as usize) - ('1' as usize);
            if let Some(&screen) = screens.get(idx) {
                app.show_nav = false;
                switch_screen(app, db, screen)?;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.nav_index + 1 < screens.len() {
                app.nav_index += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.nav_index = app.nav_index.saturating_sub(1);
        }
        KeyCode::Enter => {
            if let Some(&screen) = screens.get(app.nav_index) {
                app.show_nav = false;
                switch_screen(app, db, screen)?;
            }
        }
        _ => {
            app.show_nav = false;
        }
    }
    Ok(())
}

fn handle_confirm_input(key: event::KeyEvent, app: &mut App, db: &mut Database) -> Result<()> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(action) = app.pending_action.take() {
                use crate::ui::app::PendingAction;
                match action {
                    PendingAction::DeleteTransaction { id, description } => {
                        db.delete_transaction(id)?;
                        app.refresh_transactions(db)?;
                        app.refresh_dashboard(db)?;
                        if app.transaction_index > 0
                            && app.transaction_index >= app.transactions.len()
                        {
                            app.transaction_index = app.transactions.len().saturating_sub(1);
                        }
                        app.set_status(format!("Deleted: {description}"));
                    }
                    PendingAction::DeleteBudget { id, name } => {
                        db.delete_budget(id)?;
                        app.refresh_budgets(db)?;
                        if app.budget_index >= app.budgets.len() {
                            app.budget_index = app.budgets.len().saturating_sub(1);
                        }
                        app.set_status(format!("Deleted budget: {name}"));
                    }
                    PendingAction::DeleteRule { id, pattern } => {
                        db.delete_import_rule(id)?;
                        app.refresh_categories(db)?;
                        if app.rule_index >= app.import_rules.len() {
                            app.rule_index = app.import_rules.len().saturating_sub(1);
                        }
                        app.set_status(format!("Deleted rule: '{pattern}'"));
                    }
                    PendingAction::ImportCommit => {
                        // Run existing rules against the preview transactions
                        let rules = db.get_import_rules()?;
                        let categorizer = crate::categorize::Categorizer::new(&rules);
                        categorizer.categorize_batch(&mut app.import_preview);

                        // Check for uncategorized transactions — enter interactive categorize step
                        if app.prepare_categorize_step() {
                            let total = app.import_cat_descriptions.len();
                            app.import_step = ImportStep::Categorize;
                            app.set_status(format!(
                                "{total} unique description{} to categorize",
                                if total == 1 { "" } else { "s" }
                            ));
                        } else {
                            // All categorized — commit immediately
                            commit_import(app, db)?;
                        }
                    }
                }
            }
            app.input_mode = InputMode::Normal;
            app.confirm_message.clear();
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.pending_action = None;
            app.input_mode = InputMode::Normal;
            app.confirm_message.clear();
            app.set_status("Cancelled");
        }
        _ => {} // Ignore other keys
    }
    Ok(())
}

// ── Navigation helpers ───────────────────────────────────────

fn switch_screen(app: &mut App, db: &mut Database, screen: Screen) -> Result<()> {
    app.screen = screen;
    match screen {
        Screen::Dashboard => app.refresh_dashboard(db)?,
        Screen::Accounts => app.refresh_accounts_tab(db)?,
        Screen::Transactions => app.refresh_transactions(db)?,
        Screen::Import => {
            app.import_step = ImportStep::SelectFile;
            app.import_account_id = None;
            app.import_account_index = 0;
            app.import_account_scroll = 0;
            app.import_creating_account = false;
            app.import_new_account_name.clear();
            app.import_detected_bank = None;
            app.refresh_file_browser();
        }
        Screen::Categories => app.refresh_categories(db)?,
        Screen::Budgets => app.refresh_budgets(db)?,
    }
    Ok(())
}

fn handle_move_down(app: &mut App) {
    match app.screen {
        Screen::Accounts => {
            if app.accounts_tab_index + 1 < app.account_snapshots.len() {
                app.accounts_tab_index += 1;
                let page = app.accounts_page();
                if app.accounts_tab_index >= app.accounts_tab_scroll + page {
                    app.accounts_tab_scroll = app.accounts_tab_index.saturating_sub(page - 1);
                }
            }
        }
        Screen::Transactions => {
            if app.transaction_index + 1 < app.transactions.len() {
                app.transaction_index += 1;
                let page = app.transaction_page();
                if app.transaction_index >= app.transaction_scroll + page {
                    app.transaction_scroll = app.transaction_index.saturating_sub(page - 1);
                }
            }
        }
        Screen::Categories => {
            if app.category_view_rules {
                if app.rule_index + 1 < app.import_rules.len() {
                    app.rule_index += 1;
                    let page = app.rule_page();
                    if app.rule_index >= app.rule_scroll + page {
                        app.rule_scroll = app.rule_index.saturating_sub(page - 1);
                    }
                }
            } else if app.category_index + 1 < app.categories.len() {
                app.category_index += 1;
                let page = app.category_page();
                if app.category_index >= app.category_scroll + page {
                    app.category_scroll = app.category_index.saturating_sub(page - 1);
                }
            }
        }
        Screen::Import => match app.import_step {
            ImportStep::SelectFile => {
                let filtered_len = app.file_browser_filtered().len();
                if app.file_browser_index + 1 < filtered_len {
                    app.file_browser_index += 1;
                    let page = app.file_browser_page();
                    if app.file_browser_index >= app.file_browser_scroll + page {
                        app.file_browser_scroll = app.file_browser_index.saturating_sub(page - 1);
                    }
                }
            }
            ImportStep::MapColumns => {
                if app.import_selected_field < 6 {
                    app.import_selected_field += 1;
                }
            }
            _ => {}
        },
        Screen::Budgets => {
            if app.budget_index + 1 < app.budgets.len() {
                app.budget_index += 1;
                let page = app.budget_page();
                if app.budget_index >= app.budget_scroll + page {
                    app.budget_scroll = app.budget_index.saturating_sub(page - 1);
                }
            }
        }
        _ => {}
    }
}

fn handle_move_up(app: &mut App) {
    match app.screen {
        Screen::Accounts => {
            app.accounts_tab_index = app.accounts_tab_index.saturating_sub(1);
            if app.accounts_tab_index < app.accounts_tab_scroll {
                app.accounts_tab_scroll = app.accounts_tab_index;
            }
        }
        Screen::Transactions => {
            app.transaction_index = app.transaction_index.saturating_sub(1);
            if app.transaction_index < app.transaction_scroll {
                app.transaction_scroll = app.transaction_index;
            }
        }
        Screen::Categories => {
            if app.category_view_rules {
                app.rule_index = app.rule_index.saturating_sub(1);
                if app.rule_index < app.rule_scroll {
                    app.rule_scroll = app.rule_index;
                }
            } else {
                app.category_index = app.category_index.saturating_sub(1);
                if app.category_index < app.category_scroll {
                    app.category_scroll = app.category_index;
                }
            }
        }
        Screen::Import => match app.import_step {
            ImportStep::SelectFile => {
                if app.file_browser_index == 0 {
                    // At top of list — focus the filter input
                    app.file_browser_input_focused = true;
                } else {
                    app.file_browser_index = app.file_browser_index.saturating_sub(1);
                    if app.file_browser_index < app.file_browser_scroll {
                        app.file_browser_scroll = app.file_browser_index;
                    }
                }
            }
            ImportStep::MapColumns => {
                app.import_selected_field = app.import_selected_field.saturating_sub(1);
            }
            _ => {}
        },
        Screen::Budgets => {
            app.budget_index = app.budget_index.saturating_sub(1);
            if app.budget_index < app.budget_scroll {
                app.budget_scroll = app.budget_index;
            }
        }
        _ => {}
    }
}

fn handle_enter(app: &mut App, db: &mut Database) -> Result<()> {
    if app.screen == Screen::Accounts {
        // Drill into account's transactions
        if let Some(snap) = app.account_snapshots.get(app.accounts_tab_index) {
            let account_id = snap.account.id;
            let account_name = snap.account.name.clone();
            app.transaction_filter_account = account_id;
            app.transaction_index = 0;
            app.transaction_scroll = 0;
            app.screen = Screen::Transactions;
            app.refresh_transactions(db)?;
            app.set_status(format!("Filtered by: {account_name}"));
        }
        return Ok(());
    }

    if app.screen == Screen::Import {
        match app.import_step {
            ImportStep::SelectFile => {
                let filtered = app.file_browser_filtered();
                if let Some(&real_idx) = filtered.get(app.file_browser_index) {
                    let path = app.file_browser_entries[real_idx].clone();
                    if path.is_dir() {
                        app.file_browser_path = path;
                        app.refresh_file_browser();
                    } else {
                        app.import_path = path.display().to_string();
                        if let Err(e) = app.load_import_file() {
                            app.set_status(format!("Error loading file: {e}"));
                        }
                    }
                }
            }
            ImportStep::MapColumns => {
                // Advance to account selection step
                app.refresh_accounts(db)?;
                app.import_account_index = 0;
                app.import_account_scroll = 0;
                app.import_creating_account = false;
                app.import_new_account_name.clear();

                // Pre-select: try to match detected bank name
                if let Some(ref bank) = app.import_detected_bank {
                    let lower = bank.to_lowercase();
                    if let Some(pos) = app
                        .accounts
                        .iter()
                        .position(|a| a.name.to_lowercase() == lower)
                    {
                        app.import_account_index = pos;
                    }
                }

                // Pre-set account type based on profile
                if app.import_profile.is_credit_account {
                    // Default to CreditCard type for new accounts
                    app.import_new_account_type = AccountType::all()
                        .iter()
                        .position(|t| *t == AccountType::CreditCard)
                        .unwrap_or(0);
                } else {
                    app.import_new_account_type = 0; // Checking
                }

                app.import_step = ImportStep::SelectAccount;
            }
            ImportStep::SelectAccount => {
                // Handled by handle_select_account_input (early return)
            }
            ImportStep::Preview => {
                app.confirm_message = format!("Import {} transactions?", app.import_preview.len());
                app.pending_action = Some(crate::ui::app::PendingAction::ImportCommit);
                app.input_mode = InputMode::Confirm;
            }
            ImportStep::Categorize => {
                // Handled by handle_categorize_input (early return)
            }
            ImportStep::Complete => {
                app.screen = Screen::Transactions;
                app.refresh_transactions(db)?;
            }
        }
    }
    Ok(())
}

fn handle_escape(app: &mut App) {
    match app.screen {
        Screen::Import => match app.import_step {
            ImportStep::SelectFile => {
                if !app.file_browser_filter.is_empty() {
                    app.file_browser_filter.clear();
                    app.file_browser_index = 0;
                    app.file_browser_scroll = 0;
                } else {
                    app.screen = Screen::Dashboard;
                }
            }
            ImportStep::MapColumns => {
                app.import_step = ImportStep::SelectFile;
            }
            ImportStep::SelectAccount => {
                // Handled in handle_select_account_input — safety fallback
                app.import_step = ImportStep::MapColumns;
            }
            ImportStep::Preview => {
                app.import_step = ImportStep::SelectAccount;
            }
            ImportStep::Categorize => {
                // Handled in handle_categorize_input — this shouldn't fire
                // but as safety fallback, go back to preview
                app.import_step = ImportStep::Preview;
            }
            ImportStep::Complete => {
                app.screen = Screen::Dashboard;
            }
        },
        Screen::Transactions if app.transaction_filter_account.is_some() => {
            // Clear account filter when pressing Esc on filtered Transactions view
            app.transaction_filter_account = None;
            app.set_status("Account filter cleared");
        }
        _ => {
            app.status_message.clear();
            app.search_input.clear();
        }
    }
}

fn handle_adjust_field(app: &mut App, delta: i32) {
    if app.screen != Screen::Import || app.import_step != ImportStep::MapColumns {
        return;
    }

    let max_col = app.import_headers.len().saturating_sub(1);

    match app.import_selected_field {
        0 => {
            app.import_profile.date_column =
                adjust_usize(app.import_profile.date_column, delta, max_col);
        }
        1 => {
            app.import_profile.description_column =
                adjust_usize(app.import_profile.description_column, delta, max_col);
        }
        2 => {
            app.import_profile.amount_column =
                adjust_optional(app.import_profile.amount_column, delta, max_col);
        }
        3 => {
            app.import_profile.debit_column =
                adjust_optional(app.import_profile.debit_column, delta, max_col);
        }
        4 => {
            app.import_profile.credit_column =
                adjust_optional(app.import_profile.credit_column, delta, max_col);
        }
        5 => {
            let formats = ["%m/%d/%Y", "%Y-%m-%d", "%m-%d-%Y", "%d/%m/%Y", "%m/%d/%y"];
            let current = formats
                .iter()
                .position(|f| *f == app.import_profile.date_format)
                .unwrap_or(0);
            let next = if delta > 0 {
                (current + 1) % formats.len()
            } else if current == 0 {
                formats.len() - 1
            } else {
                current - 1
            };
            app.import_profile.date_format = formats[next].to_string();
        }
        6 => {
            app.import_profile.has_header = !app.import_profile.has_header;
        }
        _ => {}
    }
}

fn adjust_usize(val: usize, delta: i32, max: usize) -> usize {
    let new_val = val as i32 + delta;
    if new_val < 0 {
        0
    } else {
        (new_val as usize).min(max)
    }
}

fn adjust_optional(val: Option<usize>, delta: i32, max: usize) -> Option<usize> {
    match val {
        Some(col) => {
            let new_val = col as i32 + delta;
            if new_val < 0 {
                None
            } else {
                Some((new_val as usize).min(max))
            }
        }
        None => {
            if delta > 0 {
                Some(0)
            } else {
                None
            }
        }
    }
}

fn commit_import(app: &mut App, db: &mut Database) -> Result<()> {
    let txns = &app.import_preview;
    let count = db.insert_transactions_batch(txns)?;
    let dupes = txns.len() - count;
    app.import_step = ImportStep::Complete;
    app.set_status(format!(
        "Imported {count} new transactions ({dupes} duplicates skipped)"
    ));
    app.refresh_all(db)?;
    Ok(())
}

fn handle_goto_top(app: &mut App) {
    match app.screen {
        Screen::Accounts => {
            app.accounts_tab_index = 0;
            app.accounts_tab_scroll = 0;
        }
        Screen::Transactions => {
            app.transaction_index = 0;
            app.transaction_scroll = 0;
        }
        Screen::Categories => {
            if app.category_view_rules {
                app.rule_index = 0;
                app.rule_scroll = 0;
            } else {
                app.category_index = 0;
                app.category_scroll = 0;
            }
        }
        Screen::Budgets => {
            app.budget_index = 0;
            app.budget_scroll = 0;
        }
        Screen::Import if app.import_step == ImportStep::SelectFile => {
            app.file_browser_index = 0;
            app.file_browser_scroll = 0;
        }
        _ => {}
    }
}

fn handle_goto_bottom(app: &mut App) {
    match app.screen {
        Screen::Accounts => {
            if !app.account_snapshots.is_empty() {
                app.accounts_tab_index = app.account_snapshots.len() - 1;
                let page = app.accounts_page();
                app.accounts_tab_scroll = app.accounts_tab_index.saturating_sub(page - 1);
            }
        }
        Screen::Transactions => {
            if !app.transactions.is_empty() {
                app.transaction_index = app.transactions.len() - 1;
                let page = app.transaction_page();
                app.transaction_scroll = app.transaction_index.saturating_sub(page - 1);
            }
        }
        Screen::Categories => {
            if app.category_view_rules {
                if !app.import_rules.is_empty() {
                    app.rule_index = app.import_rules.len() - 1;
                    app.rule_scroll = app.rule_index.saturating_sub(app.rule_page() - 1);
                }
            } else if !app.categories.is_empty() {
                app.category_index = app.categories.len() - 1;
                app.category_scroll = app.category_index.saturating_sub(app.category_page() - 1);
            }
        }
        Screen::Budgets => {
            if !app.budgets.is_empty() {
                app.budget_index = app.budgets.len() - 1;
                let page = app.budget_page();
                app.budget_scroll = app.budget_index.saturating_sub(page - 1);
            }
        }
        Screen::Import if app.import_step == ImportStep::SelectFile => {
            let filtered_len = app.file_browser_filtered().len();
            if filtered_len > 0 {
                app.file_browser_index = filtered_len - 1;
                let page = app.file_browser_page();
                app.file_browser_scroll = app.file_browser_index.saturating_sub(page - 1);
            }
        }
        _ => {}
    }
}
