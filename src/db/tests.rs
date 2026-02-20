#![allow(clippy::unwrap_used)]

use super::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// ── Default data ──────────────────────────────────────────────

#[test]
fn test_default_categories_seeded() {
    let db = Database::open_in_memory().unwrap();
    let cats = db.get_categories().unwrap();
    assert!(!cats.is_empty());
    assert!(cats.iter().any(|c| c.name == "Income"));
    assert!(cats.iter().any(|c| c.name == "Uncategorized"));
}

#[test]
fn test_default_categories_not_reseeded() {
    let db = Database::open_in_memory().unwrap();
    let count_before = db.get_categories().unwrap().len();
    // seed_default_categories is called by open_in_memory; calling it again shouldn't dupe
    let count_after = db.get_categories().unwrap().len();
    assert_eq!(count_before, count_after);
}

// ── Account CRUD ──────────────────────────────────────────────

#[test]
fn test_account_crud() {
    let db = Database::open_in_memory().unwrap();
    let account = Account::new("Test Bank".into(), AccountType::Checking, String::new());
    let id = db.insert_account(&account).unwrap();

    let fetched = db.get_account_by_id(id).unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().name, "Test Bank");

    let all = db.get_accounts().unwrap();
    assert!(!all.is_empty());
}

#[test]
fn test_account_by_id_not_found() {
    let db = Database::open_in_memory().unwrap();
    let result = db.get_account_by_id(99999).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_multiple_accounts() {
    let db = Database::open_in_memory().unwrap();
    let a1 = Account::new(
        "Chase Checking".into(),
        AccountType::Checking,
        "Chase".into(),
    );
    let a2 = Account::new("Savings".into(), AccountType::Savings, "Bank".into());
    let a3 = Account::new("Visa".into(), AccountType::CreditCard, "Capital One".into());
    db.insert_account(&a1).unwrap();
    db.insert_account(&a2).unwrap();
    db.insert_account(&a3).unwrap();

    let all = db.get_accounts().unwrap();
    assert!(all.len() >= 3);
    // Accounts are sorted by name
    let names: Vec<&str> = all.iter().map(|a| a.name.as_str()).collect();
    let mut sorted_names = names.clone();
    sorted_names.sort();
    assert_eq!(names, sorted_names);
}

// ── Transaction CRUD ──────────────────────────────────────────

fn setup_test_data(db: &mut Database) -> i64 {
    let account = Account::new("Test".into(), AccountType::Checking, String::new());
    let account_id = db.insert_account(&account).unwrap();

    let txns = vec![
        Transaction {
            id: None,
            account_id,
            date: "2024-01-10".into(),
            description: "Starbucks Coffee".into(),
            original_description: "STARBUCKS #123".into(),
            amount: dec!(-5.25),
            category_id: None,
            notes: "morning coffee".into(),
            is_transfer: false,
            import_hash: "hash-1".into(),
            created_at: "2024-01-10T00:00:00Z".into(),
        },
        Transaction {
            id: None,
            account_id,
            date: "2024-01-15".into(),
            description: "Amazon Purchase".into(),
            original_description: "AMZN MKTP US".into(),
            amount: dec!(-42.99),
            category_id: None,
            notes: String::new(),
            is_transfer: false,
            import_hash: "hash-2".into(),
            created_at: "2024-01-15T00:00:00Z".into(),
        },
        Transaction {
            id: None,
            account_id,
            date: "2024-01-20".into(),
            description: "Salary Deposit".into(),
            original_description: "ACME CORP PAYROLL".into(),
            amount: dec!(3000.00),
            category_id: None,
            notes: String::new(),
            is_transfer: false,
            import_hash: "hash-3".into(),
            created_at: "2024-01-20T00:00:00Z".into(),
        },
        Transaction {
            id: None,
            account_id,
            date: "2024-02-05".into(),
            description: "Grocery Store".into(),
            original_description: "WHOLE FOODS #456".into(),
            amount: dec!(-87.30),
            category_id: None,
            notes: String::new(),
            is_transfer: false,
            import_hash: "hash-4".into(),
            created_at: "2024-02-05T00:00:00Z".into(),
        },
    ];

    for txn in &txns {
        db.insert_transaction(txn).unwrap();
    }

    account_id
}

#[test]
fn test_transaction_insert_and_query() {
    let mut db = Database::open_in_memory().unwrap();
    let account = Account::new("Test".into(), AccountType::Checking, String::new());
    let account_id = db.insert_account(&account).unwrap();

    let txn = Transaction {
        id: None,
        account_id,
        date: "2024-01-15".into(),
        description: "Coffee Shop".into(),
        original_description: "COFFEE SHOP #123".into(),
        amount: dec!(-4.50),
        category_id: None,
        notes: String::new(),
        is_transfer: false,
        import_hash: "test-hash-1".into(),
        created_at: "2024-01-15T00:00:00Z".into(),
    };

    assert!(txn.is_expense());
    assert!(!txn.is_income());
    assert_eq!(txn.abs_amount(), dec!(4.50));

    let txn_id = db.insert_transaction(&txn).unwrap();
    assert!(txn_id > 0);

    // Test dedup
    let batch_count = db
        .insert_transactions_batch(std::slice::from_ref(&txn))
        .unwrap();
    assert_eq!(batch_count, 0); // duplicate skipped

    let txns = db
        .get_transactions(Some(10), None, None, None, None, Some("2024-01"))
        .unwrap();
    assert_eq!(txns.len(), 1);

    // Update description
    db.update_transaction_description(txn_id, "My Coffee")
        .unwrap();
    let updated = db
        .get_transactions(Some(1), None, None, None, None, None)
        .unwrap();
    assert_eq!(updated[0].description, "My Coffee");

    // Update category
    let cats = db.get_categories().unwrap();
    let food_cat = cats.iter().find(|c| c.name == "Food & Dining").unwrap();
    db.update_transaction_category(txn_id, food_cat.id).unwrap();
}

#[test]
fn test_transaction_search() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let results = db
        .get_transactions(Some(100), None, None, None, Some("coffee"), None)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].description, "Starbucks Coffee");

    // Search by notes
    let results = db
        .get_transactions(Some(100), None, None, None, Some("morning"), None)
        .unwrap();
    assert_eq!(results.len(), 1);

    // Search by original description
    let results = db
        .get_transactions(Some(100), None, None, None, Some("AMZN"), None)
        .unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_transaction_search_no_results() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let results = db
        .get_transactions(Some(100), None, None, None, Some("nonexistent"), None)
        .unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_transaction_month_filter() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let jan = db
        .get_transactions(Some(100), None, None, None, None, Some("2024-01"))
        .unwrap();
    assert_eq!(jan.len(), 3);

    let feb = db
        .get_transactions(Some(100), None, None, None, None, Some("2024-02"))
        .unwrap();
    assert_eq!(feb.len(), 1);

    let all = db
        .get_transactions(Some(100), None, None, None, None, None)
        .unwrap();
    assert_eq!(all.len(), 4);
}

