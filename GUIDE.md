# BudgeTUI User Guide

A comprehensive guide to everything you can do in BudgeTUI.

---

## CLI Mode

BudgeTUI has a headless CLI mode for scripting, automation pipelines, and quick operations without opening the full TUI.

### Import

```bash
budgetui import statement.csv --account "Chase Checking"
budgetui import ~/Downloads/statement.csv --account "Amex Gold"
```

The `--account` flag specifies which account to import into (required when you have more than one account). The importer auto-detects bank format from CSV headers (same 11+ bank formats supported in the TUI wizard). Transactions are deduplicated by hash, auto-categorized against your existing rules, and inserted. Output goes to stdout:

```
Detected format: Chase Credit Card
Parsed 47 transactions
Auto-categorized 31/47 transactions
Imported 42 new transactions (5 duplicates skipped)
```

### Export

```bash
budgetui export ~/june.csv --month 2026-06
budgetui export                             # defaults to current month
```

Exports Date, Description, Amount, Category, Account, and Notes columns.

### Summary

```bash
budgetui summary 2026-02
budgetui summary          # defaults to current month
```

Prints income, expenses, net, net worth, total transaction count, and spending by category.

### Accounts

```bash
budgetui accounts
```

Lists all accounts with ID, name, type, and institution.

### Other

```bash
budgetui --help       # usage overview
budgetui --version    # prints version
```

### Scripting Examples

```bash
# Monthly import cron job
for f in ~/Downloads/*.csv; do
  budgetui import "$f" && mv "$f" ~/Downloads/imported/
done

# Generate monthly reports
for month in 2026-{01..12}; do
  budgetui summary "$month" >> ~/budget-report.txt
done

# Export all months
for month in 2026-{01..06}; do
  budgetui export ~/exports/"$month".csv --month "$month"
done
```

---

## Getting Started

When you first launch BudgeTUI, you'll see the Dashboard with a default checking account already created. The status bar at the bottom shows your current mode, screen, month, and context-sensitive keybinding hints.

The interface has six screens. The hint bar at the top shows your current screen name and how to navigate:

```
 Dashboard                              :nav │ 1-6 │ Tab
```

Press `1`-`6` to jump to a screen by number, `Tab`/`Shift-Tab` to cycle, or type `:nav` to open an interactive screen navigator popup. You can also use commands like `:d`, `:accounts`, `:t`, `:i`, `:c`, `:b`.

---

## Input Modes

BudgeTUI uses vim-inspired modal input. The current mode is always shown in the status bar on the left.

| Mode | How to Enter | What It Does |
|------|-------------|-------------|
| **NORMAL** | Default / `Esc` | Navigate, select, use hotkeys |
| **COMMAND** | `:` | Type commands (`:help`, `:budget Food 500`, etc.) |
| **SEARCH** | `/` | Live search with match count |
| **EDIT** | `:rename` with no args | Inline text editing |
| **CONFIRM** | Triggered by destructive actions | `y` to confirm, any other key to cancel |

In COMMAND and SEARCH modes, a blinking cursor shows your position. Press `Esc` to return to NORMAL mode. Press `Enter` to execute.

---

## Screen 1: Dashboard

The Dashboard gives you a financial overview of the current month.

### Summary Cards

Four cards across the top:

- **Income** — Total positive transactions this month, with count
- **Expenses** — Total negative transactions this month (shown as absolute value), with count
- **Net** — Income + Expenses for the month (green if positive, red if negative)
- **Net Worth** — Sum of all transactions across all time and accounts

### Spending by Category

A horizontal bar chart showing your top 12 spending categories for the current month. Category names are truncated to 10 characters to fit. Only appears when you have categorized transactions.

### Monthly Spending Trend

A sparkline showing total expenses per month over the last 12 months. Gives a quick visual of whether your spending is trending up or down.

### Dashboard-Specific Keys

| Key | Action |
|-----|--------|
| `n` | Cycle to next account |
| `p` | Cycle to previous account |
| `H` | Go to previous month |
| `L` | Go to next month |

---

## Screen 2: Accounts

The Accounts screen shows a snapshot card for each account with monthly totals and all-time balance.

### Account Cards

