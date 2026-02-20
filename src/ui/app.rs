use anyhow::Result;
use chrono::Local;
use std::path::PathBuf;

use crate::db::Database;
use crate::import::{CsvImporter, CsvProfile};
use crate::models::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Screen {
    Dashboard,
    Transactions,
    Import,
    Categories,
    Budgets,
}

impl Screen {
    pub(crate) fn all() -> &'static [Screen] {
        &[
            Self::Dashboard,
            Self::Transactions,
            Self::Import,
            Self::Categories,
            Self::Budgets,
        ]
    }
}

impl std::fmt::Display for Screen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dashboard => write!(f, "Dashboard"),
            Self::Transactions => write!(f, "Transactions"),
            Self::Import => write!(f, "Import"),
            Self::Categories => write!(f, "Categories"),
            Self::Budgets => write!(f, "Budgets"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputMode {
    Normal,
    Command,
    Search,
    Editing,
    Confirm,
}

impl std::fmt::Display for InputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Command => write!(f, "COMMAND"),
            Self::Search => write!(f, "SEARCH"),
            Self::Editing => write!(f, "EDIT"),
            Self::Confirm => write!(f, "CONFIRM"),
        }
    }
}

/// Pending action that requires user confirmation.
#[derive(Debug, Clone)]
pub(crate) enum PendingAction {
    DeleteTransaction { id: i64, description: String },
    DeleteBudget { id: i64, name: String },
    DeleteRule { id: i64, pattern: String },
    ImportCommit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImportStep {
    SelectFile,
    MapColumns,
    Preview,
    Complete,
}

impl std::fmt::Display for ImportStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelectFile => write!(f, "Select File"),
            Self::MapColumns => write!(f, "Map Columns"),
            Self::Preview => write!(f, "Preview"),
            Self::Complete => write!(f, "Complete"),
        }
    }
}

pub(crate) struct App {
    pub(crate) running: bool,
    pub(crate) screen: Screen,
    pub(crate) input_mode: InputMode,
    pub(crate) command_input: String,
    pub(crate) search_input: String,
    pub(crate) status_message: String,
    pub(crate) show_help: bool,
    pub(crate) current_month: String,

    // Dashboard
    pub(crate) monthly_income: rust_decimal::Decimal,
    pub(crate) monthly_expenses: rust_decimal::Decimal,
    pub(crate) net_worth: rust_decimal::Decimal,
    pub(crate) spending_by_category: Vec<(String, rust_decimal::Decimal)>,
    pub(crate) monthly_trend: Vec<(String, rust_decimal::Decimal, rust_decimal::Decimal)>,

    // Transactions
    pub(crate) transactions: Vec<Transaction>,
    pub(crate) transaction_index: usize,
    pub(crate) transaction_scroll: usize,
    pub(crate) transaction_filter_account: Option<i64>,
    pub(crate) transaction_count: i64,

    // Categories
    pub(crate) categories: Vec<Category>,
    pub(crate) category_index: usize,
    pub(crate) import_rules: Vec<ImportRule>,
    pub(crate) rule_index: usize,
    pub(crate) category_view_rules: bool,

    // Accounts
    pub(crate) accounts: Vec<Account>,
    pub(crate) account_index: usize,

    // Budgets
    pub(crate) budgets: Vec<Budget>,
    pub(crate) budget_index: usize,

    // Import state
    pub(crate) import_step: ImportStep,
    pub(crate) import_path: String,
    pub(crate) import_headers: Vec<String>,
    pub(crate) import_rows: Vec<Vec<String>>,
    pub(crate) import_profile: CsvProfile,
    pub(crate) import_preview: Vec<Transaction>,
    pub(crate) import_selected_field: usize,
    pub(crate) import_account_id: Option<i64>,
    pub(crate) import_detected_bank: Option<String>,

    // File browser
    pub(crate) file_browser_path: PathBuf,
    pub(crate) file_browser_entries: Vec<PathBuf>,
    pub(crate) file_browser_index: usize,
    pub(crate) file_browser_scroll: usize,
    pub(crate) file_browser_filter: String,
    pub(crate) file_browser_show_hidden: bool,
    pub(crate) file_browser_input_focused: bool,

    // Confirmation
    pub(crate) pending_action: Option<PendingAction>,
    pub(crate) confirm_message: String,

    // Layout (updated each render frame)
    pub(crate) visible_rows: usize,
}

impl App {
    pub(crate) fn new() -> Self {
        let now = Local::now();
        let current_month = now.format("%Y-%m").to_string();

        Self {
            running: true,
            screen: Screen::Dashboard,
            input_mode: InputMode::Normal,
            command_input: String::new(),
            search_input: String::new(),
            status_message: String::new(),
            show_help: false,
            current_month,

            monthly_income: rust_decimal::Decimal::ZERO,
            monthly_expenses: rust_decimal::Decimal::ZERO,
            net_worth: rust_decimal::Decimal::ZERO,
            spending_by_category: Vec::new(),
            monthly_trend: Vec::new(),

            transactions: Vec::new(),
            transaction_index: 0,
            transaction_scroll: 0,
            transaction_filter_account: None,
            transaction_count: 0,

            categories: Vec::new(),
            category_index: 0,
            import_rules: Vec::new(),
            rule_index: 0,
            category_view_rules: false,

            accounts: Vec::new(),
            account_index: 0,

            budgets: Vec::new(),
            budget_index: 0,

            import_step: ImportStep::SelectFile,
            import_path: String::new(),
            import_headers: Vec::new(),
            import_rows: Vec::new(),
            import_profile: CsvProfile::default(),
            import_preview: Vec::new(),
            import_selected_field: 0,
            import_account_id: None,
            import_detected_bank: None,

            file_browser_path: directories::UserDirs::new()
                .map(|d| d.home_dir().to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))),
            file_browser_entries: Vec::new(),
            file_browser_index: 0,
            file_browser_scroll: 0,
            file_browser_filter: String::new(),
            file_browser_show_hidden: false,
            file_browser_input_focused: false,