#[test]
fn test_transaction_month_filter_no_results() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let results = db
        .get_transactions(Some(100), None, None, None, None, Some("2025-06"))
        .unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_transaction_account_filter() {
    let mut db = Database::open_in_memory().unwrap();
    let account_id = setup_test_data(&mut db);

    let results = db
        .get_transactions(Some(100), None, Some(account_id), None, None, None)
        .unwrap();
    assert_eq!(results.len(), 4);

    let results = db
        .get_transactions(Some(100), None, Some(9999), None, None, None)
        .unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_transaction_category_filter() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let cats = db.get_categories().unwrap();
    let food_id = cats
        .iter()
        .find(|c| c.name == "Food & Dining")
        .unwrap()
        .id
        .unwrap();

    // Assign one transaction to a category
    let txns = db
        .get_transactions(Some(100), None, None, None, None, None)
        .unwrap();
    db.update_transaction_category(txns[0].id.unwrap(), Some(food_id))
        .unwrap();

    let filtered = db
        .get_transactions(Some(100), None, None, Some(food_id), None, None)
        .unwrap();
    assert_eq!(filtered.len(), 1);
}

#[test]
fn test_transaction_combined_filters() {
    let mut db = Database::open_in_memory().unwrap();
    let account_id = setup_test_data(&mut db);

    // Search + month filter
    let results = db
        .get_transactions(
            Some(100),
            None,
            Some(account_id),
            None,
            Some("coffee"),
            Some("2024-01"),
        )
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].description, "Starbucks Coffee");
}

