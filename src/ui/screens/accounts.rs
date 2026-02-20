use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use rust_decimal::Decimal;

use crate::models::AccountType;
use crate::ui::app::App;
use crate::ui::theme;

pub(crate) fn render(f: &mut Frame, area: Rect, app: &App) {
    if app.account_snapshots.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No accounts yet.",
                Style::default()
                    .fg(theme::TEXT_DIM)
                    .add_modifier(Modifier::BOLD),
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
            let is_credit = matches!(
                snap.account.account_type,
                AccountType::CreditCard | AccountType::Loan
            );
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
                Span::styled(
                    format!("  {pos_label}: "),
                    Style::default().fg(theme::TEXT_DIM),
                ),
                Span::styled(format!("${pos_val:.2}"), Style::default().fg(theme::GREEN)),
                Span::styled(
                    format!("    {neg_label}: "),
                    Style::default().fg(theme::TEXT_DIM),
                ),
                Span::styled(format!("${neg_val:.2}"), Style::default().fg(theme::RED)),
            ]);

            // Line 3: balance
            let bal_sign = if snap.balance < Decimal::ZERO {
                "-"
            } else {
                ""
            };
            let bal_color = if snap.balance >= Decimal::ZERO {
                theme::GREEN
            } else {
                theme::RED
            };
            let balance_line = Line::from(vec![
                Span::styled("  Balance: ", Style::default().fg(theme::TEXT_DIM)),
                Span::styled(
                    format!("{bal_sign}${:.2}", snap.balance.abs()),
                    Style::default().fg(bal_color).add_modifier(Modifier::BOLD),
                ),
            ]);

            // Line 4: bottom border
            let bottom_line = Line::from(Span::styled(
                "└─────────────────────────────────────────",
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
