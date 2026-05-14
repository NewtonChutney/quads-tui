use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEntry {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default = "default_true")]
    pub verify_ssl: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub default_server: Option<String>,
    #[serde(default)]
    pub servers: BTreeMap<String, ServerEntry>,
}

impl AppConfig {
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("cannot determine config directory"))?;
        Ok(config_dir.join("quads").join("quads-tui.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn add_server(&mut self, name: String, entry: ServerEntry) {
        if self.servers.is_empty() {
            self.default_server = Some(name.clone());
        }
        self.servers.insert(name, entry);
    }

    pub fn remove_server(&mut self, name: &str) {
        self.servers.remove(name);
        if self.default_server.as_deref() == Some(name) {
            self.default_server = self.servers.keys().next().cloned();
        }
    }
}
