use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::ui::app::App;
use crate::ui::theme;
use crate::ui::util::truncate;

pub(crate) fn render(f: &mut Frame, area: Rect, app: &App, categories: &[(i64, String)]) {
    if app.transactions.is_empty() {
        let msg = if !app.search_input.is_empty() {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("No transactions matching '{}'", app.search_input),
                    theme::dim_style(),
                )),
                Line::from(""),
                Line::from(Span::styled("Press Esc to clear the search", theme::dim_style())),
            ]
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No transactions for this month",
                    theme::dim_style(),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Import a CSV with :i or add one with :add-txn",
                    theme::dim_style(),
                )),
            ]
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY))
            .title(Span::styled(
                " Transactions (0) ",
                Style::default().fg(theme::TEXT_DIM).add_modifier(Modifier::BOLD),
            ));
        f.render_widget(Paragraph::new(msg).centered().block(block), area);
        return;
    }

    let header_cells = ["Date", "Description", "Category", "Amount"]
        .iter()
        .map(|h| Cell::from(*h).style(theme::header_style()));
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .transactions
        .iter()
        .enumerate()
        .skip(app.transaction_scroll)
        .take(app.visible_rows)
        .map(|(i, txn)| {
            let cat_name = txn
                .category_id
                .and_then(|cid| categories.iter().find(|(id, _)| *id == cid))
                .map(|(_, name)| name.as_str())
                .unwrap_or("—");

            let amount_style = if txn.is_income() {
                theme::income_style()
            } else {
                theme::expense_style()
            };

            let sign = if txn.is_income() { "+" } else { "" };
            let amount_str = format!("{sign}${:.2}", txn.abs_amount());

            let style = if i == app.transaction_index {
                theme::selected_style()
            } else if i % 2 == 1 {
                theme::alt_row_style()
            } else {
                theme::normal_style()
            };

            Row::new(vec![
                Cell::from(txn.date.as_str()),
                Cell::from(truncate(&txn.description, 40)),
                Cell::from(cat_name),
                Cell::from(Span::styled(amount_str, amount_style)),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Min(20),
        Constraint::Length(18),
        Constraint::Length(14),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::OVERLAY))
                .title(Span::styled(
                    format!(
                        " Transactions ({}) {} ",
                        app.transactions.len(),
                        if !app.search_input.is_empty() {
                            format!("search: '{}'", app.search_input)
                        } else {
                            String::new()
                        }
                    ),
                    Style::default()
                        .fg(theme::TEXT_DIM)
                        .add_modifier(Modifier::BOLD),
                )),
        );

    f.render_widget(table, area);
}

