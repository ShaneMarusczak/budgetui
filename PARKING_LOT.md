# Parking Lot

Future ideas and features to revisit later.

## User-Defined Themes

Allow users to define custom color themes via a YAML or TOML config file. Users could override any/all of the Catppuccin Mocha defaults with their own RGB values.

Considerations:
- Config file location: platform data dir alongside the DB, or `~/.config/budgetui/theme.toml`
- Which colors to expose: all constants in `theme.rs` (ACCENT, GREEN, RED, etc.) plus `SPENDING_COLORS`
- Runtime loading vs compile-time: would need to switch from `const` to runtime values (e.g. `LazyLock` or a `Theme` struct)
- Fallback: missing keys fall back to Catppuccin Mocha defaults
- Could ship a few preset themes (Catppuccin Latte for light mode, Dracula, Nord, etc.)
- The `SPENDING_COLORS` array would need to be user-configurable too (12 entries, or auto-generate shades from a single base color)