#[test]
fn test_transaction_limit_offset() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let limited = db
        .get_transactions(Some(2), None, None, None, None, None)
        .unwrap();
    assert_eq!(limited.len(), 2);

    let offset = db
        .get_transactions(Some(2), Some(2), None, None, None, None)
        .unwrap();
    assert_eq!(offset.len(), 2);

    // Offset results should be different from non-offset
    assert_ne!(limited[0].description, offset[0].description);
}

#[test]
fn test_transaction_delete() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let txns = db
        .get_transactions(Some(100), None, None, None, None, None)
        .unwrap();
    let count_before = txns.len();
    let id = txns[0].id.unwrap();

    db.delete_transaction(id).unwrap();

    let txns = db
        .get_transactions(Some(100), None, None, None, None, None)
        .unwrap();
    assert_eq!(txns.len(), count_before - 1);
    assert!(!txns.iter().any(|t| t.id == Some(id)));
}

#[test]
fn test_transaction_delete_batch() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let txns = db
        .get_transactions(Some(100), None, None, None, None, None)
        .unwrap();
    let count_before = txns.len();
    let ids: Vec<i64> = txns.iter().take(2).filter_map(|t| t.id).collect();

    let deleted = db.delete_transactions_batch(&ids).unwrap();
    assert_eq!(deleted, 2);

    let txns = db
        .get_transactions(Some(100), None, None, None, None, None)
        .unwrap();
    assert_eq!(txns.len(), count_before - 2);
    for id in &ids {
        assert!(!txns.iter().any(|t| t.id == Some(*id)));
    }
}

#[test]
fn test_transaction_ordering() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let txns = db
        .get_transactions(Some(100), None, None, None, None, None)
        .unwrap();
    // Should be ordered by date DESC, id DESC
    for window in txns.windows(2) {
        assert!(window[0].date >= window[1].date);
    }
}

// ── Category CRUD ─────────────────────────────────────────────

#[test]
fn test_category_crud() {
    let db = Database::open_in_memory().unwrap();
    let cat = Category::new("Test Category".into());
    let id = db.insert_category(&cat).unwrap();
    assert!(id > 0);

    let cats = db.get_categories().unwrap();
    let fetched = Category::find_by_id(&cats, id);
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().name, "Test Category");
}

#[test]
fn test_category_by_id_not_found() {
    let db = Database::open_in_memory().unwrap();
    let cats = db.get_categories().unwrap();
    let result = Category::find_by_id(&cats, 99999);
    assert!(result.is_none());
}

