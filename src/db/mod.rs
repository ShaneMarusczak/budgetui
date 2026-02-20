mod schema;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, Row};
use rust_decimal::Decimal;
use std::path::Path;
use std::str::FromStr;

use crate::models::*;

/// Parse a Decimal from a string, defaulting to zero on failure.
fn parse_decimal(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or_default()
}

/// Map a rusqlite Row to a Transaction. Expects columns in the standard order:
/// id, account_id, date, description, original_description, amount(TEXT),
/// category_id, notes, is_transfer, import_hash, created_at
fn row_to_transaction(row: &Row<'_>) -> rusqlite::Result<Transaction> {
    let amount_str: String = row.get(5)?;
    Ok(Transaction {
        id: Some(row.get(0)?),
        account_id: row.get(1)?,
        date: row.get(2)?,
        description: row.get(3)?,
        original_description: row.get(4)?,
        amount: parse_decimal(&amount_str),
        category_id: row.get(6)?,
        notes: row.get(7)?,
        is_transfer: row.get(8)?,
        import_hash: row.get(9)?,
        created_at: row.get(10)?,
    })
}

/// Standard SELECT columns for transaction queries.
const TXN_COLUMNS: &str = "t.id, t.account_id, t.date, t.description, t.original_description, \
     t.amount, t.category_id, t.notes, t.is_transfer, t.import_hash, t.created_at";

/// Build a dynamic SQL param vector and push a new boxed value, returning the placeholder string.
fn push_param(
    params: &mut Vec<Box<dyn rusqlite::types::ToSql>>,
    val: Box<dyn rusqlite::types::ToSql>,
) -> String {
    params.push(val);
    format!("?{}", params.len())
}

