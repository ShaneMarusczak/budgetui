use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;

use crate::db::Database;
use crate::import::{CsvImporter, CsvProfile};
use crate::models::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Screen {
    Dashboard,
    Accounts,
    Transactions,
    Import,
    Categories,
    Budgets,
}

impl Screen {
    pub(crate) fn all() -> &'static [Screen] {
        &[
            Self::Dashboard,
            Self::Accounts,
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
            Self::Accounts => write!(f, "Accounts"),
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
    DeleteTransactions { ids: Vec<i64>, count: usize },
    DeleteBudget { id: i64, name: String },
    DeleteRule { id: i64, pattern: String },
    ImportCommit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImportStep {
    SelectFile,
    MapColumns,
    SelectAccount,
    Preview,
    Categorize,
    Complete,
}

impl std::fmt::Display for ImportStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelectFile => write!(f, "Select File"),
            Self::MapColumns => write!(f, "Map Columns"),
            Self::SelectAccount => write!(f, "Select Account"),
            Self::Preview => write!(f, "Preview"),
            Self::Categorize => write!(f, "Categorize"),
            Self::Complete => write!(f, "Complete"),
        }
    }
}

/// Per-account snapshot for the Accounts tab.
pub(crate) struct AccountSnapshot {
    pub(crate) account: Account,
    pub(crate) month_income: rust_decimal::Decimal,
    pub(crate) month_expenses: rust_decimal::Decimal,
    pub(crate) balance: rust_decimal::Decimal,
}

pub(crate) struct App {
    pub(crate) running: bool,
    pub(crate) screen: Screen,
    pub(crate) input_mode: InputMode,
    pub(crate) command_input: String,
    pub(crate) search_input: String,
    pub(crate) status_message: String,
    pub(crate) show_help: bool,
    pub(crate) show_nav: bool,
    pub(crate) nav_index: usize,
    pub(crate) current_month: Option<String>,

    // Dashboard — totals (all accounts)
    pub(crate) monthly_income: rust_decimal::Decimal,
    pub(crate) monthly_expenses: rust_decimal::Decimal,
    pub(crate) net_worth: rust_decimal::Decimal,
    pub(crate) spending_by_category: Vec<(String, rust_decimal::Decimal)>,
    pub(crate) monthly_trend: Vec<(String, rust_decimal::Decimal, rust_decimal::Decimal)>,

    // Dashboard — debit accounts (Checking, Savings, Cash, Investment, Other)
    pub(crate) debit_income: rust_decimal::Decimal,
    pub(crate) debit_expenses: rust_decimal::Decimal,
    pub(crate) debit_balance: rust_decimal::Decimal,

    // Dashboard — credit accounts (CreditCard, Loan)
    pub(crate) credit_charges: rust_decimal::Decimal,
    pub(crate) credit_payments: rust_decimal::Decimal,
    pub(crate) credit_balance: rust_decimal::Decimal,

    // Transactions
    pub(crate) transactions: Vec<Transaction>,
    pub(crate) transaction_index: usize,
    pub(crate) transaction_scroll: usize,
    pub(crate) transaction_filter_account: Option<i64>,
    pub(crate) transaction_count: i64,
    pub(crate) selected_transactions: HashSet<i64>,

    // Categories
    pub(crate) categories: Vec<Category>,
    pub(crate) category_index: usize,
    pub(crate) category_scroll: usize,
    pub(crate) import_rules: Vec<ImportRule>,
    pub(crate) rule_index: usize,
    pub(crate) rule_scroll: usize,
    pub(crate) category_view_rules: bool,

    // Accounts tab
    pub(crate) accounts: Vec<Account>,
    pub(crate) account_index: usize,
    pub(crate) accounts_tab_index: usize,
    pub(crate) accounts_tab_scroll: usize,
    pub(crate) account_snapshots: Vec<AccountSnapshot>,

    // Budgets
    pub(crate) budgets: Vec<Budget>,
    pub(crate) budget_index: usize,
    pub(crate) budget_scroll: usize,

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

    // Import account picker (SelectAccount step)
    pub(crate) import_account_index: usize,
    pub(crate) import_account_scroll: usize,
    pub(crate) import_new_account_name: String,
    pub(crate) import_new_account_type: usize, // index into AccountType::all()
    pub(crate) import_creating_account: bool,

