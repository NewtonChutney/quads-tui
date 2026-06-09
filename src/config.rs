use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateFrequency {
    OnLaunch,
    Daily,
    #[default]
    Weekly,
    Monthly,
    Never,
}

impl UpdateFrequency {
    pub fn interval_secs(&self) -> Option<u64> {
        match self {
            Self::OnLaunch => Some(0),
            Self::Daily => Some(86400),
            Self::Weekly => Some(604800),
            Self::Monthly => Some(2592000),
            Self::Never => None,
        }
    }
}

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
    pub update_check: UpdateFrequency,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_update_check: Option<u64>,
    #[serde(default)]
    pub servers: BTreeMap<String, ServerEntry>,
}

impl AppConfig {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("quads")
    }

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

        let mut doc = if let Ok(existing) = std::fs::read_to_string(&path) {
            existing.parse::<toml_edit::DocumentMut>().unwrap_or_default()
        } else {
            toml_edit::DocumentMut::default()
        };

        match &self.default_server {
            Some(s) => { doc["default_server"] = toml_edit::value(s.as_str()); }
            None => { doc.remove("default_server"); }
        }

        if self.update_check != UpdateFrequency::default() {
            let freq_str = match self.update_check {
                UpdateFrequency::OnLaunch => "on-launch",
                UpdateFrequency::Daily => "daily",
                UpdateFrequency::Weekly => "weekly",
                UpdateFrequency::Monthly => "monthly",
                UpdateFrequency::Never => "never",
            };
            doc["update_check"] = toml_edit::value(freq_str);
        }

        match self.last_update_check {
            Some(ts) => { doc["last_update_check"] = toml_edit::value(ts as i64); }
            None => { doc.remove("last_update_check"); }
        }

        if self.servers.is_empty() {
            doc.remove("servers");
        } else {
            let servers = doc.entry("servers").or_insert_with(|| {
                let mut t = toml_edit::Table::new();
                t.set_implicit(true);
                toml_edit::Item::Table(t)
            });
            if let Some(tbl) = servers.as_table_mut() {
                tbl.set_implicit(true);
                let existing_keys: Vec<String> = tbl.iter().map(|(k, _)| k.to_string()).collect();
                for k in &existing_keys {
                    if !self.servers.contains_key(k.as_str()) {
                        tbl.remove(k);
                    }
                }

                for (name, entry) in &self.servers {
                    let is_new = !tbl.contains_key(name);
                    let server = tbl.entry(name).or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
                    if let Some(st) = server.as_table_mut() {
                        st["url"] = toml_edit::value(&entry.url);
                        match &entry.username {
                            Some(u) => { st["username"] = toml_edit::value(u.as_str()); }
                            None => { st.remove("username"); }
                        }
                        match &entry.password {
                            Some(p) => { st["password"] = toml_edit::value(p.as_str()); }
                            None => { st.remove("password"); }
                        }
                        if is_new {
                            st["verify_ssl"] = toml_edit::value(entry.verify_ssl);
                        }
                    }
                }
            }
        }

        std::fs::write(&path, doc.to_string())?;
        Ok(())
    }

    pub fn should_check_update(&self) -> bool {
        let Some(interval) = self.update_check.interval_secs() else {
            return false;
        };
        if interval == 0 {
            return true;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        match self.last_update_check {
            Some(last) => now.saturating_sub(last) >= interval,
            None => true,
        }
    }

    pub fn mark_update_checked(&mut self) {
        self.last_update_check = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
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