/// Escape LIKE special characters (`%`, `_`, `\`) so they match literally.
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

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

        for &(target_version, sql) in schema::MIGRATIONS {
            if current < target_version {
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
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))?;
        if count > 0 {
            return Ok(());
        }

        let defaults = [
            "Bills & Subscriptions",
            "Clothing",
            "Coffee Shops",
            "Doctor",
            "Education",
            "Electronics",
            "Entertainment",
            "Fees & Charges",
            "Flights",
            "Food & Dining",
            "Freelance",
            "Games",
            "Gas & Fuel",
            "Gifts & Donations",
            "Groceries",
            "Gym",
            "Health & Fitness",
            "Home & Garden",
            "Hotels",
            "Housing",
            "Income",
            "Insurance",
            "Interest",
            "Movies & Shows",
            "Parking",
            "Personal Care",
            "Pharmacy",
            "Public Transit",
            "Rent/Mortgage",
            "Restaurants",
            "Ride Share",
            "Shopping",
            "Streaming",
            "Transfer",
            "Transportation",
            "Travel",
            "Uncategorized",
            "Utilities",
        ];

        let tx = self.conn.transaction()?;
        for name in &defaults {
            tx.execute(
                "INSERT OR IGNORE INTO categories (name) VALUES (?1)",
                params![name],
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
                account_type: AccountType::parse(&row.get::<_, String>(2)?),
                institution: row.get(3)?,
                currency: row.get(4)?,
                notes: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub(crate) fn get_account_by_id(&self, id: i64) -> Result<Option<Account>> {
        let result = self.conn.query_row(
            "SELECT id, name, account_type, institution, currency, notes, created_at FROM accounts WHERE id = ?1",
            params![id],
            |row| {
                Ok(Account {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    account_type: AccountType::parse(&row.get::<_, String>(2)?),
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
        let mut sql = format!("SELECT {TXN_COLUMNS} FROM transactions t WHERE 1=1");
        let mut p: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(aid) = account_id {
            let ph = push_param(&mut p, Box::new(aid));
            sql.push_str(&format!(" AND t.account_id = {ph}"));
        }
        if let Some(cid) = category_id {
            let ph = push_param(&mut p, Box::new(cid));
            sql.push_str(&format!(" AND t.category_id = {ph}"));
        }
        if let Some(s) = search {
            let escaped = escape_like(s);
            let ph = push_param(&mut p, Box::new(format!("%{escaped}%")));
            sql.push_str(&format!(
                " AND (t.description LIKE {ph} ESCAPE '\\' \
                 OR t.original_description LIKE {ph} ESCAPE '\\' \
                 OR t.notes LIKE {ph} ESCAPE '\\')"
            ));
        }
        if let Some(m) = month {
            let ph = push_param(&mut p, Box::new(format!("{m}%")));
            sql.push_str(&format!(" AND t.date LIKE {ph}"));
        }

        sql.push_str(" ORDER BY t.date DESC, t.id DESC");

        if let Some(l) = limit {
            let ph = push_param(&mut p, Box::new(l));
            sql.push_str(&format!(" LIMIT {ph}"));
        }
        if let Some(o) = offset {
            let ph = push_param(&mut p, Box::new(o));
            sql.push_str(&format!(" OFFSET {ph}"));
        }

        let refs: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|v| v.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(refs.as_slice(), row_to_transaction)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
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

    pub(crate) fn delete_transactions_batch(&mut self, ids: &[i64]) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let mut count = 0;
        for &id in ids {
            count += tx.execute("DELETE FROM transactions WHERE id = ?1", params![id])?;
        }
        tx.commit()?;
        Ok(count)
    }

    pub(crate) fn get_all_transactions_for_export(
        &self,
        month: Option<&str>,
    ) -> Result<Vec<Transaction>> {
        let mut sql = format!("SELECT {TXN_COLUMNS} FROM transactions t");
        let mut p: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(m) = month {
            let ph = push_param(&mut p, Box::new(format!("{m}%")));
            sql.push_str(&format!(" WHERE t.date LIKE {ph}"));
        }
        sql.push_str(" ORDER BY t.date DESC, t.id DESC");

        let refs: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|v| v.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(refs.as_slice(), row_to_transaction)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    // ── Categories ────────────────────────────────────────────

    pub(crate) fn get_categories(&self) -> Result<Vec<Category>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM categories ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(Category {
                id: Some(row.get(0)?),
                name: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub(crate) fn insert_category(&self, cat: &Category) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO categories (name) VALUES (?1)",
            params![cat.name],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    // ── Budgets ───────────────────────────────────────────────

    pub(crate) fn get_budgets(&self, month: Option<&str>) -> Result<Vec<Budget>> {
        let mut sql =
            String::from("SELECT id, category_id, month, limit_amount FROM budgets");
        let mut p: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(m) = month {
            let ph = push_param(&mut p, Box::new(m.to_string()));
            sql.push_str(&format!(" WHERE month = {ph}"));
        }
        sql.push_str(" ORDER BY month DESC");
        let refs: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|v| v.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(refs.as_slice(), |row| {
            let amt_str: String = row.get(3)?;
            Ok(Budget {
                id: Some(row.get(0)?),
                category_id: row.get(1)?,
                month: row.get(2)?,
                limit_amount: parse_decimal(&amt_str),
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
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
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
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

    pub(crate) fn get_spending_by_category(
        &self,
        month: Option<&str>,
    ) -> Result<Vec<(String, Decimal)>> {
        let mut sql = String::from(
            "SELECT COALESCE(c.name, 'Uncategorized'), CAST(SUM(t.amount) AS TEXT)
             FROM transactions t
             LEFT JOIN categories c ON t.category_id = c.id
             WHERE CAST(t.amount AS REAL) < 0",
        );
        let mut p: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(m) = month {
            let ph = push_param(&mut p, Box::new(format!("{m}%")));
            sql.push_str(&format!(" AND t.date LIKE {ph}"));
        }
        sql.push_str(
            " GROUP BY COALESCE(c.name, 'Uncategorized')
             ORDER BY SUM(t.amount) ASC",
        );
        let refs: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|v| v.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(refs.as_slice(), |row| {
            let name: String = row.get(0)?;
            let amt_str: String = row.get(1)?;
            Ok((name, parse_decimal(&amt_str)))
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub(crate) fn get_monthly_totals(&self, month: Option<&str>) -> Result<(Decimal, Decimal)> {
        let query_sum = |sign: &str| -> Result<Decimal> {
            let mut sql = format!(
                "SELECT CAST(COALESCE(SUM(amount), 0) AS TEXT) FROM transactions WHERE CAST(amount AS REAL) {sign} 0"
            );
            let mut p: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            if let Some(m) = month {
                let ph = push_param(&mut p, Box::new(format!("{m}%")));
                sql.push_str(&format!(" AND date LIKE {ph}"));
            }
            let refs: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|v| v.as_ref()).collect();
            let val: String = self.conn.query_row(&sql, refs.as_slice(), |row| row.get(0))?;
            Ok(parse_decimal(&val))
        };
        Ok((query_sum(">")?, query_sum("<")?))
    }

    pub(crate) fn get_net_worth(&self) -> Result<Decimal> {
        let total: String = self.conn.query_row(
            "SELECT CAST(COALESCE(SUM(amount), 0) AS TEXT) FROM transactions",
            [],
            |row| row.get(0),
        )?;
        Ok(parse_decimal(&total))
    }

    /// Monthly income/expenses filtered by account type(s).
    pub(crate) fn get_monthly_totals_by_account_type(
        &self,
        month: Option<&str>,
        account_types: &[&str],
    ) -> Result<(Decimal, Decimal)> {
        let build_params = |sign: &str| -> (String, Vec<Box<dyn rusqlite::types::ToSql>>) {
            let mut p: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            let mut sql = String::from(
                "SELECT CAST(COALESCE(SUM(t.amount), 0) AS TEXT)
                 FROM transactions t JOIN accounts a ON t.account_id = a.id
                 WHERE CAST(t.amount AS REAL)",
            );
            sql.push_str(&format!(" {sign} 0"));
            if let Some(m) = month {
                let ph = push_param(&mut p, Box::new(format!("{m}%")));
                sql.push_str(&format!(" AND t.date LIKE {ph}"));
            }
            let placeholders: String = account_types
                .iter()
                .map(|at| push_param(&mut p, Box::new(at.to_string())))
                .collect::<Vec<_>>()
                .join(",");
            sql.push_str(&format!(" AND a.account_type IN ({placeholders})"));
            (sql, p)
        };

        let query_sum = |sign: &str| -> Result<Decimal> {
            let (sql, p) = build_params(sign);
            let refs: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|v| v.as_ref()).collect();
            let val: String = self
                .conn
                .query_row(&sql, refs.as_slice(), |row| row.get(0))?;
            Ok(parse_decimal(&val))
        };

        Ok((query_sum(">")?, query_sum("<")?))
    }

    /// All-time balance for accounts of the given type(s).
    pub(crate) fn get_balance_by_account_type(&self, account_types: &[&str]) -> Result<Decimal> {
        let placeholders: String = (0..account_types.len())
            .map(|i| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT CAST(COALESCE(SUM(t.amount), 0) AS TEXT)
             FROM transactions t JOIN accounts a ON t.account_id = a.id
             WHERE a.account_type IN ({placeholders})"
        );
        let p: Vec<Box<dyn rusqlite::types::ToSql>> = account_types
            .iter()
            .map(|at| Box::new(at.to_string()) as _)
            .collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|v| v.as_ref()).collect();
        let total: String = self
            .conn
            .query_row(&sql, refs.as_slice(), |row| row.get(0))?;
        Ok(parse_decimal(&total))
    }

    /// Income/expenses for a single account, optionally filtered by month.
    pub(crate) fn get_account_monthly_totals(
        &self,
        account_id: i64,
        month: Option<&str>,
    ) -> Result<(Decimal, Decimal)> {
        let query_sum = |sign: &str| -> Result<Decimal> {
            let mut sql = format!(
                "SELECT CAST(COALESCE(SUM(amount), 0) AS TEXT) FROM transactions WHERE account_id = ?1 AND CAST(amount AS REAL) {sign} 0"
            );
            let mut p: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            p.push(Box::new(account_id));
            if let Some(m) = month {
                let ph = push_param(&mut p, Box::new(format!("{m}%")));
                sql.push_str(&format!(" AND date LIKE {ph}"));
            }
            let refs: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|v| v.as_ref()).collect();
            let val: String = self.conn.query_row(&sql, refs.as_slice(), |row| row.get(0))?;
            Ok(parse_decimal(&val))
        };
        Ok((query_sum(">")?, query_sum("<")?))
    }

    /// All-time balance for a single account.
    pub(crate) fn get_account_balance(&self, account_id: i64) -> Result<Decimal> {
        let total: String = self.conn.query_row(
            "SELECT CAST(COALESCE(SUM(amount), 0) AS TEXT) FROM transactions WHERE account_id = ?1",
            params![account_id],
            |row| row.get(0),
        )?;
        Ok(parse_decimal(&total))
    }

    pub(crate) fn get_monthly_trend(
        &self,
        months: usize,
    ) -> Result<Vec<(String, Decimal, Decimal)>> {
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
            Ok((month, parse_decimal(&inc_str), parse_decimal(&exp_str)))
        })?;
        let mut result: Vec<_> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        result.reverse();
        Ok(result)
    }

    /// Export transactions to a CSV file. Returns the number of transactions written.
    pub(crate) fn export_to_csv(
        &self,
        path: &str,
        month: Option<&str>,
    ) -> Result<usize> {
        let txns = self.get_all_transactions_for_export(month)?;
        if txns.is_empty() {
            return Ok(0);
        }

        let categories = self.get_categories()?;
        let accounts = self.get_accounts()?;

        let mut wtr =
            csv::Writer::from_path(path).context("Failed to create export file")?;
        wtr.write_record(["Date", "Description", "Amount", "Category", "Account", "Notes"])?;

        for txn in &txns {
            let cat_name = txn
                .category_id
                .and_then(|cid| Category::find_by_id(&categories, cid))
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
        Ok(txns.len())
    }
}

#[cfg(test)]
mod tests;