    // Interactive categorization (import wizard)
    pub(crate) import_cat_descriptions: Vec<(String, usize)>, // (description, count of matching txns)
    pub(crate) import_cat_index: usize,                       // which description we're on
    pub(crate) import_cat_selected: usize,                    // which category is highlighted
    pub(crate) import_cat_scroll: usize,                      // category list viewport scroll
    pub(crate) import_cat_new_name: String, // inline new-category input (empty = not typing)
    pub(crate) import_cat_creating: bool,   // whether we're typing a new category name

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
        Self {
            running: true,
            screen: Screen::Dashboard,
            input_mode: InputMode::Normal,
            command_input: String::new(),
            search_input: String::new(),
            status_message: String::new(),
            show_help: false,
            show_nav: false,
            nav_index: 0,
            current_month: None,

            monthly_income: rust_decimal::Decimal::ZERO,
            monthly_expenses: rust_decimal::Decimal::ZERO,
            net_worth: rust_decimal::Decimal::ZERO,
            spending_by_category: Vec::new(),
            monthly_trend: Vec::new(),

            debit_income: rust_decimal::Decimal::ZERO,
            debit_expenses: rust_decimal::Decimal::ZERO,
            debit_balance: rust_decimal::Decimal::ZERO,
            credit_charges: rust_decimal::Decimal::ZERO,
            credit_payments: rust_decimal::Decimal::ZERO,
            credit_balance: rust_decimal::Decimal::ZERO,

            transactions: Vec::new(),
            transaction_index: 0,
            transaction_scroll: 0,
            transaction_filter_account: None,
            transaction_count: 0,
            selected_transactions: HashSet::new(),

            categories: Vec::new(),
            category_index: 0,
            category_scroll: 0,
            import_rules: Vec::new(),
            rule_index: 0,
            rule_scroll: 0,
            category_view_rules: false,

            accounts: Vec::new(),
            account_index: 0,
            accounts_tab_index: 0,
            accounts_tab_scroll: 0,
            account_snapshots: Vec::new(),

            budgets: Vec::new(),
            budget_index: 0,
            budget_scroll: 0,

            import_step: ImportStep::SelectFile,
            import_path: String::new(),
            import_headers: Vec::new(),
            import_rows: Vec::new(),
            import_profile: CsvProfile::default(),
            import_preview: Vec::new(),
            import_selected_field: 0,
            import_account_id: None,
            import_detected_bank: None,

            import_account_index: 0,
            import_account_scroll: 0,
            import_new_account_name: String::new(),
            import_new_account_type: 0,
            import_creating_account: false,

            import_cat_descriptions: Vec::new(),
            import_cat_index: 0,
            import_cat_selected: 0,
            import_cat_scroll: 0,
            import_cat_new_name: String::new(),
            import_cat_creating: false,

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
        let month = self.current_month.as_deref();
        let (income, expenses) = db.get_monthly_totals(month)?;
        self.monthly_income = income;
        self.monthly_expenses = expenses;
        self.net_worth = db.get_net_worth()?;
        self.spending_by_category = db.get_spending_by_category(month)?;
        self.monthly_trend = db.get_monthly_trend(12)?;
        self.transaction_count = db.get_transaction_count()?;

        // Debit accounts (Checking, Savings, Cash, Investment, Other)
        let debit_types = AccountType::debit_type_strs();
        let (di, de) = db.get_monthly_totals_by_account_type(month, debit_types)?;
        self.debit_income = di;
        self.debit_expenses = de;
        self.debit_balance = db.get_balance_by_account_type(debit_types)?;

        // Credit accounts (CreditCard, Loan)
        let credit_types = AccountType::credit_type_strs();
        let (cp, cc) = db.get_monthly_totals_by_account_type(month, credit_types)?;
        self.credit_payments = cp; // positive = payments made to card
        self.credit_charges = cc; // negative = charges/purchases
        self.credit_balance = db.get_balance_by_account_type(credit_types)?;

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
            None,
            search,
            None,
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
        self.budgets = db.get_budgets(self.current_month.as_deref())?;
        Ok(())
    }

    pub(crate) fn refresh_accounts(&mut self, db: &Database) -> Result<()> {
        self.accounts = db.get_accounts()?;
        Ok(())
    }

    pub(crate) fn refresh_accounts_tab(&mut self, db: &Database) -> Result<()> {
        self.accounts = db.get_accounts()?;
        let month = self.current_month.as_deref();
        let mut snapshots = Vec::with_capacity(self.accounts.len());
        for account in &self.accounts {
            let aid = account.id.unwrap_or(0);
            let (income, expenses) = db.get_account_monthly_totals(aid, month)?;
            let balance = db.get_account_balance(aid)?;
            snapshots.push(AccountSnapshot {
                account: account.clone(),
                month_income: income,
                month_expenses: expenses,
                balance,
            });
        }
        self.account_snapshots = snapshots;
        Ok(())
    }

