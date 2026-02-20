use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use crate::ui::app::App;
use crate::ui::theme;

pub(crate) fn render(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_category_list(f, chunks[0], app);
    render_rules_list(f, chunks[1], app);
}

fn render_category_list(f: &mut Frame, area: Rect, app: &App) {
    // Build tree structure
    let items: Vec<ListItem> = app
        .categories
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let prefix = if cat.parent_id.is_some() {
                "  └ "
            } else {
                ""
            };
            let style = if i == app.category_index {
                theme::selected_style()
            } else if cat.parent_id.is_none() {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme::normal_style()
            };

            ListItem::new(Line::from(Span::styled(
                format!("{prefix}{}", cat.name),
                style,
            )))
        })
        .collect();

    let border_color = if !app.category_view_rules {
        theme::ACCENT
    } else {
        theme::OVERLAY
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                format!(" Categories ({}) ", app.categories.len()),
                Style::default()
                    .fg(if !app.category_view_rules {
                        theme::ACCENT
                    } else {
                        theme::TEXT_DIM
                    })
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(list, area);
}

fn render_rules_list(f: &mut Frame, area: Rect, app: &App) {
    let rules_border_color = if app.category_view_rules {
        theme::ACCENT
    } else {
        theme::OVERLAY
    };
    let rules_title_color = if app.category_view_rules {
        theme::ACCENT
    } else {
        theme::TEXT_DIM
    };

    if app.import_rules.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No categorization rules yet",
                theme::dim_style(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Add rules with :rule <pattern> <category>",
                theme::dim_style(),
            )),
            Line::from(Span::styled(
                "e.g. :rule amazon Shopping",
                Style::default().fg(theme::ACCENT),
            )),
        ])
        .centered()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(rules_border_color))
                .title(Span::styled(
                    " Auto-Categorization Rules ",
                    Style::default()
                        .fg(rules_title_color)
                        .add_modifier(Modifier::BOLD),
                )),
        );
        f.render_widget(msg, area);
        return;
    }

    let header_cells = ["Pattern", "Category", "Type"]
        .iter()
        .map(|h| Cell::from(*h).style(theme::header_style()));
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .import_rules
        .iter()
        .enumerate()
        .map(|(i, rule)| {
            let cat_name = app
                .categories
                .iter()
                .find(|c| c.id == Some(rule.category_id))
                .map(|c| c.name.as_str())
                .unwrap_or("?");

            let style = if i == app.rule_index {
                theme::selected_style()
            } else {
                theme::normal_style()
            };

            Row::new(vec![
                Cell::from(rule.pattern.as_str()),
                Cell::from(cat_name),
                Cell::from(if rule.is_regex { "regex" } else { "contains" }),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Min(20),
        Constraint::Length(18),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(rules_border_color))
            .title(Span::styled(
                format!(
                    " Rules ({}) | :rule <pattern> <category> to add ",
                    app.import_rules.len()
                ),
                Style::default()
                    .fg(rules_title_color)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(table, area);
}