            pending_action: None,
            confirm_message: String::new(),

            visible_rows: 20,
        }
    }

    pub(crate) fn refresh_dashboard(&mut self, db: &Database) -> Result<()> {
        let (income, expenses) = db.get_monthly_totals(&self.current_month)?;
        self.monthly_income = income;
        self.monthly_expenses = expenses;
        self.net_worth = db.get_net_worth()?;
        self.spending_by_category = db.get_spending_by_category(&self.current_month)?;
        self.monthly_trend = db.get_monthly_trend(12)?;
        self.transaction_count = db.get_transaction_count()?;
        // Transactions are needed for dashboard card counts (income_count, expense_count)
        self.refresh_transactions(db)?;
        Ok(())
    }

    pub(crate) fn refresh_transactions(&mut self, db: &Database) -> Result<()> {
        let search = if self.search_input.is_empty() {
            None
        } else {
            Some(self.search_input.as_str())
        };
        self.transactions = db.get_transactions(
            Some(200),
            None,
            self.transaction_filter_account,
            None, // category filter (not yet implemented)
            search,
            Some(&self.current_month),
        )?;
        self.transaction_count = db.get_transaction_count()?;
        if self.transaction_index >= self.transactions.len() && !self.transactions.is_empty() {
            self.transaction_index = self.transactions.len() - 1;
        }
        Ok(())
    }

    pub(crate) fn refresh_categories(&mut self, db: &Database) -> Result<()> {
        self.categories = db.get_categories()?;
        self.import_rules = db.get_import_rules()?;
        Ok(())
    }

    pub(crate) fn refresh_budgets(&mut self, db: &Database) -> Result<()> {
        self.budgets = db.get_budgets(&self.current_month)?;
        Ok(())
    }

    pub(crate) fn refresh_accounts(&mut self, db: &Database) -> Result<()> {
        self.accounts = db.get_accounts()?;
        Ok(())
    }

    pub(crate) fn refresh_all(&mut self, db: &Database) -> Result<()> {
        self.refresh_dashboard(db)?; // also refreshes transactions
        self.refresh_categories(db)?;
        self.refresh_budgets(db)?;
        self.refresh_accounts(db)?;
        Ok(())
    }

    pub(crate) fn load_import_file(&mut self) -> Result<()> {
        let path = std::path::Path::new(&self.import_path);
        let (headers, rows) = CsvImporter::preview(path)?;

        // Try to auto-detect bank format
        let first_row = rows.first().cloned().unwrap_or_default();
        if let Some(profile) = crate::import::detect_bank_format(&headers, &first_row) {
            self.import_detected_bank = Some(profile.name.clone());
            self.import_profile = profile;
        }

        self.import_headers = headers;
        self.import_rows = rows;
        self.import_step = ImportStep::MapColumns;
        self.status_message = if let Some(ref bank) = self.import_detected_bank {
            format!("Detected format: {bank}")
        } else {
            "Custom CSV - map columns manually".into()
        };

        Ok(())
    }

    pub(crate) fn generate_import_preview(&mut self) -> Result<()> {
        let account_id = self.import_account_id.unwrap_or(1);
        self.import_preview =
            CsvImporter::parse(&self.import_rows, &self.import_profile, account_id)?;
        self.import_step = ImportStep::Preview;
        self.status_message = format!("{} transactions ready to import", self.import_preview.len());
        Ok(())
    }

    pub(crate) fn refresh_file_browser(&mut self) {
        let mut entries: Vec<PathBuf> = Vec::new();

        // Add parent directory
        if let Some(parent) = self.file_browser_path.parent() {
            entries.push(parent.to_path_buf());
        }

        if let Ok(read_dir) = std::fs::read_dir(&self.file_browser_path) {
            let is_hidden = |p: &PathBuf| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with('.'))
            };

            let all: Vec<PathBuf> = read_dir
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    (self.file_browser_show_hidden || !is_hidden(p))
                        && (p.is_dir()
                            || p.extension().and_then(|e| e.to_str()).is_some_and(|ext| {
                                matches!(ext.to_ascii_lowercase().as_str(), "csv" | "tsv" | "ofx" | "qfx" | "qif")
                            }))
                })
                .collect();

            // Dirs first, then files, each sorted alphabetically
            let mut dirs: Vec<PathBuf> = all.iter().filter(|p| p.is_dir()).cloned().collect();
            let mut files: Vec<PathBuf> = all.iter().filter(|p| !p.is_dir()).cloned().collect();
            dirs.sort();
            files.sort();
            entries.extend(dirs);
            entries.extend(files);
        }

        self.file_browser_entries = entries;
        self.file_browser_index = 0;
        self.file_browser_scroll = 0;
        self.file_browser_filter.clear();
        self.file_browser_input_focused = false;
    }

    /// Returns filtered file browser entries (indices into `file_browser_entries`).
    /// When filter is empty, returns all. The `..` entry always passes.
    pub(crate) fn file_browser_filtered(&self) -> Vec<usize> {
        if self.file_browser_filter.is_empty() {
            return (0..self.file_browser_entries.len()).collect();
        }
        let filter = self.file_browser_filter.to_ascii_lowercase();
        self.file_browser_entries
            .iter()
            .enumerate()
            .filter(|(_, path)| {
                // Parent (..) always passes
                if Some(path.as_path()) == self.file_browser_path.parent() {
                    return true;
                }
                path.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|name| name.to_ascii_lowercase().contains(&filter))
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub(crate) fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
    }
}
