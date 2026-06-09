mod api;
mod app;
mod config;
mod event;
mod session;
mod ui;
mod update;

use anyhow::Result;
use app::{
    ActionResult, App, AssignmentPickerItem, AssignmentPickerState, AuthForm, AuthFormField,
    ConnectError, ConnectResult, HostFilterPopup, HostInfoState, NewAssignmentForm, Popup,
    RefreshUpdate, ScheduleResult, SchedulingProgress, Screen, ServerForm, ServerFormField,
    TextInput,
};
use config::{AppConfig, ServerEntry};
use crossterm::ExecutableCommand;
use crossterm::event::{
    DisableFocusChange, EnableFocusChange, KeyCode, KeyEventKind, KeyModifiers,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use event::{Event, EventHandler};
use fs2::FileExt;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use session::Session;
use simplelog::{LevelFilter, WriteLogger};
use std::fs::File;
use std::io;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

const AUTO_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

fn get_log_path_display() -> String {
    let log_path = config::AppConfig::config_dir().join("quads-tui.log");

    // Format path for display, replacing home directory with ~ on Unix-like systems
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = dirs::home_dir()
            && let Ok(relative) = log_path.strip_prefix(&home)
        {
            return format!("~/{}", relative.display());
        }
    }

    log_path.display().to_string()
}

fn open_in_file_manager(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("quads-tui {}", update::VERSION);
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "quads-tui {} — Terminal UI for QUADS bare-metal scheduling",
            update::VERSION
        );
        println!();
        println!("USAGE:");
        println!("    quads-tui [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("    -h, --help                  Print this help message");
        println!("    -V, --version               Print version");
        println!("    --update-check=FREQUENCY    Set update check frequency");
        println!("                                (on_launch, daily, weekly, monthly, never)");
        println!();
        println!("ENVIRONMENT:");
        println!("    RUST_LOG         Set log level (default: info, e.g. RUST_LOG=debug)");
        println!();
        println!("CONFIG:");
        #[cfg(target_os = "macos")]
        println!("    ~/Library/Application Support/quads/quads-tui.toml");
        #[cfg(target_os = "linux")]
        println!("    ~/.config/quads/quads-tui.toml");
        #[cfg(target_os = "windows")]
        println!("    %APPDATA%\\quads\\quads-tui.toml");
        println!();
        println!("LOGS:");
        #[cfg(target_os = "macos")]
        println!("    ~/Library/Application Support/quads/quads-tui.log");
        #[cfg(target_os = "linux")]
        println!("    ~/.config/quads/quads-tui.log");
        #[cfg(target_os = "windows")]
        println!("    %APPDATA%\\quads\\quads-tui.log");
        return Ok(());
    }

    let config_dir = config::AppConfig::config_dir();
    std::fs::create_dir_all(&config_dir)?;

    let lock_file = File::create(config_dir.join("quads-tui.lock"))?;
    if lock_file.try_lock_exclusive().is_err() {
        eprintln!("Another instance of quads-tui is already running.");
        std::process::exit(1);
    }

    let log_level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(LevelFilter::Info);
    let log_file = File::create(config_dir.join("quads-tui.log"))?;
    WriteLogger::init(log_level, simplelog::Config::default(), log_file)?;
    log::info!("quads-tui starting (log level: {})", log_level);

    let mut config = AppConfig::load().unwrap_or_default();

    for arg in &args {
        if let Some(freq) = arg.strip_prefix("--update-check=") {
            config.update_check = match freq {
                "on_launch" => config::UpdateFrequency::OnLaunch,
                "daily" => config::UpdateFrequency::Daily,
                "weekly" => config::UpdateFrequency::Weekly,
                "monthly" => config::UpdateFrequency::Monthly,
                "never" => config::UpdateFrequency::Never,
                _ => {
                    eprintln!("Invalid --update-check value: {}", freq);
                    eprintln!("Valid options: on_launch, daily, weekly, monthly, never");
                    std::process::exit(1);
                }
            };
            let _ = config.save();
            println!("Update check set to: {}", freq);
            return Ok(());
        }
    }

    let mut app = App::new(config);
    if app.config.should_check_update() {
        app.update_rx = Some(update::spawn_update_check());
        app.config.mark_update_checked();
        let _ = app.config.save();
    }

    if let Some(ref default_name) = app.config.default_server.clone()
        && let Some(entry) = app.config.servers.get(default_name).cloned()
    {
        let server_names: Vec<String> = app.config.servers.keys().cloned().collect();
        if let Some(idx) = server_names.iter().position(|n| n == default_name) {
            app.server_selected = idx;
        }
        if let Ok(session) = Session::new(default_name, &entry) {
            app.sessions.add_session(session);
            app.sessions.switch_to(default_name);
            spawn_refresh(&mut app);
        }
    }

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    io::stdout().execute(EnableFocusChange)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut events = EventHandler::new(Duration::from_millis(250));
    let mut last_refresh = Instant::now();
    while app.running {
        terminal.draw(|f| ui::render(f, &app))?;

        match events.next().await? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if matches!(app.popup, Some(Popup::ConnectSuccess(..))) {
                    app.popup = None;
                } else if matches!(app.popup, Some(Popup::Connecting(_))) {
                    if key.code == KeyCode::Esc {
                        app.popup = None;
                        app.pending_connect = None;
                    }
                } else {
                    handle_key(&mut app, key.code, key.modifiers).await;
                }
            }
            Event::FocusGained => {
                app.focused = true;
            }
            Event::FocusLost => {
                app.focused = false;
            }
            Event::Tick => {
                app.tick = app.tick.wrapping_add(1);

                if let Some(ref mut rx) = app.pending_connect
                    && let Ok(result) = rx.try_recv()
                {
                    app.pending_connect = None;
                    handle_connect_result(&mut app, result);
                }

                if let Some(ref mut rx) = app.pending_schedule
                    && let Ok(result) = rx.try_recv()
                {
                    app.pending_schedule = None;
                    handle_schedule_result(&mut app, result);
                }

                if let Some(ref mut rx) = app.pending_action
                    && let Ok(result) = rx.try_recv()
                {
                    app.pending_action = None;
                    handle_action_result(&mut app, result);
                }

                if let Some(ref mut rx) = app.update_rx
                    && let Ok(result) = rx.try_recv()
                {
                    app.update_rx = None;
                    if let Some(info) = result {
                        log::info!("update available: v{}", info.latest_version);
                        app.update_available = Some(info);
                    }
                }

                if let Some(Popup::ConnectSuccess(_, start_tick)) = &app.popup
                    && app.tick.wrapping_sub(*start_tick) >= 4
                {
                    app.popup = None;
                }

                {
                    let mut refresh_done = false;
                    if let Some(ref mut rx) = app.refresh_rx {
                        while let Ok(update) = rx.try_recv() {
                            match update {
                                RefreshUpdate::Hosts(server_name, data) => {
                                    if let Some(s) = app.sessions.get_session_mut(&server_name) {
                                        s.hosts = data;
                                    }
                                }
                                RefreshUpdate::Clouds(server_name, data) => {
                                    if let Some(s) = app.sessions.get_session_mut(&server_name) {
                                        s.clouds = data;
                                    }
                                }
                                RefreshUpdate::CloudSummaries(server_name, data) => {
                                    if let Some(s) = app.sessions.get_session_mut(&server_name) {
                                        s.cloud_summaries = data;
                                    }
                                }
                                RefreshUpdate::Assignments(server_name, data) => {
                                    if let Some(s) = app.sessions.get_session_mut(&server_name) {
                                        s.assignments = data;
                                        recalc_my_assignments(s);
                                    }
                                }
                                RefreshUpdate::Schedules(server_name, data) => {
                                    if let Some(s) = app.sessions.get_session_mut(&server_name) {
                                        s.schedules = data;
                                    }
                                }
                                RefreshUpdate::Error(msg) => {
                                    if app.popup.is_none() {
                                        let log_path = get_log_path_display();
                                        let hint = if msg.contains("error sending request") {
                                            format!(
                                                "\n\nServer unreachable, check connectivity.\nSee {} for details.",
                                                log_path
                                            )
                                        } else {
                                            format!("\nSee {} for details.", log_path)
                                        };
                                        app.popup = Some(Popup::Error(format!(
                                            "Data refresh failed: {}{}",
                                            msg, hint
                                        )));
                                    }
                                }
                                RefreshUpdate::Done => {
                                    refresh_done = true;
                                }
                            }
                        }
                    }
                    if refresh_done {
                        app.refresh_rx = None;
                        app.loading = false;
                    }
                }

                if app.auto_refresh
                    && app.focused
                    && app.screen == Screen::Dashboard
                    && app.popup.is_none()
                    && app.refresh_rx.is_none()
                    && last_refresh.elapsed() >= AUTO_REFRESH_INTERVAL
                {
                    spawn_refresh(&mut app);
                    last_refresh = Instant::now();
                }
            }
            _ => {}
        }
    }

    io::stdout().execute(DisableFocusChange)?;
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}

async fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        app.running = false;
        return;
    }

    if let Some(ref popup) = app.popup {
        match popup {
            Popup::ConfirmTerminate(id) => {
                let id = *id;
                match code {
                    KeyCode::Enter => {
                        spawn_terminate(app, id);
                    }
                    KeyCode::Esc => {
                        app.popup = None;
                    }
                    _ => {}
                }
                return;
            }
            Popup::ConfirmUnschedule { schedule_id, .. } => {
                let sid = *schedule_id;
                match code {
                    KeyCode::Enter => {
                        spawn_unschedule(app, sid);
                    }
                    KeyCode::Esc => {
                        app.popup = None;
                    }
                    _ => {}
                }
                return;
            }
            Popup::Error(_) => {
                app.popup = None;
                return;
            }
            Popup::UpdateComplete(_) => {
                app.running = false;
                return;
            }
            Popup::ServerForm(_) => {
                handle_server_form_key(app, code, modifiers);
                return;
            }
            Popup::AuthForm(_) => {
                handle_auth_form_key(app, code, modifiers);
                return;
            }
            Popup::HostInfo(_) => {
                handle_host_info_key(app, code);
                return;
            }
            Popup::HostFilter(_) => {
                handle_host_filter_key(app, code);
                return;
            }
            Popup::AssignmentPicker(_) => {
                handle_assignment_picker_key(app, code);
                return;
            }
            Popup::NewAssignmentForm(_) => {
                handle_new_assignment_form_key(app, code, modifiers);
                return;
            }
            Popup::Scheduling(_) | Popup::Working(_) => {
                return;
            }
            Popup::ConfigHelp => {
                match code {
                    KeyCode::Char('c' | 'C') => {
                        let config_dir = config::AppConfig::config_dir();
                        let config_file = config_dir.join("quads-tui.toml");
                        open_in_file_manager(&config_file);
                        app.popup = None;
                    }
                    KeyCode::Char('l' | 'L') => {
                        let config_dir = config::AppConfig::config_dir();
                        let log_file = config_dir.join("quads-tui.log");
                        open_in_file_manager(&log_file);
                        app.popup = None;
                    }
                    KeyCode::Char('d' | 'D') => {
                        let config_dir = config::AppConfig::config_dir();
                        open_in_file_manager(&config_dir);
                        app.popup = None;
                    }
                    KeyCode::Esc | KeyCode::Char('q' | 'Q') => {
                        app.popup = None;
                    }
                    _ => {}
                }
                return;
            }
            _ => {
                if code == KeyCode::Esc {
                    app.popup = None;
                    return;
                }
            }
        }
    }

    if matches!(code, KeyCode::Char('U' | 'u'))
        && let Some(ref info) = app.update_available
    {
        let url = info.download_url.clone();
        let ver = info.latest_version.clone();
        app.popup = Some(Popup::Working(format!("Updating to v{}...", ver)));
        app.pending_action = Some(update::spawn_self_update(url, ver));
        return;
    }

    match app.screen {
        Screen::Dashboard => handle_dashboard_key(app, code),
        Screen::Hosts => handle_hosts_key(app, code),
        Screen::Assignments => handle_assignments_key(app, code),
        Screen::Clouds => handle_clouds_key(app, code),
    }
}

fn handle_dashboard_key(app: &mut App, code: KeyCode) {
    let server_count = app.config.servers.len();

    match code {
        KeyCode::Char('q' | 'Q') => app.running = false,
        KeyCode::Char('h' | 'H') => app.navigate(Screen::Hosts),
        KeyCode::Char('a' | 'A') => app.navigate(Screen::Assignments),
        KeyCode::Char('c' | 'C') => app.navigate(Screen::Clouds),
        KeyCode::Char('r' | 'R') => spawn_refresh(app),
        KeyCode::Char('x' | 'X') => {
            app.auto_refresh = !app.auto_refresh;
        }
        KeyCode::Up | KeyCode::Char('k' | 'K') if app.server_selected > 0 => {
            app.server_selected -= 1;
        }
        KeyCode::Down | KeyCode::Char('j' | 'J')
            if server_count > 0 && app.server_selected < server_count - 1 =>
        {
            app.server_selected += 1;
        }
        KeyCode::Enter => {
            let server_names: Vec<String> = app.config.servers.keys().cloned().collect();
            if let Some(name) = server_names.get(app.server_selected) {
                let name = name.clone();
                let is_active = app
                    .sessions
                    .active_session()
                    .map(|s| s.name == name)
                    .unwrap_or(false);

                if is_active {
                    let is_connected = app
                        .sessions
                        .active_session()
                        .map(|s| s.connected)
                        .unwrap_or(false);
                    if is_connected {
                        disconnect_current(app);
                    } else {
                        connect_selected_server(app);
                    }
                } else {
                    let entry = app.config.servers[&name].clone();

                    app.config.default_server = Some(name.clone());
                    let _ = app.config.save();

                    if app.sessions.sessions.contains_key(&name) {
                        app.sessions.switch_to(&name);
                    } else if let Ok(session) = Session::new(&name, &entry) {
                        app.sessions.add_session(session);
                        app.sessions.switch_to(&name);
                        spawn_refresh(app);
                    }
                    app.status_message = Some(format!("Selected {}", name));
                }
            }
        }
        KeyCode::Char('n' | 'N') => {
            app.popup = Some(Popup::ServerForm(ServerForm::new()));
        }
        KeyCode::Char('e' | 'E') => {
            let server_names: Vec<String> = app.config.servers.keys().cloned().collect();
            if let Some(name) = server_names.get(app.server_selected) {
                let entry = &app.config.servers[name];
                let mut form = ServerForm::new();
                form.name = TextInput::from(name.clone());
                form.url = TextInput::from(entry.url.clone());
                form.verify_ssl = entry.verify_ssl;
                form.editing_existing = Some(name.clone());
                app.popup = Some(Popup::ServerForm(form));
            }
        }
        KeyCode::Char('?') => {
            app.popup = Some(Popup::ConfigHelp);
        }
        KeyCode::Right => app.navigate(app.screen.next()),
        KeyCode::Left => app.navigate(app.screen.prev()),
        _ => {}
    }
}

