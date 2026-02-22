use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::db::Database;
use crate::models::{Account, AccountType};
use crate::ui::app::{App, ImportStep, InputMode, PendingAction, Screen};
use crate::ui::commands;
use crate::ui::util::{scroll_down, scroll_to_bottom, scroll_to_top, scroll_up};

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

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    db: &mut Database,
) -> Result<()> {
    while app.running {
        terminal.draw(|f| {
            let content_height = f.area().height.saturating_sub(3) as usize;
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
    if app.screen == Screen::Import
        && app.import_step == ImportStep::SelectFile
        && app.file_browser_input_focused
    {
        return handle_file_browser_input(key, app);
    }

    if app.screen == Screen::Import && app.import_step == ImportStep::Categorize {
        return handle_categorize_input(key, app, db);
    }

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
        KeyCode::Char('q') | KeyCode::Char('c')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
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
            if app.selected_transactions.is_empty() {
                commands::handle_command("delete-txn", app, db)?;
            } else {
                let ids: Vec<i64> = app.selected_transactions.iter().copied().collect();
                let count = ids.len();
                app.confirm_message = format!(
                    "Delete {count} transaction{}?",
                    if count == 1 { "" } else { "s" }
                );
                app.pending_action = Some(PendingAction::DeleteTransactions { ids, count });
                app.input_mode = InputMode::Confirm;
            }
        }
        KeyCode::Char(' ') if app.screen == Screen::Transactions => {
            if let Some(txn) = app.transactions.get(app.transaction_index) {
                if let Some(id) = txn.id {
                    if !app.selected_transactions.remove(&id) {
                        app.selected_transactions.insert(id);
                    }
                }
            }
            handle_move_down(app);
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
                app.file_browser_input_focused = false;
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_categorize_input(key: event::KeyEvent, app: &mut App, db: &mut Database) -> Result<()> {
    if app.import_cat_creating {
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
                    let cat = crate::models::Category::new(name.clone());
                    let cat_id = db.insert_category(&cat)?;

                    if let Some((desc, _)) = app.import_cat_descriptions.get(app.import_cat_index) {
                        if let Ok(pattern) = crate::categorize::suggest_rule(desc) {
                            let rule = crate::models::ImportRule::new_contains(pattern, cat_id);
                            db.insert_import_rule(&rule)?;
                        }
                    }

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

    let page = app.categorize_visible_rows();
    let cat_len = app.categories.len();
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            scroll_down(
                &mut app.import_cat_selected,
                &mut app.import_cat_scroll,
                cat_len,
                page,
            );
        }
        KeyCode::Char('k') | KeyCode::Up => {
            scroll_up(&mut app.import_cat_selected, &mut app.import_cat_scroll);
        }
        KeyCode::Char('g') => {
            scroll_to_top(&mut app.import_cat_selected, &mut app.import_cat_scroll);
        }
        KeyCode::Char('G') => {
            scroll_to_bottom(
                &mut app.import_cat_selected,
                &mut app.import_cat_scroll,
                cat_len,
                page,
            );
        }
        KeyCode::Char('s') => {
            if !app.advance_categorize() {
                commit_import(app, db)?;
            } else {
                app.set_status("Skipped — moving to next");
            }
        }
        KeyCode::Char('S') => {
            commit_import(app, db)?;
        }
        KeyCode::Char('n') => {
            app.import_cat_creating = true;
            app.import_cat_new_name.clear();
        }
        KeyCode::Enter => {
            if let Some(cat) = app.categories.get(app.import_cat_selected) {
                if let Some(cat_id) = cat.id {
                    let cat_name = cat.name.clone();

                    if let Some((desc, _)) = app.import_cat_descriptions.get(app.import_cat_index) {
                        if let Ok(pattern) = crate::categorize::suggest_rule(desc) {
                            let rule =
                                crate::models::ImportRule::new_contains(pattern.clone(), cat_id);
                            db.insert_import_rule(&rule)?;
                            app.refresh_categories(db)?;
                        }
                    }

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
            app.import_step = ImportStep::Preview;
            app.set_status("Back to preview — categories already assigned will be kept");
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            for _ in 0..page / 2 {
                scroll_down(
                    &mut app.import_cat_selected,
                    &mut app.import_cat_scroll,
                    cat_len,
                    page,
                );
            }
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            for _ in 0..page / 2 {
                scroll_up(&mut app.import_cat_selected, &mut app.import_cat_scroll);
            }
        }
        KeyCode::Char(c) => {
            let lower = c.to_ascii_lowercase();
            if let Some(idx) = app.categories.iter().position(|cat| {
                cat.name
                    .starts_with(|ch: char| ch.to_ascii_lowercase() == lower)
            }) {
                app.import_cat_selected = idx;
                if idx < app.import_cat_scroll {
                    app.import_cat_scroll = idx;
                } else if idx >= app.import_cat_scroll + page {
                    app.import_cat_scroll = idx.saturating_sub(page.saturating_sub(1));
                }
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
                    let is_credit = acct_type.is_credit();
                    let acct = Account::new(name.clone(), acct_type, String::new());
                    let id = db.insert_account(&acct)?;
                    app.import_account_id = Some(id);
                    app.refresh_accounts(db)?;

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

    let page = app.import_account_page();
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            scroll_down(
                &mut app.import_account_index,
                &mut app.import_account_scroll,
                app.accounts.len(),
                page,
            );
        }
        KeyCode::Char('k') | KeyCode::Up => {
            scroll_up(
                &mut app.import_account_index,
                &mut app.import_account_scroll,
            );
        }
        KeyCode::Char('g') => {
            scroll_to_top(
                &mut app.import_account_index,
                &mut app.import_account_scroll,
            );
        }
        KeyCode::Char('G') => {
            scroll_to_bottom(
                &mut app.import_account_index,
                &mut app.import_account_scroll,
                app.accounts.len(),
                page,
            );
        }
        KeyCode::Char('n') => {
            app.import_creating_account = true;
            app.import_new_account_name.clear();
        }
        KeyCode::Enter => {
            if let Some(acct) = app.accounts.get(app.import_account_index) {
                app.import_account_id = acct.id;
                let is_credit = acct.account_type.is_credit();
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
                app.import_creating_account = true;
                app.import_new_account_name.clear();
            }
        }
        KeyCode::Esc => {
            app.import_step = ImportStep::MapColumns;
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            for _ in 0..page / 2 {
                scroll_down(
                    &mut app.import_account_index,
                    &mut app.import_account_scroll,
                    app.accounts.len(),
                    page,
                );
            }
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            for _ in 0..page / 2 {
                scroll_up(
                    &mut app.import_account_index,
                    &mut app.import_account_scroll,
                );
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
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.command_input.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let trimmed = app.command_input.trim_end();
            if let Some(pos) = trimmed.rfind(' ') {
                app.command_input.truncate(pos + 1);
            } else {
                app.command_input.clear();
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
            app.screen = Screen::Transactions;
            app.transaction_index = 0;
            app.transaction_scroll = 0;
            app.refresh_transactions(db)?;
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
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
                    PendingAction::DeleteTransactions { ids, count } => {
                        db.delete_transactions_batch(&ids)?;
                        app.clear_selections();
                        app.refresh_transactions(db)?;
                        app.refresh_dashboard(db)?;
                        if app.transaction_index >= app.transactions.len()
                            && !app.transactions.is_empty()
                        {
                            app.transaction_index = app.transactions.len().saturating_sub(1);
                        }
                        app.set_status(format!("Deleted {count} transactions"));
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
                        let rules = db.get_import_rules()?;
                        let (categorizer, bad_patterns) =
                            crate::categorize::Categorizer::new(&rules);
                        if !bad_patterns.is_empty() {
                            app.set_status(format!(
                                "Warning: invalid regex rule(s): {}",
                                bad_patterns.join(", ")
                            ));
                        }
                        categorizer.categorize_batch(&mut app.import_preview);

                        if app.prepare_categorize_step() {
                            let total = app.import_cat_descriptions.len();
                            app.import_step = ImportStep::Categorize;
                            app.set_status(format!(
                                "{total} unique description{} to categorize",
                                if total == 1 { "" } else { "s" }
                            ));
                        } else {
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
        _ => {}
    }
    Ok(())
}

// ── Navigation helpers ───────────────────────────────────────

fn switch_screen(app: &mut App, db: &mut Database, screen: Screen) -> Result<()> {
    app.clear_selections();
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
    app.set_status(format!("{screen}"));
    Ok(())
}

fn handle_move_down(app: &mut App) {
    match app.screen {
        Screen::Accounts => {
            let page = app.accounts_page();
            scroll_down(
                &mut app.accounts_tab_index,
                &mut app.accounts_tab_scroll,
                app.account_snapshots.len(),
                page,
            );
        }
        Screen::Transactions => {
            let page = app.transaction_page();
            scroll_down(
                &mut app.transaction_index,
                &mut app.transaction_scroll,
                app.transactions.len(),
                page,
            );
        }
        Screen::Categories => {
            if app.category_view_rules {
                let page = app.rule_page();
                scroll_down(
                    &mut app.rule_index,
                    &mut app.rule_scroll,
                    app.import_rules.len(),
                    page,
                );
            } else {
                let page = app.category_page();
                scroll_down(
                    &mut app.category_index,
                    &mut app.category_scroll,
                    app.categories.len(),
                    page,
                );
            }
        }
        Screen::Import => match app.import_step {
            ImportStep::SelectFile => {
                let filtered_len = app.file_browser_filtered().len();
                let page = app.file_browser_page();
                scroll_down(
                    &mut app.file_browser_index,
                    &mut app.file_browser_scroll,
                    filtered_len,
                    page,
                );
            }
            ImportStep::MapColumns => {
                if app.import_selected_field < 6 {
                    app.import_selected_field += 1;
                }
            }
            _ => {}
        },
        Screen::Budgets => {
            let page = app.budget_page();
            scroll_down(
                &mut app.budget_index,
                &mut app.budget_scroll,
                app.budgets.len(),
                page,
            );
        }
        _ => {}
    }
}

fn handle_move_up(app: &mut App) {
    match app.screen {
        Screen::Accounts => scroll_up(&mut app.accounts_tab_index, &mut app.accounts_tab_scroll),
        Screen::Transactions => scroll_up(&mut app.transaction_index, &mut app.transaction_scroll),
        Screen::Categories => {
            if app.category_view_rules {
                scroll_up(&mut app.rule_index, &mut app.rule_scroll);
            } else {
                scroll_up(&mut app.category_index, &mut app.category_scroll);
            }
        }
        Screen::Import => match app.import_step {
            ImportStep::SelectFile => {
                if app.file_browser_index == 0 {
                    app.file_browser_input_focused = true;
                } else {
                    scroll_up(&mut app.file_browser_index, &mut app.file_browser_scroll);
                }
            }
            ImportStep::MapColumns => {
                app.import_selected_field = app.import_selected_field.saturating_sub(1);
            }
            _ => {}
        },
        Screen::Budgets => scroll_up(&mut app.budget_index, &mut app.budget_scroll),
        _ => {}
    }
}

fn handle_enter(app: &mut App, db: &mut Database) -> Result<()> {
    if app.screen == Screen::Accounts {
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
                app.refresh_accounts(db)?;
                app.import_account_index = 0;
                app.import_account_scroll = 0;
                app.import_creating_account = false;
                app.import_new_account_name.clear();

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

                if app.import_profile.is_credit_account {
                    app.import_new_account_type = AccountType::all()
                        .iter()
                        .position(|t| *t == AccountType::CreditCard)
                        .unwrap_or(0);
                } else {
                    app.import_new_account_type = 0;
                }

                app.import_step = ImportStep::SelectAccount;
            }
            ImportStep::SelectAccount => {}
            ImportStep::Preview => {
                app.confirm_message = format!("Import {} transactions?", app.import_preview.len());
                app.pending_action = Some(crate::ui::app::PendingAction::ImportCommit);
                app.input_mode = InputMode::Confirm;
            }
            ImportStep::Categorize => {}
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
                app.import_step = ImportStep::MapColumns;
            }
            ImportStep::Preview => {
                app.import_step = ImportStep::SelectAccount;
            }
            ImportStep::Categorize => {
                app.import_step = ImportStep::Preview;
            }
            ImportStep::Complete => {
                app.screen = Screen::Dashboard;
            }
        },
        Screen::Transactions if !app.selected_transactions.is_empty() => {
            app.clear_selections();
            app.set_status("Selection cleared");
        }
        Screen::Transactions if app.transaction_filter_account.is_some() => {
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
            scroll_to_top(&mut app.accounts_tab_index, &mut app.accounts_tab_scroll)
        }
        Screen::Transactions => {
            scroll_to_top(&mut app.transaction_index, &mut app.transaction_scroll)
        }
        Screen::Categories => {
            if app.category_view_rules {
                scroll_to_top(&mut app.rule_index, &mut app.rule_scroll);
            } else {
                scroll_to_top(&mut app.category_index, &mut app.category_scroll);
            }
        }
        Screen::Budgets => scroll_to_top(&mut app.budget_index, &mut app.budget_scroll),
        Screen::Import if app.import_step == ImportStep::SelectFile => {
            scroll_to_top(&mut app.file_browser_index, &mut app.file_browser_scroll);
        }
        _ => {}
    }
}

fn handle_goto_bottom(app: &mut App) {
    match app.screen {
        Screen::Accounts => {
            let page = app.accounts_page();
            scroll_to_bottom(
                &mut app.accounts_tab_index,
                &mut app.accounts_tab_scroll,
                app.account_snapshots.len(),
                page,
            );
        }
        Screen::Transactions => {
            let page = app.transaction_page();
            scroll_to_bottom(
                &mut app.transaction_index,
                &mut app.transaction_scroll,
                app.transactions.len(),
                page,
            );
        }
        Screen::Categories => {
            if app.category_view_rules {
                let page = app.rule_page();
                scroll_to_bottom(
                    &mut app.rule_index,
                    &mut app.rule_scroll,
                    app.import_rules.len(),
                    page,
                );
            } else {
                let page = app.category_page();
                scroll_to_bottom(
                    &mut app.category_index,
                    &mut app.category_scroll,
                    app.categories.len(),
                    page,
                );
            }
        }
        Screen::Budgets => {
            let page = app.budget_page();
            scroll_to_bottom(
                &mut app.budget_index,
                &mut app.budget_scroll,
                app.budgets.len(),
                page,
            );
        }
        Screen::Import if app.import_step == ImportStep::SelectFile => {
            let filtered_len = app.file_browser_filtered().len();
            let page = app.file_browser_page();
            scroll_to_bottom(
                &mut app.file_browser_index,
                &mut app.file_browser_scroll,
                filtered_len,
                page,
            );
        }
        _ => {}
    }
}
