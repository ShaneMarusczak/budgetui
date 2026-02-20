mod schema;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use rusqlite::{params, Connection};
use std::path::Path;
use std::str::FromStr;

use crate::models::*;

pub(crate) struct Database {
    conn: Connection,
}

impl Database {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database: {}", path.display()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .context("Failed to set database pragmas")?;
        let mut db = Self { conn };
        db.migrate().context("Database migration failed")?;
        db.seed_default_categories()?;
        Ok(db)
    }

    #[cfg(test)]
    pub(crate) fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let mut db = Self { conn };
        db.migrate()?;
        db.seed_default_categories()?;
        Ok(db)
    }

    fn migrate(&mut self) -> Result<()> {
        // Check if schema_version table exists
        let has_version_table: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_version')",
            [],
            |row| row.get(0),
        )?;

        if !has_version_table {
            // Fresh database - apply full schema
            self.conn.execute_batch(schema::SCHEMA_V1)?;
            self.conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                params![schema::CURRENT_VERSION],
            )?;
            return Ok(());
        }

        // Existing database - check version and apply migrations
        let current: i32 = self
            .conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        for &(from_version, sql) in schema::MIGRATIONS {
            if current <= from_version {
                self.conn.execute_batch(sql)?;
            }
        }

        if current < schema::CURRENT_VERSION {
            self.conn.execute(
                "UPDATE schema_version SET version = ?1",
                params![schema::CURRENT_VERSION],
            )?;
        }

        Ok(())
    }

    fn seed_default_categories(&mut self) -> Result<()> {
        let count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))?;
        if count > 0 {
            return Ok(());
        }

        let defaults = vec![
            ("Income", None::<&str>),
            ("Salary", Some("Income")),
            ("Freelance", Some("Income")),
            ("Interest", Some("Income")),
            ("Housing", None),
            ("Rent/Mortgage", Some("Housing")),
            ("Utilities", Some("Housing")),
            ("Insurance", Some("Housing")),
            ("Food & Dining", None),
            ("Groceries", Some("Food & Dining")),
            ("Restaurants", Some("Food & Dining")),
            ("Coffee Shops", Some("Food & Dining")),
            ("Transportation", None),
            ("Gas & Fuel", Some("Transportation")),
            ("Parking", Some("Transportation")),
            ("Public Transit", Some("Transportation")),
            ("Ride Share", Some("Transportation")),
            ("Shopping", None),
            ("Clothing", Some("Shopping")),
            ("Electronics", Some("Shopping")),
            ("Home & Garden", Some("Shopping")),
            ("Health & Fitness", None),
            ("Gym", Some("Health & Fitness")),
            ("Pharmacy", Some("Health & Fitness")),
            ("Doctor", Some("Health & Fitness")),
            ("Entertainment", None),
            ("Streaming", Some("Entertainment")),
            ("Movies & Shows", Some("Entertainment")),
            ("Games", Some("Entertainment")),
            ("Travel", None),
            ("Hotels", Some("Travel")),
            ("Flights", Some("Travel")),
            ("Personal Care", None),
            ("Education", None),
            ("Gifts & Donations", None),
            ("Bills & Subscriptions", None),
            ("Fees & Charges", None),
            ("Transfer", None),
            ("Uncategorized", None),
        ];

        let tx = self.conn.transaction()?;
        for (name, parent_name) in &defaults {
            let parent_id: Option<i64> = if let Some(pname) = parent_name {
                tx.query_row(
                    "SELECT id FROM categories WHERE name = ?1",
                    params![pname],
                    |row| row.get(0),
                )
                .ok()
            } else {
                None
            };
            tx.execute(
                "INSERT OR IGNORE INTO categories (name, parent_id, icon, color) VALUES (?1, ?2, '', '')",
                params![name, parent_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    // ── Accounts ──────────────────────────────────────────────

    pub(crate) fn insert_account(&self, account: &Account) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO accounts (name, account_type, institution, currency, notes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                account.name,
                account.account_type.as_str(),
                account.institution,
                account.currency,
                account.notes,
                account.created_at,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub(crate) fn get_accounts(&self) -> Result<Vec<Account>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, account_type, institution, currency, notes, created_at FROM accounts ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(Account {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                account_type: AccountType::from_str(
                    &row.get::<_, String>(2)?,
                ),
                institution: row.get(3)?,
                currency: row.get(4)?,
                notes: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub(crate) fn get_account_by_id(&self, id: i64) -> Result<Option<Account>> {
        let result = self.conn.query_row(
            "SELECT id, name, account_type, institution, currency, notes, created_at FROM accounts WHERE id = ?1",
            params![id],
            |row| {
                Ok(Account {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    account_type: AccountType::from_str(&row.get::<_, String>(2)?),
                    institution: row.get(3)?,
                    currency: row.get(4)?,
                    notes: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        );
        match result {
            Ok(a) => Ok(Some(a)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // ── Transactions ──────────────────────────────────────────

    pub(crate) fn insert_transaction(&self, txn: &Transaction) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO transactions (account_id, date, description, original_description, amount, category_id, notes, is_transfer, import_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                txn.account_id,
                txn.date,
                txn.description,
                txn.original_description,
                txn.amount.to_string(),
                txn.category_id,
                txn.notes,
                txn.is_transfer,
                txn.import_hash,
                txn.created_at,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub(crate) fn insert_transactions_batch(&mut self, txns: &[Transaction]) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let mut count = 0;
        for txn in txns {
            // Skip duplicates based on import_hash (only when hash is non-empty)
            if !txn.import_hash.is_empty() {
                let exists: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM transactions WHERE import_hash = ?1 AND import_hash != '')",
                    params![txn.import_hash],
                    |row| row.get(0),
                )?;
                if exists {
                    continue;
                }
            }
            tx.execute(
                "INSERT INTO transactions (account_id, date, description, original_description, amount, category_id, notes, is_transfer, import_hash, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    txn.account_id,
                    txn.date,
                    txn.description,
                    txn.original_description,
                    txn.amount.to_string(),
                    txn.category_id,
                    txn.notes,
                    txn.is_transfer,
                    txn.import_hash,
                    txn.created_at,
                ],
            )?;
            count += 1;
        }
        tx.commit()?;
        Ok(count)
    }

    pub(crate) fn get_transactions(
        &self,
        limit: Option<u32>,
        offset: Option<u32>,
        account_id: Option<i64>,
        category_id: Option<i64>,
        search: Option<&str>,
        month: Option<&str>,
    ) -> Result<Vec<Transaction>> {
        let mut sql = String::from(
            "SELECT t.id, t.account_id, t.date, t.description, t.original_description,
                    t.amount, t.category_id, t.notes, t.is_transfer, t.import_hash, t.created_at
             FROM transactions t WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(aid) = account_id {
            sql.push_str(&format!(
                " AND t.account_id = ?{}",
                param_values.len() + 1
            ));
            param_values.push(Box::new(aid));
        }
        if let Some(cid) = category_id {
            sql.push_str(&format!(
                " AND t.category_id = ?{}",
                param_values.len() + 1
            ));
            param_values.push(Box::new(cid));
        }
        if let Some(s) = search {
            sql.push_str(&format!(
                " AND (t.description LIKE ?{0} OR t.original_description LIKE ?{0} OR t.notes LIKE ?{0})",
                param_values.len() + 1
            ));
            param_values.push(Box::new(format!("%{s}%")));
        }
        if let Some(m) = month {
            sql.push_str(&format!(
                " AND t.date LIKE ?{}",
                param_values.len() + 1
            ));
            param_values.push(Box::new(format!("{m}%")));
        }

        sql.push_str(" ORDER BY t.date DESC, t.id DESC");

        if let Some(l) = limit {
            sql.push_str(&format!(" LIMIT {l}"));
        }
        if let Some(o) = offset {
            sql.push_str(&format!(" OFFSET {o}"));
        }

        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_ref.as_slice(), |row| {
            let amount_str: String = row.get(5)?;
            Ok(Transaction {
                id: Some(row.get(0)?),
                account_id: row.get(1)?,
                date: row.get(2)?,
                description: row.get(3)?,
                original_description: row.get(4)?,
                amount: Decimal::from_str(&amount_str).unwrap_or_default(),
                category_id: row.get(6)?,
                notes: row.get(7)?,
                is_transfer: row.get(8)?,
                import_hash: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub(crate) fn get_transaction_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| row.get(0))?)
    }

    pub(crate) fn update_transaction_category(
        &self,
        transaction_id: i64,
        category_id: Option<i64>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE transactions SET category_id = ?1 WHERE id = ?2",
            params![category_id, transaction_id],
        )?;
        Ok(())
    }

    pub(crate) fn update_transaction_description(
        &self,
        transaction_id: i64,
        description: &str,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE transactions SET description = ?1 WHERE id = ?2",
            params![description, transaction_id],
        )?;
        Ok(())
    }

    pub(crate) fn delete_transaction(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM transactions WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub(crate) fn get_all_transactions_for_export(
        &self,
        month: Option<&str>,
    ) -> Result<Vec<Transaction>> {
        let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(m) = month {
            (
                "SELECT t.id, t.account_id, t.date, t.description, t.original_description,
                        t.amount, t.category_id, t.notes, t.is_transfer, t.import_hash, t.created_at
                 FROM transactions t WHERE t.date LIKE ?1
                 ORDER BY t.date DESC, t.id DESC".into(),
                vec![Box::new(format!("{m}%"))],
            )
        } else {
            (
                "SELECT t.id, t.account_id, t.date, t.description, t.original_description,
                        t.amount, t.category_id, t.notes, t.is_transfer, t.import_hash, t.created_at
                 FROM transactions t
                 ORDER BY t.date DESC, t.id DESC".into(),
                vec![],
            )
        };

        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_ref.as_slice(), |row| {
            let amount_str: String = row.get(5)?;
            Ok(Transaction {
                id: Some(row.get(0)?),
                account_id: row.get(1)?,
                date: row.get(2)?,
                description: row.get(3)?,
                original_description: row.get(4)?,
                amount: Decimal::from_str(&amount_str).unwrap_or_default(),
                category_id: row.get(6)?,
                notes: row.get(7)?,
                is_transfer: row.get(8)?,
                import_hash: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    // ── Categories ────────────────────────────────────────────

    pub(crate) fn get_categories(&self) -> Result<Vec<Category>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, parent_id, icon, color FROM categories ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(Category {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                parent_id: row.get(2)?,
                icon: row.get(3)?,
                color: row.get(4)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub(crate) fn get_category_by_id(&self, id: i64) -> Result<Option<Category>> {
        let result = self.conn.query_row(
            "SELECT id, name, parent_id, icon, color FROM categories WHERE id = ?1",
            params![id],
            |row| {
                Ok(Category {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    parent_id: row.get(2)?,
                    icon: row.get(3)?,
                    color: row.get(4)?,
                })
            },
        );
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub(crate) fn insert_category(&self, cat: &Category) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO categories (name, parent_id, icon, color) VALUES (?1, ?2, ?3, ?4)",
            params![cat.name, cat.parent_id, cat.icon, cat.color],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    // ── Budgets ───────────────────────────────────────────────

    pub(crate) fn get_budgets(&self, month: &str) -> Result<Vec<Budget>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category_id, month, limit_amount FROM budgets WHERE month = ?1",
        )?;
        let rows = stmt.query_map(params![month], |row| {
            let amt_str: String = row.get(3)?;
            Ok(Budget {
                id: Some(row.get(0)?),
                category_id: row.get(1)?,
                month: row.get(2)?,
                limit_amount: Decimal::from_str(&amt_str).unwrap_or_default(),
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub(crate) fn upsert_budget(&self, budget: &Budget) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO budgets (category_id, month, limit_amount)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(category_id, month) DO UPDATE SET limit_amount = ?3",
            params![
                budget.category_id,
                budget.month,
                budget.limit_amount.to_string(),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub(crate) fn delete_budget(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM budgets WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Import Rules ──────────────────────────────────────────

    pub(crate) fn get_import_rules(&self) -> Result<Vec<ImportRule>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, pattern, category_id, is_regex, priority FROM import_rules ORDER BY priority DESC, pattern",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ImportRule {
                id: Some(row.get(0)?),
                pattern: row.get(1)?,
                category_id: row.get(2)?,
                is_regex: row.get(3)?,
                priority: row.get(4)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub(crate) fn insert_import_rule(&self, rule: &ImportRule) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO import_rules (pattern, category_id, is_regex, priority)
             VALUES (?1, ?2, ?3, ?4)",
            params![rule.pattern, rule.category_id, rule.is_regex, rule.priority],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub(crate) fn delete_import_rule(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM import_rules WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Analytics ─────────────────────────────────────────────

    pub(crate) fn get_spending_by_category(&self, month: &str) -> Result<Vec<(String, Decimal)>> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(c.name, 'Uncategorized'), CAST(SUM(t.amount) AS TEXT)
             FROM transactions t
             LEFT JOIN categories c ON t.category_id = c.id
             WHERE t.date LIKE ?1 AND CAST(t.amount AS REAL) < 0
             GROUP BY COALESCE(c.name, 'Uncategorized')
             ORDER BY SUM(t.amount) ASC",
        )?;
        let rows = stmt.query_map(params![format!("{month}%")], |row| {
            let name: String = row.get(0)?;
            let amt_str: String = row.get(1)?;
            Ok((name, Decimal::from_str(&amt_str).unwrap_or_default()))
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub(crate) fn get_monthly_totals(&self, month: &str) -> Result<(Decimal, Decimal)> {
        let income: String = self
            .conn
            .query_row(
                "SELECT CAST(COALESCE(SUM(amount), 0) AS TEXT) FROM transactions WHERE date LIKE ?1 AND CAST(amount AS REAL) > 0",
                params![format!("{month}%")],
                |row| row.get(0),
            )?;
        let expenses: String = self
            .conn
            .query_row(
                "SELECT CAST(COALESCE(SUM(amount), 0) AS TEXT) FROM transactions WHERE date LIKE ?1 AND CAST(amount AS REAL) < 0",
                params![format!("{month}%")],
                |row| row.get(0),
            )?;
        Ok((
            Decimal::from_str(&income).unwrap_or_default(),
            Decimal::from_str(&expenses).unwrap_or_default(),
        ))
    }

    pub(crate) fn get_net_worth(&self) -> Result<Decimal> {
        let total: String = self.conn.query_row(
            "SELECT CAST(COALESCE(SUM(amount), 0) AS TEXT) FROM transactions",
            [],
            |row| row.get(0),
        )?;
        Ok(Decimal::from_str(&total).unwrap_or_default())
    }

    pub(crate) fn get_monthly_trend(&self, months: usize) -> Result<Vec<(String, Decimal, Decimal)>> {
        let mut stmt = self.conn.prepare(
            "SELECT strftime('%Y-%m', date) as month,
                    CAST(SUM(CASE WHEN CAST(amount AS REAL) > 0 THEN amount ELSE 0 END) AS TEXT) as income,
                    CAST(SUM(CASE WHEN CAST(amount AS REAL) < 0 THEN amount ELSE 0 END) AS TEXT) as expenses
             FROM transactions
             GROUP BY month
             ORDER BY month DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![months as i64], |row| {
            let month: String = row.get(0)?;
            let inc_str: String = row.get(1)?;
            let exp_str: String = row.get(2)?;
            Ok((
                month,
                Decimal::from_str(&inc_str).unwrap_or_default(),
                Decimal::from_str(&exp_str).unwrap_or_default(),
            ))
        })?;
        let mut result: Vec<_> = rows.filter_map(|r| r.ok()).collect();
        result.reverse();
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
