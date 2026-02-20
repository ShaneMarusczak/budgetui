use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Paragraph, Sparkline},
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
            Constraint::Length(7), // Summary cards
            Constraint::Min(10),   // Charts
            Constraint::Length(3), // Monthly trend sparkline
        ])
        .split(area);

    render_summary_cards(f, chunks[0], app);
    render_spending_chart(f, chunks[1], app);
    render_trend_sparkline(f, chunks[2], app);
}

fn render_summary_cards(f: &mut Frame, area: Rect, app: &App) {
    let cards = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let net = app.monthly_income + app.monthly_expenses;
    let income_count = app.transactions.iter().filter(|t| t.is_income()).count();
    let expense_count = app.transactions.iter().filter(|t| t.is_expense()).count();

    render_card(
        f,
        cards[0],
        "Income",
        app.monthly_income,
        theme::GREEN,
        Some(format!("{income_count} txns")),
    );
    render_card(
        f,
        cards[1],
        "Expenses",
        app.monthly_expenses.abs(),
        theme::RED,
        Some(format!("{expense_count} txns")),
    );
    render_card(
        f,
        cards[2],
        "Net",
        net,
        if net >= Decimal::ZERO {
            theme::GREEN
        } else {
            theme::RED
        },
        None,
    );
    render_card(
        f,
        cards[3],
        "Net Worth",
        app.net_worth,
        if app.net_worth >= Decimal::ZERO {
            theme::GREEN
        } else {
            theme::RED
        },
        None,
    );
}

fn render_card(
    f: &mut Frame,
    area: Rect,
    title: &str,
    amount: Decimal,
    color: ratatui::style::Color,
    subtitle: Option<String>,
) {
    let sign = if amount < Decimal::ZERO { "-" } else { "" };
    let display = format!("{}${:.2}", sign, amount.abs());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::OVERLAY))
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(theme::TEXT_DIM)
                .add_modifier(Modifier::BOLD),
        ));

    let sub_text = subtitle.unwrap_or_default();

    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            display,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(sub_text, theme::dim_style())),
    ])
    .centered()
    .block(block);

    f.render_widget(text, area);
}

fn render_spending_chart(f: &mut Frame, area: Rect, app: &App) {
    if app.spending_by_category.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY))
            .title(Span::styled(
                " Spending by Category ",
                Style::default()
                    .fg(theme::TEXT_DIM)
                    .add_modifier(Modifier::BOLD),
            ));
        let msg = Paragraph::new(Line::from(Span::styled(
            "No transactions for this month. Import a CSV with :i",
            theme::dim_style(),
        )))
        .centered()
        .block(block);
        f.render_widget(msg, area);
        return;
    }

    let bars: Vec<Bar> = app
        .spending_by_category
        .iter()
        .take(12)
        .map(|(name, amt)| {
            let val = amt.abs().to_u64().unwrap_or(0);
            let label = truncate(name, 10);
            Bar::default()
                .value(val)
                .label(Line::from(label))
                .style(Style::default().fg(theme::ACCENT))
                .value_style(
                    Style::default()
                        .fg(theme::TEXT)
                        .add_modifier(Modifier::BOLD),
                )
        })
        .collect();

    let chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::OVERLAY))
                .title(Span::styled(
                    " Spending by Category ",
                    Style::default()
                        .fg(theme::TEXT_DIM)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(10)
        .bar_gap(1)
        .bar_style(Style::default().fg(theme::ACCENT))
        .value_style(Style::default().fg(theme::TEXT));

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