Each account renders as a card:

```
┌─ Chase Checking (Checking) ─────────────────────┐
│  Income: $3,000.00    Expenses: $450.25          │
│  Balance: $12,540.75                             │
└──────────────────────────────────────────────────┘
```

For credit card and loan accounts, labels change to "Payments" and "Charges" instead of "Income" and "Expenses."

The selected card is highlighted with an accent-colored border. Balance is green when positive, red when negative.

### Navigation

| Key | Action |
|-----|--------|
| `j` / `k` | Move between account cards |
| `g` / `G` | Jump to first / last account |
| `Ctrl-d` / `Ctrl-u` | Page down / up |
| `Enter` | Drill into account — switches to Transactions filtered by this account |
| `Esc` (in Transactions) | Clear account filter and show all transactions |

### Empty State

When no accounts exist, the screen shows guidance on how to create one with `:account <name> [type]` or by importing a CSV.

---

## Screen 3: Transactions

A table view of all transactions for the current month.

### Table Columns

| Column | Description |
|--------|-------------|
| Date | Transaction date (YYYY-MM-DD) |
| Description | Transaction description (truncated to 40 chars) |
| Category | Assigned category, or "---" if uncategorized |
| Amount | Green with `+` prefix for income, red for expenses |

Alternating row backgrounds improve readability. The selected row is highlighted in blue with dark text.

### Navigation

| Key | Action |
|-----|--------|
| `j` / `k` or arrows | Move selection up/down |
| `g` | Jump to first transaction |
| `G` | Jump to last transaction |
| `Ctrl-d` | Page down (half terminal height) |
| `Ctrl-u` | Page up (half terminal height) |

Scrolling adapts to your terminal height automatically.

### Actions

| Key / Command | Action |
|---------------|--------|
| `/` | Live search — filters as you type, shows match count |
| `D` | Delete selected transaction (with confirmation) |
| `:rename` | Enter edit mode to rename the selected transaction |
| `:rename New Name` | Rename directly without edit mode |
| `:recat CategoryName` | Re-categorize the selected transaction |
| `:add-txn 2024-01-15 Coffee -4.50` | Manually add a transaction |
| `:filter-account Chase` | Show only transactions from a specific account |
| `:filter-account` | Clear account filter (show all) |
| `:export` | Export current month's transactions to CSV |
| `:export ~/budget.csv` | Export to a specific path |

### Search

Press `/` to enter search mode. Results filter live as you type. The command bar shows the match count (e.g., `/coffee (3 matches)`). Press `Enter` to keep the filter, or `Esc` to clear it.

You can also search via command: `:search coffee` or `:s coffee`.

### Empty State

When there are no transactions for the current month, the screen shows helpful guidance on how to import or manually add transactions.

---

## Screen 4: Import

A 6-step wizard for importing bank CSV files. A step indicator bar at the top shows your progress:

```
 1:File  >  2:Map  >  3:Account  >  4:Preview  >  5:Categorize  >  6:Done
```

Completed steps show in green, the current step is highlighted, and future steps are dimmed.

### Step 1: Select File

A file browser showing directories and importable files (`.csv`, `.tsv`, `.ofx`, `.qfx`, `.qif`).

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate the file list |
| `Enter` | Open directory or select file |
| `Esc` | Go back / cancel |
| `g` / `G` | Jump to top / bottom |

The current directory path is shown at the top. Select `..` to go up.

### Step 2: Map Columns

After selecting a file, BudgeTUI attempts to auto-detect your bank's format. If detected, you'll see "Auto-detected: Chase Credit Card" (or similar). You can adjust the mapping if needed.

**Configurable fields:**

| Field | Description |
|-------|-------------|
| Date Column | Which column contains the date (0-indexed) |
| Description Column | Which column contains the description |
| Amount Column | Single amount column (set to "---" if using debit/credit) |
| Debit Column | Debit amounts column (optional) |
| Credit Column | Credit amounts column (optional) |
| Date Format | Cycle through common formats: `%m/%d/%Y`, `%Y-%m-%d`, etc. |
| Has Header | Whether the first row is a header |

