use tokio::sync::oneshot;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_API_URL: &str =
    "https://api.github.com/repos/NewtonChutney/quads-tui/releases/latest";

#[derive(Debug)]
pub struct UpdateInfo {
    pub latest_version: String,
}

async fn check_for_update() -> anyhow::Result<Option<UpdateInfo>> {
    let client = reqwest::Client::builder()
        .user_agent(format!("quads-tui/{}", VERSION))
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp: serde_json::Value = client.get(GITHUB_API_URL).send().await?.json().await?;

    let tag = resp
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let remote = tag.strip_prefix('v').unwrap_or(tag);
    let remote_ver = semver::Version::parse(remote)?;
    let local_ver = semver::Version::parse(VERSION)?;

    if remote_ver > local_ver {
        Ok(Some(UpdateInfo {
            latest_version: remote_ver.to_string(),
        }))
    } else {
        Ok(None)
    }
}

pub fn spawn_update_check() -> oneshot::Receiver<Option<UpdateInfo>> {
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        match check_for_update().await {
            Ok(info) => {
                let _ = tx.send(info);
            }
            Err(e) => {
                log::warn!("update check failed: {}", e);
                let _ = tx.send(None);
            }
        }
    });
    rx
}