    pub(crate) fn refresh_all(&mut self, db: &Database) -> Result<()> {
        self.refresh_dashboard(db)?;
        self.refresh_transactions(db)?;
        self.refresh_categories(db)?;
        self.refresh_budgets(db)?;
        self.refresh_accounts(db)?;
        self.refresh_accounts_tab(db)?;
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
                                matches!(ext.to_ascii_lowercase().as_str(), "csv" | "tsv")
                            }))
                })
                .collect();

            // Files first (the things you want), dirs at the bottom (just for navigation)
            let mut files: Vec<PathBuf> = all.iter().filter(|p| !p.is_dir()).cloned().collect();
            let mut dirs: Vec<PathBuf> = all.iter().filter(|p| p.is_dir()).cloned().collect();
            files.sort();
            dirs.sort();
            entries.extend(files);
            entries.extend(dirs);
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

    /// Collect unique uncategorized descriptions from import_preview and their counts.
    /// Returns true if there are descriptions to categorize (step should be entered).
    pub(crate) fn prepare_categorize_step(&mut self) -> bool {
        use std::collections::HashMap;
        // Count occurrences first
        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for txn in &self.import_preview {
            if txn.category_id.is_none() {
                let entry = counts.entry(txn.original_description.clone()).or_insert(0);
                if *entry == 0 {
                    order.push(txn.original_description.clone());
                }
                *entry += 1;
            }
        }
        self.import_cat_descriptions = order
            .into_iter()
            .map(|desc| {
                let count = counts.get(&desc).copied().unwrap_or(1);
                (desc, count)
            })
            .collect();
        self.import_cat_index = 0;
        self.import_cat_selected = 0;
        self.import_cat_scroll = 0;
        self.import_cat_new_name.clear();
        self.import_cat_creating = false;
        !self.import_cat_descriptions.is_empty()
    }

    /// Apply a category to the current description in the categorize step.
    /// Sets category_id on all matching transactions in import_preview.
    pub(crate) fn apply_category_to_current(&mut self, category_id: i64) {
        if let Some((desc, _)) = self.import_cat_descriptions.get(self.import_cat_index) {
            let desc = desc.clone();
            for txn in &mut self.import_preview {
                if txn.original_description == desc && txn.category_id.is_none() {
                    txn.category_id = Some(category_id);
                }
            }
        }
    }

    /// Advance to the next uncategorized description, or return false if done.
    pub(crate) fn advance_categorize(&mut self) -> bool {
        if self.import_cat_index + 1 < self.import_cat_descriptions.len() {
            self.import_cat_index += 1;
            self.import_cat_selected = 0;
            self.import_cat_scroll = 0;
            self.import_cat_creating = false;
            self.import_cat_new_name.clear();
            true
        } else {
            false
        }
    }

    /// Effective visible rows for the transaction table (borders + header = 3).
    pub(crate) fn transaction_page(&self) -> usize {
        self.visible_rows.saturating_sub(3).max(1)
    }

    /// Effective visible rows for the file browser list.
    /// Import step indicator (1) + path input box (3) + list borders (2) = 6.
    pub(crate) fn file_browser_page(&self) -> usize {
        self.visible_rows.saturating_sub(6).max(1)
    }

    /// Effective visible rows for the category list (borders = 2).
    pub(crate) fn category_page(&self) -> usize {
        self.visible_rows.saturating_sub(2).max(1)
    }

    /// Effective visible rows for the rules table (borders + header = 3).
    pub(crate) fn rule_page(&self) -> usize {
        self.visible_rows.saturating_sub(3).max(1)
    }

    /// Effective visible rows for the budget list (borders = 2).
    pub(crate) fn budget_page(&self) -> usize {
        self.visible_rows.saturating_sub(2).max(1)
    }

    /// Effective visible account cards. Each card is 4 rows high, plus 2 for borders on the outer block.
    pub(crate) fn accounts_page(&self) -> usize {
        // Each card is 4 lines. Outer block has no overhead since cards are direct.
        (self.visible_rows / 4).max(1)
    }

    /// Effective visible rows for the import account picker.
    /// Step indicator (1) + top info box (3) + list borders (2) = 6.
    pub(crate) fn import_account_page(&self) -> usize {
        self.visible_rows.saturating_sub(6).max(1)
    }

    /// Effective visible rows for the categorize category picker.
    /// Import step indicator (1) + description block (5) + list borders (2) = 8.
    pub(crate) fn categorize_visible_rows(&self) -> usize {
        self.visible_rows.saturating_sub(8).max(1)
    }

    pub(crate) fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
    }

    pub(crate) fn clear_selections(&mut self) {
        self.selected_transactions.clear();
    }
}