| Key | Action |
|-----|--------|
| `j` / `k` | Move between fields |
| `+` / `-` | Adjust the selected field value |
| `Enter` | Generate preview with current settings |
| `Esc` | Go back to file selection |

A sample data table at the bottom shows the first 5 rows with column indices (`[0] Date`, `[1] Description`, etc.) so you can see which column is which.

### Step 3: Select Account

Choose which account this import belongs to, or create a new one inline.

```
┌─ Select Account ──────────────────────────────────┐
│  Detected: Chase Credit Card                      │
│  Suggested type: Credit Card                      │
└───────────────────────────────────────────────────┘
┌─ j/k navigate | Enter select | n new ────────────┐
│  > Chase Sapphire (Credit Card)                   │
│    Chase Freedom (Credit Card)                    │
│    Wells Fargo Checking (Checking)                │
└───────────────────────────────────────────────────┘
```

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate accounts |
| `g` / `G` | Jump to first / last |
| `Enter` | Select highlighted account and advance to Preview |
| `n` | Create a new account (opens inline form) |
| `Esc` | Go back to column mapping |

**Creating a new account** (press `n`):
- Type the account name
- Use `+`/`-` or `Tab` to cycle through account types (Checking, Savings, Credit Card, etc.)
- Press `Enter` to create and select it
- Press `Esc` to cancel

The account type determines how amounts are handled — Credit Card and Loan accounts automatically negate amounts for correct sign treatment.

### Step 4: Preview

Shows a preview of the parsed transactions (up to 50 rows) with Date, Description, and Amount columns. Income amounts are green, expenses are red.

| Key | Action |
|-----|--------|
| `Enter` | Confirm and import (with confirmation dialog) |
| `Esc` | Go back to account selection |

### Step 5: Categorize

After importing, uncategorized transactions are presented one by one for manual categorization. Pick an existing category or create new ones.

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate category list |
| `Enter` | Assign selected category |
| `s` | Skip this transaction |
| `S` | Skip all remaining uncategorized |
| `n` | Create a new category |

### Step 6: Complete

Shows the import result: how many transactions were imported, how many duplicates were skipped, and any suggested categorization rules for uncategorized transactions.

Press `Enter` to go to the Transactions screen, or `:d` for the Dashboard.

### Deduplication

BudgeTUI generates a stable hash (FNV-1a) from each transaction's date, description, and amount. If you re-import the same CSV, duplicate transactions are automatically skipped. The hash algorithm is stable across Rust compiler versions, so upgrading Rust won't cause false duplicates.

### Supported Banks

BudgeTUI auto-detects these bank CSV formats:

| Bank | Detection Method |
|------|-----------------|
| Wells Fargo | No headers, 5 columns, 3rd column is `*` |
| American Express | "Card Member" column header |
| Bank of America (Credit) | "Reference Number" + "Address" columns |
| Bank of America (Checking) | "Running Bal." column |
| USAA | "Original Description" column |
| Citi | "Status" + "Debit" + "Credit" columns |
| Capital One (Credit) | "Card No." + ISO date format |
| Capital One (Checking) | "Account Number" + "Transaction Amount" |
| Discover | "Trans. Date" column |
| Chase (Checking) | "Details" + "Check or Slip #" |
| Chase (Credit) | "Transaction Date" + "Post Date" columns |

For any other format, map the columns manually in Step 2.

---

## Screen 5: Categories

A split-panel view with the category list on the left and categorization rules on the right.

### Category List (Left Panel)

Shows all categories with bold, blue styling.

Navigate with `j`/`k`. The active panel has a blue border; the inactive panel has a dim border.

### Rules Table (Right Panel)

Shows all auto-categorization rules with columns: Pattern, Category, Type (contains or regex).

Press `r` to toggle focus between the category list and the rules table.

### Commands

| Command | Description |
|---------|-------------|
| `:category Groceries` | Create a category |
| `:rule amazon Shopping` | Auto-categorize transactions containing "amazon" as "Shopping" |
| `:regex-rule ^SQ \* Coffee` | Auto-categorize Square transactions matching regex as "Coffee" |
| `:delete-rule` | Delete the selected rule (with confirmation) |

### How Auto-Categorization Works

When you import transactions, BudgeTUI runs all rules against each uncategorized transaction:

1. **Contains rules** — Case-insensitive substring match. The pattern `amazon` matches "AMAZON.COM PURCHASE", "Amazon Prime", etc.
2. **Regex rules** — Full regex matching against the original description. Case-sensitive by default; use `(?i)` for case-insensitive.
3. **Priority** — Rules are checked in order. The first match wins. If no rule matches, the transaction stays uncategorized.

After import, the status bar suggests rules for uncategorized transactions.

---

## Screen 6: Budgets

View and manage monthly spending budgets.

### Budget Display

Each budget shows:
- Category name
- Spent vs. limit amounts (e.g., `$342/$500`)
- A visual progress bar
- Percentage used

**Color coding:**
- Green: < 70% used
- Yellow: 70-90% used
- Red: > 90% used

### Commands

| Command | Description |
|---------|-------------|
| `:budget Food & Dining 500` | Set a $500 monthly budget for "Food & Dining" |
| `:budget Groceries 300` | Set or update a budget (upserts) |
| `:delete-budget` | Delete the selected budget (with confirmation) |

Budgets are per-month. Use `H`/`L` or `:month YYYY-MM` to navigate between months.

### Empty State

When no budgets are set, the screen shows instructions on how to create one.

---

## Accounts

BudgeTUI supports multiple accounts for organizing transactions. Accounts can be created via the `:account` command or inline during the import wizard (Step 3: Select Account).

### Account Types

| Type | Typical Use |
|------|-------------|
| Checking | Bank checking accounts |
| Savings | Savings accounts |
| Credit Card | Credit cards |
| Investment | Brokerage, retirement accounts |
| Cash | Cash on hand |
| Loan | Mortgages, auto loans, student loans |
| Other | Anything else |

### Commands

| Command | Description |
|---------|-------------|
| `:account Chase Checking` | Create an account named "Chase" with type Checking |
| `:account Amex Credit` | Create a credit card account |
| `:account MyBank` | Create with default type (Checking) |
| `:accounts` | Go to the Accounts tab |
| `:filter-account Chase` | Show only transactions from "Chase" |
| `:filter-account` | Clear filter, show all transactions |

### Viewing Accounts

The Accounts tab (Screen 2) shows per-account snapshot cards with monthly income/expenses and all-time balance. Press `Enter` on a card to drill into that account's transactions. On the Dashboard, press `n`/`p` to cycle through accounts.

---

## Month Navigation

All screens respect the current month. Change it with:

| Method | Example |
|--------|---------|
| `H` | Previous month |
| `L` | Next month |
| `:month 2024-06` | Jump to June 2024 |
| `:m 3` | Jump to March of the current year |
| `:next-month` | Explicit command for next month |
| `:prev-month` | Explicit command for previous month |

The current month is always visible in the status bar.

---

## Exporting Data

Export your transactions to CSV:

```
:export                    # Exports to ~/budgetui-export-YYYY-MM.csv
:export ~/my-budget.csv    # Exports to a specific path
```

The exported CSV includes: Date, Description, Amount, Category, Account, Notes.

Only transactions for the current month are exported.

---

## Manual Transactions

Add transactions without importing a CSV:

```
:add-txn 2024-01-15 Coffee Shop -4.50
:add-txn 2024-01-31 Paycheck 3500.00
```

Format: `:add-txn <date> <description> <amount>`

- Negative amounts are expenses, positive are income
- The transaction is added to the currently active account
- Manual transactions get a `manual-` prefixed hash for dedup

---

## Keyboard Reference

### Global Keys (All Screens)

| Key | Action |
|-----|--------|
| `1`-`6` | Switch to screen by number |
| `Tab` / `Shift-Tab` | Cycle screens |
| `j` / `k` | Move down / up |
| `g` / `G` | Top / bottom |
| `Ctrl-d` / `Ctrl-u` | Half-page down / up |
| `H` / `L` | Previous / next month |
| `:` | Command mode |
| `/` | Search mode |
| `?` | Help overlay |
| `Esc` | Cancel / go back |
| `Ctrl-q` | Quit |

### Screen-Specific Keys