#[test]
fn test_categories_sorted_by_name() {
    let db = Database::open_in_memory().unwrap();
    let cats = db.get_categories().unwrap();
    let names: Vec<&str> = cats.iter().map(|c| c.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
}

// ── Budget CRUD ───────────────────────────────────────────────

#[test]
fn test_budget_crud() {
    let db = Database::open_in_memory().unwrap();
    let cats = db.get_categories().unwrap();
    let food_id = cats
        .iter()
        .find(|c| c.name == "Food & Dining")
        .unwrap()
        .id
        .unwrap();

    let budget = Budget::new(food_id, "2024-01".into(), dec!(500));
    let id = db.upsert_budget(&budget).unwrap();
    assert!(id > 0);

    let budgets = db.get_budgets(Some("2024-01")).unwrap();
    assert_eq!(budgets.len(), 1);
    assert_eq!(budgets[0].limit_amount, dec!(500));

    // Upsert with new amount
    let updated = Budget::new(food_id, "2024-01".into(), dec!(600));
    db.upsert_budget(&updated).unwrap();
    let budgets = db.get_budgets(Some("2024-01")).unwrap();
    assert_eq!(budgets.len(), 1);
    assert_eq!(budgets[0].limit_amount, dec!(600));

    db.delete_budget(budgets[0].id.unwrap()).unwrap();
    let budgets = db.get_budgets(Some("2024-01")).unwrap();
    assert!(budgets.is_empty());
}

#[test]
fn test_budget_different_months() {
    let db = Database::open_in_memory().unwrap();
    let cats = db.get_categories().unwrap();
    let food_id = cats
        .iter()
        .find(|c| c.name == "Food & Dining")
        .unwrap()
        .id
        .unwrap();

    db.upsert_budget(&Budget::new(food_id, "2024-01".into(), dec!(500)))
        .unwrap();
    db.upsert_budget(&Budget::new(food_id, "2024-02".into(), dec!(600)))
        .unwrap();

    assert_eq!(db.get_budgets(Some("2024-01")).unwrap().len(), 1);
    assert_eq!(db.get_budgets(Some("2024-02")).unwrap().len(), 1);
    assert_eq!(db.get_budgets(Some("2024-03")).unwrap().len(), 0);
}

// ── Import Rule CRUD ──────────────────────────────────────────

#[test]
fn test_import_rule_crud() {
    let db = Database::open_in_memory().unwrap();
    let cats = db.get_categories().unwrap();
    let shopping_id = cats
        .iter()
        .find(|c| c.name == "Shopping")
        .unwrap()
        .id
        .unwrap();

    let rule = ImportRule::new_contains("amazon".into(), shopping_id);
    let id = db.insert_import_rule(&rule).unwrap();
    assert!(id > 0);

    let regex_rule = ImportRule::new_regex("^AMZN.*".into(), shopping_id);
    let regex_id = db.insert_import_rule(&regex_rule).unwrap();
    assert!(regex_id > 0);

    let rules = db.get_import_rules().unwrap();
    assert!(rules.len() >= 2);

    db.delete_import_rule(id).unwrap();
    let rules = db.get_import_rules().unwrap();
    assert!(rules.iter().all(|r| r.pattern != "amazon"));
}

#[test]
fn test_import_rules_ordered_by_priority() {
    let db = Database::open_in_memory().unwrap();
    let cats = db.get_categories().unwrap();
    let cat_id = cats
        .iter()
        .find(|c| c.name == "Shopping")
        .unwrap()
        .id
        .unwrap();

    let mut low = ImportRule::new_contains("low".into(), cat_id);
    low.priority = 1;
    let mut high = ImportRule::new_contains("high".into(), cat_id);
    high.priority = 10;
    db.insert_import_rule(&low).unwrap();
    db.insert_import_rule(&high).unwrap();

    let rules = db.get_import_rules().unwrap();
    // Higher priority first
    let high_idx = rules.iter().position(|r| r.pattern == "high").unwrap();
    let low_idx = rules.iter().position(|r| r.pattern == "low").unwrap();
    assert!(high_idx < low_idx);
}

// ── Analytics ─────────────────────────────────────────────────

#[test]
fn test_monthly_totals() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let (income, expenses) = db.get_monthly_totals(Some("2024-01")).unwrap();
    assert_eq!(income, dec!(3000.00));
    assert!(expenses < Decimal::ZERO);
    assert_eq!(expenses, dec!(-5.25) + dec!(-42.99));
}

#[test]
fn test_monthly_totals_empty_month() {
    let db = Database::open_in_memory().unwrap();
    let (income, expenses) = db.get_monthly_totals(Some("2099-01")).unwrap();
    assert_eq!(income, Decimal::ZERO);
    assert_eq!(expenses, Decimal::ZERO);
}

#[test]
fn test_net_worth() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let net = db.get_net_worth().unwrap();
    // 3000 - 5.25 - 42.99 - 87.30 = 2864.46
    assert_eq!(net, dec!(2864.46));
}

#[test]
fn test_net_worth_empty() {
    let db = Database::open_in_memory().unwrap();
    let net = db.get_net_worth().unwrap();
    assert_eq!(net, Decimal::ZERO);
}

#[test]
fn test_spending_by_category() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let spending = db.get_spending_by_category(Some("2024-01")).unwrap();
    // All uncategorized expenses in January
    assert!(!spending.is_empty());
    // All amounts should be negative (expenses)
    for (_, amount) in &spending {
        assert!(*amount < Decimal::ZERO);
    }
}

#[test]
fn test_spending_by_category_empty_month() {
    let db = Database::open_in_memory().unwrap();
    let spending = db.get_spending_by_category(Some("2099-01")).unwrap();
    assert!(spending.is_empty());
}

#[test]
fn test_monthly_trend() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let trend = db.get_monthly_trend(12).unwrap();
    // Should have 2 months (2024-01 and 2024-02)
    assert_eq!(trend.len(), 2);
    assert_eq!(trend[0].0, "2024-01");
    assert_eq!(trend[1].0, "2024-02");
    // First month has income
    assert!(trend[0].1 > Decimal::ZERO);
    // Both months have expenses
    assert!(trend[0].2 < Decimal::ZERO);
    assert!(trend[1].2 < Decimal::ZERO);
}

