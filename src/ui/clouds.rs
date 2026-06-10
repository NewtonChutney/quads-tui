use crate::app::App;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, TableState};
use ratatui::Frame;

use crate::app::fuzzy_match;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let Some(session) = app.sessions.active_session() else {
        let empty = Paragraph::new("  No server selected. Press [d] to go to dashboard.")
            .block(Block::default().borders(Borders::ALL).title(" Clouds "))
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, area);
        return;
    };

    let summaries = &session.cloud_summaries;
    let authenticated = session.user_email.is_some();

    if !app.cloud_show_all && !authenticated {
        let placeholder = Paragraph::new(vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  Login to view your clouds",
                Style::default().fg(Color::Yellow),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "  Press [Tab] to view all clouds",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(Block::default().borders(Borders::ALL).title(" Clouds [mine] "));
        f.render_widget(placeholder, area);
        return;
    }

    let (table_area, search_area) = if app.cloud_search.active || app.cloud_search.query.is_some() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    if let Some(sa) = search_area {
        let query = app.cloud_search.query.as_deref().unwrap_or("");
        let cursor = if app.cloud_search.active { "\u{2588}" } else { "" };
        let search_line = Line::from(vec![
            Span::styled(" /", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(format!("{}{} ", query, cursor)),
        ]);
        let search_bar = Paragraph::new(search_line)
            .style(Style::default().bg(Color::Rgb(30, 30, 30)));
        f.render_widget(search_bar, sa);
    }

    let my_username = session.username();

    let filtered: Vec<_> = if app.cloud_show_all {
        summaries.iter().collect()
    } else {
        summaries
            .iter()
            .filter(|c| {
                match (&c.owner, &my_username) {
                    (Some(owner), Some(username)) => owner == username,
                    _ => false,
                }
            })
            .collect()
    };

    let filtered: Vec<_> = filtered
        .into_iter()
        .filter(|c| {
            if let Some(ref search) = app.cloud_search.query {
                fuzzy_match(&c.name, search)
                    || fuzzy_match(c.owner.as_deref().unwrap_or(""), search)
                    || fuzzy_match(c.ticket.as_deref().unwrap_or(""), search)
                    || fuzzy_match(c.description.as_deref().unwrap_or(""), search)
            } else {
                true
            }
        })
        .collect();

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, cloud)| {
            let style = if i == app.cloud_search.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                cloud.name.clone(),
                cloud.owner.clone().unwrap_or_else(|| "(free)".into()),
                cloud
                    .count
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "0".into()),
                cloud.ticket.clone().unwrap_or_else(|| "--".into()),
                cloud
                    .description
                    .as_deref()
                    .unwrap_or("--")
                    .chars()
                    .take(30)
                    .collect::<String>(),
            ])
            .style(style)
            .height(1)
        })
        .collect();

    let filter_label = if app.cloud_show_all { "all" } else { "mine" };

    let title = Line::from(vec![
        Span::raw(" Clouds "),
        Span::styled(
            format!("[{}]", filter_label),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(
            format!(" ({}) ", filtered.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let widths = [
        ratatui::layout::Constraint::Percentage(15),
        ratatui::layout::Constraint::Percentage(20),
        ratatui::layout::Constraint::Percentage(10),
        ratatui::layout::Constraint::Percentage(15),
        ratatui::layout::Constraint::Percentage(40),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Cloud", "Owner", "Hosts", "Ticket", "Description"])
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .bottom_margin(1),
        )
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(Style::default());

    let mut state = TableState::default().with_selected(Some(app.cloud_search.selected));
    f.render_stateful_widget(table, table_area, &mut state);
}
