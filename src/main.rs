mod categorize;
mod db;
mod import;
mod models;
mod run;
mod ui;

use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

fn main() -> Result<()> {
    let db_path = get_db_path()?;
    let mut db = db::Database::open(&db_path)?;

    if db.get_accounts()?.is_empty() {
        let default_account =
            models::Account::new("Default".into(), models::AccountType::Checking, String::new());
        db.insert_account(&default_account)?;
    }

    let mut app = ui::app::App::new();
    app.refresh_all(&db)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run::run_app(&mut terminal, &mut app, &mut db);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e:?}");
    }

    Ok(())
}

fn get_db_path() -> Result<std::path::PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "budgetui", "BudgeTUI")
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;
    let data_dir = proj_dirs.data_dir();
    std::fs::create_dir_all(data_dir)
        .with_context(|| format!("Failed to create data directory: {}", data_dir.display()))?;
    Ok(data_dir.join("budgetui.db"))
}