#[test]
fn test_monthly_trend_limited() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let trend = db.get_monthly_trend(1).unwrap();
    assert_eq!(trend.len(), 1);
}

#[test]
fn test_transaction_count() {
    let mut db = Database::open_in_memory().unwrap();
    assert_eq!(db.get_transaction_count().unwrap(), 0);

    setup_test_data(&mut db);
    assert_eq!(db.get_transaction_count().unwrap(), 4);
}

// ── Export ─────────────────────────────────────────────────────

#[test]
fn test_export_all() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let all = db.get_all_transactions_for_export(None).unwrap();
    assert_eq!(all.len(), 4);
}

#[test]
fn test_export_by_month() {
    let mut db = Database::open_in_memory().unwrap();
    setup_test_data(&mut db);

    let jan = db.get_all_transactions_for_export(Some("2024-01")).unwrap();
    assert_eq!(jan.len(), 3);

    let feb = db.get_all_transactions_for_export(Some("2024-02")).unwrap();
    assert_eq!(feb.len(), 1);
}

#[test]
fn test_export_empty() {
    let db = Database::open_in_memory().unwrap();
    let all = db.get_all_transactions_for_export(None).unwrap();
    assert!(all.is_empty());
}

// ── Batch insert dedup ────────────────────────────────────────

#[test]
fn test_batch_insert_dedup() {
    let mut db = Database::open_in_memory().unwrap();
    let account = Account::new("Test".into(), AccountType::Checking, String::new());
    let account_id = db.insert_account(&account).unwrap();

    let txn = Transaction {
        id: None,
        account_id,
        date: "2024-01-15".into(),
        description: "Coffee".into(),
        original_description: "COFFEE".into(),
        amount: dec!(-4.50),
        category_id: None,
        notes: String::new(),
        is_transfer: false,
        import_hash: "unique-hash".into(),
        created_at: "2024-01-15T00:00:00Z".into(),
    };

    let count1 = db
        .insert_transactions_batch(std::slice::from_ref(&txn))
        .unwrap();
    assert_eq!(count1, 1);

    // Same hash -> skipped
    let count2 = db
        .insert_transactions_batch(std::slice::from_ref(&txn))
        .unwrap();
    assert_eq!(count2, 0);
}

#[test]
fn test_batch_insert_empty_hash_not_deduped() {
    let mut db = Database::open_in_memory().unwrap();
    let account = Account::new("Test".into(), AccountType::Checking, String::new());
    let account_id = db.insert_account(&account).unwrap();

    let txn = Transaction {
        id: None,
        account_id,
        date: "2024-01-15".into(),
        description: "Manual Entry".into(),
        original_description: "Manual".into(),
        amount: dec!(-10.00),
        category_id: None,
        notes: String::new(),
        is_transfer: false,
        import_hash: String::new(), // Empty hash
        created_at: "2024-01-15T00:00:00Z".into(),
    };

    let count1 = db
        .insert_transactions_batch(std::slice::from_ref(&txn))
        .unwrap();
    assert_eq!(count1, 1);

    // Empty hash -> should NOT be deduped
    let count2 = db
        .insert_transactions_batch(std::slice::from_ref(&txn))
        .unwrap();
    assert_eq!(count2, 1);

    assert_eq!(db.get_transaction_count().unwrap(), 2);
}

#[test]
fn test_batch_insert_multiple() {
    let mut db = Database::open_in_memory().unwrap();
    let account = Account::new("Test".into(), AccountType::Checking, String::new());
    let account_id = db.insert_account(&account).unwrap();

    let txns: Vec<Transaction> = (0..10)
        .map(|i| Transaction {
            id: None,
            account_id,
            date: format!("2024-01-{:02}", i + 1),
            description: format!("Transaction {i}"),
            original_description: format!("TXN {i}"),
            amount: dec!(-10.00),
            category_id: None,
            notes: String::new(),
            is_transfer: false,
            import_hash: format!("batch-hash-{i}"),
            created_at: String::new(),
        })
        .collect();

    let count = db.insert_transactions_batch(&txns).unwrap();
    assert_eq!(count, 10);
    assert_eq!(db.get_transaction_count().unwrap(), 10);
}

// ── Schema migration ──────────────────────────────────────────

#[test]
fn test_schema_version_set() {
    let db = Database::open_in_memory().unwrap();
    let version: i32 = db
        .conn
        .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(version, schema::CURRENT_VERSION);
}

