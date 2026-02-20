mod categorize;
mod db;
mod import;
mod models;
mod run;
mod ui;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let db_path = get_db_path()?;
    let mut db = db::Database::open(&db_path)?;
    ensure_default_account(&mut db)?;

    match args.len() {
        1 => run::as_tui(&mut db),
        2.. => run::as_cli(&args, &mut db),
        _ => {
            eprintln!("Usage: budgetui [command]");
            Ok(())
        }
    }
}

fn ensure_default_account(db: &mut db::Database) -> Result<()> {
    if db.get_accounts()?.is_empty() {
        let account = models::Account::new(
            "Default".into(),
            models::AccountType::Checking,
            String::new(),
        );
        db.insert_account(&account)?;
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
