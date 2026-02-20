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
        ImportStep::SelectAccount => render_select_account(f, chunks[1], app),
        ImportStep::Preview => render_preview(f, chunks[1], app),
        ImportStep::Categorize => render_categorize(f, chunks[1], app),
        ImportStep::Complete => render_complete(f, chunks[1], app),
    }
}

fn render_step_indicator(f: &mut Frame, area: Rect, app: &App) {
    let steps = [
        (ImportStep::SelectFile, "1:File"),
        (ImportStep::MapColumns, "2:Map"),
        (ImportStep::SelectAccount, "3:Account"),
        (ImportStep::Preview, "4:Preview"),
        (ImportStep::Categorize, "5:Categorize"),
        (ImportStep::Complete, "6:Done"),
    ];
    let current_idx = steps
        .iter()
        .position(|(s, _)| *s == app.import_step)
        .unwrap_or(0);

    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(" ", Style::default().bg(theme::HEADER_BG)));
    for (i, (_, label)) in steps.iter().enumerate() {
        let style = if i == current_idx {
            Style::default()
                .fg(theme::HEADER_BG)
                .bg(theme::ACCENT)
                .add_modifier(Modifier::BOLD)
        } else if i < current_idx {
            Style::default()
                .fg(theme::GREEN)
                .add_modifier(Modifier::BOLD)
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

    // Path input box — shows current path + filter text when focused
    let input_focused = app.file_browser_input_focused;
    let mut spans = vec![
        Span::styled(" Path: ", Style::default().fg(theme::TEXT_DIM)),
        Span::styled(
            app.file_browser_path.display().to_string(),
            Style::default().fg(theme::ACCENT),
        ),
    ];
    if !app.file_browser_filter.is_empty() || input_focused {
        spans.push(Span::styled(
            "  Filter: ",
            Style::default().fg(theme::TEXT_DIM),
        ));
        spans.push(Span::styled(
            &app.file_browser_filter,
            Style::default().fg(theme::TEXT),
        ));
        if input_focused {
            spans.push(Span::styled("█", Style::default().fg(theme::ACCENT)));
        }
    }

    let input_border = if input_focused {
        theme::ACCENT
    } else {
        theme::OVERLAY
    };
    let path_display = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(input_border))
            .title(Span::styled(
                " Select CSV File ",
                Style::default()
                    .fg(theme::TEXT_DIM)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(path_display, chunks[0]);

    // File list — uses filtered entries with viewport scrolling
    let filtered = app.file_browser_filtered();
    let file_list_rows = chunks[1].height.saturating_sub(2) as usize; // minus borders
    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .skip(app.file_browser_scroll)
        .take(file_list_rows)
        .map(|(display_idx, &real_idx)| {
            let path = &app.file_browser_entries[real_idx];
            let name = if Some(path.as_path()) == app.file_browser_path.parent() {
                "📁 ..".to_string()
            } else if path.is_dir() {
                format!(
                    "📁 {}",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
                )
            } else {
                format!(
                    "📄 {}",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
                )
            };

            let style = if display_idx == app.file_browser_index {
                theme::selected_style()
            } else {
                theme::normal_style()
            };

            ListItem::new(Line::from(Span::styled(name, style)))
        })
        .collect();

    let list_border = if input_focused {
        theme::OVERLAY
    } else {
        theme::ACCENT
    };
    let hidden_hint = if app.file_browser_show_hidden {
        " . hide dotfiles"
    } else {
        " . show dotfiles"
    };
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(list_border))
            .title(Span::styled(
                format!(" Tab to filter | j/k nav | Enter select |{hidden_hint} | Esc back "),
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
            Constraint::Length(11), // Column mapping fields (7 fields + borders)
            Constraint::Min(5),     // Sample data
        ])
        .split(area);

    // Bank detection status
    let bank_msg = if let Some(ref bank) = app.import_detected_bank {
        format!("Auto-detected: {bank} | Adjust mappings if needed")
    } else {
        "Custom CSV - set column mappings below".into()
    };
    let status = Paragraph::new(Line::from(Span::styled(
        bank_msg,
        Style::default().fg(theme::ACCENT),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY))
            .title(Span::styled(
                " Column Mapping ",
                Style::default()
                    .fg(theme::TEXT_DIM)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(status, chunks[0]);

    // Column mapping fields
    let fields = [
        ("Date Column", format!("{}", app.import_profile.date_column)),
        (
            "Description Column",
            format!("{}", app.import_profile.description_column),
        ),
        (
            "Amount Column",
            app.import_profile
                .amount_column
                .map(|c| c.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
        (
            "Debit Column",
            app.import_profile
                .debit_column
                .map(|c| c.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
        (
            "Credit Column",
            app.import_profile
                .credit_column
                .map(|c| c.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
        ("Date Format", app.import_profile.date_format.clone()),
        (
            "Has Header",
            if app.import_profile.has_header {
                "Yes"
            } else {
                "No"
            }
            .into(),
        ),
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
    let widths: Vec<Constraint> = (0..col_count).map(|_| Constraint::Min(12)).collect();

    let table = Table::new(sample_rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY))
            .title(Span::styled(
                " Sample Data (first 5 rows) ",
                theme::dim_style(),
            )),
    );
    f.render_widget(table, chunks[2]);
}

fn render_select_account(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    // Top: detection info + suggested type
    let bank_msg = if let Some(ref bank) = app.import_detected_bank {
        format!("Detected: {bank}")
    } else {
        "Custom CSV".into()
    };
    let type_hint = if app.import_profile.is_credit_account {
        "Suggested type: Credit Card"
    } else {
        "Suggested type: Checking"
    };
    let info = Paragraph::new(Line::from(vec![
        Span::styled(format!("  {bank_msg}"), Style::default().fg(theme::ACCENT)),
        Span::styled("  |  ", Style::default().fg(theme::TEXT_DIM)),
        Span::styled(type_hint, Style::default().fg(theme::TEXT_DIM)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY))
            .title(Span::styled(
                " Select Account ",
                Style::default()
                    .fg(theme::TEXT_DIM)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(info, chunks[0]);

    // Bottom: account list or new-account form
    if app.import_creating_account {
        let type_name = crate::models::AccountType::all()
            .get(app.import_new_account_type)
            .map(|t| t.as_str())
            .unwrap_or("Checking");

        let form = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Name: ", Style::default().fg(theme::TEXT_DIM)),
                Span::styled(
                    &app.import_new_account_name,
                    Style::default().fg(theme::TEXT),
                ),
                Span::styled("█", Style::default().fg(theme::ACCENT)),
            ]),
            Line::from(vec![
                Span::styled("  Type: ", Style::default().fg(theme::TEXT_DIM)),
                Span::styled(
                    type_name,
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  (+/- to change)", Style::default().fg(theme::TEXT_DIM)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Enter to create, Esc to cancel",
                Style::default().fg(theme::TEXT_DIM),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::ACCENT))
                .title(Span::styled(
                    " New Account ",
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                )),
        );
        f.render_widget(form, chunks[1]);
    } else {
        let list_rows = chunks[1].height.saturating_sub(2) as usize;
        let items: Vec<ListItem> = app
            .accounts
            .iter()
            .enumerate()
            .skip(app.import_account_scroll)
            .take(list_rows)
            .map(|(i, acct)| {
                let label = format!("{} ({})", acct.name, acct.account_type);
                let style = if i == app.import_account_index {
                    theme::selected_style()
                } else {
                    theme::normal_style()
                };
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

        let hint = if app.accounts.is_empty() {
            " No accounts — press n to create one "
        } else {
            " j/k navigate | Enter select | n new account | Esc back "
        };
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::OVERLAY))
                .title(Span::styled(hint, theme::dim_style())),
        );
        f.render_widget(list, chunks[1]);
    }
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
                Cell::from(Span::styled(format!("${:.2}", txn.amount), amount_style)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Min(20),
        Constraint::Length(14),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY))
            .title(Span::styled(
                {
                    let total = app.import_preview.len();
                    let shown = total.min(50);
                    if shown < total {
                        format!(" Preview: showing {shown} of {total} transactions | Enter to commit, Esc to go back ")
                    } else {
                        format!(" Preview: {total} transactions | Enter to commit, Esc to go back ")
                    }
                },
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(table, area);
}

fn render_categorize(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Current description being categorized
            Constraint::Min(5),    // Category picker list
        ])
        .split(area);

    // ── Description being categorized ────────────────────────
    let total = app.import_cat_descriptions.len();
    let current = app.import_cat_index + 1;

    let (desc, count) = app
        .import_cat_descriptions
        .get(app.import_cat_index)
        .cloned()
        .unwrap_or_else(|| ("?".into(), 0));

    let desc_block = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("  Description: ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled(
                truncate(&desc, 60),
                Style::default()
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Transactions: ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled(format!("{count}"), Style::default().fg(theme::ACCENT)),
            Span::styled(
                format!("  ({current} of {total} unique descriptions)"),
                Style::default().fg(theme::TEXT_DIM),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT))
            .title(Span::styled(
                " What category is this? ",
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(desc_block, chunks[0]);

    // ── Category picker list ─────────────────────────────────
    let cat_visible = chunks[1].height.saturating_sub(2) as usize; // minus list borders
    let mut items: Vec<ListItem> = app
        .categories
        .iter()
        .enumerate()
        .skip(app.import_cat_scroll)
        .take(cat_visible)
        .map(|(i, cat)| {
            let style = if i == app.import_cat_selected {
                theme::selected_style()
            } else {
                theme::normal_style()
            };
            ListItem::new(Line::from(Span::styled(&cat.name, style)))
        })
        .collect();

    // Show "new category" input at the bottom if creating
    if app.import_cat_creating {
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                "  + New: ",
                Style::default()
                    .fg(theme::GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&app.import_cat_new_name, Style::default().fg(theme::TEXT)),
            Span::styled("█", Style::default().fg(theme::ACCENT)),
        ])));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::OVERLAY))
            .title(Span::styled(
                format!(
                    " Categories ({}) | Enter assign | s skip | S skip all | n new ",
                    app.categories.len()
                ),
                theme::dim_style(),
            )),
    );
    f.render_widget(list, chunks[1]);
}

fn render_complete(f: &mut Frame, area: Rect, app: &App) {
    let msg = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "✓ Import complete!",
            Style::default()
                .fg(theme::GREEN)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(&app.status_message, theme::normal_style())),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Enter ",
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("to finish  ", theme::dim_style()),
            Span::styled(
                "i ",
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("import another file", theme::dim_style()),
        ]),
    ])
    .centered()
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::GREEN)),
    );
    f.render_widget(msg, area);
}
