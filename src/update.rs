use crate::app::ActionResult;
use tokio::sync::oneshot;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_REPO: &str = "NewtonChutney/quads-tui";
const GITHUB_API_URL: &str =
    "https://api.github.com/repos/NewtonChutney/quads-tui/releases/latest";

#[derive(Debug)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub download_url: String,
}

fn asset_name() -> Option<String> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        _ => return None,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return None,
    };
    if os == "windows" {
        Some(format!("quads-tui-{}-{}.exe", os, arch))
    } else {
        Some(format!("quads-tui-{}-{}", os, arch))
    }
}

fn build_client() -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(format!("quads-tui/{}", VERSION))
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

async fn check_for_update() -> anyhow::Result<Option<UpdateInfo>> {
    let client = build_client()?;
    let resp: serde_json::Value = client.get(GITHUB_API_URL).send().await?.json().await?;

    let tag = resp
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let remote = tag.strip_prefix('v').unwrap_or(tag);
    let remote_ver = semver::Version::parse(remote)?;
    let local_ver = semver::Version::parse(VERSION)?;

    if remote_ver > local_ver {
        let asset = asset_name().unwrap_or_default();
        let download_url = format!(
            "https://github.com/{}/releases/download/{}/{}",
            GITHUB_REPO, tag, asset
        );
        Ok(Some(UpdateInfo {
            latest_version: remote_ver.to_string(),
            download_url,
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

async fn self_update(download_url: &str, version: &str) -> anyhow::Result<String> {
    let current_exe = std::env::current_exe()?.canonicalize()?;

    let Some(_asset) = asset_name() else {
        return Err(anyhow::anyhow!("unsupported platform for self-update"));
    };

    let client = build_client()?;
    let resp = client.get(download_url).send().await?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!(
            "download failed: HTTP {}",
            resp.status()
        ));
    }

    let bytes = resp.bytes().await?;
    if bytes.is_empty() {
        return Err(anyhow::anyhow!("downloaded empty file"));
    }

    let parent = current_exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("cannot determine binary directory"))?;
    let tmp_path = parent.join(".quads-tui.update");

    tokio::fs::write(&tmp_path, &bytes).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755)).await?;
    }

    #[cfg(windows)]
    {
        let old_path = parent.join(".quads-tui.old");
        let _ = tokio::fs::remove_file(&old_path).await;
        tokio::fs::rename(&current_exe, &old_path).await?;
    }

    tokio::fs::rename(&tmp_path, &current_exe).await?;

    Ok(format!("Updated to v{} — restart to apply", version))
}

pub fn spawn_self_update(download_url: String, version: String) -> oneshot::Receiver<ActionResult> {
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let result = match self_update(&download_url, &version).await {
            Ok(msg) => ActionResult {
                success: true,
                message: msg,
                clear_detail: false,
                exit_after: true,
            },
            Err(e) => ActionResult {
                success: false,
                message: format!("Update failed: {}", e),
                clear_detail: false,
                exit_after: false,
            },
        };
        let _ = tx.send(result);
    });
    rx
}
