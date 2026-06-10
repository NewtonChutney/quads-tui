use crate::app::{
    App, AuthForm, AuthFormField, HostFilterFlags, HostFilterPopup, Screen, ServerForm,
    ServerFormField,
};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

pub fn render_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![Span::styled(
        " QUADS ",
        Style::default().fg(Color::Black).bg(Color::Cyan),
    )];

    if let Some(session) = app.sessions.active_session() {
        spans.push(Span::raw(" "));
        let status = if session.connected { "●" } else { "○" };
        let color = if session.connected {
            Color::Green
        } else {
            Color::Red
        };
        spans.push(Span::styled(status, Style::default().fg(color)));
        spans.push(Span::raw(format!(" {} ", session.name)));

        if let Some(ref v) = session.version {
            // QUADS server version
            spans.push(Span::styled(
                format!("({}) ", v),
                Style::default().fg(Color::DarkGray),
            ));
        }
    } else {
        spans.push(Span::styled(
            " No connection ",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // QUADS TUI version
    let mut right_spans = vec![Span::styled(
        format!("v{}", crate::update::VERSION),
        Style::default().fg(Color::DarkGray),
    )];

    if let Some(ref info) = app.update_available {
        right_spans.push(Span::styled(
            format!(" (v{} available)", info.latest_version),
            Style::default().fg(Color::Yellow),
        ));
        right_spans.push(Span::styled(
            " [U]pdate",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let left_width: usize = spans.iter().map(|s| s.width()).sum();
    let right_width: usize = right_spans.iter().map(|s| s.width()).sum();
    let total = area.width as usize;
    if total > left_width + right_width {
        spans.push(Span::raw(" ".repeat(total - left_width - right_width)));
    }
    spans.extend(right_spans);

    let bar = Paragraph::new(Line::from(spans));
    f.render_widget(bar, area);
}

fn bracketed_hint(label: &str, key_pos: usize) -> Vec<Span<'static>> {
    let chars: Vec<char> = label.chars().collect();
    let mut spans = Vec::new();
    spans.push(Span::raw(" "));
    for (i, ch) in chars.iter().enumerate() {
        if i == key_pos {
            spans.push(Span::styled(
                format!("[{}]", ch),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(Color::Gray),
            ));
        }
    }
    spans
}

fn key_hint(key: &str, desc: &str) -> Vec<Span<'static>> {
    vec![
        Span::raw(" "),
        Span::styled(
            format!("[{}]", key),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {}", desc), Style::default().fg(Color::Gray)),
    ]
}

pub fn render_help_bar(f: &mut Frame, area: Rect, app: &App) {
    let mut spans: Vec<Span> = vec![
        Span::styled(
            "[Q]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled("uit", Style::default().fg(Color::Red)),
    ];

    match app.screen {
        Screen::Dashboard => {
            let server_names: Vec<String> = app.config.servers.keys().cloned().collect();
            let is_active = server_names
                .get(app.server_selected)
                .map(|name| {
                    app.sessions
                        .active_session()
                        .map(|s| s.name == *name)
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            if is_active {
                let is_connected = app
                    .sessions
                    .active_session()
                    .map(|s| s.connected)
                    .unwrap_or(false);
                if is_connected {
                    spans.extend(key_hint("Enter", "logout"));
                } else {
                    spans.extend(key_hint("Enter", "login"));
                }
            } else {
                spans.extend(key_hint("Enter", "select"));
            }
            spans.extend(key_hint("n", "new server"));
            spans.extend(key_hint("e", "edit"));
        }
        Screen::Hosts => {
            spans.extend(key_hint("Enter", "info"));
            if app.host_self_schedule_only {
                spans.extend(key_hint("Space", "select"));
                if !app.host_multi_select.is_empty() {
                    spans.extend(key_hint("Esc", "clear selection"));
                }
                spans.extend(key_hint("s", "schedule"));
            } else {
                if app.host_search.query.is_some() {
                    spans.extend(key_hint("Esc", "clear search"));
                }
                spans.extend(key_hint("f", "filter"));
            }
            spans.extend(key_hint("Tab", "change view"));
            spans.extend(key_hint("/", "search"));
        }
        Screen::Assignments => {
            if app.assignment_search.query.is_some() {
                spans.extend(key_hint("Esc", "clear search"));
            }
            if app.assignment_detail_selected.is_some() {
                spans.extend(key_hint("Enter", "host info"));
                spans.extend(key_hint("u", "unschedule"));
                spans.extend(key_hint("Esc", "back"));
            } else {
                spans.extend(key_hint("Enter", "hosts"));
                spans.extend(bracketed_hint("terminate", 0));
                spans.extend(key_hint("Tab", "all/mine"));
                spans.extend(key_hint("/", "search"));
            }
        }
        Screen::Clouds => {
            if app.cloud_search.query.is_some() {
                spans.extend(key_hint("Esc", "clear search"));
            }
            spans.extend(key_hint("Enter", "detail"));
            spans.extend(key_hint("Tab", "all/mine"));
            spans.extend(key_hint("/", "search"));
        }
    }

    let mut right_spans: Vec<Span> = Vec::new();
    right_spans.extend(key_hint("?", "config/logs"));
    right_spans.extend(key_hint("r", "refresh"));
    let ar_label = if app.auto_refresh {
        "auto-refresh [on]"
    } else {
        "auto-refresh"
    };
    right_spans.extend(key_hint("x", ar_label));
    right_spans.push(Span::raw(" "));
    right_spans.push(Span::styled(
        "[j/k/⬆️/⬇️]",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));
    right_spans.push(Span::styled(" navigate", Style::default().fg(Color::Gray)));

    let left_width: usize = spans.iter().map(|s| s.width()).sum();
    let right_width: usize = right_spans.iter().map(|s| s.width()).sum();
    let total_width = area.width as usize;
    let pad = total_width.saturating_sub(left_width + right_width);
    spans.push(Span::raw(" ".repeat(pad)));
    spans.extend(right_spans);

    let bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Rgb(30, 30, 30)));
    f.render_widget(bar, area);
}

pub fn render_tab_bar(f: &mut Frame, area: Rect, active: Screen) {
    let tabs: &[(Screen, &str, usize)] = &[
        (Screen::Dashboard, "Dashboard", 0),
        (Screen::Hosts, "Hosts", 0),
        (Screen::Assignments, "Assignments", 0),
        (Screen::Clouds, "Clouds", 0),
    ];

    let mut spans: Vec<Span> = Vec::new();

    for (screen, label, key_pos) in tabs {
        let is_active = *screen == active;
        let chars: Vec<char> = label.chars().collect();

        spans.push(Span::raw(" "));
        for (i, ch) in chars.iter().enumerate() {
            if i == *key_pos {
                let style = if is_active {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                };
                spans.push(Span::styled(format!("[{}]", ch), style));
            } else {
                let style = if is_active {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                spans.push(Span::styled(ch.to_string(), style));
            }
        }
    }

    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        "[⬅️/➡️]",
        Style::default().fg(Color::DarkGray),
    ));
    spans.push(Span::raw("switch tab"));

    let bar = Paragraph::new(Line::from(spans));
    f.render_widget(bar, area);
}

pub fn render_update_complete_popup(f: &mut Frame, message: &str) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);

    let text = format!("{}\n\nPress any key to exit.", message);
    let popup = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Update Complete ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::Green));

    f.render_widget(popup, area);
}

