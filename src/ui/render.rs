use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
    Frame,
};

use super::app::{App, ImportStep, InputMode, Screen};
use super::commands;
use super::theme;

pub(crate) fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tab bar
            Constraint::Min(5),    // Main content
            Constraint::Length(1), // Status bar
            Constraint::Length(1), // Command bar
        ])
        .split(f.area());

    render_tab_bar(f, chunks[0], app);
    render_screen(f, chunks[1], app);
    render_status_bar(f, chunks[2], app);
    render_command_bar(f, chunks[3], app);

    if app.show_help {
        render_help_overlay(f, f.area());
    }
}

fn render_tab_bar(f: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = Screen::all()
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let num = format!("{}", i + 1);
            if *s == app.screen {
                Line::from(vec![
                    Span::styled(format!("{num}:"), Style::default().fg(theme::TEXT_DIM)),
                    Span::styled(
                        format!("{s}"),
                        Style::default()
                            .fg(theme::ACCENT)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(Span::styled(
                    format!("{num}:{s}"),
                    Style::default().fg(theme::TEXT_DIM),
                ))
            }
        })
        .collect();

    let tabs = Tabs::new(titles)
        .divider(Span::styled(" | ", Style::default().fg(theme::OVERLAY)))
        .style(Style::default().bg(theme::HEADER_BG));

    f.render_widget(tabs, area);
}

fn render_screen(f: &mut Frame, area: Rect, app: &App) {
    match app.screen {
        Screen::Dashboard => super::screens::dashboard::render(f, area, app),
        Screen::Transactions => {
            let cat_lookup: Vec<(i64, String)> = app
                .categories
                .iter()
                .filter_map(|c| c.id.map(|id| (id, c.name.clone())))
                .collect();
            super::screens::transactions::render(f, area, app, &cat_lookup);
        }
        Screen::Import => super::screens::import::render(f, area, app),
        Screen::Categories => super::screens::categories::render(f, area, app),
        Screen::Budgets => {
            let spending = &app.spending_by_category;
            super::screens::budgets::render(f, area, app, spending);
        }
    }
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let mode_label = format!(" {} ", app.input_mode);
    let mode_style = match app.input_mode {
        InputMode::Normal => Style::default()
            .fg(theme::HEADER_BG)
            .bg(theme::ACCENT)
            .add_modifier(Modifier::BOLD),
        InputMode::Command => Style::default()
            .fg(theme::HEADER_BG)
            .bg(theme::GREEN)
            .add_modifier(Modifier::BOLD),
        InputMode::Search => Style::default()
            .fg(theme::HEADER_BG)
            .bg(theme::YELLOW)
            .add_modifier(Modifier::BOLD),
        InputMode::Editing => Style::default()
            .fg(theme::HEADER_BG)
            .bg(theme::GREEN)
            .add_modifier(Modifier::BOLD),
        InputMode::Confirm => Style::default()
            .fg(theme::HEADER_BG)
            .bg(theme::RED)
            .add_modifier(Modifier::BOLD),
    };

    let info = format!(
        " {} | {} | {} txns",
        app.screen, app.current_month, app.transaction_count
    );

    let right = match app.screen {
        Screen::Dashboard => " H/L month | n/p account | ? help ",
        Screen::Transactions => " D delete | /search | :recat | ? help ",
        Screen::Import => match app.import_step {
            ImportStep::SelectFile => " j/k navigate | Enter select | Esc back ",
            ImportStep::MapColumns => " +/- adjust | Enter preview | Esc back ",
            ImportStep::Preview => " Enter import | Esc back ",
            ImportStep::Complete => " Enter view txns | :d dashboard ",
        },
        Screen::Categories => " r toggle rules | :rule add | ? help ",
        Screen::Budgets => " :budget set | :delete-budget | ? help ",
    };

    let available = area.width as usize;
    let used = mode_label.len() + info.len() + right.len();
    let pad = available.saturating_sub(used);

    let bar = Paragraph::new(Line::from(vec![
        Span::styled(&mode_label, mode_style),
        Span::styled(&info, theme::status_bar_style()),
        Span::styled(" ".repeat(pad), theme::status_bar_style()),
        Span::styled(right, theme::status_bar_style()),
    ]));
    f.render_widget(bar, area);
}

fn render_command_bar(f: &mut Frame, area: Rect, app: &App) {
    let (content, cursor_offset) = match app.input_mode {
        InputMode::Command => (
            Line::from(vec![
                Span::styled(":", Style::default().fg(theme::ACCENT)),
                Span::styled(&app.command_input, theme::command_bar_style()),
            ]),
            Some(1 + app.command_input.len() as u16),
        ),
        InputMode::Search => {
            let match_info = if !app.search_input.is_empty() {
                format!("  ({} matches)", app.transactions.len())
            } else {
                String::new()
            };
            (
                Line::from(vec![
                    Span::styled("/", Style::default().fg(theme::YELLOW)),
                    Span::styled(&app.search_input, theme::command_bar_style()),
                    Span::styled(match_info, theme::dim_style()),
                ]),
                Some(1 + app.search_input.len() as u16),
            )
        }
        InputMode::Editing => (
            Line::from(vec![
                Span::styled("edit> ", Style::default().fg(theme::GREEN)),
                Span::styled(&app.command_input, theme::command_bar_style()),
            ]),
            Some(6 + app.command_input.len() as u16),
        ),
        InputMode::Confirm => (
            Line::from(vec![
                Span::styled(&app.confirm_message, Style::default().fg(theme::YELLOW)),
                Span::styled(" [y/N] ", Style::default().fg(theme::RED)),
            ]),
            None,
        ),
        InputMode::Normal => (
            if app.status_message.is_empty() {
                Line::from(Span::styled(
                    " Press : for commands, / to search, ? for help",
                    theme::dim_style(),
                ))
            } else {
                Line::from(Span::styled(
                    &app.status_message,
                    theme::command_bar_style(),
                ))
            },
            None,
        ),
    };

    let bar = Paragraph::new(content).style(Style::default().bg(theme::COMMAND_BG));
    f.render_widget(bar, area);

    if let Some(offset) = cursor_offset {
        f.set_cursor_position((area.x + offset, area.y));
    }
}

fn render_help_overlay(f: &mut Frame, area: Rect) {
    let mut help_text = vec![
        Line::from(Span::styled(
            " BudgeTUI Help ",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Navigation",
            Style::default()
                .fg(theme::YELLOW)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  j/k or Up/Down   Move cursor           1-5        Switch tabs",
            theme::normal_style(),
        )),
        Line::from(Span::styled(
            "  Tab/Shift-Tab    Cycle tabs            g/G        Top/Bottom",
            theme::normal_style(),
        )),
        Line::from(Span::styled(
            "  H/L              Prev/Next month       Ctrl-d/u   Page Down/Up",
            theme::normal_style(),
        )),
        Line::from(Span::styled(
            "  n/p (Dashboard)  Cycle accounts        Ctrl-q     Quit",
            theme::normal_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Actions",
            Style::default()
                .fg(theme::YELLOW)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  :               Command mode           /          Search (live)",
            theme::normal_style(),
        )),
        Line::from(Span::styled(
            "  D (Transactions) Delete transaction    r (Categories) Toggle rules",
            theme::normal_style(),
        )),
        Line::from(Span::styled(
            "  Enter           Select/Confirm         Esc        Cancel/Back",
            theme::normal_style(),
        )),
        Line::from(Span::styled(
            "  +/- (Import)    Adjust field value",
            theme::normal_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Commands",
            Style::default()
                .fg(theme::YELLOW)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    // Build command list dynamically from COMMANDS registry
    let mut seen = std::collections::HashSet::new();
    let mut cmd_lines: Vec<(&str, &str)> = Vec::new();
    for (&name, cmd) in commands::COMMANDS.iter() {
        if name.len() <= 2 {
            continue;
        }
        if seen.insert(cmd.description) {
            cmd_lines.push((name, cmd.description));
        }
    }
    cmd_lines.sort_by_key(|(name, _)| *name);
    for (name, desc) in &cmd_lines {
        help_text.push(Line::from(Span::styled(
            format!("  :{name:<22} {desc}"),
            theme::normal_style(),
        )));
    }

    help_text.push(Line::from(""));
    help_text.push(Line::from(Span::styled(
        " Press any key to close ",
        Style::default().fg(theme::TEXT_DIM),
    )));

    // Center the popup, clamped to terminal height
    let popup_height = (help_text.len() as u16 + 2).min(area.height.saturating_sub(2));
    let popup_width = 72.min(area.width.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);
    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT))
            .style(Style::default().bg(theme::HEADER_BG)),
    );
    f.render_widget(help, popup_area);
}
