use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Sparkline},
    Frame,
};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::ui::app::App;
use crate::ui::theme;
use crate::ui::util::truncate;

pub(crate) fn render(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Debit accounts row
            Constraint::Length(5), // Credit accounts row
            Constraint::Length(3), // Net worth
            Constraint::Min(8),    // Charts
            Constraint::Length(3), // Monthly trend sparkline
        ])
        .split(area);

    render_debit_row(f, chunks[0], app);
    render_credit_row(f, chunks[1], app);
    render_net_worth(f, chunks[2], app);
    render_spending_chart(f, chunks[3], app);
    render_trend_sparkline(f, chunks[4], app);
}

fn render_debit_row(f: &mut Frame, area: Rect, app: &App) {
    let cards = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    let debit_net = app.debit_income + app.debit_expenses;

    render_card(
        f,
        cards[0],
        "Debit",
        "Income",
        app.debit_income,
        theme::GREEN,
    );
    render_card(
        f,
        cards[1],
        "Debit",
        "Expenses",
        app.debit_expenses.abs(),
        theme::RED,
    );
    render_card(
        f,
        cards[2],
        "Debit",
        "Net",
        debit_net,
        if debit_net >= Decimal::ZERO {
            theme::GREEN
        } else {
            theme::RED
        },
    );
}

fn render_credit_row(f: &mut Frame, area: Rect, app: &App) {
    let cards = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    render_card(
        f,
        cards[0],
        "Credit",
        "Charges",
        app.credit_charges.abs(),
        theme::RED,
    );
    render_card(
        f,
        cards[1],
        "Credit",
        "Payments",
        app.credit_payments,
        theme::GREEN,
    );
    render_card(
        f,
        cards[2],
        "Credit",
        "Balance",
        app.credit_balance,
        if app.credit_balance >= Decimal::ZERO {
            theme::GREEN
        } else {
            theme::RED
        },
    );
}

fn render_net_worth(f: &mut Frame, area: Rect, app: &App) {
    let sign = if app.net_worth < Decimal::ZERO {
        "-"
    } else {
        ""
    };
    let display = format!("{}${:.2}", sign, app.net_worth.abs());
    let color = if app.net_worth >= Decimal::ZERO {
        theme::GREEN
    } else {
        theme::RED
    };

    let bar = Paragraph::new(Line::from(vec![
        Span::styled(
            " Net Worth  ",
            Style::default()
                .fg(theme::TEXT_DIM)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            display,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY)),
    );
    f.render_widget(bar, area);
}

fn render_card(
    f: &mut Frame,
    area: Rect,
    group: &str,
    title: &str,
    amount: Decimal,
    color: ratatui::style::Color,
) {
    let sign = if amount < Decimal::ZERO { "-" } else { "" };
    let display = format!("{}${:.2}", sign, amount.abs());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::OVERLAY))
        .title(Span::styled(
            format!(" {group}: {title} "),
            Style::default()
                .fg(theme::TEXT_DIM)
                .add_modifier(Modifier::BOLD),
        ));

    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            display,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
    ])
    .centered()
    .block(block);

    f.render_widget(text, area);
}

fn render_spending_chart(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::OVERLAY))
        .title(Span::styled(
            " Spending by Category ",
            Style::default()
                .fg(theme::TEXT_DIM)
                .add_modifier(Modifier::BOLD),
        ));

    if app.spending_by_category.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "No transactions for this month. Import a CSV with :i",
            theme::dim_style(),
        )))
        .centered()
        .block(block);
        f.render_widget(msg, area);
        return;
    }

    let inner = block.inner(area);
    let width = inner.width as usize;

    let categories: Vec<_> = app
        .spending_by_category
        .iter()
        .take(12)
        .map(|(name, amt)| (truncate(name, 14), amt.abs()))
        .collect();

    let max_val = categories
        .iter()
        .map(|(_, a)| a.to_f64().unwrap_or(0.0))
        .fold(0.0_f64, f64::max);

    let label_width = 15; // right-aligned label column
    let amount_width = 12; // right-aligned dollar amount
    let bar_area = width.saturating_sub(label_width + amount_width + 2); // 2 for spacing

    let count = categories.len();
    let inner_rows = inner.height as usize;
    let rows_per = if count > 0 {
        (inner_rows / count).max(1)
    } else {
        1
    };

    let mut lines: Vec<Line> = Vec::new();

    for (i, (name, amt)) in categories.iter().enumerate() {
        let color = theme::CHART_COLORS[i % theme::CHART_COLORS.len()];
        let val = amt.to_f64().unwrap_or(0.0);
        let bar_len = if max_val > 0.0 {
            ((val / max_val) * bar_area as f64).round() as usize
        } else {
            0
        };
        let amount_str = format!("${:.2}", amt);

        // Right-align the label
        let padded_label = format!("{:>width$}", name, width = label_width);
        // Build the bar: filled + empty
        let bar_filled: String = "\u{2588}".repeat(bar_len);
        let bar_empty: String = " ".repeat(bar_area.saturating_sub(bar_len));
        // Right-align the amount
        let padded_amount = format!("{:>width$}", amount_str, width = amount_width);

        let line = Line::from(vec![
            Span::styled(padded_label, Style::default().fg(theme::TEXT)),
            Span::raw(" "),
            Span::styled(bar_filled, Style::default().fg(color)),
            Span::raw(bar_empty),
            Span::raw(" "),
            Span::styled(
                padded_amount,
                Style::default()
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        lines.push(line);

        // Add blank lines to fill space when fewer categories
        for _ in 1..rows_per {
            lines.push(Line::from(""));
        }
    }

    let chart = Paragraph::new(lines).block(block);
    f.render_widget(chart, area);
}

fn render_trend_sparkline(f: &mut Frame, area: Rect, app: &App) {
    let data: Vec<u64> = app
        .monthly_trend
        .iter()
        .map(|(_, _, exp)| exp.abs().to_u64().unwrap_or(0))
        .collect();

    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::OVERLAY))
                .title(Span::styled(
                    " Monthly Spending Trend ",
                    Style::default()
                        .fg(theme::TEXT_DIM)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .data(&data)
        .style(Style::default().fg(theme::YELLOW));

    f.render_widget(sparkline, area);
}
