pub(crate) const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS accounts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    account_type TEXT NOT NULL DEFAULT 'Checking',
    institution TEXT NOT NULL DEFAULT '',
    currency    TEXT NOT NULL DEFAULT 'USD',
    notes       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS categories (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    name      TEXT NOT NULL UNIQUE,
    parent_id INTEGER REFERENCES categories(id),
    icon      TEXT NOT NULL DEFAULT '',
    color     TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS transactions (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id            INTEGER NOT NULL REFERENCES accounts(id),
    date                  TEXT NOT NULL,
    description           TEXT NOT NULL,
    original_description  TEXT NOT NULL DEFAULT '',
    amount                TEXT NOT NULL,
    category_id           INTEGER REFERENCES categories(id),
    notes                 TEXT NOT NULL DEFAULT '',
    is_transfer           BOOLEAN NOT NULL DEFAULT 0,
    import_hash           TEXT NOT NULL DEFAULT '',
    created_at            TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions(date);
CREATE INDEX IF NOT EXISTS idx_transactions_account ON transactions(account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_category ON transactions(category_id);
CREATE INDEX IF NOT EXISTS idx_transactions_hash ON transactions(import_hash);
CREATE UNIQUE INDEX IF NOT EXISTS idx_transactions_hash_unique ON transactions(import_hash) WHERE import_hash != '';

CREATE TABLE IF NOT EXISTS budgets (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id   INTEGER NOT NULL REFERENCES categories(id),
    month         TEXT NOT NULL,
    limit_amount  TEXT NOT NULL,
    UNIQUE(category_id, month)
);

CREATE TABLE IF NOT EXISTS import_rules (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    pattern     TEXT NOT NULL,
    category_id INTEGER NOT NULL REFERENCES categories(id),
    is_regex    BOOLEAN NOT NULL DEFAULT 0,
    priority    INTEGER NOT NULL DEFAULT 0
);

"#;

pub(crate) const CURRENT_VERSION: i32 = 1;

/// Migrations from version N to N+1.
/// Each entry is (from_version, sql).
pub(crate) const MIGRATIONS: &[(i32, &str)] = &[
    // Future migrations go here:
    // (1, "ALTER TABLE transactions ADD COLUMN recurring BOOLEAN NOT NULL DEFAULT 0;"),
];
