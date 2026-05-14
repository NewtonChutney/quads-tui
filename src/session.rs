use crate::api::models::*;
use crate::api::ApiClient;
use crate::config::ServerEntry;
use anyhow::Result;

pub struct Session {
    pub name: String,
    pub client: ApiClient,
    pub connected: bool,
    pub version: Option<String>,
    pub user_email: Option<String>,
    pub is_admin: bool,

    pub hosts: Vec<Host>,
    pub clouds: Vec<Cloud>,
    pub cloud_summaries: Vec<CloudSummary>,
    pub assignments: Vec<Assignment>,
    pub my_assignments: Vec<Assignment>,
    pub schedules: Vec<Schedule>,
}

impl Session {
    pub fn new(name: &str, entry: &ServerEntry) -> Result<Self> {
        let client = ApiClient::new(&entry.url, entry.verify_ssl)?;
        Ok(Self {
            name: name.to_string(),
            client,
            connected: false,
            version: None,
            user_email: None,
            is_admin: false,

            hosts: Vec::new(),
            clouds: Vec::new(),
            cloud_summaries: Vec::new(),
            assignments: Vec::new(),
            my_assignments: Vec::new(),
            schedules: Vec::new(),
        })
    }

    pub async fn connect(&mut self, username: &str, password: &str) -> Result<()> {
        self.client.login(username, password).await?;
        self.connected = true;
        self.user_email = Some(username.to_string());

        if let Ok(v) = self.client.get_version().await {
            self.version = Some(v);
        }

        Ok(())
    }

    pub async fn register_and_login(&mut self, username: &str, password: &str) -> Result<()> {
        self.client.register(username, password).await?;
        self.connect(username, password).await
    }

    pub fn username(&self) -> Option<&str> {
        self.user_email
            .as_deref()
            .map(|e| e.split('@').next().unwrap_or(e))
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
        self.version = None;
        self.user_email = None;
        self.is_admin = false;
        self.client.clear_token();
        self.my_assignments.clear();
    }

}

pub struct SessionManager {
    pub sessions: Vec<Session>,
    pub active: Option<usize>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            active: None,
        }
    }

    pub fn active_session(&self) -> Option<&Session> {
        self.active.and_then(|i| self.sessions.get(i))
    }

    pub fn active_session_mut(&mut self) -> Option<&mut Session> {
        self.active.and_then(|i| self.sessions.get_mut(i))
    }

    pub fn add_session(&mut self, session: Session) -> usize {
        let idx = self.sessions.len();
        self.sessions.push(session);
        if self.active.is_none() {
            self.active = Some(idx);
        }
        idx
    }

    pub fn switch_to(&mut self, index: usize) -> bool {
        if index < self.sessions.len() {
            self.active = Some(index);
            true
        } else {
            false
        }
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

}
