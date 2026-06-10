use crate::app::App;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, TableState};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if app.sessions.active_session().is_none() {
        let empty = Paragraph::new("  No server selected. Press [d] to go to dashboard.")
            .block(Block::default().borders(Borders::ALL).title(" Hosts "))
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, area);
        return;
    }

    let (table_area, search_area) = if app.host_search.active || app.host_search.query.is_some() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    if let Some(sa) = search_area {
        let query = app.host_search.query.as_deref().unwrap_or("");
        let cursor = if app.host_search.active { "\u{2588}" } else { "" };
        let search_line = Line::from(vec![
            Span::styled(" /", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(format!("{}{} ", query, cursor)),
        ]);
        let search_bar = Paragraph::new(search_line)
            .style(Style::default().bg(Color::Rgb(30, 30, 30)));
        f.render_widget(search_bar, sa);
    }

    let filtered = app.filtered_hosts();
    let show_select = app.host_self_schedule_only || !app.host_multi_select.is_empty();

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, host)| {
            let status = if host.broken == Some(true) {
                ("broken", Color::Red)
            } else if host.retired == Some(true) {
                ("retired", Color::DarkGray)
            } else if host.cloud_name() == host.default_cloud_name() {
                ("available", Color::Green)
            } else {
                ("scheduled", Color::Yellow)
            };

            let style = if i == app.host_search.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let marker = if show_select {
                if app.host_multi_select.contains(&host.name) {
                    "[x]"
                } else if host.can_self_schedule == Some(true) {
                    "[ ]"
                } else {
                    "   "
                }
            } else {
                "   "
            };

            let cells = vec![
                marker.to_string(),
                host.name.clone(),
                host.model.clone().unwrap_or_default(),
                host.cloud_name().unwrap_or("--").to_string(),
                status.0.to_string(),
                if host.can_self_schedule == Some(true) { "✅" } else { "❌" }.to_string(),
            ];

            Row::new(cells).style(style).height(1)
        })
        .collect();

    let view_label = if app.host_self_schedule_only {
        "self-schedulable"
    } else {
        "all"
    };

    let mut title_spans = vec![
        Span::raw(" Hosts "),
        Span::styled(
            format!("[{}]", view_label),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(
            format!(" ({}) ", filtered.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ];

    if !app.host_multi_select.is_empty() {
        title_spans.push(Span::styled(
            format!("{} selected ", app.host_multi_select.len()),
            Style::default().fg(Color::Green),
        ));
    }

    let title = Line::from(title_spans);

    let widths = vec![
        Constraint::Length(3),
        Constraint::Percentage(24),
        Constraint::Percentage(15),
        Constraint::Percentage(20),
        Constraint::Percentage(17),
        Constraint::Percentage(17),
    ];

    let header_cells = vec!["", "Name", "Model", "Cloud", "Status", "SSM"];

    let table = Table::new(rows, widths)
        .header(
            Row::new(header_cells)
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .bottom_margin(1),
        )
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(Style::default());

    let mut state = TableState::default().with_selected(Some(app.host_search.selected));
    f.render_stateful_widget(table, table_area, &mut state);
}
