use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use rust_decimal::Decimal;

use crate::ui::app::App;
use crate::ui::theme;
use crate::ui::util::format_amount;

pub(crate) fn render(f: &mut Frame, area: Rect, app: &App) {
    if app.account_snapshots.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No accounts yet.",
                theme::dim_style().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Create one with :account <name> [type] or import a CSV.",
                theme::dim_style(),
            )),
        ])
        .centered()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::OVERLAY)),
        );
        f.render_widget(msg, area);
        return;
    }

    // Each account card is 4 lines: 1 blank, 1 income/expenses, 1 balance, 1 blank
    // We render them as ListItems with 4-line height inside a bordered list.
    let card_height = 4_usize;
    let visible = area.height.saturating_sub(2) as usize; // minus list borders
    let cards_per_page = (visible / card_height).max(1);

    let items: Vec<ListItem> = app
        .account_snapshots
        .iter()
        .enumerate()
        .skip(app.accounts_tab_scroll)
        .take(cards_per_page)
        .map(|(i, snap)| {
            let is_credit = snap.account.account_type.is_credit();
            let selected = i == app.accounts_tab_index;

            let border_color = if selected {
                theme::ACCENT
            } else {
                theme::OVERLAY
            };

            let title = format!(" {} ({}) ", snap.account.name, snap.account.account_type);

            // Line 1: title with border chars
            let title_line = Line::from(vec![
                Span::styled("┌─", Style::default().fg(border_color)),
                Span::styled(
                    title,
                    Style::default()
                        .fg(if selected {
                            theme::ACCENT
                        } else {
                            theme::TEXT_DIM
                        })
                        .add_modifier(Modifier::BOLD),
                ),
            ]);

            // Line 2: income/expenses (or charges/payments)
            let (pos_label, neg_label) = if is_credit {
                ("Payments", "Charges")
            } else {
                ("Income", "Expenses")
            };
            let pos_val = snap.month_income;
            let neg_val = snap.month_expenses.abs();

            let detail_line = Line::from(vec![
                Span::styled(format!("  {pos_label}: "), theme::dim_style()),
                Span::styled(format_amount(pos_val), Style::default().fg(theme::GREEN)),
                Span::styled(format!("    {neg_label}: "), theme::dim_style()),
                Span::styled(format_amount(neg_val), Style::default().fg(theme::RED)),
            ]);

            // Line 3: balance
            let bal_color = if snap.balance >= Decimal::ZERO {
                theme::GREEN
            } else {
                theme::RED
            };
            let balance_line = Line::from(vec![
                Span::styled("  Balance: ", theme::dim_style()),
                Span::styled(
                    format_amount(snap.balance),
                    Style::default().fg(bal_color).add_modifier(Modifier::BOLD),
                ),
            ]);

            // Line 4: bottom border (dynamic width)
            let border_width = (area.width as usize).saturating_sub(3);
            let bottom_line = Line::from(Span::styled(
                format!("└{}", "─".repeat(border_width)),
                Style::default().fg(border_color),
            ));

            ListItem::new(vec![title_line, detail_line, balance_line, bottom_line])
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY))
            .title(Span::styled(
                format!(
                    " {} Accounts | j/k navigate | Enter view transactions ",
                    app.account_snapshots.len()
                ),
                theme::dim_style(),
            )),
    );
    f.render_widget(list, area);
}
