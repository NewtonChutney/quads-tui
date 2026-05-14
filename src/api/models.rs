use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudRef {
    pub id: Option<i64>,
    pub name: String,
    #[serde(default)]
    pub last_redefined: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub bios_id: Option<String>,
    #[serde(default)]
    pub mac_address: Option<String>,
    #[serde(default)]
    pub switch_ip: Option<String>,
    #[serde(default)]
    pub switch_port: Option<String>,
    #[serde(default)]
    pub speed: Option<i64>,
    #[serde(default)]
    pub vendor: Option<String>,
    #[serde(default)]
    pub pxe_boot: Option<bool>,
    #[serde(default)]
    pub maintenance: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disk {
    #[serde(default)]
    pub disk_id: Option<i64>,
    #[serde(default)]
    pub disk_type: Option<String>,
    #[serde(default)]
    pub size_gb: Option<i64>,
    #[serde(default)]
    pub count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub size_gb: Option<i64>,
    #[serde(default)]
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Processor {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub handle: Option<String>,
    #[serde(default)]
    pub vendor: Option<String>,
    #[serde(default)]
    pub product: Option<String>,
    #[serde(default)]
    pub cores: Option<i64>,
    #[serde(default)]
    pub threads: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    #[serde(default)]
    pub id: Option<i64>,
    pub name: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub host_type: Option<String>,
    #[serde(default)]
    pub cloud: Option<CloudRef>,
    #[serde(default)]
    pub default_cloud: Option<CloudRef>,
    #[serde(default)]
    pub broken: Option<bool>,
    #[serde(default)]
    pub retired: Option<bool>,
    #[serde(default)]
    pub build: Option<bool>,
    #[serde(default)]
    pub validated: Option<bool>,
    #[serde(default)]
    pub can_self_schedule: Option<bool>,
    #[serde(default)]
    pub rack: Option<String>,
    #[serde(default)]
    pub switch_config_applied: Option<bool>,
    #[serde(default)]
    pub last_build: Option<String>,
    #[serde(default)]
    pub interfaces: Vec<Interface>,
    #[serde(default)]
    pub disks: Vec<Disk>,
    #[serde(default)]
    pub memory: Vec<Memory>,
    #[serde(default)]
    pub processors: Vec<Processor>,
}

impl Host {
    pub fn cloud_name(&self) -> Option<&str> {
        self.cloud.as_ref().map(|c| c.name.as_str())
    }

    pub fn default_cloud_name(&self) -> Option<&str> {
        self.default_cloud.as_ref().map(|c| c.name.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cloud {
    pub id: Option<i64>,
    pub name: String,
    #[serde(default)]
    pub last_redefined: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(default)]
    pub provisioned: Option<bool>,
    #[serde(default)]
    pub validated: Option<bool>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub ticket: Option<String>,
    #[serde(default)]
    pub qinq: Option<i64>,
    #[serde(default)]
    pub wipe: Option<bool>,
    #[serde(default)]
    pub ccuser: Option<serde_json::Value>,
    #[serde(default)]
    pub cloud: Option<CloudRef>,
    #[serde(default)]
    pub vlan: Option<serde_json::Value>,
    #[serde(default)]
    pub is_self_schedule: Option<bool>,
    #[serde(default)]
    pub created_at: Option<String>,
}

impl Assignment {
    pub fn cloud_name(&self) -> Option<&str> {
        self.cloud.as_ref().map(|c| c.name.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub build_start: Option<String>,
    #[serde(default)]
    pub build_end: Option<String>,
    #[serde(default)]
    pub host: Option<serde_json::Value>,
    #[serde(default)]
    pub cloud: Option<serde_json::Value>,
    #[serde(default)]
    pub assignment_id: Option<i64>,
}

impl Schedule {
    pub fn host_name(&self) -> &str {
        self.host
            .as_ref()
            .and_then(|v| v.get("name").and_then(|n| n.as_str()))
            .unwrap_or("unknown")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSummary {
    pub name: String,
    #[serde(default)]
    pub count: Option<i64>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub ticket: Option<String>,
    #[serde(default)]
    pub ccuser: Option<serde_json::Value>,
    #[serde(default)]
    pub provisioned: Option<bool>,
    #[serde(default)]
    pub validated: Option<bool>,
    #[serde(default)]
    pub is_self_schedule: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SelfAssignmentResponse {
    pub cloud_name: String,
}