pub fn render_error_popup(f: &mut Frame, message: &str) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);

    let popup = Paragraph::new(message)
        .block(
            Block::default()
                .title(" Error ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::Red));

    f.render_widget(popup, area);
}

pub fn render_server_form(f: &mut Frame, form: &ServerForm) {
    let area = centered_rect(60, 35, f.area());
    f.render_widget(Clear, area);

    let title = if form.editing_existing.is_some() {
        " Edit Server "
    } else {
        " New Server "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let field_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    let fields = [
        (ServerFormField::Name, "Name", &form.name),
        (ServerFormField::Url, "URL", &form.url),
    ];

    for (i, (field, label, input)) in fields.iter().enumerate() {
        let is_active = form.active_field == *field;
        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let cursor_style = Style::default().add_modifier(Modifier::REVERSED);
        let line = if is_active {
            Line::from(vec![
                Span::raw(input.before_cursor()),
                Span::styled(input.char_at_cursor(), cursor_style),
                Span::raw(input.after_cursor_char()),
            ])
        } else {
            Line::raw(&input.value)
        };

        let widget = Paragraph::new(line).block(
            Block::default()
                .title(format!(" {} ", label))
                .borders(Borders::ALL)
                .border_style(border_style),
        );

        f.render_widget(widget, field_chunks[i]);
    }

    let ssl_active = form.active_field == ServerFormField::VerifySsl;
    render_checkbox(
        f,
        field_chunks[2],
        "Verify SSL",
        form.verify_ssl,
        ssl_active,
    );

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" next field  "),
        Span::styled("Space", Style::default().fg(Color::Yellow)),
        Span::raw(" toggle  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" save  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, field_chunks[3]);
}

pub fn render_auth_form(f: &mut Frame, form: &AuthForm) {
    if form.register_prompt {
        render_register_prompt(f, form);
        return;
    }

    let area = centered_rect(60, 40, f.area());
    f.render_widget(Clear, area);

    let title = Line::from(vec![
        Span::raw(format!(" {} — ", form.server_name)),
        Span::styled(
            "Login",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let has_error = form.error.is_some();
    let field_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_error {
            vec![
                Constraint::Length(4),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
        } else {
            vec![
                Constraint::Length(0),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
        })
        .split(inner);

    if let Some(ref err) = form.error {
        let err_style = Style::default().fg(Color::Red);
        let lines: Vec<Line> = err
            .split('\n')
            .enumerate()
            .map(|(i, line)| {
                if i == 0 {
                    Line::from(vec![
                        Span::styled(" \u{2717} ", err_style.add_modifier(Modifier::BOLD)),
                        Span::styled(line, err_style),
                    ])
                } else {
                    Line::from(Span::styled(format!("   {}", line), err_style))
                }
            })
            .collect();
        let error_msg = Paragraph::new(lines).wrap(Wrap { trim: true });
        f.render_widget(error_msg, field_chunks[0]);
    }

    use crate::app::TextInput;
    let fields: [(AuthFormField, &str, &TextInput, bool); 2] = [
        (AuthFormField::Username, "Username", &form.username, false),
        (AuthFormField::Password, "Password", &form.password, true),
    ];

    for (i, (field, label, input, is_password)) in fields.iter().enumerate() {
        let is_active = form.active_field == *field;
        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let cursor_style = Style::default().add_modifier(Modifier::REVERSED);
        let line = if input.is_empty() && *field == AuthFormField::Username {
            if is_active {
                Line::from(vec![
                    Span::styled(
                        "u",
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::REVERSED),
                    ),
                    Span::styled("ser@example.com", Style::default().fg(Color::DarkGray)),
                ])
            } else {
                Line::from(Span::styled(
                    "user@example.com",
                    Style::default().fg(Color::DarkGray),
                ))
            }
        } else if *is_password {
            let masked_before = "*".repeat(input.before_cursor().chars().count());
            let cursor_char = if input.cursor < input.value.len() {
                "*"
            } else {
                " "
            };
            let masked_after_count = input.after_cursor_char().chars().count();
            let masked_after = "*".repeat(masked_after_count);
            if is_active {
                Line::from(vec![
                    Span::raw(masked_before),
                    Span::styled(cursor_char, cursor_style),
                    Span::raw(masked_after),
                ])
            } else {
                Line::raw("*".repeat(input.value.chars().count()))
            }
        } else if is_active {
            Line::from(vec![
                Span::raw(input.before_cursor()),
                Span::styled(input.char_at_cursor(), cursor_style),
                Span::raw(input.after_cursor_char()),
            ])
        } else {
            Line::raw(&input.value)
        };

        let widget = Paragraph::new(line).block(
            Block::default()
                .title(format!(" {} ", label))
                .borders(Borders::ALL)
                .border_style(border_style),
        );

        f.render_widget(widget, field_chunks[i + 1]);
    }

    let enter_action = if form.active_field == AuthFormField::Username && form.password.is_empty() {
        " next  "
    } else {
        " connect  "
    };
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" next  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(enter_action),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, field_chunks[3]);
}

fn render_register_prompt(f: &mut Frame, form: &AuthForm) {
    let area = centered_rect(50, 25, f.area());
    f.render_widget(Clear, area);

    let title = Line::from(vec![
        Span::raw(format!(" {} — ", form.server_name)),
        Span::styled(
            "Register",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let has_error = form.error.is_some();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_error {
            vec![
                Constraint::Length(4),
                Constraint::Length(2),
                Constraint::Min(1),
            ]
        } else {
            vec![
                Constraint::Length(0),
                Constraint::Length(2),
                Constraint::Min(1),
            ]
        })
        .split(inner);

    if let Some(ref err) = form.error {
        let err_style = Style::default().fg(Color::Red);
        let lines: Vec<Line> = err
            .split('\n')
            .enumerate()
            .map(|(i, line)| {
                if i == 0 {
                    Line::from(vec![
                        Span::styled(" \u{2717} ", err_style.add_modifier(Modifier::BOLD)),
                        Span::styled(line, err_style),
                    ])
                } else {
                    Line::from(Span::styled(format!("   {}", line), err_style))
                }
            })
            .collect();
        let error_msg = Paragraph::new(lines).wrap(Wrap { trim: true });
        f.render_widget(error_msg, chunks[0]);
    }

    let prompt = Paragraph::new(Line::from(vec![
        Span::raw(" Register as "),
        Span::styled(
            &form.username.value,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("?"),
    ]));
    f.render_widget(prompt, chunks[1]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" register  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[2]);
}

pub fn render_confirm_popup(f: &mut Frame, message: &str) {
    let area = centered_rect(50, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Confirm ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let msg = Paragraph::new(Line::from(vec![Span::raw(" "), Span::raw(message)]));
    f.render_widget(msg, chunks[0]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" confirm  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[1]);
}

pub fn render_connect_success_popup(f: &mut Frame, server_name: &str) {
    let area = centered_rect(40, 15, f.area());
    f.render_widget(Clear, area);

    let text = format!(" \u{2714} Connected to {}", server_name);
    let popup = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Connected ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .style(Style::default().fg(Color::Green));

    f.render_widget(popup, area);
}

pub fn render_connecting_popup(f: &mut Frame, server_name: &str, spinner: char) {
    let area = centered_rect(40, 15, f.area());
    f.render_widget(Clear, area);

    let text = format!(" {} Connecting to {}...", spinner, server_name);
    let popup = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Connecting ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(popup, area);
}

pub fn render_working_popup(f: &mut Frame, msg: &str, spinner: char) {
    let area = centered_rect(40, 15, f.area());
    f.render_widget(Clear, area);

    let text = format!(" {} {}", spinner, msg);
    let popup = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::Yellow));

    f.render_widget(popup, area);
}

pub fn render_host_info_popup(f: &mut Frame, app: &App, info: &crate::app::HostInfoState) {
    let area = centered_rect(80, 80, f.area());
    f.render_widget(Clear, area);

    let session = match app.sessions.active_session() {
        Some(s) => s,
        None => return,
    };
    let host = if let Some(ref name) = info.host_name {
        match session.hosts.iter().find(|h| &h.name == name) {
            Some(h) => h,
            None => return,
        }
    } else {
        let filtered = app.filtered_hosts();
        match filtered.get(info.host_idx) {
            Some(h) => *h,
            None => return,
        }
    };

    let mut lines = vec![];

    let bold = Style::default().add_modifier(Modifier::BOLD);

    lines.push(Line::from(vec![
        Span::styled("  Name: ", bold),
        Span::raw(&host.name),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Model: ", bold),
        Span::raw(host.model.as_deref().unwrap_or("--")),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Type: ", bold),
        Span::raw(host.host_type.as_deref().unwrap_or("--")),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Cloud: ", bold),
        Span::raw(host.cloud_name().unwrap_or("--")),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Default Cloud: ", bold),
        Span::raw(host.default_cloud_name().unwrap_or("--")),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Rack: ", bold),
        Span::raw(host.rack.as_deref().unwrap_or("--")),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Last Build: ", bold),
        Span::raw(host.last_build.as_deref().unwrap_or("--")),
    ]));

    lines.push(Line::raw(""));

    let status = if host.broken == Some(true) {
        ("Broken", Color::Red)
    } else if host.retired == Some(true) {
        ("Retired", Color::DarkGray)
    } else if host.cloud_name() == host.default_cloud_name() {
        ("Available", Color::Green)
    } else {
        ("Scheduled", Color::Yellow)
    };
    lines.push(Line::from(vec![
        Span::styled("  Status: ", bold),
        Span::styled(status.0, Style::default().fg(status.1)),
    ]));

    let flags: Vec<&str> = [
        (host.build == Some(true), "build"),
        (host.validated == Some(true), "validated"),
        (host.can_self_schedule == Some(true), "self-schedule"),
        (host.switch_config_applied == Some(true), "switch-config"),
    ]
    .iter()
    .filter(|(v, _)| *v)
    .map(|(_, l)| *l)
    .collect();

    if !flags.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Flags: ", bold),
            Span::raw(flags.join(", ")),
        ]));
    }

    lines.push(Line::raw(""));

    let gpu_count = host.processors.iter().filter(|p| p.is_gpu()).count();
    let cpu_count = host.processors.len() - gpu_count;

    let section_names = ["Interfaces", "Disks", "Memory", "Processors", "GPUs"];
    let section_counts = [
        host.interfaces.len(),
        host.disks.len(),
        host.memory.len(),
        cpu_count,
        gpu_count,
    ];

    for (si, (name, count)) in section_names.iter().zip(section_counts.iter()).enumerate() {
        let expanded = info.sections[si];
        let arrow = if expanded { "\u{25be}" } else { "\u{25b8}" };
        let is_cursor = info.cursor == si;

        let header_style = if is_cursor {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            bold
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", arrow), header_style),
            Span::styled(format!("{} ({})", name, count), header_style),
        ]));

        if expanded {
            match si {
                0 => {
                    for iface in &host.interfaces {
                        let name_str = iface.name.as_deref().unwrap_or("--");
                        let mac = iface.mac_address.as_deref().unwrap_or("--");
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("      {} ", name_str),
                                Style::default().fg(Color::Cyan),
                            ),
                            Span::raw(mac),
                        ]));
                        let switch = format!(
                            "{}:{}",
                            iface.switch_ip.as_deref().unwrap_or("--"),
                            iface.switch_port.as_deref().unwrap_or("--")
                        );
                        let speed = iface
                            .speed
                            .map(|s| format!("{}G", s))
                            .unwrap_or_else(|| "--".into());
                        let vendor = iface.vendor.as_deref().unwrap_or("--");
                        let pxe = if iface.pxe_boot == Some(true) {
                            "pxe"
                        } else {
                            ""
                        };
                        lines.push(Line::from(Span::styled(
                            format!("        sw:{} {}  {} {}", switch, speed, vendor, pxe),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }
                1 => {
                    for disk in &host.disks {
                        let dtype = disk.disk_type.as_deref().unwrap_or("--");
                        let size = disk
                            .size_gb
                            .map(|s| format!("{}GB", s))
                            .unwrap_or_else(|| "--".into());
                        let count = disk.count.map(|c| format!("x{}", c)).unwrap_or_default();
                        lines.push(Line::from(Span::styled(
                            format!("      {} {} {}", dtype, size, count),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }
                2 => {
                    for mem in &host.memory {
                        let handle = mem.handle.as_deref().unwrap_or("--");
                        let size = mem
                            .size_gb
                            .map(|s| format!("{}GB", s))
                            .unwrap_or_else(|| "--".into());
                        lines.push(Line::from(Span::styled(
                            format!("      {} {}", handle, size),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }
                3 => {
                    for proc in host.processors.iter().filter(|p| !p.is_gpu()) {
                        let vendor = proc.vendor.as_deref().unwrap_or("--");
                        let product = proc.product.as_deref().unwrap_or("--");
                        let cores = proc
                            .cores
                            .map(|c| format!("{}c", c))
                            .unwrap_or_else(|| "--".into());
                        let threads = proc
                            .threads
                            .map(|t| format!("{}t", t))
                            .unwrap_or_else(|| "--".into());
                        lines.push(Line::from(Span::styled(
                            format!("      {} {} {}/{}", vendor, product, cores, threads),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }
                4 => {
                    for gpu in host.processors.iter().filter(|p| p.is_gpu()) {
                        let vendor = gpu.vendor.as_deref().unwrap_or("--");
                        let product = gpu.product.as_deref().unwrap_or("--");
                        lines.push(Line::from(Span::styled(
                            format!("      {} {}", vendor, product),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }
                _ => {}
            }
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(" [", Style::default().fg(Color::DarkGray)),
        Span::styled("\u{2191}\u{2193}", Style::default().fg(Color::Yellow)),
        Span::styled("] navigate  [", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::styled("] expand/collapse  [", Style::default().fg(Color::DarkGray)),
        Span::styled("a", Style::default().fg(Color::Yellow)),
        Span::styled("] expand/collapse all  [", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::styled("] close", Style::default().fg(Color::DarkGray)),
    ]));

    let popup = Paragraph::new(lines)
        .block(
            Block::default()
                .title(format!(" {} ", host.name))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .scroll((info.scroll, 0));

    f.render_widget(popup, area);
}

pub fn render_host_filter_popup(f: &mut Frame, popup: &HostFilterPopup) {
    use crate::app::FilterPane;

    let area = centered_rect(40, 40, f.area());
    f.render_widget(Clear, area);

    let title = match popup.pane {
        FilterPane::Status => " Filter on Status ",
        FilterPane::SelfSchedule => " Filter on Properties ",
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[0]);

    let status_focused = popup.pane == FilterPane::Status;
    let mut status_lines = vec![];
    for (i, label) in HostFilterFlags::LABELS.iter().enumerate() {
        let checked = if popup.flags.get(i) { "x" } else { " " };
        let is_cursor = status_focused && popup.cursor == i;
        let style = if is_cursor {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if status_focused {
            Style::default()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        status_lines.push(Line::from(Span::styled(
            format!("  [{}] {}", checked, label),
            style,
        )));
    }
    let status_widget = Paragraph::new(status_lines);
    f.render_widget(status_widget, panes[0]);

    let right_focused = popup.pane == FilterPane::SelfSchedule;
    let right_items: [(&str, bool); 2] = [
        ("SSM only", popup.ssm_only),
        ("GPU only", popup.gpu_only),
    ];
    let mut right_lines = vec![];
    for (i, (label, checked)) in right_items.iter().enumerate() {
        let check = if *checked { "x" } else { " " };
        let is_cursor = right_focused && popup.right_cursor == i;
        let style = if is_cursor {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if right_focused {
            Style::default()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        right_lines.push(Line::from(Span::styled(
            format!("  [{}] {}", check, label),
            style,
        )));
    }
    let right_widget = Paragraph::new(right_lines);
    f.render_widget(right_widget, panes[1]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" [", Style::default().fg(Color::DarkGray)),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::styled("] pane  [", Style::default().fg(Color::DarkGray)),
        Span::styled("Space", Style::default().fg(Color::Yellow)),
        Span::styled("] toggle  [", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::styled("] apply  [", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::styled("] cancel", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(help, chunks[1]);
}

pub fn render_assignment_picker(f: &mut Frame, state: &crate::app::AssignmentPickerState) {
    let area = centered_rect(50, 50, f.area());
    f.render_widget(Clear, area);

    let title = Line::from(vec![
        Span::raw(" Schedule "),
        Span::styled(
            format!("{} host(s)", state.selected_hosts.len()),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let items: Vec<Line> = state
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == state.cursor;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            match item {
                crate::app::AssignmentPickerItem::Existing {
                    cloud_name,
                    description,
                    ..
                } => Line::from(vec![
                    Span::styled(
                        format!("  {} ", cloud_name),
                        style.fg(if is_selected {
                            Color::Black
                        } else {
                            Color::Cyan
                        }),
                    ),
                    Span::styled(
                        if description.is_empty() {
                            "(no description)".to_string()
                        } else {
                            description.clone()
                        },
                        style,
                    ),
                ]),
                crate::app::AssignmentPickerItem::NewAssignment => Line::from(Span::styled(
                    "  [+ New Assignment]",
                    style.fg(if is_selected {
                        Color::Black
                    } else {
                        Color::Yellow
                    }),
                )),
            }
        })
        .collect();

    let list = Paragraph::new(items);
    f.render_widget(list, chunks[0]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" select  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[1]);
}

pub fn render_new_assignment_form(f: &mut Frame, form: &crate::app::NewAssignmentForm, app: &App) {
    let area = centered_rect(50, 35, f.area());
    f.render_widget(Clear, area);

    let title = Line::from(vec![
        Span::raw(" New Assignment — "),
        Span::styled(
            format!("{} host(s)", form.selected_hosts.len()),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    let owner = app
        .sessions
        .active_session()
        .and_then(|s| s.user_email.as_deref())
        .unwrap_or("--");
    let owner_line = Paragraph::new(Line::from(vec![
        Span::styled(" Owner: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(owner, Style::default().fg(Color::Cyan)),
    ]));
    f.render_widget(owner_line, chunks[0]);

    let desc_active = form.active_field == crate::app::NewAssignmentField::Description;
    let desc_border = if desc_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let cursor_style = Style::default().add_modifier(Modifier::REVERSED);
    let desc_line = if desc_active {
        Line::from(vec![
            Span::raw(form.description.before_cursor()),
            Span::styled(form.description.char_at_cursor(), cursor_style),
            Span::raw(form.description.after_cursor_char()),
        ])
    } else {
        Line::raw(&form.description.value)
    };
    let desc_input = Paragraph::new(desc_line).block(
        Block::default()
            .title(" Description ")
            .borders(Borders::ALL)
            .border_style(desc_border),
    );
    f.render_widget(desc_input, chunks[1]);

    render_checkbox(
        f,
        chunks[2],
        "QinQ",
        form.qinq,
        form.active_field == crate::app::NewAssignmentField::Qinq,
    );
    render_checkbox(
        f,
        chunks[3],
        "Wipe",
        form.wipe,
        form.active_field == crate::app::NewAssignmentField::Wipe,
    );

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" next  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" create  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[4]);
}

pub fn render_scheduling_popup(
    f: &mut Frame,
    progress: &crate::app::SchedulingProgress,
    spinner: char,
) {
    let area = centered_rect(40, 15, f.area());
    f.render_widget(Clear, area);

    let text = format!(
        " {} Scheduling {} host(s) to {}...",
        spinner, progress.total, progress.cloud_name
    );
    let popup = Paragraph::new(text)
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Scheduling "),
        );
    f.render_widget(popup, area);
}

fn render_checkbox(f: &mut Frame, area: Rect, label: &str, checked: bool, active: bool) {
    let style = if active {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let mark = if checked { "[x]" } else { "[ ]" };
    let line = Paragraph::new(Line::from(vec![Span::styled(
        format!(" {} {}", mark, label),
        style,
    )]));
    f.render_widget(line, area);
}

pub fn render_config_help_popup(f: &mut Frame) {
    let area = centered_rect(50, 30, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Config & Logs ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(vec![
            Span::styled(
                "[C]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Open config"),
        ]),
        Line::from(vec![
            Span::styled(
                "[L]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Open log"),
        ]),
        Line::from(vec![
            Span::styled(
                "[D]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Open config/log dir"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "[Esc] or [Q] to close",
            Style::default().fg(Color::Gray),
        )),
    ];

    let content = Paragraph::new(lines).style(Style::default().fg(Color::White));
    f.render_widget(content, inner);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    let x = (r.width.saturating_sub(popup_width)) / 2;
    let y = (r.height.saturating_sub(popup_height)) / 2;
    Rect::new(r.x + x, r.y + y, popup_width, popup_height)
}
