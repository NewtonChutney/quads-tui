pub mod assignments;
pub mod clouds;
pub mod dashboard;
pub mod hosts;
pub mod widgets;

use crate::app::{App, Popup, Screen};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    widgets::render_status_bar(f, chunks[0], app);
    widgets::render_tab_bar(f, chunks[1], app.screen);

    let content_area = chunks[2];

    match app.screen {
        Screen::Dashboard => dashboard::render(f, content_area, app),
        Screen::Hosts => hosts::render(f, content_area, app),
        Screen::Assignments => assignments::render(f, content_area, app),
        Screen::Clouds => clouds::render(f, content_area, app),
    }

    widgets::render_help_bar(f, chunks[3], app);

    if let Some(ref popup) = app.popup {
        match popup {
            Popup::Error(msg) => widgets::render_error_popup(f, msg),
            Popup::ConfirmTerminate(id) => {
                widgets::render_confirm_popup(f, &format!("Terminate assignment #{}?", id));
            }
            Popup::ServerForm(form) => {
                widgets::render_server_form(f, form);
            }
            Popup::AuthForm(form) => {
                widgets::render_auth_form(f, form);
            }
            Popup::Connecting(server_name) => {
                widgets::render_connecting_popup(f, server_name, app.spinner_char());
            }
            Popup::ConnectSuccess(server_name, _) => {
                widgets::render_connect_success_popup(f, server_name);
            }
            Popup::HostInfo(state) => {
                widgets::render_host_info_popup(f, app, state);
            }
            Popup::HostFilter(popup) => {
                widgets::render_host_filter_popup(f, popup);
            }
            Popup::AssignmentPicker(state) => {
                widgets::render_assignment_picker(f, state);
            }
            Popup::NewAssignmentForm(form) => {
                widgets::render_new_assignment_form(f, form, app);
            }
            Popup::Scheduling(progress) => {
                widgets::render_scheduling_popup(f, progress, app.spinner_char());
            }
            Popup::ConfirmUnschedule { host_name, .. } => {
                widgets::render_confirm_popup(f, &format!("Remove {} from assignment?", host_name));
            }
            Popup::Working(msg) => {
                widgets::render_working_popup(f, msg, app.spinner_char());
            }
            Popup::UpdateComplete(msg) => {
                widgets::render_update_complete_popup(f, msg);
            }
            Popup::ConfigHelp => {
                widgets::render_config_help_popup(f);
            }
        }
    }
}
