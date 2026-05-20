use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let Some(session) = app.sessions.active_session() else {
        let empty = Paragraph::new("  No server selected. Press [d] to manage servers.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Assignments "),
            )
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, area);
        return;
    };

    let authenticated = session.user_email.is_some();

    if !app.assignment_show_all && !authenticated {
        let placeholder = Paragraph::new(vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  Login to view your assignments",
                Style::default().fg(Color::Yellow),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "  Press [Tab] to view all assignments",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Assignments [mine] "),
        );
        f.render_widget(placeholder, area);
        return;
    }

    let filtered = app.filtered_sorted_assignments();

    let (content_area, search_area) = if app.assignment_searching || app.assignment_search.is_some()
    {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    if let Some(sa) = search_area {
        let query = app.assignment_search.as_deref().unwrap_or("");
        let cursor = if app.assignment_searching {
            "\u{2588}"
        } else {
            ""
        };
        let search_line = Line::from(vec![
            Span::styled(
                " /",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{}{} ", query, cursor)),
        ]);
        let search_bar =
            Paragraph::new(search_line).style(Style::default().bg(Color::Rgb(30, 30, 30)));
        f.render_widget(search_bar, sa);
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(content_area);

    render_sidebar(f, chunks[0], app, &filtered);
    render_detail(f, chunks[1], app, &filtered);
}

fn render_sidebar(f: &mut Frame, area: Rect, app: &App, assignments: &[&crate::api::Assignment]) {
    let max_id_width = assignments
        .iter()
        .map(|a| a.id.map(|i| format!("#{}", i).len()).unwrap_or(0))
        .max()
        .unwrap_or(0);

    let items: Vec<ListItem> = assignments
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let cloud = a.cloud_name().unwrap_or("--");
            let id = a.id.map(|i| format!("#{}", i)).unwrap_or_default();

            let style = if i == app.assignment_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:>width$} ", id, width = max_id_width),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(cloud.to_string()),
            ]))
            .style(style)
        })
        .collect();

    let filter_label = if app.assignment_show_all {
        "all"
    } else {
        "mine"
    };

    let title = Line::from(vec![
        Span::raw(" Assignments "),
        Span::styled(
            format!("[{}]", filter_label),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(
            format!(" ({}) ", assignments.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default());

    let mut state = ListState::default().with_selected(Some(app.assignment_selected));
    f.render_stateful_widget(list, area, &mut state);
}

fn render_detail(f: &mut Frame, area: Rect, app: &App, assignments: &[&crate::api::Assignment]) {
    let Some(assignment) = assignments.get(app.assignment_selected) else {
        let empty = Paragraph::new("  Select an assignment")
            .block(Block::default().borders(Borders::ALL).title(" Detail "))
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, area);
        return;
    };

    let mut lines = vec![];

    if let Some(id) = assignment.id {
        lines.push(Line::from(vec![
            Span::styled("  ID: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{}", id)),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("  Cloud: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(assignment.cloud_name().unwrap_or("--")),
    ]));

    lines.push(Line::from(vec![
        Span::styled("  Owner: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(assignment.owner.as_deref().unwrap_or("--")),
    ]));

    lines.push(Line::from(vec![
        Span::styled(
            "  Description: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(assignment.description.as_deref().unwrap_or("--")),
    ]));

    if let Some(ticket) = &assignment.ticket {
        lines.push(Line::from(vec![
            Span::styled("  Ticket: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(ticket),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("  Active: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            if assignment.active == Some(true) {
                "Yes"
            } else {
                "No"
            },
            if assignment.active == Some(true) {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            },
        ),
    ]));

    if let Some(wipe) = assignment.wipe {
        lines.push(Line::from(vec![
            Span::styled("  Wipe: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(if wipe { "Yes" } else { "No" }),
        ]));
    }

    if let Some(ref created) = assignment.created_at {
        lines.push(Line::from(vec![
            Span::styled("  Created: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(created),
        ]));
    }

    lines.push(Line::raw(""));

    let session = app.sessions.active_session().unwrap();
    let related_schedules: Vec<_> = session
        .schedules
        .iter()
        .filter(|s| s.assignment_id == assignment.id)
        .collect();

    lines.push(Line::from(Span::styled(
        format!("  Hosts ({}):", related_schedules.len()),
        Style::default().add_modifier(Modifier::BOLD),
    )));

    for (i, sched) in related_schedules.iter().enumerate() {
        let host = sched.host_name();
        let start = sched.start.as_deref().unwrap_or("--");
        let end = sched.end.as_deref().unwrap_or("--");
        let is_selected = app.assignment_detail_selected == Some(i);
        if is_selected {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  > {} ", host),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} -> {}", start, end),
                    Style::default().fg(Color::Black).bg(Color::Cyan),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("    {} ", host), Style::default().fg(Color::Cyan)),
                Span::raw(format!("{} -> {}", start, end)),
            ]));
        }
    }

    let detail = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Detail "))
        .wrap(Wrap { trim: true });

    f.render_widget(detail, area);
}
