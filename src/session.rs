use crate::api::models::*;
use crate::api::ApiClient;
use crate::config::ServerEntry;
use anyhow::Result;
use std::collections::HashMap;

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
    pub sessions: HashMap<String, Session>,
    pub active_server: Option<String>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            active_server: None,
        }
    }

    pub fn active_session(&self) -> Option<&Session> {
        self.active_server.as_ref().and_then(|name| self.sessions.get(name))
    }

    pub fn active_session_mut(&mut self) -> Option<&mut Session> {
        self.active_server.as_ref().cloned().and_then(|name| self.sessions.get_mut(&name))
    }

    pub fn add_session(&mut self, session: Session) {
        let name = session.name.clone();
        self.sessions.insert(name.clone(), session);
        if self.active_server.is_none() {
            self.active_server = Some(name);
        }
    }

    pub fn switch_to(&mut self, name: &str) -> bool {
        if self.sessions.contains_key(name) {
            self.active_server = Some(name.to_string());
            true
        } else {
            false
        }
    }

    pub fn get_session(&self, name: &str) -> Option<&Session> {
        self.sessions.get(name)
    }

    pub fn get_session_mut(&mut self, name: &str) -> Option<&mut Session> {
        self.sessions.get_mut(name)
    }

}
