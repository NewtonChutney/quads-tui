use super::models::*;
use anyhow::Result;
use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

fn extract_message(parsed: &serde_json::Value, fallback: &str) -> String {
    parsed
        .get("message")
        .or_else(|| parsed.get("error"))
        .or_else(|| parsed.get("detail"))
        .and_then(|v| v.as_str())
        .unwrap_or(fallback)
        .to_string()
}

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl ApiClient {
    pub fn new(base_url: &str, verify_ssl: bool) -> Result<Self> {
        let client = Client::builder()
            .danger_accept_invalid_certs(!verify_ssl)
            .timeout(REQUEST_TIMEOUT)
            .pool_idle_timeout(POOL_IDLE_TIMEOUT)
            .build()?;
        let base_url = base_url.trim_end_matches('/').to_string();
        Ok(Self {
            client,
            base_url,
            token: None,
        })
    }

    pub fn clear_token(&mut self) {
        self.token = None;
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/api/v3{}", self.base_url, path)
    }

    fn auth_request(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref token) = self.token {
            req.header("Authorization", format!("Bearer {}", token))
        } else {
            req
        }
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<String> {
        let resp = self
            .client
            .post(self.api_url("/login/"))
            .basic_auth(username, Some(password))
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|_| {
            anyhow::anyhow!(
                "server returned non-JSON response ({}):\n{}",
                status,
                body.chars().take(200).collect::<String>()
            )
        })?;

        if let Some(token) = parsed.get("auth_token").and_then(|v| v.as_str()) {
            self.token = Some(token.to_string());
            return Ok(token.to_string());
        }

        if let Some(token) = parsed.get("token").and_then(|v| v.as_str()) {
            self.token = Some(token.to_string());
            return Ok(token.to_string());
        }

        Err(anyhow::anyhow!("{}", extract_message(&parsed, "unknown error")))
    }

    pub async fn register(&self, email: &str, password: &str) -> Result<()> {
        let resp = self
            .client
            .post(self.api_url("/register/"))
            .json(&serde_json::json!({
                "email": email,
                "password": password,
            }))
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|_| {
            anyhow::anyhow!(
                "server returned non-JSON response ({}):\n{}",
                status,
                body.chars().take(200).collect::<String>()
            )
        })?;

        if status.is_success() {
            return Ok(());
        }

        Err(anyhow::anyhow!("registration failed: {}", extract_message(&parsed, "unknown error")))
    }

    pub async fn get_version(&self) -> Result<String> {
        let resp = self.client.get(self.api_url("/version/")).send().await?;
        let body = resp.text().await?;
        let trimmed = body.trim().trim_matches('"');
        if trimmed.is_empty() {
            return Ok("unknown".to_string());
        }
        Ok(trimmed.to_string())
    }

    pub async fn get_hosts(&self, filters: Option<HashMap<String, String>>) -> Result<Vec<Host>> {
        let url = self.api_url("/hosts/");
        log::debug!("GET {}", url);
        let mut req = self.client.get(&url);
        req = self.auth_request(req);
        if let Some(f) = filters {
            req = req.query(&f);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            log::error!("GET {} returned {}: {}", url, status, &body[..body.len().min(200)]);
        }
        let hosts: Vec<Host> = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse hosts: {}", e))?;
        log::debug!("GET {} returned {} hosts", url, hosts.len());
        Ok(hosts)
    }

    pub async fn get_clouds(&self) -> Result<Vec<Cloud>> {
        let url = self.api_url("/clouds/");
        log::debug!("GET {}", url);
        let req = self.auth_request(self.client.get(&url));
        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            log::error!("GET {} returned {}: {}", url, status, &body[..body.len().min(200)]);
        }
        let clouds: Vec<Cloud> = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse clouds: {}", e))?;
        log::debug!("GET {} returned {} clouds", url, clouds.len());
        Ok(clouds)
    }

    pub async fn get_cloud_summary(&self) -> Result<Vec<CloudSummary>> {
        let url = self.api_url("/clouds/summary/");
        log::debug!("GET {}", url);
        let req = self.auth_request(self.client.get(&url));
        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            log::error!("GET {} returned {}: {}", url, status, &body[..body.len().min(200)]);
        }
        let summaries: Vec<CloudSummary> = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse cloud summaries: {}", e))?;
        log::debug!("GET {} returned {} summaries", url, summaries.len());
        Ok(summaries)
    }

    pub async fn get_active_assignments(&self) -> Result<Vec<Assignment>> {
        let url = self.api_url("/assignments/active/");
        log::debug!("GET {}", url);
        let req = self.auth_request(self.client.get(&url));
        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            log::error!("GET {} returned {}: {}", url, status, &body[..body.len().min(200)]);
        }
        let assignments: Vec<Assignment> = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse assignments: {}", e))?;
        log::debug!("GET {} returned {} assignments", url, assignments.len());
        Ok(assignments)
    }

    pub async fn terminate_assignment(&self, assignment_id: i64) -> Result<String> {
        let req = self.auth_request(
            self.client
                .post(self.api_url(&format!("/assignments/terminate/{}/", assignment_id))),
        );
        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();

        if status.is_client_error() || status.is_server_error() {
            return Err(anyhow::anyhow!("{}", extract_message(&parsed, "termination failed")));
        }

        Ok(extract_message(&parsed, "assignment terminated"))
    }

    pub async fn delete_schedule(&self, schedule_id: i64) -> Result<String> {
        let req = self.auth_request(
            self.client
                .delete(self.api_url(&format!("/schedules/{}/", schedule_id))),
        );
        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();

        if status.is_client_error() || status.is_server_error() {
            return Err(anyhow::anyhow!("{}", extract_message(&parsed, "failed to delete schedule")));
        }

        Ok(extract_message(&parsed, "schedule deleted"))
    }

    pub async fn get_current_schedules(
        &self,
        filters: Option<HashMap<String, String>>,
    ) -> Result<Vec<Schedule>> {
        let url = self.api_url("/schedules/current/");
        log::debug!("GET {}", url);
        let mut req = self.client.get(&url);
        req = self.auth_request(req);
        if let Some(f) = filters {
            req = req.query(&f);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            log::error!("GET {} returned {}: {}", url, status, &body[..body.len().min(200)]);
        }
        let schedules: Vec<Schedule> = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse schedules: {}", e))?;
        log::debug!("GET {} returned {} schedules", url, schedules.len());
        Ok(schedules)
    }

    pub async fn create_self_assignment(
        &self,
        description: &str,
        owner: &str,
        qinq: i64,
        wipe: bool,
    ) -> Result<SelfAssignmentResponse> {
        let req = self.auth_request(
            self.client
                .post(self.api_url("/assignments/self/"))
                .json(&serde_json::json!({
                    "description": description,
                    "owner": owner,
                    "qinq": qinq,
                    "wipe": wipe,
                })),
        );
        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|_| {
            anyhow::anyhow!(
                "server returned non-JSON response ({}):\n{}",
                status,
                body.chars().take(200).collect::<String>()
            )
        })?;

        if status.is_success() || status.as_u16() == 403 {
            let cloud_name = parsed
                .get("cloud")
                .and_then(|c| c.get("name").and_then(|v| v.as_str()).or(c.as_str()))
                .unwrap_or("")
                .to_string();
            return Ok(SelfAssignmentResponse { cloud_name });
        }

        Err(anyhow::anyhow!("{}", extract_message(&parsed, "unknown error")))
    }

    pub async fn create_schedule(&self, cloud: &str, hostname: &str) -> Result<i64> {
        let req = self.auth_request(
            self.client
                .post(self.api_url("/schedules/"))
                .json(&serde_json::json!({
                    "cloud": cloud,
                    "hostname": hostname,
                })),
        );
        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|_| {
            anyhow::anyhow!("server returned non-JSON response ({})", status)
        })?;

        if status.is_success() {
            let id = parsed.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            return Ok(id);
        }

        Err(anyhow::anyhow!("{}", extract_message(&parsed, "schedule creation failed")))
    }
}
