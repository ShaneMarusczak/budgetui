use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::ui::app::App;
use crate::ui::theme;

pub(crate) fn render(f: &mut Frame, area: Rect, app: &App, spending: &[(String, Decimal)]) {
    if app.budgets.is_empty() {
        render_empty(f, area);
        return;
    }

    let items: Vec<ListItem> = app
        .budgets
        .iter()
        .enumerate()
        .map(|(i, budget)| {
            let cat_name = app
                .categories
                .iter()
                .find(|c| c.id == Some(budget.category_id))
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown");

            let spent = spending
                .iter()
                .find(|(name, _)| name == cat_name)
                .map(|(_, amt)| amt.abs())
                .unwrap_or(Decimal::ZERO);

            let ratio = if budget.limit_amount > Decimal::ZERO {
                (spent / budget.limit_amount)
                    .to_f64()
                    .unwrap_or(0.0)
                    .min(1.0)
            } else {
                0.0
            };

            let color = if ratio > 0.9 {
                theme::RED
            } else if ratio > 0.7 {
                theme::YELLOW
            } else {
                theme::GREEN
            };

            let style = if i == app.budget_index {
                theme::selected_style()
            } else {
                theme::normal_style()
            };

            let bar = create_progress_bar(ratio, 20);

            ListItem::new(Line::from(vec![
                Span::styled(format!("{cat_name:<18}"), style),
                Span::styled(
                    format!("${:.0}/{:.0} ", spent, budget.limit_amount),
                    Style::default().fg(color),
                ),
                Span::styled(bar, Style::default().fg(color)),
                Span::styled(
                    format!(" {:.0}%", ratio * 100.0),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY))
            .title(Span::styled(
                format!(" Budgets for {} ", app.current_month),
                Style::default()
                    .fg(theme::TEXT_DIM)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(list, area);
}

fn render_empty(f: &mut Frame, area: Rect) {
    let msg = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "No budgets set for this month",
            theme::dim_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Use :budget <category> <amount> to set a spending limit",
            theme::dim_style(),
        )),
    ])
    .centered()
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY))
            .title(Span::styled(
                " Budgets ",
                Style::default()
                    .fg(theme::TEXT_DIM)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(msg, area);
}

fn create_progress_bar(ratio: f64, width: usize) -> String {
    let filled = (ratio * width as f64) as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
