use crate::api::models::*;
use crate::config::AppConfig;
use crate::session::{Session, SessionManager};
use std::collections::HashSet;
use tokio::sync::oneshot;

#[derive(Debug, Clone)]
pub struct TextInput {
    pub value: String,
    pub cursor: usize,
}

impl TextInput {
    pub fn new() -> Self {
        Self { value: String::new(), cursor: 0 }
    }

    pub fn from(s: String) -> Self {
        let cursor = s.len();
        Self { value: s, cursor }
    }

    pub fn insert(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.value[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.value.remove(prev);
            self.cursor = prev;
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.value[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += self.value[self.cursor..].chars().next().map(|c| c.len_utf8()).unwrap_or(0);
        }
    }

    pub fn move_word_left(&mut self) {
        let bytes = self.value.as_bytes();
        let mut pos = self.cursor;
        while pos > 0 && !bytes[pos - 1].is_ascii_alphanumeric() {
            pos -= 1;
        }
        while pos > 0 && bytes[pos - 1].is_ascii_alphanumeric() {
            pos -= 1;
        }
        self.cursor = pos;
    }

    pub fn move_word_right(&mut self) {
        let bytes = self.value.as_bytes();
        let len = bytes.len();
        let mut pos = self.cursor;
        while pos < len && bytes[pos].is_ascii_alphanumeric() {
            pos += 1;
        }
        while pos < len && !bytes[pos].is_ascii_alphanumeric() {
            pos += 1;
        }
        self.cursor = pos;
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.value.len();
    }

    pub fn before_cursor(&self) -> &str {
        &self.value[..self.cursor]
    }

    pub fn char_at_cursor(&self) -> &str {
        if self.cursor < self.value.len() {
            let end = self.cursor + self.value[self.cursor..].chars().next().map(|c| c.len_utf8()).unwrap_or(0);
            &self.value[self.cursor..end]
        } else {
            " "
        }
    }

    pub fn after_cursor_char(&self) -> &str {
        if self.cursor < self.value.len() {
            let skip = self.value[self.cursor..].chars().next().map(|c| c.len_utf8()).unwrap_or(0);
            &self.value[self.cursor + skip..]
        } else {
            ""
        }
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
}

pub fn fuzzy_match(haystack: &str, needle: &str) -> bool {
    let mut hay = haystack.chars().flat_map(|c| c.to_lowercase());
    for nc in needle.chars().flat_map(|c| c.to_lowercase()) {
        if !hay.any(|hc| hc == nc) {
            return false;
        }
    }
    true
}

pub fn assignment_matches_search(a: &crate::api::models::Assignment, search: &str) -> bool {
    fuzzy_match(a.cloud_name().unwrap_or(""), search)
        || fuzzy_match(a.owner.as_deref().unwrap_or(""), search)
        || fuzzy_match(a.description.as_deref().unwrap_or(""), search)
        || fuzzy_match(a.ticket.as_deref().unwrap_or(""), search)
        || fuzzy_match(&a.id.map(|i| i.to_string()).unwrap_or_default(), search)
}

pub struct ConnectResult {
    pub server_name: String,
    pub username: String,
    pub password: String,
    pub result: Result<Session, ConnectError>,
}

pub struct ConnectError {
    pub message: String,
    pub is_credential_error: bool,
}

pub enum RefreshUpdate {
    Hosts(String, Vec<Host>),
    Clouds(String, Vec<Cloud>),
    CloudSummaries(String, Vec<CloudSummary>),
    Assignments(String, Vec<Assignment>),
    Schedules(String, Vec<Schedule>),
    Error(String),
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Hosts,
    Assignments,
    Clouds,
}

impl Screen {
    pub fn next(self) -> Self {
        match self {
            Self::Dashboard => Self::Hosts,
            Self::Hosts => Self::Assignments,
            Self::Assignments => Self::Clouds,
            Self::Clouds => Self::Dashboard,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Dashboard => Self::Clouds,
            Self::Hosts => Self::Dashboard,
            Self::Assignments => Self::Hosts,
            Self::Clouds => Self::Assignments,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HostFilterFlags {
    pub available: bool,
    pub scheduled: bool,
    pub broken: bool,
    pub retired: bool,
}

impl HostFilterFlags {
    pub fn default_filters() -> Self {
        Self {
            available: true,
            scheduled: true,
            broken: false,
            retired: false,
        }
    }

    pub fn get(&self, idx: usize) -> bool {
        match idx {
            0 => self.available,
            1 => self.scheduled,
            2 => self.broken,
            3 => self.retired,
            _ => false,
        }
    }

    pub fn toggle(&mut self, idx: usize) {
        match idx {
            0 => self.available = !self.available,
            1 => self.scheduled = !self.scheduled,
            2 => self.broken = !self.broken,
            3 => self.retired = !self.retired,
            _ => {}
        }
    }

    pub const LABELS: [&str; 4] = [
        "Available",
        "Scheduled",
        "Broken",
        "Retired",
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterPane {
    Status,
    SelfSchedule,
}

#[derive(Debug)]
pub struct HostFilterPopup {
    pub cursor: usize,
    pub flags: HostFilterFlags,
    pub pane: FilterPane,
    pub ssm_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerFormField {
    Name,
    Url,
    VerifySsl,
}

impl ServerFormField {
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Url,
            Self::Url => Self::VerifySsl,
            Self::VerifySsl => Self::Name,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Name => Self::VerifySsl,
            Self::Url => Self::Name,
            Self::VerifySsl => Self::Url,
        }
    }
}

#[derive(Debug)]
pub struct ServerForm {
    pub name: TextInput,
    pub url: TextInput,
    pub verify_ssl: bool,
    pub active_field: ServerFormField,
    pub editing_existing: Option<String>,
}

impl ServerForm {
    pub fn new() -> Self {
        Self {
            name: TextInput::new(),
            url: TextInput::new(),
            verify_ssl: true,
            active_field: ServerFormField::Name,
            editing_existing: None,
        }
    }

    pub fn active_input_mut(&mut self) -> Option<&mut TextInput> {
        match self.active_field {
            ServerFormField::Name => Some(&mut self.name),
            ServerFormField::Url => Some(&mut self.url),
            ServerFormField::VerifySsl => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthFormField {
    Username,
    Password,
}

impl AuthFormField {
    pub fn next(self) -> Self {
        match self {
            Self::Username => Self::Password,
            Self::Password => Self::Username,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Username => Self::Password,
            Self::Password => Self::Username,
        }
    }
}

#[derive(Debug)]
pub struct AuthForm {
    pub server_name: String,
    pub username: TextInput,
    pub password: TextInput,
    pub active_field: AuthFormField,
    pub error: Option<String>,
    pub register_prompt: bool,
}

impl AuthForm {
    pub fn new(server_name: String) -> Self {
        Self {
            server_name,
            username: TextInput::new(),
            password: TextInput::new(),
            active_field: AuthFormField::Username,
            error: None,
            register_prompt: false,
        }
    }

    pub fn with_credentials(server_name: String, username: String, password: String) -> Self {
        Self {
            server_name,
            username: TextInput::from(username),
            password: TextInput::from(password),
            active_field: AuthFormField::Username,
            error: None,
            register_prompt: false,
        }
    }

    pub fn active_input_mut(&mut self) -> &mut TextInput {
        match self.active_field {
            AuthFormField::Username => &mut self.username,
            AuthFormField::Password => &mut self.password,
        }
    }
}

#[derive(Debug)]
pub struct HostInfoState {
    pub host_idx: usize,
    pub host_name: Option<String>,
    pub scroll: u16,
    pub sections: [bool; 4],
    pub cursor: usize,
}

impl HostInfoState {
    pub fn new(host_idx: usize) -> Self {
        Self {
            host_idx,
            host_name: None,
            scroll: 0,
            sections: [false; 4],
            cursor: 0,
        }
    }

    pub fn from_name(name: String) -> Self {
        Self {
            host_idx: 0,
            host_name: Some(name),
            scroll: 0,
            sections: [false; 4],
            cursor: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AssignmentPickerItem {
    Existing {
        cloud_name: String,
        description: String,
    },
    NewAssignment,
}

#[derive(Debug)]
pub struct AssignmentPickerState {
    pub selected_hosts: Vec<String>,
    pub cursor: usize,
    pub items: Vec<AssignmentPickerItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewAssignmentField {
    Description,
    Qinq,
    Wipe,
}

impl NewAssignmentField {
    pub fn next(self) -> Self {
        match self {
            Self::Description => Self::Qinq,
            Self::Qinq => Self::Wipe,
            Self::Wipe => Self::Description,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Description => Self::Wipe,
            Self::Qinq => Self::Description,
            Self::Wipe => Self::Qinq,
        }
    }
}

#[derive(Debug)]
pub struct NewAssignmentForm {
    pub selected_hosts: Vec<String>,
    pub description: TextInput,
    pub qinq: bool,
    pub wipe: bool,
    pub active_field: NewAssignmentField,
}

impl NewAssignmentForm {
    pub fn new(selected_hosts: Vec<String>) -> Self {
        Self {
            selected_hosts,
            description: TextInput::new(),
            qinq: false,
            wipe: true,
            active_field: NewAssignmentField::Description,
        }
    }
}

#[derive(Debug)]
pub struct SchedulingProgress {
    pub total: usize,
    pub cloud_name: String,
}

pub struct ScheduleResult {
    pub success_count: usize,
    pub fail_count: usize,
    pub errors: Vec<String>,
}

pub struct ActionResult {
    pub success: bool,
    pub message: String,
    pub clear_detail: bool,
    pub exit_after: bool,
}

#[derive(Debug)]
pub enum Popup {
    HostInfo(HostInfoState),
    HostFilter(HostFilterPopup),
    ConfirmTerminate(i64),
    ServerForm(ServerForm),
    AuthForm(AuthForm),
    Connecting(String),
    ConnectSuccess(String, usize),
    AssignmentPicker(AssignmentPickerState),
    NewAssignmentForm(NewAssignmentForm),
    Scheduling(SchedulingProgress),
    ConfirmUnschedule { schedule_id: i64, host_name: String },
    Working(String),
    UpdateComplete(String),
    Error(String),
}

pub struct App {
    pub running: bool,
    pub screen: Screen,
    pub previous_screen: Option<Screen>,
    pub config: AppConfig,
    pub sessions: SessionManager,
    pub popup: Option<Popup>,

    pub host_filters: HostFilterFlags,
    pub host_ssm_filter: bool,
    pub host_self_schedule_only: bool,
    pub host_selected: usize,
    pub host_search: Option<String>,
    pub host_searching: bool,
    pub host_multi_select: HashSet<String>,

    pub assignment_selected: usize,
    pub assignment_show_all: bool,
    pub assignment_search: Option<String>,
    pub assignment_searching: bool,
    pub assignment_detail_selected: Option<usize>,

    pub cloud_selected: usize,
    pub cloud_show_all: bool,
    pub cloud_search: Option<String>,
    pub cloud_searching: bool,

    pub server_selected: usize,

    pub status_message: Option<String>,
    pub loading: bool,

    pub focused: bool,
    pub auto_refresh: bool,
    pub tick: usize,
    pub pending_connect: Option<oneshot::Receiver<ConnectResult>>,
    pub pending_schedule: Option<oneshot::Receiver<ScheduleResult>>,
    pub pending_action: Option<oneshot::Receiver<ActionResult>>,
    pub refresh_rx: Option<tokio::sync::mpsc::Receiver<RefreshUpdate>>,
    pub update_rx: Option<oneshot::Receiver<Option<crate::update::UpdateInfo>>>,
    pub update_available: Option<crate::update::UpdateInfo>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        Self {
            running: true,
            screen: Screen::Dashboard,
            previous_screen: None,
            config,
            sessions: SessionManager::new(),
            popup: None,

            host_filters: HostFilterFlags::default_filters(),
            host_ssm_filter: false,
            host_self_schedule_only: false,
            host_selected: 0,
            host_search: None,
            host_searching: false,
            host_multi_select: HashSet::new(),

            assignment_selected: 0,
            assignment_show_all: false,
            assignment_search: None,
            assignment_searching: false,
            assignment_detail_selected: None,

            cloud_selected: 0,
            cloud_show_all: true,
            cloud_search: None,
            cloud_searching: false,

            server_selected: 0,

            status_message: None,
            loading: false,

            focused: true,
            auto_refresh: false,
            tick: 0,
            pending_connect: None,
            pending_schedule: None,
            pending_action: None,
            refresh_rx: None,
            update_rx: None,
            update_available: None,
        }
    }

    pub fn spinner_char(&self) -> char {
        const FRAMES: &[char] = &['|', '/', '-', '\\'];
        FRAMES[self.tick % FRAMES.len()]
    }

    pub fn navigate(&mut self, screen: Screen) {
        self.previous_screen = Some(self.screen);
        self.screen = screen;
    }

    pub fn go_back(&mut self) {
        if self.popup.is_some() {
            self.popup = None;
        } else if self.screen != Screen::Dashboard {
            self.screen = Screen::Dashboard;
        }
    }

    pub fn set_error(&mut self, msg: String) {
        self.popup = Some(Popup::Error(msg));
    }

    pub fn filtered_hosts(&self) -> Vec<&crate::api::models::Host> {
        let Some(session) = self.sessions.active_session() else {
            return Vec::new();
        };
        let scheduled_hosts: HashSet<&str> = if self.host_self_schedule_only {
            session
                .schedules
                .iter()
                .map(|s| s.host_name())
                .collect()
        } else {
            HashSet::new()
        };

        session
            .hosts
            .iter()
            .filter(|h| {
                if let Some(ref search) = self.host_search
                    && !fuzzy_match(&h.name, search) {
                        return false;
                    }

                if self.host_self_schedule_only {
                    return h.can_self_schedule == Some(true)
                        && h.broken != Some(true)
                        && h.retired != Some(true)
                        && h.cloud_name() == h.default_cloud_name()
                        && !scheduled_hosts.contains(h.name.as_str());
                }

                if self.host_ssm_filter && h.can_self_schedule != Some(true) {
                    return false;
                }

                let is_broken = h.broken == Some(true);
                let is_retired = h.retired == Some(true) && !is_broken;
                let is_available = !is_broken
                    && !is_retired
                    && h.cloud_name() == h.default_cloud_name();
                let is_scheduled = !is_broken
                    && !is_retired
                    && h.cloud_name() != h.default_cloud_name();

                let f = &self.host_filters;
                let none_selected = !f.available && !f.scheduled && !f.broken && !f.retired;
                none_selected
                    || (f.available && is_available)
                    || (f.scheduled && is_scheduled)
                    || (f.broken && is_broken)
                    || (f.retired && is_retired)
            })
            .collect()
    }
}