fn handle_hosts_key(app: &mut App, code: KeyCode) {
    if app.host_searching {
        match code {
            KeyCode::Esc => {
                app.host_searching = false;
                app.host_search = None;
                app.host_selected = 0;
            }
            KeyCode::Enter => {
                app.host_searching = false;
            }
            KeyCode::Backspace => {
                if let Some(ref mut s) = app.host_search {
                    s.pop();
                    if s.is_empty() {
                        app.host_search = None;
                    }
                }
                app.host_selected = 0;
            }
            KeyCode::Char(c) => {
                app.host_search.get_or_insert_with(String::new).push(c);
                app.host_selected = 0;
            }
            KeyCode::Up | KeyCode::Down => {
                let host_count = app.filtered_hosts().len();
                match code {
                    KeyCode::Up if app.host_selected > 0 => {
                        app.host_selected -= 1;
                    }
                    KeyCode::Down if host_count > 0 && app.host_selected < host_count - 1 => {
                        app.host_selected += 1;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        return;
    }

    let host_count = app.filtered_hosts().len();

    match code {
        KeyCode::Esc => {
            if !app.host_multi_select.is_empty() {
                app.host_multi_select.clear();
            } else if app.host_search.is_some() {
                app.host_search = None;
                app.host_selected = 0;
            } else {
                app.go_back();
            }
        }
        KeyCode::Char('q' | 'Q') => app.running = false,
        KeyCode::Up | KeyCode::Char('k' | 'K') if app.host_selected > 0 => {
            app.host_selected -= 1;
        }
        KeyCode::Down | KeyCode::Char('j' | 'J')
            if host_count > 0 && app.host_selected < host_count - 1 =>
        {
            app.host_selected += 1;
        }
        KeyCode::PageUp => {
            app.host_selected = app.host_selected.saturating_sub(20);
        }
        KeyCode::PageDown if host_count > 0 => {
            app.host_selected = (app.host_selected + 20).min(host_count - 1);
        }
        KeyCode::Home => {
            app.host_selected = 0;
        }
        KeyCode::End if host_count > 0 => {
            app.host_selected = host_count - 1;
        }
        KeyCode::Char('/') => {
            app.host_searching = true;
            app.host_search = None;
            app.host_selected = 0;
        }
        KeyCode::Tab => {
            app.host_self_schedule_only = !app.host_self_schedule_only;
            app.host_selected = 0;
        }
        KeyCode::Char('f' | 'F') if !app.host_self_schedule_only => {
            app.popup = Some(Popup::HostFilter(HostFilterPopup {
                cursor: 0,
                flags: app.host_filters.clone(),
                pane: app::FilterPane::Status,
                ssm_only: app.host_ssm_filter,
            }));
        }
        KeyCode::Enter => {
            app.popup = Some(Popup::HostInfo(HostInfoState::new(app.host_selected)));
        }
        KeyCode::Char(' ') => {
            let hosts = app.filtered_hosts();
            let host_count = hosts.len();
            if let Some(host) = hosts.get(app.host_selected) {
                if host.can_self_schedule == Some(true) {
                    let name = host.name.clone();
                    if app.host_multi_select.contains(&name) {
                        app.host_multi_select.remove(&name);
                    } else {
                        app.host_multi_select.insert(name);
                    }
                    if host_count > 0 && app.host_selected < host_count - 1 {
                        app.host_selected += 1;
                    }
                } else {
                    app.status_message = Some("Host is not self-schedulable".into());
                }
            }
        }
        KeyCode::Char('s' | 'S') => {
            open_assignment_picker(app);
        }
        KeyCode::Char('r' | 'R') => spawn_refresh(app),
        KeyCode::Char('x' | 'X') => {
            app.auto_refresh = !app.auto_refresh;
        }
        KeyCode::Char('d' | 'D') => app.navigate(Screen::Dashboard),
        KeyCode::Char('a' | 'A') => app.navigate(Screen::Assignments),
        KeyCode::Char('c' | 'C') => app.navigate(Screen::Clouds),
        KeyCode::Right => app.navigate(app.screen.next()),
        KeyCode::Left => app.navigate(app.screen.prev()),
        _ => {}
    }
}

fn filtered_assignment_count(app: &App) -> usize {
    app.filtered_sorted_assignments().len()
}

fn get_selected_assignment(app: &App) -> Option<&crate::api::models::Assignment> {
    app.filtered_sorted_assignments()
        .get(app.assignment_selected)
        .copied()
}

fn assignment_schedule_count(app: &App) -> usize {
    let Some(session) = app.sessions.active_session() else {
        return 0;
    };
    match get_selected_assignment(app) {
        Some(assignment) => session
            .schedules
            .iter()
            .filter(|s| s.assignment_id == assignment.id)
            .count(),
        None => 0,
    }
}

fn handle_assignments_key(app: &mut App, code: KeyCode) {
    if app.assignment_searching {
        match code {
            KeyCode::Esc => {
                app.assignment_searching = false;
                app.assignment_search = None;
                app.assignment_selected = 0;
            }
            KeyCode::Enter => {
                app.assignment_searching = false;
            }
            KeyCode::Backspace => {
                if let Some(ref mut s) = app.assignment_search {
                    s.pop();
                    if s.is_empty() {
                        app.assignment_search = None;
                    }
                }
                app.assignment_selected = 0;
            }
            KeyCode::Char(c) => {
                app.assignment_search
                    .get_or_insert_with(String::new)
                    .push(c);
                app.assignment_selected = 0;
            }
            KeyCode::Up | KeyCode::Down => {
                let count = filtered_assignment_count(app);
                match code {
                    KeyCode::Up if app.assignment_selected > 0 => {
                        app.assignment_selected -= 1;
                    }
                    KeyCode::Down if count > 0 && app.assignment_selected < count - 1 => {
                        app.assignment_selected += 1;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        return;
    }

    let assignment_count = filtered_assignment_count(app);

    if let Some(detail_idx) = app.assignment_detail_selected {
        let schedule_count = assignment_schedule_count(app);
        match code {
            KeyCode::Esc => {
                app.assignment_detail_selected = None;
            }
            KeyCode::Up | KeyCode::Char('k' | 'K') if detail_idx > 0 => {
                app.assignment_detail_selected = Some(detail_idx - 1);
            }
            KeyCode::Down | KeyCode::Char('j' | 'J')
                if schedule_count > 0 && detail_idx < schedule_count - 1 =>
            {
                app.assignment_detail_selected = Some(detail_idx + 1);
            }
            KeyCode::Enter => {
                if let Some(session) = app.sessions.active_session() {
                    let assignment = get_selected_assignment(app);
                    if let Some(assignment) = assignment {
                        let scheds: Vec<_> = session
                            .schedules
                            .iter()
                            .filter(|s| s.assignment_id == assignment.id)
                            .collect();
                        if let Some(sched) = scheds.get(detail_idx) {
                            let host = sched.host_name().to_string();
                            app.popup = Some(Popup::HostInfo(HostInfoState::from_name(host)));
                        }
                    }
                }
            }
            KeyCode::Char('u' | 'U') => {
                if let Some(session) = app.sessions.active_session() {
                    let assignment = get_selected_assignment(app);
                    if let Some(assignment) = assignment {
                        let scheds: Vec<_> = session
                            .schedules
                            .iter()
                            .filter(|s| s.assignment_id == assignment.id)
                            .collect();
                        if let Some(sched) = scheds.get(detail_idx)
                            && let Some(sid) = sched.id
                        {
                            let host = sched.host_name().to_string();
                            app.popup = Some(Popup::ConfirmUnschedule {
                                schedule_id: sid,
                                host_name: host,
                            });
                        }
                    }
                }
            }
            KeyCode::Char('q' | 'Q') => app.running = false,
            KeyCode::Char('d' | 'D') => app.navigate(Screen::Dashboard),
            KeyCode::Char('h' | 'H') => app.navigate(Screen::Hosts),
            KeyCode::Char('c' | 'C') => app.navigate(Screen::Clouds),
            KeyCode::Right => app.navigate(app.screen.next()),
            KeyCode::Left => app.navigate(app.screen.prev()),
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Esc => {
            if app.assignment_search.is_some() {
                app.assignment_search = None;
                app.assignment_selected = 0;
            } else {
                app.go_back();
            }
        }
        KeyCode::Char('q' | 'Q') => app.running = false,
        KeyCode::Up | KeyCode::Char('k' | 'K') if app.assignment_selected > 0 => {
            app.assignment_selected -= 1;
            app.assignment_detail_selected = None;
        }
        KeyCode::Down | KeyCode::Char('j' | 'J')
            if assignment_count > 0 && app.assignment_selected < assignment_count - 1 =>
        {
            app.assignment_selected += 1;
            app.assignment_detail_selected = None;
        }
        KeyCode::Enter => {
            let count = assignment_schedule_count(app);
            if count > 0 {
                app.assignment_detail_selected = Some(0);
            }
        }
        KeyCode::PageUp => {
            app.assignment_selected = app.assignment_selected.saturating_sub(20);
        }
        KeyCode::PageDown if assignment_count > 0 => {
            app.assignment_selected = (app.assignment_selected + 20).min(assignment_count - 1);
        }
        KeyCode::Home => {
            app.assignment_selected = 0;
        }
        KeyCode::End if assignment_count > 0 => {
            app.assignment_selected = assignment_count - 1;
        }
        KeyCode::Tab => {
            app.assignment_show_all = !app.assignment_show_all;
            app.assignment_selected = 0;
            app.assignment_detail_selected = None;
        }
        KeyCode::Char('/') => {
            app.assignment_searching = true;
            app.assignment_search = None;
            app.assignment_selected = 0;
            app.assignment_detail_selected = None;
        }
        KeyCode::Char('t' | 'T') => {
            if let Some(session) = app.sessions.active_session() {
                let assignments = if app.assignment_show_all {
                    &session.assignments
                } else {
                    &session.my_assignments
                };
                if let Some(a) = assignments.get(app.assignment_selected)
                    && let Some(id) = a.id
                {
                    app.popup = Some(Popup::ConfirmTerminate(id));
                }
            }
        }
        KeyCode::Char('r' | 'R') => spawn_refresh(app),
        KeyCode::Char('x' | 'X') => {
            app.auto_refresh = !app.auto_refresh;
        }
        KeyCode::Char('d' | 'D') => app.navigate(Screen::Dashboard),
        KeyCode::Char('h' | 'H') => app.navigate(Screen::Hosts),
        KeyCode::Char('c' | 'C') => app.navigate(Screen::Clouds),
        KeyCode::Right => app.navigate(app.screen.next()),
        KeyCode::Left => app.navigate(app.screen.prev()),
        _ => {}
    }
}

fn filtered_cloud_count(app: &App) -> usize {
    let Some(session) = app.sessions.active_session() else {
        return 0;
    };
    let my_username = session.username();

    session
        .cloud_summaries
        .iter()
        .filter(|c| {
            if !app.cloud_show_all {
                match (&c.owner, &my_username) {
                    (Some(owner), Some(username)) => {
                        if owner != username {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
            if let Some(ref search) = app.cloud_search
                && !app::fuzzy_match(&c.name, search)
                && !app::fuzzy_match(c.owner.as_deref().unwrap_or(""), search)
                && !app::fuzzy_match(c.ticket.as_deref().unwrap_or(""), search)
                && !app::fuzzy_match(c.description.as_deref().unwrap_or(""), search)
            {
                return false;
            }
            true
        })
        .count()
}

fn handle_clouds_key(app: &mut App, code: KeyCode) {
    if app.cloud_searching {
        match code {
            KeyCode::Esc => {
                app.cloud_searching = false;
                app.cloud_search = None;
                app.cloud_selected = 0;
            }
            KeyCode::Enter => {
                app.cloud_searching = false;
            }
            KeyCode::Backspace => {
                if let Some(ref mut s) = app.cloud_search {
                    s.pop();
                    if s.is_empty() {
                        app.cloud_search = None;
                    }
                }
                app.cloud_selected = 0;
            }
            KeyCode::Char(c) => {
                app.cloud_search.get_or_insert_with(String::new).push(c);
                app.cloud_selected = 0;
            }
            KeyCode::Up | KeyCode::Down => {
                let cloud_count = filtered_cloud_count(app);
                match code {
                    KeyCode::Up if app.cloud_selected > 0 => {
                        app.cloud_selected -= 1;
                    }
                    KeyCode::Down if cloud_count > 0 && app.cloud_selected < cloud_count - 1 => {
                        app.cloud_selected += 1;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        return;
    }

    let cloud_count = filtered_cloud_count(app);

    match code {
        KeyCode::Esc => {
            if app.cloud_search.is_some() {
                app.cloud_search = None;
                app.cloud_selected = 0;
            } else {
                app.go_back();
            }
        }
        KeyCode::Char('q' | 'Q') => app.running = false,
        KeyCode::Up | KeyCode::Char('k' | 'K') if app.cloud_selected > 0 => {
            app.cloud_selected -= 1;
        }
        KeyCode::Down | KeyCode::Char('j' | 'J')
            if cloud_count > 0 && app.cloud_selected < cloud_count - 1 =>
        {
            app.cloud_selected += 1;
        }
        KeyCode::PageUp => {
            app.cloud_selected = app.cloud_selected.saturating_sub(20);
        }
        KeyCode::PageDown if cloud_count > 0 => {
            app.cloud_selected = (app.cloud_selected + 20).min(cloud_count - 1);
        }
        KeyCode::Home => {
            app.cloud_selected = 0;
        }
        KeyCode::End if cloud_count > 0 => {
            app.cloud_selected = cloud_count - 1;
        }
        KeyCode::Tab => {
            app.cloud_show_all = !app.cloud_show_all;
            app.cloud_selected = 0;
        }
        KeyCode::Char('/') => {
            app.cloud_searching = true;
            app.cloud_search = None;
            app.cloud_selected = 0;
        }
        KeyCode::Char('r' | 'R') => spawn_refresh(app),
        KeyCode::Char('x' | 'X') => {
            app.auto_refresh = !app.auto_refresh;
        }
        KeyCode::Char('d' | 'D') => app.navigate(Screen::Dashboard),
        KeyCode::Char('h' | 'H') => app.navigate(Screen::Hosts),
        KeyCode::Char('a' | 'A') => app.navigate(Screen::Assignments),
        KeyCode::Right => app.navigate(app.screen.next()),
        KeyCode::Left => app.navigate(app.screen.prev()),
        _ => {}
    }
}

fn handle_host_filter_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.popup = None;
        }
        KeyCode::Tab => {
            if let Some(Popup::HostFilter(ref mut popup)) = app.popup {
                popup.pane = match popup.pane {
                    app::FilterPane::Status => app::FilterPane::SelfSchedule,
                    app::FilterPane::SelfSchedule => app::FilterPane::Status,
                };
            }
        }
        KeyCode::Up | KeyCode::Char('k' | 'K') => {
            if let Some(Popup::HostFilter(ref mut popup)) = app.popup
                && popup.pane == app::FilterPane::Status
                && popup.cursor > 0
            {
                popup.cursor -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j' | 'J') => {
            if let Some(Popup::HostFilter(ref mut popup)) = app.popup
                && popup.pane == app::FilterPane::Status
                && popup.cursor < 3
            {
                popup.cursor += 1;
            }
        }
        KeyCode::Char(' ') => {
            if let Some(Popup::HostFilter(ref mut popup)) = app.popup {
                match popup.pane {
                    app::FilterPane::Status => popup.flags.toggle(popup.cursor),
                    app::FilterPane::SelfSchedule => popup.ssm_only = !popup.ssm_only,
                }
            }
        }
        KeyCode::Enter => {
            if let Some(Popup::HostFilter(popup)) = app.popup.take() {
                app.host_filters = popup.flags;
                app.host_ssm_filter = popup.ssm_only;
                app.host_selected = 0;
            }
        }
        _ => {}
    }
}

fn handle_host_info_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.popup = None;
        }
        KeyCode::Up | KeyCode::Char('k' | 'K') => {
            if let Some(Popup::HostInfo(ref mut state)) = app.popup
                && state.cursor > 0
            {
                state.cursor -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j' | 'J') => {
            if let Some(Popup::HostInfo(ref mut state)) = app.popup
                && state.cursor < 3
            {
                state.cursor += 1;
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if let Some(Popup::HostInfo(ref mut state)) = app.popup {
                state.sections[state.cursor] = !state.sections[state.cursor];
            }
        }
        KeyCode::PageUp => {
            if let Some(Popup::HostInfo(ref mut state)) = app.popup {
                state.scroll = state.scroll.saturating_sub(10);
            }
        }
        KeyCode::PageDown => {
            if let Some(Popup::HostInfo(ref mut state)) = app.popup {
                state.scroll = state.scroll.saturating_add(10);
            }
        }
        KeyCode::Home => {
            if let Some(Popup::HostInfo(ref mut state)) = app.popup {
                state.scroll = 0;
            }
        }
        _ => {}
    }
}

fn handle_server_form_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Esc => {
            app.popup = None;
        }
        KeyCode::Tab | KeyCode::Down => {
            if let Some(Popup::ServerForm(ref mut form)) = app.popup {
                form.active_field = form.active_field.next();
            }
        }
        KeyCode::BackTab | KeyCode::Up => {
            if let Some(Popup::ServerForm(ref mut form)) = app.popup {
                form.active_field = form.active_field.prev();
            }
        }
        KeyCode::Char(' ') if matches!(&app.popup, Some(Popup::ServerForm(f)) if f.active_field == ServerFormField::VerifySsl) => {
            if let Some(Popup::ServerForm(ref mut form)) = app.popup {
                form.verify_ssl = !form.verify_ssl;
            }
        }
        KeyCode::Left => {
            if let Some(Popup::ServerForm(ref mut form)) = app.popup
                && let Some(input) = form.active_input_mut()
            {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    input.move_word_left();
                } else {
                    input.move_left();
                }
            }
        }
        KeyCode::Right => {
            if let Some(Popup::ServerForm(ref mut form)) = app.popup
                && let Some(input) = form.active_input_mut()
            {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    input.move_word_right();
                } else {
                    input.move_right();
                }
            }
        }
        KeyCode::Home => {
            if let Some(Popup::ServerForm(ref mut form)) = app.popup
                && let Some(input) = form.active_input_mut()
            {
                input.move_home();
            }
        }
        KeyCode::End => {
            if let Some(Popup::ServerForm(ref mut form)) = app.popup
                && let Some(input) = form.active_input_mut()
            {
                input.move_end();
            }
        }
        KeyCode::Delete => {
            if let Some(Popup::ServerForm(ref mut form)) = app.popup
                && let Some(input) = form.active_input_mut()
            {
                input.delete();
            }
        }
        KeyCode::Backspace => {
            if let Some(Popup::ServerForm(ref mut form)) = app.popup
                && let Some(input) = form.active_input_mut()
            {
                input.backspace();
            }
        }
        KeyCode::Char(c) => {
            if let Some(Popup::ServerForm(ref mut form)) = app.popup
                && let Some(input) = form.active_input_mut()
            {
                input.insert(c);
            }
        }
        KeyCode::Enter => {
            if let Some(Popup::ServerForm(form)) = app.popup.take() {
                if form.name.is_empty() || form.url.is_empty() {
                    app.popup = Some(Popup::Error("Name and URL are required".into()));
                    return;
                }

                if let Some(ref old_name) = form.editing_existing
                    && *old_name != form.name.value
                {
                    app.config.remove_server(old_name);
                }

                let existing = app.config.servers.get(&form.name.value);
                app.config.add_server(
                    form.name.value,
                    ServerEntry {
                        url: form.url.value,
                        username: existing.and_then(|e| e.username.clone()),
                        password: existing.and_then(|e| e.password.clone()),
                        verify_ssl: form.verify_ssl,
                    },
                );

                if let Err(e) = app.config.save() {
                    app.popup = Some(Popup::Error(format!("Failed to save config: {}", e)));
                    return;
                }

                if app.sessions.active_session().is_none() {
                    app.server_selected = app.config.servers.len() - 1;
                    connect_selected_server(app);
                }
            }
        }
        _ => {}
    }
}

fn handle_auth_form_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    let is_register_prompt = matches!(&app.popup, Some(Popup::AuthForm(f)) if f.register_prompt);

    if is_register_prompt {
        match code {
            KeyCode::Enter => {
                if let Some(Popup::AuthForm(form)) = app.popup.take() {
                    spawn_register(app, form);
                }
            }
            KeyCode::Esc => {
                app.popup = None;
            }
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Esc => {
            app.popup = None;
        }
        KeyCode::Tab | KeyCode::Down => {
            if let Some(Popup::AuthForm(ref mut form)) = app.popup {
                form.active_field = form.active_field.next();
            }
        }
        KeyCode::BackTab | KeyCode::Up => {
            if let Some(Popup::AuthForm(ref mut form)) = app.popup {
                form.active_field = form.active_field.prev();
            }
        }
        KeyCode::Left => {
            if let Some(Popup::AuthForm(ref mut form)) = app.popup {
                let input = form.active_input_mut();
                if modifiers.contains(KeyModifiers::CONTROL) {
                    input.move_word_left();
                } else {
                    input.move_left();
                }
            }
        }
        KeyCode::Right => {
            if let Some(Popup::AuthForm(ref mut form)) = app.popup {
                let input = form.active_input_mut();
                if modifiers.contains(KeyModifiers::CONTROL) {
                    input.move_word_right();
                } else {
                    input.move_right();
                }
            }
        }
        KeyCode::Home => {
            if let Some(Popup::AuthForm(ref mut form)) = app.popup {
                form.active_input_mut().move_home();
            }
        }
        KeyCode::End => {
            if let Some(Popup::AuthForm(ref mut form)) = app.popup {
                form.active_input_mut().move_end();
            }
        }
        KeyCode::Delete => {
            if let Some(Popup::AuthForm(ref mut form)) = app.popup {
                form.active_input_mut().delete();
            }
        }
        KeyCode::Backspace => {
            if let Some(Popup::AuthForm(ref mut form)) = app.popup {
                form.active_input_mut().backspace();
            }
        }
        KeyCode::Char(c) => {
            if let Some(Popup::AuthForm(ref mut form)) = app.popup {
                form.active_input_mut().insert(c);
            }
        }
        KeyCode::Enter => {
            if let Some(Popup::AuthForm(ref mut form)) = app.popup
                && !form.username.is_empty()
                && form.password.is_empty()
            {
                form.active_field = AuthFormField::Password;
                return;
            }
            if let Some(Popup::AuthForm(form)) = app.popup.take() {
                if form.username.is_empty() {
                    app.popup = Some(Popup::Error("Username is required".into()));
                    return;
                }
                spawn_connect(app, form);
            }
        }
        _ => {}
    }
}

fn connect_selected_server(app: &mut App) {
    let server_names: Vec<String> = app.config.servers.keys().cloned().collect();
    let Some(name) = server_names.get(app.server_selected) else {
        return;
    };

    if let Some(session) = app.sessions.get_session(name)
        && session.connected
    {
        app.sessions.switch_to(name);
        app.status_message = Some(format!("Switched to {}", name));
        return;
    }

    let entry = app.config.servers[name].clone();

    app.config.default_server = Some(name.clone());
    let _ = app.config.save();

    if !app.sessions.sessions.contains_key(name) {
        if let Ok(session) = Session::new(name, &entry) {
            app.sessions.add_session(session);
            app.sessions.switch_to(name);
            spawn_refresh(app);
        }
    } else {
        app.sessions.switch_to(name);
    }

    match (&entry.username, &entry.password) {
        (Some(u), Some(p)) => {
            let form = AuthForm::with_credentials(name.clone(), u.clone(), p.clone());
            spawn_connect(app, form);
        }
        (Some(u), None) => {
            let mut f = AuthForm::new(name.clone());
            f.username = TextInput::from(u.clone());
            app.popup = Some(Popup::AuthForm(f));
        }
        _ => {
            app.popup = Some(Popup::AuthForm(AuthForm::new(name.clone())));
        }
    }
}

fn spawn_connect(app: &mut App, form: AuthForm) {
    let server_name = form.server_name.clone();
    let username = form.username.value.clone();
    let password = form.password.value.clone();

    let Some(entry) = app.config.servers.get(&server_name).cloned() else {
        app.set_error(format!("Server '{}' not found in config", server_name));
        return;
    };

    let mut session = match Session::new(&server_name, &entry) {
        Ok(s) => s,
        Err(e) => {
            app.set_error(format!("Failed to create session: {}", e));
            return;
        }
    };

    app.popup = Some(Popup::Connecting(server_name.clone()));

    let (tx, rx) = oneshot::channel();
    app.pending_connect = Some(rx);

    let u = username.clone();
    let p = password.clone();
    let sn = server_name.clone();

    tokio::spawn(async move {
        match session.connect(&u, &p).await {
            Ok(()) => {
                let _ = tx.send(ConnectResult {
                    server_name: sn,
                    username: u,
                    password: p,
                    result: Ok(session),
                });
            }
            Err(login_err) => {
                let err_str = login_err.to_string();
                let is_network_error = err_str.contains("error sending request");
                let message = if is_network_error {
                    format!(
                        "Login failed: {}\n\nServer unreachable, check connectivity.",
                        login_err
                    )
                } else {
                    format!("Login failed: {}", login_err)
                };
                let _ = tx.send(ConnectResult {
                    server_name: sn,
                    username: u,
                    password: p,
                    result: Err(ConnectError {
                        message,
                        is_credential_error: !is_network_error,
                    }),
                });
            }
        }
    });
}

fn spawn_register(app: &mut App, form: AuthForm) {
    let server_name = form.server_name.clone();
    let username = form.username.value.clone();
    let password = form.password.value.clone();

    let Some(entry) = app.config.servers.get(&server_name).cloned() else {
        app.set_error(format!("Server '{}' not found in config", server_name));
        return;
    };

    let mut session = match Session::new(&server_name, &entry) {
        Ok(s) => s,
        Err(e) => {
            app.set_error(format!("Failed to create session: {}", e));
            return;
        }
    };

    app.popup = Some(Popup::Connecting(server_name.clone()));

    let (tx, rx) = oneshot::channel();
    app.pending_connect = Some(rx);

    let u = username.clone();
    let p = password.clone();
    let sn = server_name.clone();

    tokio::spawn(async move {
        match session.register_and_login(&u, &p).await {
            Ok(()) => {
                let _ = tx.send(ConnectResult {
                    server_name: sn,
                    username: u,
                    password: p,
                    result: Ok(session),
                });
            }
            Err(e) => {
                let err_msg = e.to_string().to_lowercase();
                let is_network_error = err_msg.contains("error sending request");
                let user_exists = err_msg.contains("already exist")
                    || err_msg.contains("conflict")
                    || err_msg.contains("duplicate");
                let message = if is_network_error {
                    format!(
                        "Registration failed: {}\n\nServer unreachable, check connectivity.",
                        e
                    )
                } else if user_exists {
                    format!(
                        "Registration failed: {}\nContact your QUADS admin to reset your password.",
                        e
                    )
                } else {
                    format!("Registration failed: {}", e)
                };
                let _ = tx.send(ConnectResult {
                    server_name: sn,
                    username: u,
                    password: p,
                    result: Err(ConnectError {
                        message,
                        is_credential_error: false,
                    }),
                });
            }
        }
    });
}

fn handle_connect_result(app: &mut App, cr: ConnectResult) {
    match cr.result {
        Ok(session) => {
            let server_name = cr.server_name.clone();
            if let Some(entry) = app.config.servers.get(&server_name).cloned() {
                let creds_changed = entry.username.as_deref() != Some(&cr.username)
                    || entry.password.as_deref() != Some(&cr.password);
                if creds_changed {
                    app.config.add_server(
                        server_name.clone(),
                        ServerEntry {
                            url: entry.url,
                            username: Some(cr.username),
                            password: Some(cr.password),
                            verify_ssl: entry.verify_ssl,
                        },
                    );
                    let _ = app.config.save();
                }
            }

            let had_data = app
                .sessions
                .get_session(&server_name)
                .map(|s| !s.hosts.is_empty())
                .unwrap_or(false);

            if let Some(old) = app.sessions.get_session_mut(&server_name) {
                old.connected = session.connected;
                old.user_email = session.user_email.clone();
                old.is_admin = session.is_admin;
                old.version = session.version.clone();
                old.client = session.client.clone();
            } else {
                app.sessions.add_session(session);
            }
            app.sessions.switch_to(&server_name);
            app.popup = Some(Popup::ConnectSuccess(server_name.clone(), app.tick));
            app.status_message = Some(format!("Connected to {}", server_name));

            if let Some(session) = app.sessions.get_session_mut(&server_name) {
                recalc_my_assignments(session);
            }
            if !had_data {
                spawn_refresh(app);
            }
        }
        Err(err) => {
            if err.is_credential_error {
                let mut form = AuthForm::with_credentials(cr.server_name, cr.username, cr.password);
                form.error = Some(err.message);
                form.register_prompt = true;
                app.popup = Some(Popup::AuthForm(form));
            } else {
                let mut retry_form = AuthForm::new(cr.server_name);
                retry_form.username = TextInput::from(cr.username);
                retry_form.error = Some(err.message);
                app.popup = Some(Popup::AuthForm(retry_form));
            }
        }
    }
}

fn open_assignment_picker(app: &mut App) {
    let connected = app
        .sessions
        .active_session()
        .map(|s| s.connected)
        .unwrap_or(false);
    if !connected {
        app.set_error("Must be logged in to schedule hosts".into());
        return;
    }

    let hosts = app.filtered_hosts();
    let selected_hosts: Vec<String> = if app.host_multi_select.is_empty() {
        if let Some(host) = hosts.get(app.host_selected) {
            if host.can_self_schedule == Some(true) {
                vec![host.name.clone()]
            } else {
                app.set_error("Host is not self-schedulable".into());
                return;
            }
        } else {
            return;
        }
    } else {
        app.host_multi_select.iter().cloned().collect()
    };

    let session = app.sessions.active_session().unwrap();
    let mut items: Vec<AssignmentPickerItem> = session
        .my_assignments
        .iter()
        .filter(|a| a.is_self_schedule == Some(true) && a.active == Some(true))
        .map(|a| AssignmentPickerItem::Existing {
            cloud_name: a.cloud_name().unwrap_or("--").to_string(),
            description: a.description.clone().unwrap_or_default(),
        })
        .collect();
    items.push(AssignmentPickerItem::NewAssignment);

    app.popup = Some(Popup::AssignmentPicker(AssignmentPickerState {
        selected_hosts,
        cursor: 0,
        items,
    }));
}

fn handle_assignment_picker_key(app: &mut App, code: KeyCode) {
    let item_count = match &app.popup {
        Some(Popup::AssignmentPicker(state)) => state.items.len(),
        _ => return,
    };

    match code {
        KeyCode::Esc => {
            app.popup = None;
        }
        KeyCode::Up | KeyCode::Char('k' | 'K') => {
            if let Some(Popup::AssignmentPicker(ref mut state)) = app.popup
                && state.cursor > 0
            {
                state.cursor -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j' | 'J') => {
            if let Some(Popup::AssignmentPicker(ref mut state)) = app.popup
                && item_count > 0
                && state.cursor < item_count - 1
            {
                state.cursor += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(Popup::AssignmentPicker(state)) = app.popup.take()
                && let Some(item) = state.items.get(state.cursor)
            {
                match item {
                    AssignmentPickerItem::Existing { cloud_name, .. } => {
                        spawn_schedule_to_existing(app, cloud_name.clone(), state.selected_hosts);
                    }
                    AssignmentPickerItem::NewAssignment => {
                        app.popup = Some(Popup::NewAssignmentForm(NewAssignmentForm::new(
                            state.selected_hosts,
                        )));
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_new_assignment_form_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Esc => {
            app.popup = None;
        }
        KeyCode::Tab | KeyCode::Down => {
            if let Some(Popup::NewAssignmentForm(ref mut form)) = app.popup {
                form.active_field = form.active_field.next();
            }
        }
        KeyCode::BackTab | KeyCode::Up => {
            if let Some(Popup::NewAssignmentForm(ref mut form)) = app.popup {
                form.active_field = form.active_field.prev();
            }
        }
        KeyCode::Char(' ') => {
            if let Some(Popup::NewAssignmentForm(ref mut form)) = app.popup {
                match form.active_field {
                    app::NewAssignmentField::Wipe => form.wipe = !form.wipe,
                    app::NewAssignmentField::Qinq => form.qinq = !form.qinq,
                    app::NewAssignmentField::Description => form.description.insert(' '),
                }
            }
        }
        KeyCode::Left => {
            if let Some(Popup::NewAssignmentForm(ref mut form)) = app.popup
                && form.active_field == app::NewAssignmentField::Description
            {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    form.description.move_word_left();
                } else {
                    form.description.move_left();
                }
            }
        }
        KeyCode::Right => {
            if let Some(Popup::NewAssignmentForm(ref mut form)) = app.popup
                && form.active_field == app::NewAssignmentField::Description
            {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    form.description.move_word_right();
                } else {
                    form.description.move_right();
                }
            }
        }
        KeyCode::Home => {
            if let Some(Popup::NewAssignmentForm(ref mut form)) = app.popup
                && form.active_field == app::NewAssignmentField::Description
            {
                form.description.move_home();
            }
        }
        KeyCode::End => {
            if let Some(Popup::NewAssignmentForm(ref mut form)) = app.popup
                && form.active_field == app::NewAssignmentField::Description
            {
                form.description.move_end();
            }
        }
        KeyCode::Delete => {
            if let Some(Popup::NewAssignmentForm(ref mut form)) = app.popup
                && form.active_field == app::NewAssignmentField::Description
            {
                form.description.delete();
            }
        }
        KeyCode::Backspace => {
            if let Some(Popup::NewAssignmentForm(ref mut form)) = app.popup
                && form.active_field == app::NewAssignmentField::Description
            {
                form.description.backspace();
            }
        }
        KeyCode::Char(c) => {
            if let Some(Popup::NewAssignmentForm(ref mut form)) = app.popup
                && form.active_field == app::NewAssignmentField::Description
            {
                form.description.insert(c);
            }
        }
        KeyCode::Enter => {
            if let Some(Popup::NewAssignmentForm(form)) = app.popup.take() {
                if form.description.is_empty() {
                    app.set_error("Description is required".into());
                    return;
                }
                spawn_schedule_new_assignment(app, form);
            }
        }
        _ => {}
    }
}

fn spawn_schedule_to_existing(app: &mut App, cloud_name: String, hosts: Vec<String>) {
    let Some(session) = app.sessions.active_session() else {
        return;
    };
    let client = session.client.clone();
    let total = hosts.len();

    app.popup = Some(Popup::Scheduling(SchedulingProgress {
        total,
        cloud_name: cloud_name.clone(),
    }));

    let (tx, rx) = oneshot::channel();
    app.pending_schedule = Some(rx);

    tokio::spawn(async move {
        let mut success_count = 0;
        let mut errors = Vec::new();
        for hostname in &hosts {
            match client.create_schedule(&cloud_name, hostname).await {
                Ok(_) => success_count += 1,
                Err(e) => errors.push(format!("{}: {}", hostname, e)),
            }
        }
        let _ = tx.send(ScheduleResult {
            success_count,
            fail_count: errors.len(),
            errors,
        });
    });
}

fn spawn_schedule_new_assignment(app: &mut App, form: NewAssignmentForm) {
    let Some(session) = app.sessions.active_session() else {
        return;
    };
    let client = session.client.clone();
    let owner = session.username().unwrap_or_default().to_string();
    let hosts = form.selected_hosts.clone();
    let total = hosts.len();

    app.popup = Some(Popup::Scheduling(SchedulingProgress {
        total,
        cloud_name: "creating...".to_string(),
    }));

    let (tx, rx) = oneshot::channel();
    app.pending_schedule = Some(rx);

    let description = form.description.value.clone();
    let qinq: i64 = if form.qinq { 1 } else { 0 };
    let wipe = form.wipe;

    tokio::spawn(async move {
        let cloud_name = match client
            .create_self_assignment(&description, &owner, qinq, wipe)
            .await
        {
            Ok(resp) => resp.cloud_name,
            Err(e) => {
                let _ = tx.send(ScheduleResult {
                    success_count: 0,
                    fail_count: total,
                    errors: vec![format!("Assignment creation failed: {}", e)],
                });
                return;
            }
        };

        let mut success_count = 0;
        let mut errors = Vec::new();
        for hostname in &hosts {
            match client.create_schedule(&cloud_name, hostname).await {
                Ok(_) => success_count += 1,
                Err(e) => errors.push(format!("{}: {}", hostname, e)),
            }
        }
        let _ = tx.send(ScheduleResult {
            success_count,
            fail_count: errors.len(),
            errors,
        });
    });
}

fn handle_schedule_result(app: &mut App, result: ScheduleResult) {
    app.host_multi_select.clear();
    if result.fail_count == 0 {
        app.status_message = Some(format!(
            "Scheduled {} host(s) successfully",
            result.success_count
        ));
        app.popup = None;
        spawn_refresh(app);
    } else {
        let msg = format!(
            "Scheduled {}/{} hosts.\nErrors:\n{}",
            result.success_count,
            result.success_count + result.fail_count,
            result.errors.join("\n")
        );
        app.popup = Some(Popup::Error(msg));
        spawn_refresh(app);
    }
}

fn disconnect_current(app: &mut App) {
    if let Some(session) = app.sessions.active_session_mut() {
        let name = session.name.clone();
        session.disconnect();
        app.status_message = Some(format!("Disconnected from {}", name));
    }
}

fn spawn_refresh(app: &mut App) {
    app.refresh_rx = None;

    let Some(server_name) = app.sessions.active_server.clone() else {
        return;
    };
    let Some(session) = app.sessions.get_session(&server_name) else {
        return;
    };

    let client = session.client.clone();
    app.loading = true;
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    app.refresh_rx = Some(rx);

    log::info!("refreshing data");
    tokio::spawn(async move {
        let tx1 = tx.clone();
        let c1 = client.clone();
        let server_name1 = server_name.clone();
        let h1 = tokio::spawn(async move {
            match c1.get_hosts(None).await {
                Ok(data) => {
                    let _ = tx1.send(RefreshUpdate::Hosts(server_name1, data)).await;
                }
                Err(e) => {
                    log::error!("refresh hosts failed: {}", e);
                    let _ = tx1.send(RefreshUpdate::Error(e.to_string())).await;
                }
            }
        });

        let tx2 = tx.clone();
        let c2 = client.clone();
        let server_name2 = server_name.clone();
        let h2 = tokio::spawn(async move {
            match c2.get_clouds().await {
                Ok(data) => {
                    let _ = tx2.send(RefreshUpdate::Clouds(server_name2, data)).await;
                }
                Err(e) => {
                    log::error!("refresh clouds failed: {}", e);
                    let _ = tx2.send(RefreshUpdate::Error(e.to_string())).await;
                }
            }
        });

        let tx3 = tx.clone();
        let c3 = client.clone();
        let server_name3 = server_name.clone();
        let h3 = tokio::spawn(async move {
            match c3.get_cloud_summary().await {
                Ok(data) => {
                    let _ = tx3
                        .send(RefreshUpdate::CloudSummaries(server_name3, data))
                        .await;
                }
                Err(e) => {
                    log::error!("refresh cloud summaries failed: {}", e);
                    let _ = tx3.send(RefreshUpdate::Error(e.to_string())).await;
                }
            }
        });

        let tx4 = tx.clone();
        let c4 = client.clone();
        let server_name4 = server_name.clone();
        let h4 = tokio::spawn(async move {
            match c4.get_active_assignments().await {
                Ok(data) => {
                    let _ = tx4
                        .send(RefreshUpdate::Assignments(server_name4, data))
                        .await;
                }
                Err(e) => {
                    log::error!("refresh assignments failed: {}", e);
                    let _ = tx4.send(RefreshUpdate::Error(e.to_string())).await;
                }
            }
        });

        let tx5 = tx.clone();
        let c5 = client.clone();
        let server_name5 = server_name.clone();
        let h5 = tokio::spawn(async move {
            match c5.get_current_schedules(None).await {
                Ok(data) => {
                    let _ = tx5.send(RefreshUpdate::Schedules(server_name5, data)).await;
                }
                Err(e) => {
                    log::error!("refresh schedules failed: {}", e);
                    let _ = tx5.send(RefreshUpdate::Error(e.to_string())).await;
                }
            }
        });

        let _ = tokio::join!(h1, h2, h3, h4, h5);
        let _ = tx.send(RefreshUpdate::Done).await;
    });
}

fn recalc_my_assignments(session: &mut Session) {
    if let Some(username) = session.username() {
        let username = username.to_string();
        session.my_assignments = session
            .assignments
            .iter()
            .filter(|a| a.owner.as_deref() == Some(&username))
            .cloned()
            .collect();
    } else {
        session.my_assignments.clear();
    }
}

fn spawn_terminate(app: &mut App, assignment_id: i64) {
    let Some(session) = app.sessions.active_session() else {
        return;
    };
    let client = session.client.clone();
    let (tx, rx) = oneshot::channel();
    app.pending_action = Some(rx);
    app.popup = Some(Popup::Working("Terminating assignment...".into()));

    tokio::spawn(async move {
        match client.terminate_assignment(assignment_id).await {
            Ok(msg) => {
                let _ = tx.send(ActionResult {
                    success: true,
                    message: msg,
                    clear_detail: false,
                    exit_after: false,
                });
            }
            Err(e) => {
                let _ = tx.send(ActionResult {
                    success: false,
                    message: format!("Termination failed: {}", e),
                    clear_detail: false,
                    exit_after: false,
                });
            }
        }
    });
}

fn spawn_unschedule(app: &mut App, schedule_id: i64) {
    let Some(session) = app.sessions.active_session() else {
        return;
    };
    let client = session.client.clone();
    let (tx, rx) = oneshot::channel();
    app.pending_action = Some(rx);
    app.popup = Some(Popup::Working("Unscheduling host...".into()));

    tokio::spawn(async move {
        match client.delete_schedule(schedule_id).await {
            Ok(msg) => {
                let _ = tx.send(ActionResult {
                    success: true,
                    message: msg,
                    clear_detail: true,
                    exit_after: false,
                });
            }
            Err(e) => {
                let _ = tx.send(ActionResult {
                    success: false,
                    message: format!("Unschedule failed: {}", e),
                    clear_detail: false,
                    exit_after: false,
                });
            }
        }
    });
}

fn handle_action_result(app: &mut App, result: ActionResult) {
    if result.success {
        if result.exit_after {
            app.popup = Some(Popup::UpdateComplete(result.message));
        } else {
            app.status_message = Some(result.message);
            app.popup = None;
            if result.clear_detail {
                app.assignment_detail_selected = None;
            }
            spawn_refresh(app);
        }
    } else {
        app.popup = Some(Popup::Error(result.message));
    }
}