#[test]
fn test_double_migrate_idempotent() {
    let mut db = Database::open_in_memory().unwrap();
    // Running migrate again should not fail
    db.migrate().unwrap();
    let version: i32 = db
        .conn
        .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(version, schema::CURRENT_VERSION);
}

// ── Account-type-filtered analytics ───────────────────────────

fn setup_multi_account_data(db: &mut Database) -> (i64, i64) {
    let checking = Account::new(
        "Chase Checking".into(),
        AccountType::Checking,
        String::new(),
    );
    let credit = Account::new("Chase Visa".into(), AccountType::CreditCard, String::new());
    let checking_id = db.insert_account(&checking).unwrap();
    let credit_id = db.insert_account(&credit).unwrap();

    // Checking transactions: salary +3000, coffee -5.25
    db.insert_transaction(&Transaction {
        id: None,
        account_id: checking_id,
        date: "2024-01-15".into(),
        description: "Salary".into(),
        original_description: "ACME PAYROLL".into(),
        amount: dec!(3000.00),
        category_id: None,
        notes: String::new(),
        is_transfer: false,
        import_hash: "chk-1".into(),
        created_at: String::new(),
    })
    .unwrap();
    db.insert_transaction(&Transaction {
        id: None,
        account_id: checking_id,
        date: "2024-01-18".into(),
        description: "Coffee".into(),
        original_description: "STARBUCKS".into(),
        amount: dec!(-5.25),
        category_id: None,
        notes: String::new(),
        is_transfer: false,
        import_hash: "chk-2".into(),
        created_at: String::new(),
    })
    .unwrap();

    // Credit card transactions: charge -45.00, payment +45.00
    db.insert_transaction(&Transaction {
        id: None,
        account_id: credit_id,
        date: "2024-01-10".into(),
        description: "Amazon".into(),
        original_description: "AMZN MKTP".into(),
        amount: dec!(-45.00),
        category_id: None,
        notes: String::new(),
        is_transfer: false,
        import_hash: "cc-1".into(),
        created_at: String::new(),
    })
    .unwrap();
    db.insert_transaction(&Transaction {
        id: None,
        account_id: credit_id,
        date: "2024-01-20".into(),
        description: "Payment".into(),
        original_description: "PAYMENT THANK YOU".into(),
        amount: dec!(45.00),
        category_id: None,
        notes: String::new(),
        is_transfer: false,
        import_hash: "cc-2".into(),
        created_at: String::new(),
    })
    .unwrap();

    (checking_id, credit_id)
}

#[test]
fn test_monthly_totals_by_account_type_debit() {
    let mut db = Database::open_in_memory().unwrap();
    setup_multi_account_data(&mut db);

    let debit_types = &["Checking", "Savings", "Cash", "Investment", "Other"];
    let (income, expenses) = db
        .get_monthly_totals_by_account_type(Some("2024-01"), debit_types)
        .unwrap();
    assert_eq!(income, dec!(3000.00));
    assert_eq!(expenses, dec!(-5.25));
}

#[test]
fn test_monthly_totals_by_account_type_credit() {
    let mut db = Database::open_in_memory().unwrap();
    setup_multi_account_data(&mut db);

    let credit_types = &["Credit Card", "Loan"];
    let (income, expenses) = db
        .get_monthly_totals_by_account_type(Some("2024-01"), credit_types)
        .unwrap();
    // Credit card: payment +45 is "income" (positive), charge -45 is "expense"
    assert_eq!(income, dec!(45.00));
    assert_eq!(expenses, dec!(-45.00));
}

#[test]
fn test_monthly_totals_by_account_type_empty_month() {
    let mut db = Database::open_in_memory().unwrap();
    setup_multi_account_data(&mut db);

    let (income, expenses) = db
        .get_monthly_totals_by_account_type(Some("2099-01"), &["Checking"])
        .unwrap();
    assert_eq!(income, Decimal::ZERO);
    assert_eq!(expenses, Decimal::ZERO);
}

#[test]
fn test_balance_by_account_type_debit() {
    let mut db = Database::open_in_memory().unwrap();
    setup_multi_account_data(&mut db);

    let debit_types = &["Checking", "Savings", "Cash", "Investment", "Other"];
    let balance = db.get_balance_by_account_type(debit_types).unwrap();
    // 3000.00 - 5.25 = 2994.75
    assert_eq!(balance, dec!(2994.75));
}

