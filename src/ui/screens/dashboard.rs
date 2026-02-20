use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Paragraph},
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
        .spacing(1)
        .constraints([
            Constraint::Length(5),  // Debit accounts row
            Constraint::Length(5),  // Credit accounts row
            Constraint::Length(3),  // Net worth
            Constraint::Min(8),    // Spending by category
            Constraint::Length(5),  // Monthly trend
        ])
        .split(area);

    render_debit_row(f, chunks[0], app);
    render_credit_row(f, chunks[1], app);
    render_net_worth(f, chunks[2], app);
    render_spending_chart(f, chunks[3], app);
    render_trend_chart(f, chunks[4], app);
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
    // Use 2 rows per category (bar + blank) when space allows, else 1
    let rows_per = if count > 0 {
        let natural = inner_rows / count;
        if natural >= 2 { natural } else { 1 }
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

fn render_trend_chart(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::OVERLAY))
        .title(Span::styled(
            " Monthly Spending Trend ",
            Style::default()
                .fg(theme::TEXT_DIM)
                .add_modifier(Modifier::BOLD),
        ));

    if app.monthly_trend.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "No trend data yet",
            theme::dim_style(),
        )))
        .centered()
        .block(block);
        f.render_widget(msg, area);
        return;
    }

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Show up to 12 most recent months
    let visible: Vec<_> = app
        .monthly_trend
        .iter()
        .rev()
        .take(12)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let n = visible.len();
    let width = inner.width as usize;

    // Compute bar_width and bar_gap to distribute evenly
    // Each slot = bar_width + bar_gap (last bar has no trailing gap)
    // total = n * bar_width + (n-1) * bar_gap
    // Target: bar ~55% of slot, gap ~45%
    let slot = if n > 0 { width / n } else { 1 };
    let bar_w = (slot * 5 / 9).clamp(3, 7) as u16;
    let bar_g = (slot as u16).saturating_sub(bar_w).max(1);

    // Center horizontally
    let total_used = (n as u16) * bar_w + (n as u16).saturating_sub(1) * bar_g;
    let left_pad = inner.width.saturating_sub(total_used) / 2;

    let chart_rect = Rect::new(
        inner.x + left_pad,
        inner.y,
        total_used.min(inner.width),
        inner.height,
    );

    let bars: Vec<Bar> = visible
        .iter()
        .map(|(month_str, _income, expenses)| {
            let label = parse_month_label(month_str);
            let val = expenses.abs().to_f64().unwrap_or(0.0) as u64;
            Bar::default()
                .value(val)
                .text_value(String::new())
                .label(Line::from(Span::styled(
                    label,
                    Style::default().fg(theme::TEXT_DIM),
                )))
                .style(Style::default().fg(theme::ACCENT))
        })
        .collect();

    let chart = BarChart::default()
        .data(BarGroup::default().bars(&bars))
        .bar_width(bar_w)
        .bar_gap(bar_g);

    f.render_widget(chart, chart_rect);
}

/// Parse "YYYY-MM" into single-letter month label.
fn parse_month_label(month_str: &str) -> &'static str {
    const MONTHS: [&str; 12] = [
        "J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D",
    ];
    month_str
        .get(5..7)
        .and_then(|m| m.parse::<usize>().ok())
        .and_then(|m| MONTHS.get(m.wrapping_sub(1)))
        .copied()
        .unwrap_or("?")
}
