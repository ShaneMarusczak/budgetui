use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use crate::ui::app::{App, ImportStep};
use crate::ui::theme;
use crate::ui::util::truncate;

pub(crate) fn render(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(5)])
        .split(area);

    render_step_indicator(f, chunks[0], app);

    match app.import_step {
        ImportStep::SelectFile => render_file_browser(f, chunks[1], app),
        ImportStep::MapColumns => render_column_mapper(f, chunks[1], app),
        ImportStep::Preview => render_preview(f, chunks[1], app),
        ImportStep::Complete => render_complete(f, chunks[1], app),
    }
}

fn render_step_indicator(f: &mut Frame, area: Rect, app: &App) {
    let steps = [
        (ImportStep::SelectFile, "1:File"),
        (ImportStep::MapColumns, "2:Map"),
        (ImportStep::Preview, "3:Preview"),
        (ImportStep::Complete, "4:Done"),
    ];
    let current_idx = steps.iter().position(|(s, _)| *s == app.import_step).unwrap_or(0);

    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(" ", Style::default().bg(theme::HEADER_BG)));
    for (i, (_, label)) in steps.iter().enumerate() {
        let style = if i == current_idx {
            Style::default().fg(theme::HEADER_BG).bg(theme::ACCENT).add_modifier(Modifier::BOLD)
        } else if i < current_idx {
            Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT_DIM)
        };
        spans.push(Span::styled(format!(" {label} "), style));
        if i < steps.len() - 1 {
            let connector_style = if i < current_idx {
                Style::default().fg(theme::GREEN)
            } else {
                Style::default().fg(theme::TEXT_DIM)
            };
            spans.push(Span::styled(" > ", connector_style));
        }
    }

    let bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(theme::HEADER_BG));
    f.render_widget(bar, area);
}

fn render_file_browser(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let path_display = Paragraph::new(Line::from(vec![
        Span::styled(" Path: ", Style::default().fg(theme::TEXT_DIM)),
        Span::styled(
            app.file_browser_path.display().to_string(),
            Style::default().fg(theme::ACCENT),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY))
            .title(Span::styled(
                " Select CSV File ",
                Style::default()
                    .fg(theme::TEXT_DIM)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(path_display, chunks[0]);

    let items: Vec<ListItem> = app
        .file_browser_entries
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let name = if Some(path.as_path()) == app.file_browser_path.parent() {
                "📁 ..".to_string()
            } else if path.is_dir() {
                format!("📁 {}", path.file_name().and_then(|n| n.to_str()).unwrap_or("?"))
            } else {
                format!("📄 {}", path.file_name().and_then(|n| n.to_str()).unwrap_or("?"))
            };

            let style = if i == app.file_browser_index {
                theme::selected_style()
            } else {
                theme::normal_style()
            };

            ListItem::new(Line::from(Span::styled(name, style)))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY))
            .title(Span::styled(
                " j/k to navigate, Enter to select, Esc to cancel ",
                theme::dim_style(),
            )),
    );
    f.render_widget(list, chunks[1]);
}

fn render_column_mapper(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Detected bank
            Constraint::Length(12), // Column mapping fields
            Constraint::Min(5),    // Sample data
        ])
        .split(area);

    // Bank detection status
    let bank_msg = if let Some(ref bank) = app.import_detected_bank {
        format!("Auto-detected: {bank} | Adjust mappings if needed")
    } else {
        "Custom CSV - set column mappings below".into()
    };
    let status = Paragraph::new(Line::from(Span::styled(bank_msg, Style::default().fg(theme::ACCENT))))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::OVERLAY))
                .title(Span::styled(" Column Mapping ", Style::default().fg(theme::TEXT_DIM).add_modifier(Modifier::BOLD))),
        );
    f.render_widget(status, chunks[0]);

    // Column mapping fields
    let fields = [
        ("Date Column", format!("{}", app.import_profile.date_column)),
        ("Description Column", format!("{}", app.import_profile.description_column)),
        ("Amount Column", app.import_profile.amount_column.map(|c| c.to_string()).unwrap_or_else(|| "—".into())),
        ("Debit Column", app.import_profile.debit_column.map(|c| c.to_string()).unwrap_or_else(|| "—".into())),
        ("Credit Column", app.import_profile.credit_column.map(|c| c.to_string()).unwrap_or_else(|| "—".into())),
        ("Date Format", app.import_profile.date_format.clone()),
        ("Has Header", if app.import_profile.has_header { "Yes" } else { "No" }.into()),
        ("Negate Amounts", if app.import_profile.negate_amounts { "Yes" } else { "No" }.into()),
    ];

    let field_items: Vec<ListItem> = fields
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let style = if i == app.import_selected_field {
                theme::selected_style()
            } else {
                theme::normal_style()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{label:<22}"), Style::default().fg(theme::TEXT_DIM)),
                Span::styled(value.as_str(), style),
            ]))
        })
        .collect();

    let field_list = List::new(field_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY))
            .title(Span::styled(
                " j/k navigate, +/- adjust, Enter to preview, Tab for account ",
                theme::dim_style(),
            )),
    );
    f.render_widget(field_list, chunks[1]);

    // Sample data preview
    let header_cells: Vec<Cell> = app
        .import_headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let label = format!("[{i}] {h}");
            Cell::from(label).style(theme::header_style())
        })
        .collect();
    let header = Row::new(header_cells).height(1);

    let sample_rows: Vec<Row> = app
        .import_rows
        .iter()
        .take(5)
        .map(|row| {
            let cells: Vec<Cell> = row.iter().map(|c| Cell::from(c.as_str())).collect();
            Row::new(cells).style(theme::normal_style())
        })
        .collect();

    let col_count = app.import_headers.len().max(1);
    let widths: Vec<Constraint> = (0..col_count)
        .map(|_| Constraint::Min(12))
        .collect();

    let table = Table::new(sample_rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::OVERLAY))
                .title(Span::styled(" Sample Data (first 5 rows) ", theme::dim_style())),
        );
    f.render_widget(table, chunks[2]);
}

fn render_preview(f: &mut Frame, area: Rect, app: &App) {
    let header_cells = ["Date", "Description", "Amount"]
        .iter()
        .map(|h| Cell::from(*h).style(theme::header_style()));
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .import_preview
        .iter()
        .take(50)
        .map(|txn| {
            let amount_style = if txn.amount >= rust_decimal::Decimal::ZERO {
                theme::income_style()
            } else {
                theme::expense_style()
            };
            Row::new(vec![
                Cell::from(txn.date.as_str()),
                Cell::from(truncate(&txn.description, 50)),
                Cell::from(Span::styled(
                    format!("${:.2}", txn.amount),
                    amount_style,
                )),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Min(20),
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
                        " Preview: {} transactions | Enter to commit, Esc to go back ",
                        app.import_preview.len()
                    ),
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                )),
        );
    f.render_widget(table, area);
}

fn render_complete(f: &mut Frame, area: Rect, app: &App) {
    let msg = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "Import complete!",
            Style::default()
                .fg(theme::GREEN)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(&app.status_message, theme::normal_style())),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to view transactions, or :d for dashboard",
            theme::dim_style(),
        )),
    ])
    .centered()
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::GREEN)),
    );
    f.render_widget(msg, area);
}

