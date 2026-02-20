use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::db::Database;
use crate::ui::app::{App, ImportStep, InputMode, Screen};
use crate::ui::commands;

pub(crate) fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    db: &mut Database,
) -> Result<()> {
    while app.running {
        terminal.draw(|f| {
            // Update visible rows based on terminal height (subtract tab, status, command bars + borders/header)
            let content_height = f.area().height.saturating_sub(6) as usize; // 1 tab + 1 status + 1 cmd + 2 borders + 1 header
            app.visible_rows = content_height.max(1);
            crate::ui::render::render(f, app);
        })?;

        if let Event::Key(key) = event::read()? {
            if app.show_help {
                app.show_help = false;
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

fn handle_normal_input(
    key: event::KeyEvent,
    app: &mut App,
    db: &mut Database,
) -> Result<()> {
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
        KeyCode::Char('2') => switch_screen(app, db, Screen::Transactions)?,
        KeyCode::Char('3') => switch_screen(app, db, Screen::Import)?,
        KeyCode::Char('4') => switch_screen(app, db, Screen::Categories)?,
        KeyCode::Char('5') => switch_screen(app, db, Screen::Budgets)?,
        KeyCode::Tab => {
            let screens = Screen::all();
            let idx = screens.iter().position(|s| *s == app.screen).unwrap_or(0);
            let next = (idx + 1) % screens.len();
            switch_screen(app, db, screens[next])?;
        }
        KeyCode::BackTab => {
            let screens = Screen::all();
            let idx = screens.iter().position(|s| *s == app.screen).unwrap_or(0);
            let prev = if idx == 0 {
                screens.len() - 1
            } else {
                idx - 1
            };
            switch_screen(app, db, screens[prev])?;
        }
        KeyCode::Enter => handle_enter(app, db)?,
        KeyCode::Esc => handle_escape(app),
        KeyCode::Char('+') | KeyCode::Char('=') => handle_adjust_field(app, 1),
        KeyCode::Char('-') => handle_adjust_field(app, -1),
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
        _ => {}
    }
    Ok(())
}

fn handle_command_input(
    key: event::KeyEvent,
    app: &mut App,
    db: &mut Database,
) -> Result<()> {
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

fn handle_search_input(
    key: event::KeyEvent,
    app: &mut App,
    db: &mut Database,
) -> Result<()> {
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

fn handle_editing_input(
    key: event::KeyEvent,
    app: &mut App,
    db: &mut Database,
) -> Result<()> {
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

fn handle_confirm_input(
    key: event::KeyEvent,
    app: &mut App,
    db: &mut Database,
) -> Result<()> {
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
                        if app.budget_index > 0 {
                            app.budget_index -= 1;
                        }
                        app.set_status(format!("Deleted budget: {name}"));
                    }
                    PendingAction::DeleteRule { id, pattern } => {
                        db.delete_import_rule(id)?;
                        app.refresh_categories(db)?;
                        if app.rule_index > 0 {
                            app.rule_index -= 1;
                        }
                        app.set_status(format!("Deleted rule: '{pattern}'"));
                    }
                    PendingAction::ImportCommit => {
                        let mut txns = app.import_preview.clone();
                        let rules = db.get_import_rules()?;
                        let categorizer = crate::categorize::Categorizer::new(&rules);
                        categorizer.categorize_batch(&mut txns);

                        let uncategorized: Vec<&str> = txns
                            .iter()
                            .filter(|t| t.category_id.is_none())
                            .map(|t| t.original_description.as_str())
                            .take(3)
                            .collect();
                        let suggestions: Vec<String> = uncategorized
                            .iter()
                            .filter_map(|desc| {
                                crate::categorize::suggest_rule(desc).ok()
                            })
                            .collect();

                        let count = db.insert_transactions_batch(&txns)?;
                        app.import_step = ImportStep::Complete;

                        let mut msg = format!(
                            "Imported {count} new transactions ({} duplicates skipped)",
                            txns.len() - count
                        );
                        if !suggestions.is_empty() {
                            msg.push_str(&format!(
                                ". Suggested rules: {}",
                                suggestions
                                    .iter()
                                    .map(|s| format!(":rule {s} <category>"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                        }
                        app.set_status(msg);
                        app.refresh_all(db)?;
                    }
                }
            }
            app.input_mode = InputMode::Normal;
            app.confirm_message.clear();
        }
        _ => {
            // Any other key = cancel
            app.pending_action = None;
            app.input_mode = InputMode::Normal;
            app.confirm_message.clear();
            app.set_status("Cancelled");
        }
    }
    Ok(())
}

// ── Navigation helpers ───────────────────────────────────────

fn switch_screen(app: &mut App, db: &mut Database, screen: Screen) -> Result<()> {
    app.screen = screen;
    match screen {
        Screen::Dashboard => app.refresh_dashboard(db)?,
        Screen::Transactions => app.refresh_transactions(db)?,
        Screen::Import => {
            app.import_step = ImportStep::SelectFile;
            app.refresh_file_browser();
        }
        Screen::Categories => app.refresh_categories(db)?,
        Screen::Budgets => app.refresh_budgets(db)?,
    }
    Ok(())
}

fn handle_move_down(app: &mut App) {
    match app.screen {
        Screen::Transactions => {
            if app.transaction_index + 1 < app.transactions.len() {
                app.transaction_index += 1;
                let page = app.visible_rows.max(1);
                if app.transaction_index >= app.transaction_scroll + page {
                    app.transaction_scroll = app.transaction_index.saturating_sub(page - 1);
                }
            }
        }
        Screen::Categories => {
            if app.category_view_rules {
                if app.rule_index + 1 < app.import_rules.len() {
                    app.rule_index += 1;
                }
            } else if app.category_index + 1 < app.categories.len() {
                app.category_index += 1;
            }
        }
        Screen::Import => match app.import_step {
            ImportStep::SelectFile => {
                if app.file_browser_index + 1 < app.file_browser_entries.len() {
                    app.file_browser_index += 1;
                }
            }
            ImportStep::MapColumns => {
                if app.import_selected_field < 7 {
                    app.import_selected_field += 1;
                }
            }
            _ => {}
        },
        Screen::Budgets => {
            if app.budget_index + 1 < app.budgets.len() {
                app.budget_index += 1;
            }
        }
        _ => {}
    }
}

fn handle_move_up(app: &mut App) {
    match app.screen {
        Screen::Transactions => {
            app.transaction_index = app.transaction_index.saturating_sub(1);
            if app.transaction_index < app.transaction_scroll {
                app.transaction_scroll = app.transaction_index;
            }
        }
        Screen::Categories => {
            if app.category_view_rules {
                app.rule_index = app.rule_index.saturating_sub(1);
            } else {
                app.category_index = app.category_index.saturating_sub(1);
            }
        }
        Screen::Import => match app.import_step {
            ImportStep::SelectFile => {
                app.file_browser_index = app.file_browser_index.saturating_sub(1);
            }
            ImportStep::MapColumns => {
                app.import_selected_field = app.import_selected_field.saturating_sub(1);
            }
            _ => {}
        },
        Screen::Budgets => {
            app.budget_index = app.budget_index.saturating_sub(1);
        }
        _ => {}
    }
}

fn handle_enter(app: &mut App, db: &mut Database) -> Result<()> {
    if app.screen == Screen::Import {
        match app.import_step {
            ImportStep::SelectFile => {
                if let Some(path) = app.file_browser_entries.get(app.file_browser_index).cloned() {
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
                if app.import_account_id.is_none() {
                    let accounts = db.get_accounts()?;
                    if let Some(first) = accounts.first() {
                        app.import_account_id = first.id;
                    }
                }
                if let Err(e) = app.generate_import_preview() {
                    app.set_status(format!("Error generating preview: {e}"));
                }
            }
            ImportStep::Preview => {
                app.confirm_message = format!(
                    "Import {} transactions?",
                    app.import_preview.len()
                );
                app.pending_action =
                    Some(crate::ui::app::PendingAction::ImportCommit);
                app.input_mode = InputMode::Confirm;
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
            ImportStep::MapColumns => {
                app.import_step = ImportStep::SelectFile;
            }
            ImportStep::Preview => {
                app.import_step = ImportStep::MapColumns;
            }
            _ => {
                app.screen = Screen::Dashboard;
            }
        },
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
        7 => {
            app.import_profile.negate_amounts = !app.import_profile.negate_amounts;
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

fn handle_goto_top(app: &mut App) {
    match app.screen {
        Screen::Transactions => {
            app.transaction_index = 0;
            app.transaction_scroll = 0;
        }
        Screen::Categories => {
            if app.category_view_rules {
                app.rule_index = 0;
            } else {
                app.category_index = 0;
            }
        }
        Screen::Import if app.import_step == ImportStep::SelectFile => {
            app.file_browser_index = 0;
        }
        _ => {}
    }
}

fn handle_goto_bottom(app: &mut App) {
    match app.screen {
        Screen::Transactions => {
            if !app.transactions.is_empty() {
                app.transaction_index = app.transactions.len() - 1;
                let page = app.visible_rows.max(1);
                app.transaction_scroll = app.transaction_index.saturating_sub(page - 1);
            }
        }
        Screen::Categories => {
            if app.category_view_rules {
                if !app.import_rules.is_empty() {
                    app.rule_index = app.import_rules.len() - 1;
                }
            } else if !app.categories.is_empty() {
                app.category_index = app.categories.len() - 1;
            }
        }
        Screen::Import if app.import_step == ImportStep::SelectFile => {
            if !app.file_browser_entries.is_empty() {
                app.file_browser_index = app.file_browser_entries.len() - 1;
            }
        }
        _ => {}
    }
}