| Screen | Key | Action |
|--------|-----|--------|
| Dashboard | `n` / `p` | Cycle accounts |
| Accounts | `Enter` | Drill into account's transactions |
| Transactions | `D` | Delete transaction |
| Transactions | `Esc` | Clear account filter (when filtered) |
| Categories | `r` | Toggle category/rules focus |
| Import | `+` / `-` | Adjust column mapping value |
| Import | `n` | Create new account (in account picker) |
| Import | `Enter` | Advance to next step |
| Import | `Esc` | Go back one step |

### Input Mode Keys

| Mode | Key | Action |
|------|-----|--------|
| Command | `Enter` | Execute command |
| Command | `Esc` | Cancel |
| Command | `Backspace` | Delete character (exits mode if empty) |
| Search | `Enter` | Keep search filter |
| Search | `Esc` | Clear search |
| Confirm | `y` / `Y` | Confirm action |
| Confirm | Any other key | Cancel |

---

## Command Quick Reference

| Command | Alias | Description |
|---------|-------|-------------|
| `:dashboard` | `:d` | Go to Dashboard |
| `:accounts` | | Go to Accounts |
| `:transactions` | `:t` | Go to Transactions |
| `:import` | `:i` | Go to Import |
| `:categories` | `:c` | Go to Categories |
| `:budgets` | `:b` | Go to Budgets |
| `:nav` | | Open screen navigator |
| `:help` | `:h` | Show help overlay |
| `:quit` | `:q` | Quit |
| `:month YYYY-MM` | `:m` | Set month |
| `:next-month` | | Next month |
| `:prev-month` | | Previous month |
| `:account <name> [type]` | `:a` | Create account |
| `:filter-account <name>` | `:fa` | Filter by account |
| `:category <name>` | | Create category |
| `:rule <pattern> <category>` | `:r` | Add contains rule |
| `:regex-rule <pattern> <category>` | | Add regex rule |
| `:delete-rule` | | Delete selected rule |
| `:budget <category> <amount>` | | Set budget |
| `:delete-budget` | | Delete selected budget |
| `:add-txn <date> <desc> <amount>` | | Add manual transaction |
| `:delete-txn` | | Delete selected transaction |
| `:rename [new_name]` | | Rename transaction |
| `:recat <category>` | | Re-categorize transaction |
| `:search <query>` | `:s` | Search transactions |
| `:export [path]` | | Export to CSV |

Mistyped a command? BudgeTUI uses fuzzy matching to suggest the closest valid command.

---

## Data Storage

All data is stored locally in a SQLite database:

- **macOS**: `~/Library/Application Support/com.budgetui.BudgeTUI/budgetui.db`
- **Linux**: `~/.local/share/budgetui/budgetui.db`

The database uses WAL (Write-Ahead Logging) mode for safe concurrent reads and foreign key constraints for data integrity.

### Schema

- **accounts** — id, name, type, institution, currency, notes
- **categories** — id, name, parent_id, icon, color
- **transactions** — id, account_id, date, description, original_description, amount, category_id, notes, is_transfer, import_hash
- **budgets** — id, category_id, month, limit_amount (unique per category+month)
- **import_rules** — id, pattern, category_id, is_regex, priority

### Backup

To back up your data, simply copy the database file. To reset, delete it and BudgeTUI will create a fresh one on next launch.

---

## Tips and Tricks

- **Quick categorization workflow**: Import a CSV, then go to Transactions. For each uncategorized transaction, use `:recat CategoryName`. Once you see a pattern, create a rule with `:rule pattern Category` so future imports are auto-categorized.

- **Regex rules for complex patterns**: Bank descriptions like `SQ *COFFEE SHOP #123` can be matched with `:regex-rule ^SQ \* Coffee`.

- **Month shortcuts**: Instead of typing `:month 2024-03`, just type `:m 3` to jump to March of the current year.

- **Fast screen switching**: Use `1`-`6` instead of `:dashboard`, `:accounts`, `:transactions`, etc.

- **Search then act**: Use `/` to find a transaction, then `D` to delete it or `:recat` to re-categorize. The search is live, so results update as you type.

- **Export for tax time**: Switch to each month with `H`/`L`, then `:export` to create a CSV for each month's transactions.
