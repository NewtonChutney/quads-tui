use crate::app::App;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let Some(session) = app.sessions.active_session() else {
        render_no_connection(f, area);
        return;
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Min(3),
        ])
        .split(area);

    render_hosts_bar(f, rows[0], session);
    render_clouds_bar(f, rows[1], session);

    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[2]);

    render_server_widget(f, bottom_cols[0], app);
    render_assignments_summary(f, bottom_cols[1], app, session);
}

fn render_no_connection(f: &mut Frame, area: Rect) {
    let msg = Paragraph::new(vec![
        Line::raw(""),
        Line::from(Span::styled(
            "  No server selected",
            Style::default().fg(Color::Yellow),
        )),
        Line::raw(""),
        Line::raw("  Press [n] to add a server, or [Enter] to select one."),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Status "));
    f.render_widget(msg, area);
}

fn render_server_widget(f: &mut Frame, area: Rect, app: &App) {
    let server_names: Vec<String> = app.config.servers.keys().cloned().collect();

    let rows: Vec<Row> = server_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let entry = &app.config.servers[name];
            let connected = app
                .sessions
                .sessions
                .iter()
                .any(|s| s.name == *name && s.connected);

            let has_session = app.sessions.sessions.iter().any(|s| s.name == *name);

            let (status_icon, status_color, status_label) = if connected {
                ("\u{25cf}", Color::Green, " connected")
            } else if has_session {
                ("\u{25cf}", Color::Red, " read-only")
            } else {
                ("\u{25cb}", Color::DarkGray, "")
            };

            let is_default = app.config.default_server.as_deref() == Some(name.as_str());
            let name_display = if is_default {
                format!("{} *", name)
            } else {
                name.clone()
            };

            let is_selected = i == app.server_selected;
            let base_style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let status_cell = Cell::from(Line::from(vec![
                Span::styled(status_icon, Style::default().fg(if is_selected { Color::Black } else { status_color })),
                Span::styled(status_label, base_style),
            ]));

            Row::new(vec![
                Cell::from(name_display).style(base_style),
                Cell::from(entry.url.clone()).style(base_style),
                status_cell,
            ])
            .style(base_style)
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Percentage(15),
        Constraint::Percentage(55),
        Constraint::Percentage(30),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Name", "URL", "Status"])
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .bottom_margin(1),
        )
        .block(Block::default().borders(Borders::ALL).title(" Servers "))
        .row_highlight_style(Style::default());

    let mut state = TableState::default().with_selected(Some(app.server_selected));
    f.render_stateful_widget(table, area, &mut state);
}

struct BarSegment {
    count: usize,
    color: Color,
    label: &'static str,
}

fn render_bar(
    lines: &mut Vec<Line<'static>>,
    segments: &[BarSegment],
    total: usize,
    bar_width: usize,
) {
    if total == 0 || bar_width == 0 {
        lines.push(Line::from(Span::styled(
            "  (no data)",
            Style::default().fg(Color::DarkGray),
        )));
        return;
    }

    let mut bar_spans: Vec<Span> = vec![Span::raw("  ")];
    let mut used = 0;
    let non_zero: Vec<_> = segments.iter().filter(|s| s.count > 0).collect();
    for (i, seg) in non_zero.iter().enumerate() {
        let width = if i == non_zero.len() - 1 {
            bar_width - used
        } else {
            let w = (seg.count * bar_width + total - 1) / total;
            w.min(bar_width - used)
        };
        if width > 0 {
            bar_spans.push(Span::styled(
                "\u{2588}".repeat(width),
                Style::default().fg(seg.color),
            ));
            used += width;
        }
    }
    if used < bar_width {
        bar_spans.push(Span::styled(
            "\u{2591}".repeat(bar_width - used),
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines.push(Line::from(bar_spans));

    let mut legend_spans: Vec<Span> = vec![Span::raw("  ")];
    for seg in segments {
        if seg.count == 0 {
            continue;
        }
        if legend_spans.len() > 1 {
            legend_spans.push(Span::raw("  "));
        }
        legend_spans.push(Span::styled("\u{25cf}", Style::default().fg(seg.color)));
        legend_spans.push(Span::raw(format!(" {} {}", seg.count, seg.label)));
    }
    lines.push(Line::from(legend_spans));
}

fn render_hosts_bar(f: &mut Frame, area: Rect, session: &crate::session::Session) {
    let hosts = &session.hosts;
    let total = hosts.len();

    let broken = hosts.iter().filter(|h| h.broken == Some(true)).count();
    let retired = hosts
        .iter()
        .filter(|h| h.retired == Some(true) && h.broken != Some(true))
        .count();

    let unassigned: Vec<_> = hosts
        .iter()
        .filter(|h| {
            h.broken != Some(true)
                && h.retired != Some(true)
                && h.cloud_name() == h.default_cloud_name()
        })
        .collect();

    let self_schedulable = unassigned
        .iter()
        .filter(|h| h.can_self_schedule == Some(true))
        .count();
    let unassigned_count = unassigned.len() - self_schedulable;

    let assigned = total - broken - retired - unassigned.len();

    let self_scheduled = hosts
        .iter()
        .filter(|h| {
            h.broken != Some(true)
                && h.retired != Some(true)
                && h.cloud_name() != h.default_cloud_name()
                && h.can_self_schedule == Some(true)
        })
        .count();
    let scheduled = assigned - self_scheduled;

    let graph_total = total - retired;

    let segments = [
        BarSegment { count: scheduled, color: Color::Green, label: "scheduled" },
        BarSegment { count: self_scheduled, color: Color::Blue, label: "self-scheduled" },
        BarSegment { count: self_schedulable, color: Color::Yellow, label: "self-schedulable" },
        BarSegment { count: unassigned_count, color: Color::DarkGray, label: "unassigned" },
        BarSegment { count: broken, color: Color::Red, label: "broken" },
    ];

    let mut lines = vec![Line::from(vec![
        Span::styled("  Active: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!("{}", total - broken - retired)),
        Span::styled(
            format!("  (+ {} broken, + {} retired)", broken, retired),
            Style::default().fg(Color::DarkGray),
        ),
    ])];

    let bar_width = (area.width as usize).saturating_sub(6);
    render_bar(&mut lines, &segments, graph_total, bar_width);

    let widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Hosts "));
    f.render_widget(widget, area);
}

fn render_clouds_bar(f: &mut Frame, area: Rect, session: &crate::session::Session) {
    let summaries = &session.cloud_summaries;
    let total = summaries.len();

    let mut provisioned = 0usize;
    let mut self_scheduled = 0usize;
    let mut free = 0usize;

    for c in summaries {
        let has_owner =
            c.owner.is_some() && c.owner.as_deref() != Some("");
        let has_hosts = c.count.unwrap_or(0) > 0;
        if has_owner && has_hosts {
            if c.is_self_schedule == Some(true) {
                self_scheduled += 1;
            } else {
                provisioned += 1;
            }
        } else {
            free += 1;
        }
    }

    let segments = [
        BarSegment { count: provisioned, color: Color::Green, label: "provisioned" },
        BarSegment { count: self_scheduled, color: Color::Blue, label: "self-scheduled" },
        BarSegment { count: free, color: Color::Yellow, label: "free" },
    ];

    let mut lines = vec![Line::from(vec![
        Span::styled("  Clouds: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!("{}", total)),
        Span::styled(
            format!("  ({} free)", free),
            Style::default().fg(Color::DarkGray),
        ),
    ])];

    let bar_width = (area.width as usize).saturating_sub(6);
    render_bar(&mut lines, &segments, total, bar_width);

    let widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Clouds "));
    f.render_widget(widget, area);
}

fn render_assignments_summary(
    f: &mut Frame,
    area: Rect,
    _app: &App,
    session: &crate::session::Session,
) {
    let authenticated = session.user_email.is_some();

    let mut lines = vec![Line::from(Span::styled(
        format!(
            "  My Assignments: {}  (total active: {})",
            if authenticated {
                session.my_assignments.len().to_string()
            } else {
                "--".to_string()
            },
            session.assignments.len()
        ),
        Style::default().add_modifier(Modifier::BOLD),
    ))];

    if !authenticated {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  Login to view your assignments",
            Style::default().fg(Color::Yellow),
        )));
    } else if session.my_assignments.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No active assignments",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for a in &session.my_assignments {
            let cloud = a.cloud_name().unwrap_or("--");

            let host_count = session
                .schedules
                .iter()
                .filter(|s| s.assignment_id == a.id)
                .count();

            let end_date = session
                .schedules
                .iter()
                .filter(|s| s.assignment_id == a.id)
                .filter_map(|s| s.end.as_deref())
                .max()
                .unwrap_or("--");

            let desc = a
                .description
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(40)
                .collect::<String>();

            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", cloud),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(desc, Style::default().fg(Color::White)),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("      {} hosts", host_count),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("      expires {}", end_date),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Assignments "),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(widget, area);
}
