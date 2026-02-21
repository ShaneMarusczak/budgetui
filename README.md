# BudgeTUI

[![Rust](https://github.com/ShaneMarusczak/budgetui/workflows/Rust/badge.svg)](https://github.com/ShaneMarusczak/budgetui/actions/workflows/rust.yml)

A local-only, privacy-first personal finance tracker for the terminal.

BudgeTUI gives you full control over your financial data without ever sending it to the cloud. Import bank CSVs, categorize transactions, set budgets, and track spending — all from a fast, keyboard-driven terminal interface with a Catppuccin Mocha color scheme.

## Features

**Dashboard** — Monthly income/expense summary cards split by debit and credit accounts, spending-by-category bar chart, 12-month trend sparkline, and net worth at a glance.

**Accounts** — Per-account snapshot cards showing monthly income/expenses and all-time balance. Press Enter to drill into an account's transactions. Credit accounts display "Charges/Payments" labels; debit accounts show "Income/Expenses." Supports 7 account types: Checking, Savings, Credit Card, Investment, Cash, Loan, and Other. Create accounts via `:account` command or inline during import.

**Transactions** — Browse, search, filter by account or category, rename descriptions, re-categorize, and manually add or delete transactions. Export to CSV. Live search with match count. Alternating row backgrounds for readability.

**CSV Import** — 6-step wizard with step indicator bar (File > Map > Account > Preview > Categorize > Done). Automatic bank format detection for 11+ banks. Explicit account selection with inline account creation. Deduplication via stable FNV-1a hashing prevents re-importing the same transactions. Auto-categorization step for uncategorized transactions.

**Categories** — Flat category list with split-panel view and active panel highlighting. Create auto-categorization rules using simple pattern matching or full regex.

**Budgets** — Set monthly spending limits per category with color-coded progress bars (green < 70%, yellow 70-90%, red > 90%).

**UX Polish** — Mode indicator in status bar (NORMAL/COMMAND/SEARCH/EDIT/CONFIRM). Context-sensitive keybinding hints that change per screen. Adaptive scrolling based on terminal height. Cursor display in input modes. Confirmation dialogs for all destructive actions. Empty states with helpful guidance on every screen.

## Supported Banks

Bank format is auto-detected from CSV headers and structure. Supported formats:

| Bank | Notes |
|------|-------|
| Wells Fargo | Headerless 5-column format |
| American Express | Inverted amounts |
| Bank of America | Credit card and checking |
| USAA | Original Description column |
| Citi | Debit/Credit split columns |
| Capital One | Credit card and checking |
| Discover | Trans. Date column |
| Chase | Credit card and checking |

Column mapping can be adjusted manually for any CSV format not auto-detected.

## Installation

```
cargo install --path .
```

Or build from source:

```
cargo build --release
```

The binary will be at `target/release/budgetui`.

## Usage

```
budgetui
```

Data is stored in a local SQLite database at your platform's data directory:
- **macOS**: `~/Library/Application Support/com.budgetui.BudgeTUI/budgetui.db`
- **Linux**: `~/.local/share/budgetui/budgetui.db`

### CLI Mode

Run subcommands directly without opening the TUI — useful for scripting and automation pipelines.

```bash
# Import a bank CSV (auto-detects format)
budgetui import ~/Downloads/chase-june.csv

# Import into a specific account
budgetui import statement.csv --account "Chase Checking"

# Monthly summary
budgetui summary 2026-02
budgetui summary          # defaults to current month

# Export transactions to CSV
budgetui export ~/june.csv --month 2026-06
budgetui export           # exports current month to ~/budgetui-export-YYYY-MM.csv

# List all accounts
budgetui accounts

# Version / help
budgetui --version
budgetui --help
```

### TUI Navigation

| Key | Action |
|-----|--------|
| `1`-`6` | Switch screens |
| `:nav` | Open screen navigator popup |
| `j` / `k` | Move selection down / up |
| `g` / `G` | Jump to top / bottom |
| `Ctrl-d` / `Ctrl-u` | Page down / up (adaptive to terminal height) |
| `H` / `L` | Previous / next month |
| `Tab` / `Shift-Tab` | Cycle screens forward / backward |
| `:` | Enter command mode |
| `/` | Live search (shows match count) |
| `?` | Show help overlay |
| `D` | Delete selected transaction (on Transactions screen) |
| `r` | Toggle rules panel (on Categories screen) |
| `a`-`z` | Jump to first matching category (Import Categorize step) |
| `n` / `p` | Cycle accounts (on Dashboard) |
| `Ctrl-q` | Quit |

### Commands

Type `:` to enter command mode, then any command below.

**Navigation**
| Command | Aliases | Description |
|---------|---------|-------------|
| `:dashboard` | `:d` | Go to dashboard |
| `:accounts` | | Go to accounts |
| `:transactions` | `:t` | Go to transactions |
| `:import` | `:i` | Go to import wizard |
| `:categories` | `:c` | Go to categories |
| `:budgets` | `:b` | Go to budgets |
| `:month YYYY-MM` | `:m` | Navigate to a specific month |
| `:next-month` | | Go to next month |
| `:prev-month` | | Go to previous month |
| `:nav` | | Open screen navigator |
| `:help` | `:h` | Show all commands |

**Data Management**
| Command | Description |
|---------|-------------|
| `:account <name> [type]` | Create an account (types: checking, savings, credit, investment, cash, loan) |
| `:filter-account <name>` | Filter transactions by account |
| `:category <name>` | Create a category |
| `:rule <pattern> <category>` | Add a contains-match categorization rule |
| `:regex-rule <pattern> <category>` | Add a regex categorization rule |
| `:delete-rule` | Delete the selected rule (with confirmation) |
| `:budget <category> <amount>` | Set a monthly budget |
| `:delete-budget` | Delete the selected budget (with confirmation) |
| `:add-txn <date> <desc> <amount>` | Manually add a transaction |
| `:delete-txn` | Delete selected transaction (with confirmation) |
| `:rename <new_name>` | Rename selected transaction |
| `:recat <category>` | Re-categorize selected transaction |
| `:search <query>` | Search transactions |
| `:export [path]` | Export transactions to CSV |
| `:quit` | Exit the application |

## Tech Stack

- **[Ratatui](https://ratatui.rs)** — Terminal UI framework
- **[rusqlite](https://github.com/rusqlite/rusqlite)** — Embedded SQLite (bundled, no system dependency)
- **[rust_decimal](https://github.com/paupino/rust-decimal)** — Precise decimal arithmetic for financial data
- **[chrono](https://github.com/chronotope/chrono)** — Date handling
- **[regex](https://github.com/rust-lang/regex)** — Pattern matching for categorization rules

## Design Principles

**Privacy first** — All data stays on your machine. No network calls, no telemetry, no cloud sync. Your financial data is yours alone.

**Precision** — Uses `rust_decimal` for all monetary calculations. No floating-point rounding errors.

**Speed** — Optimized release build with LTO, single codegen unit, and size optimization. SQLite with WAL mode and proper indexing.

**Keyboard-driven** — Vim-inspired navigation (j/k, `:command` mode). Every action is accessible from the keyboard. Context-sensitive hints always visible.

## License

MIT
