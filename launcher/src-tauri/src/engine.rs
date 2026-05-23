//! Engine install pipeline: manifest fetch, download, verify, extract.
//!
//! Single-version policy: the engine lives at `paths::engine_dir()` and a
//! marker file `.aracdia-version` records what's installed. Re-installing
//! wipes the directory atomically.

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::download::{self, DownloadError};
use crate::paths;
use crate::settings;

/// User-Agent sent on all HTTP requests. GitHub API requires one and rejects
/// generic libcurl-style agents on some endpoints.
const USER_AGENT: &str = "AracdiaLauncher/0.1 (+https://aracdia.example)";

/// Maximum time to wait for the manifest fetch — the install download has its
/// own (longer) timeout so a slow CDN can't time-out a 200 MB transfer.
const MANIFEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Tauri event names emitted during install. Mirror these on the JS side.
mod events {
    pub const PROGRESS: &str = "engine://progress";
    pub const COMPLETE: &str = "engine://complete";
    pub const ERROR: &str = "engine://error";
}

// ---------------------------------------------------------------------------
// Public types (frontend-facing)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EngineAsset {
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EngineRelease {
    pub version: String,
    pub target: String,
    pub asset: EngineAsset,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum EngineStatus {
    NotInstalled,
    Installed { version: String, path: String },
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum InstallPhase {
    Downloading,
    Verifying,
    Extracting,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub phase: InstallPhase,
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstallComplete {
    pub version: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstallError {
    pub message: String,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("manifest is missing required field: {0}")]
    BadManifest(&'static str),
    #[error("no engine asset published for target `{0}`")]
    NoAssetForTarget(String),
    #[error("an install is already running")]
    AlreadyInstalling,
    #[error("settings error: {0}")]
    Settings(String),
}

impl From<EngineError> for String {
    fn from(err: EngineError) -> Self {
        err.to_string()
    }
}

// ---------------------------------------------------------------------------
// Lock so two installs cannot run concurrently
// ---------------------------------------------------------------------------

/// Application-wide state: a mutex guarding install operations.
pub struct EngineLock(pub Arc<Mutex<()>>);

impl Default for EngineLock {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(())))
    }
}

// ---------------------------------------------------------------------------
// Target resolution
// ---------------------------------------------------------------------------

/// Returns a stable identifier for the running OS+arch, matching the keys we
/// expect under `engine.assets` in the release manifest.
///
/// Same convention as Rust target triples — e.g. `aarch64-apple-darwin`.
pub fn current_target() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let triple = match (arch, os) {
        ("aarch64", "macos") => "aarch64-apple-darwin",
        ("x86_64", "macos") => "x86_64-apple-darwin",
        ("x86_64", "windows") => "x86_64-pc-windows-msvc",
        ("aarch64", "windows") => "aarch64-pc-windows-msvc",
        ("x86_64", "linux") => "x86_64-unknown-linux-gnu",
        ("aarch64", "linux") => "aarch64-unknown-linux-gnu",
        _ => return format!("{arch}-{os}"),
    };
    triple.to_owned()
}

// ---------------------------------------------------------------------------
// Manifest fetching
// ---------------------------------------------------------------------------

/// Top-level manifest as we expect it. Designed to be the GitHub Releases API
/// payload (we look at `assets[]`) — see `parse_github_release` below.
#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

/// Asset filename convention we expect on `aracdia-engine` releases:
/// `aracdia-engine-<target>.zip` paired with `aracdia-engine-<target>.zip.sha256`
/// (the sha256 file contains the hex digest, optionally followed by spaces and
/// the original filename, à la `sha256sum`).
fn asset_basename(target: &str) -> String {
    format!("aracdia-engine-{target}.zip")
}

fn http_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(MANIFEST_TIMEOUT)
        .build()
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, EngineError> {
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?;
    Ok(response.text().await?)
}

/// Reads settings to know where to fetch the manifest, then resolves a release
/// for the current target.
pub async fn resolve_release_for_current_target() -> Result<EngineRelease, EngineError> {
    let settings = settings::load_settings().map_err(EngineError::Settings)?;
    let target = current_target();

    let client = http_client()?;
    let body = fetch_text(&client, &settings.manifest_url).await?;
    let release: GithubRelease = serde_json::from_str(&body).map_err(|e| {
        EngineError::Settings(format!("invalid GitHub release JSON: {e}"))
    })?;

    let zip_name = asset_basename(&target);
    let sha_name = format!("{zip_name}.sha256");

    let zip_asset = release
        .assets
        .iter()
        .find(|a| a.name == zip_name)
        .ok_or_else(|| EngineError::NoAssetForTarget(target.clone()))?;

    let sha_asset = release
        .assets
        .iter()
        .find(|a| a.name == sha_name)
        .ok_or(EngineError::BadManifest("missing .sha256 sidecar"))?;

    let sha_body = fetch_text(&client, &sha_asset.browser_download_url).await?;
    let sha_hex = sha_body
        .split_whitespace()
        .next()
        .ok_or(EngineError::BadManifest("empty .sha256 sidecar"))?
        .to_owned();

    Ok(EngineRelease {
        version: release.tag_name,
        target,
        asset: EngineAsset {
            url: zip_asset.browser_download_url.clone(),
            sha256: sha_hex,
            size_bytes: zip_asset.size,
        },
    })
}

