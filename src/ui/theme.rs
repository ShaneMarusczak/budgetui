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