#[test]
fn test_balance_by_account_type_credit() {
    let mut db = Database::open_in_memory().unwrap();
    setup_multi_account_data(&mut db);

    let credit_types = &["Credit Card", "Loan"];
    let balance = db.get_balance_by_account_type(credit_types).unwrap();
    // -45.00 + 45.00 = 0.00
    assert_eq!(balance, Decimal::ZERO);
}

#[test]
fn test_balance_by_account_type_no_matching_accounts() {
    let db = Database::open_in_memory().unwrap();
    let balance = db.get_balance_by_account_type(&["Loan"]).unwrap();
    assert_eq!(balance, Decimal::ZERO);
}

// ── Per-account queries ───────────────────────────────────────

#[test]
fn test_account_monthly_totals() {
    let mut db = Database::open_in_memory().unwrap();
    let (checking_id, credit_id) = setup_multi_account_data(&mut db);

    // Checking: salary +3000, coffee -5.25
    let (income, expenses) = db
        .get_account_monthly_totals(checking_id, Some("2024-01"))
        .unwrap();
    assert_eq!(income, dec!(3000.00));
    assert_eq!(expenses, dec!(-5.25));

    // Credit: charge -45, payment +45
    let (inc, exp) = db.get_account_monthly_totals(credit_id, Some("2024-01")).unwrap();
    assert_eq!(inc, dec!(45.00));
    assert_eq!(exp, dec!(-45.00));
}

#[test]
fn test_account_balance() {
    let mut db = Database::open_in_memory().unwrap();
    let (checking_id, credit_id) = setup_multi_account_data(&mut db);

    // Checking: 3000 - 5.25 = 2994.75
    let bal = db.get_account_balance(checking_id).unwrap();
    assert_eq!(bal, dec!(2994.75));

    // Credit: -45 + 45 = 0
    let bal = db.get_account_balance(credit_id).unwrap();
    assert_eq!(bal, Decimal::ZERO);
}

#[test]
fn test_account_balance_empty() {
    let db = Database::open_in_memory().unwrap();
    let acct = Account::new("Empty".into(), AccountType::Savings, String::new());
    let id = db.insert_account(&acct).unwrap();
    let bal = db.get_account_balance(id).unwrap();
    assert_eq!(bal, Decimal::ZERO);
}

#[test]
fn test_account_monthly_totals_empty_month() {
    let mut db = Database::open_in_memory().unwrap();
    let (checking_id, _) = setup_multi_account_data(&mut db);

    let (income, expenses) = db
        .get_account_monthly_totals(checking_id, Some("2099-01"))
        .unwrap();
    assert_eq!(income, Decimal::ZERO);
    assert_eq!(expenses, Decimal::ZERO);
}

// ── Decimal precision ─────────────────────────────────────────

#[test]
fn test_decimal_precision_preserved() {
    let db = Database::open_in_memory().unwrap();
    let account = Account::new("Test".into(), AccountType::Checking, String::new());
    let account_id = db.insert_account(&account).unwrap();

    let txn = Transaction {
        id: None,
        account_id,
        date: "2024-01-15".into(),
        description: "Precise".into(),
        original_description: "Precise".into(),
        amount: dec!(1234.5678),
        category_id: None,
        notes: String::new(),
        is_transfer: false,
        import_hash: "precision-test".into(),
        created_at: String::new(),
    };

    db.insert_transaction(&txn).unwrap();
    let fetched = db
        .get_transactions(Some(1), None, None, None, None, None)
        .unwrap();
    assert_eq!(fetched[0].amount, dec!(1234.5678));
}

#[test]
fn test_large_transaction_amounts() {
    let db = Database::open_in_memory().unwrap();
    let account = Account::new("Test".into(), AccountType::Checking, String::new());
    let account_id = db.insert_account(&account).unwrap();

    let txn = Transaction {
        id: None,
        account_id,
        date: "2024-01-15".into(),
        description: "House".into(),
        original_description: "House".into(),
        amount: dec!(-350000.00),
        category_id: None,
        notes: String::new(),
        is_transfer: false,
        import_hash: "large-amount".into(),
        created_at: String::new(),
    };

    db.insert_transaction(&txn).unwrap();
    let fetched = db
        .get_transactions(Some(1), None, None, None, None, None)
        .unwrap();
    assert_eq!(fetched[0].amount, dec!(-350000.00));
}