// ---------------------------------------------------------------------------
// Install pipeline
// ---------------------------------------------------------------------------

fn read_installed_version(custom_install: Option<&std::path::Path>) -> Option<String> {
    let path = paths::engine_version_file(custom_install).ok()?;
    fs::read_to_string(path).ok().map(|s| s.trim().to_owned())
}

fn write_installed_version(
    custom_install: Option<&std::path::Path>,
    version: &str,
) -> std::io::Result<()> {
    let path = paths::engine_version_file(custom_install)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, version.as_bytes())
}

fn current_install_dir() -> Option<std::path::PathBuf> {
    let s = settings::load_settings().ok()?;
    s.install_dir
}

async fn install_inner(
    app: &AppHandle,
    release: &EngineRelease,
) -> Result<(), EngineError> {
    let custom = current_install_dir();
    let custom_ref = custom.as_deref();

    let cache = paths::cache_dir()?;
    let zip_path = cache.join(format!("aracdia-engine-{}.zip", release.target));
    let dest = paths::engine_dir(custom_ref)?;

    // 1. Download
    let app_for_progress = app.clone();
    let progress_total = std::sync::atomic::AtomicU64::new(0);
    let on_progress = move |bytes_done: u64, bytes_total: Option<u64>| {
        // Keep the latest reported total so we always emit a coherent value.
        if let Some(t) = bytes_total {
            progress_total.store(t, std::sync::atomic::Ordering::Relaxed);
        }
        let total = match bytes_total {
            Some(t) => Some(t),
            None => match progress_total.load(std::sync::atomic::Ordering::Relaxed) {
                0 => None,
                v => Some(v),
            },
        };
        let _ = app_for_progress.emit(
            events::PROGRESS,
            InstallProgress {
                phase: InstallPhase::Downloading,
                bytes_done,
                bytes_total: total,
            },
        );
    };

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(60 * 30))
        .build()?;

    download::download_to_file(&client, &release.asset.url, &zip_path, &on_progress)
        .await?;

    // 2. Verify
    let _ = app.emit(
        events::PROGRESS,
        InstallProgress {
            phase: InstallPhase::Verifying,
            bytes_done: 0,
            bytes_total: Some(release.asset.size_bytes),
        },
    );
    download::verify_sha256(&zip_path, &release.asset.sha256)?;

    // 3. Extract
    let _ = app.emit(
        events::PROGRESS,
        InstallProgress {
            phase: InstallPhase::Extracting,
            bytes_done: 0,
            bytes_total: None,
        },
    );
    let zip_path_for_extract = zip_path.clone();
    let dest_for_extract = dest.clone();
    tokio::task::spawn_blocking(move || {
        download::extract_zip(&zip_path_for_extract, &dest_for_extract)
    })
    .await
    .map_err(|e| EngineError::Settings(format!("extract task panicked: {e}")))??;

    // 4. Marker file
    write_installed_version(custom_ref, &release.version)?;

    // 5. Cleanup the staged zip — best effort
    let _ = fs::remove_file(&zip_path);

    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn engine_status() -> Result<EngineStatus, String> {
    let custom = current_install_dir();
    let custom_ref = custom.as_deref();
    let dir = paths::engine_dir(custom_ref).map_err(|e| e.to_string())?;
    match read_installed_version(custom_ref) {
        Some(version) if dir.exists() => Ok(EngineStatus::Installed {
            version,
            path: dir.to_string_lossy().into_owned(),
        }),
        _ => Ok(EngineStatus::NotInstalled),
    }
}

#[tauri::command]
pub fn engine_current_target() -> String {
    current_target()
}

#[tauri::command]
pub async fn fetch_engine_release() -> Result<EngineRelease, String> {
    resolve_release_for_current_target()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_engine(
    app: AppHandle,
    lock: State<'_, EngineLock>,
    release: EngineRelease,
) -> Result<(), String> {
    let mutex = lock.0.clone();
    let guard = mutex.try_lock();
    let _guard = match guard {
        Ok(g) => g,
        Err(_) => return Err(EngineError::AlreadyInstalling.to_string()),
    };

    match install_inner(&app, &release).await {
        Ok(()) => {
            let _ = app.emit(
                events::COMPLETE,
                InstallComplete {
                    version: release.version.clone(),
                },
            );
            Ok(())
        }
        Err(e) => {
            let msg = e.to_string();
            let _ = app.emit(
                events::ERROR,
                InstallError {
                    message: msg.clone(),
                },
            );
            Err(msg)
        }
    }
}

#[tauri::command]
pub fn uninstall_engine() -> Result<(), String> {
    let custom = current_install_dir();
    let dir = paths::engine_dir(custom.as_deref()).map_err(|e| e.to_string())?;
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_target_is_non_empty() {
        let t = current_target();
        assert!(!t.is_empty());
        assert!(t.contains('-'));
    }

    #[test]
    fn asset_basename_format() {
        assert_eq!(
            asset_basename("aarch64-apple-darwin"),
            "aracdia-engine-aarch64-apple-darwin.zip"
        );
    }
}
