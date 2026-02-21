use ratatui::style::{Color, Modifier, Style};

pub(crate) const HEADER_BG: Color = Color::Rgb(30, 30, 46);
pub(crate) const HEADER_FG: Color = Color::Rgb(205, 214, 244);
pub(crate) const ACCENT: Color = Color::Rgb(137, 180, 250);
pub(crate) const GREEN: Color = Color::Rgb(166, 227, 161);
pub(crate) const RED: Color = Color::Rgb(243, 139, 168);
pub(crate) const YELLOW: Color = Color::Rgb(249, 226, 175);
pub(crate) const SURFACE: Color = Color::Rgb(49, 50, 68);
pub(crate) const TEXT: Color = Color::Rgb(205, 214, 244);
pub(crate) const TEXT_DIM: Color = Color::Rgb(127, 132, 156);
pub(crate) const OVERLAY: Color = Color::Rgb(69, 71, 90);
pub(crate) const COMMAND_BG: Color = Color::Rgb(24, 24, 37);

/// 12 shades of blue by rank: lightest (lowest spender) to deepest (highest).
pub(crate) const SPENDING_COLORS: [Color; 12] = [
    Color::Rgb(198, 219, 252), // 0  — ice
    Color::Rgb(180, 208, 250), // 1
    Color::Rgb(162, 197, 248), // 2
    Color::Rgb(147, 186, 246), // 3
    Color::Rgb(132, 175, 244), // 4
    Color::Rgb(117, 164, 240), // 5
    Color::Rgb(102, 153, 234), // 6
    Color::Rgb(87, 140, 226),  // 7
    Color::Rgb(72, 127, 218),  // 8
    Color::Rgb(58, 114, 208),  // 9
    Color::Rgb(45, 101, 198),  // 10
    Color::Rgb(33, 88, 188),   // 11 — deep
];

pub(crate) fn header_style() -> Style {
    Style::default()
        .fg(HEADER_FG)
        .bg(HEADER_BG)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn selected_style() -> Style {
    Style::default().fg(HEADER_BG).bg(ACCENT)
}

pub(crate) fn normal_style() -> Style {
    Style::default().fg(TEXT)
}

pub(crate) fn dim_style() -> Style {
    Style::default().fg(TEXT_DIM)
}

pub(crate) fn income_style() -> Style {
    Style::default().fg(GREEN)
}

pub(crate) fn expense_style() -> Style {
    Style::default().fg(RED)
}

pub(crate) fn alt_row_style() -> Style {
    Style::default().fg(TEXT).bg(SURFACE)
}

pub(crate) fn command_bar_style() -> Style {
    Style::default().fg(TEXT).bg(COMMAND_BG)
}

pub(crate) fn status_bar_style() -> Style {
    Style::default().fg(TEXT_DIM).bg(SURFACE)
}
