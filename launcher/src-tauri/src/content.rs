//! Game content install pipeline: manifest fetch, download, verify, extract.
//!
//! Mirrors `engine.rs` but for the cross-OS Aracdia *game* (Lua mods, textures,
//! menu assets). Releases are tagged `game-v<version>` and ship a single
//! universal asset `aracdia-game.zip` paired with a `.sha256` sidecar.
//!
//! The destination is `<luanti_user>/games/<gameid>/`. A marker file
//! `.aracdia-content-version` records the deployed version so that:
//! - the launcher can show "Content vX.Y.Z" / "update available";
//! - `game::deploy_game` (which copies the bundled fallback) does not
//!   overwrite a freshly downloaded content tree.

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

/// Marker file written next to the deployed game content. Distinct from the
/// `.aracdia-game-version` written by the bundle deploy path so we can tell
/// the two apart.
pub const CONTENT_MARKER_FILE: &str = ".aracdia-content-version";

const USER_AGENT: &str = "AracdiaLauncher/0.1 (+https://aracdia.example)";
const MANIFEST_TIMEOUT: Duration = Duration::from_secs(15);
const TAG_PREFIX: &str = "game-v";
const ASSET_NAME: &str = "aracdia-game.zip";

mod events {
    pub const PROGRESS: &str = "content://progress";
    pub const COMPLETE: &str = "content://complete";
    pub const ERROR: &str = "content://error";
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContentAsset {
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContentRelease {
    /// Version stripped of the `game-v` prefix (e.g. `0.1.0`).
    pub version: String,
    /// Original git tag (e.g. `game-v0.1.0`).
    pub tag: String,
    pub asset: ContentAsset,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ContentStatus {
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
pub enum ContentError {
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("manifest is missing required field: {0}")]
    BadManifest(&'static str),
    #[error("no release with tag prefix `{TAG_PREFIX}` found in manifest")]
    NoMatchingRelease,
    #[error("an install is already running")]
    AlreadyInstalling,
    #[error("settings error: {0}")]
    Settings(String),
    #[error("game.conf is missing — is the bundle malformed?")]
    MissingGameConf,
}

impl From<ContentError> for String {
    fn from(err: ContentError) -> Self {
        err.to_string()
    }
}

// ---------------------------------------------------------------------------
// Lock so two installs cannot run concurrently
// ---------------------------------------------------------------------------

/// Application-wide state: a mutex guarding install operations.
pub struct ContentLock(pub Arc<Mutex<()>>);

impl Default for ContentLock {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(())))
    }
}

// ---------------------------------------------------------------------------
// GitHub release manifest parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

fn http_client(timeout: Duration) -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(timeout)
        .build()
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, ContentError> {
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?;
    Ok(response.text().await?)
}

/// Filters releases by tag prefix and picks the newest.
fn pick_release(releases: Vec<GithubRelease>) -> Option<GithubRelease> {
    releases
        .into_iter()
        .filter(|r| !r.draft && r.tag_name.starts_with(TAG_PREFIX))
        .max_by(|a, b| {
            // Use `published_at` when available, fall back to lexicographic
            // tag comparison (works fine for semver-y tags).
            match (&a.published_at, &b.published_at) {
                (Some(x), Some(y)) => x.cmp(y),
                _ => a.tag_name.cmp(&b.tag_name),
            }
        })
}

pub async fn resolve_latest_release() -> Result<ContentRelease, ContentError> {
    let settings = settings::load_settings().map_err(ContentError::Settings)?;

    let client = http_client(MANIFEST_TIMEOUT)?;
    let body = fetch_text(&client, &settings.content_manifest_url).await?;

    // The endpoint can be either a single-release object (`/releases/latest`)
    // or an array (`/releases?...`). Handle both.
    let releases: Vec<GithubRelease> =
        match serde_json::from_str::<Vec<GithubRelease>>(&body) {
            Ok(v) => v,
            Err(_) => match serde_json::from_str::<GithubRelease>(&body) {
                Ok(single) => vec![single],
                Err(e) => {
                    return Err(ContentError::Settings(format!(
                        "invalid GitHub release JSON: {e}"
                    )));
                }
            },
        };

    let release = pick_release(releases).ok_or(ContentError::NoMatchingRelease)?;

    // Skip prereleases unless they are the only candidates. Re-running pick
    // would be ideal but keeping the simple path: if the picked one is a
    // prerelease we still accept it for now (stable channel comes later).
    let _ = release.prerelease;

    let zip = release
        .assets
        .iter()
        .find(|a| a.name == ASSET_NAME)
        .ok_or(ContentError::BadManifest("aracdia-game.zip"))?;
    let sha = release
        .assets
        .iter()
        .find(|a| a.name == format!("{ASSET_NAME}.sha256"))
        .ok_or(ContentError::BadManifest("aracdia-game.zip.sha256"))?;

    let sha_body = fetch_text(&client, &sha.browser_download_url).await?;
    let sha_hex = sha_body
        .split_whitespace()
        .next()
        .ok_or(ContentError::BadManifest("empty .sha256 sidecar"))?
        .to_owned();

    let version = release
        .tag_name
        .strip_prefix(TAG_PREFIX)
        .unwrap_or(&release.tag_name)
        .to_owned();

    Ok(ContentRelease {
        version,
        tag: release.tag_name,
        asset: ContentAsset {
            url: zip.browser_download_url.clone(),
            sha256: sha_hex,
            size_bytes: zip.size,
        },
    })
}

// ---------------------------------------------------------------------------
// Status reading
// ---------------------------------------------------------------------------

fn deployed_dir(gameid: &str) -> Result<std::path::PathBuf, std::io::Error> {
    Ok(paths::luanti_user_games_dir()?.join(gameid))
}

pub fn read_installed_version(gameid: &str) -> Option<String> {
    let dir = deployed_dir(gameid).ok()?;
    let marker = dir.join(CONTENT_MARKER_FILE);
    fs::read_to_string(marker).ok().map(|s| s.trim().to_owned())
}

fn read_gameid_from_dir(source: &std::path::Path) -> Result<String, ContentError> {
    let conf = source.join("game.conf");
    let content =
        fs::read_to_string(&conf).map_err(|_| ContentError::MissingGameConf)?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("name") {
            if let Some(rest) = rest.trim_start().strip_prefix('=') {
                let id = rest.trim();
                if !id.is_empty() {
                    return Ok(id.to_owned());
                }
            }
        }
    }
    Err(ContentError::MissingGameConf)
}

// ---------------------------------------------------------------------------
// Install pipeline
// ---------------------------------------------------------------------------

async fn install_inner(
    app: &AppHandle,
    release: &ContentRelease,
) -> Result<String, ContentError> {
    let cache = paths::cache_dir()?;
    let zip_path = cache.join(format!("aracdia-game-{}.zip", release.version));

    // Stage the zip into a fresh tmp directory so we can extract, validate
    // game.conf, and only then atomically swap into the final location.
    let stage = paths::cache_dir()?.join(format!("staging-content-{}", release.version));
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    fs::create_dir_all(&stage)?;

    // 1. Download
    let app_for_progress = app.clone();
    let progress_total = std::sync::atomic::AtomicU64::new(0);
    let on_progress = move |bytes_done: u64, bytes_total: Option<u64>| {
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

    let client = http_client(Duration::from_secs(60 * 30))?;
    download::download_to_file(&client, &release.asset.url, &zip_path, &on_progress).await?;

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

    // 3. Extract into staging
    let _ = app.emit(
        events::PROGRESS,
        InstallProgress {
            phase: InstallPhase::Extracting,
            bytes_done: 0,
            bytes_total: None,
        },
    );
    let zip_for_extract = zip_path.clone();
    let stage_for_extract = stage.clone();
    tokio::task::spawn_blocking(move || {
        download::extract_zip(&zip_for_extract, &stage_for_extract)
    })
    .await
    .map_err(|e| ContentError::Settings(format!("extract task panicked: {e}")))??;

    // 4. Validate that the staged tree looks like a game (has game.conf at the
    //    root) and resolve gameid.
    let gameid = read_gameid_from_dir(&stage)?;

    // 5. Atomic swap into <luanti_user>/games/<gameid>/
    let dest = deployed_dir(&gameid)?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    // Rename the staging dir into place. Same parent (cache dir) and games
    // dir might be on different filesystems on Linux/Windows so fall back to
    // a recursive copy if rename fails with EXDEV.
    if let Err(err) = fs::rename(&stage, &dest) {
        if err.raw_os_error() == Some(libc_exdev()) || err.kind() == std::io::ErrorKind::Other
        {
            copy_dir_recursive(&stage, &dest)?;
            let _ = fs::remove_dir_all(&stage);
        } else {
            return Err(ContentError::Io(err));
        }
    }

    // 6. Marker file with the version
    fs::write(dest.join(CONTENT_MARKER_FILE), release.version.as_bytes())?;

    // 7. Best-effort cleanup of the cached zip
    let _ = fs::remove_file(&zip_path);

    Ok(gameid)
}

/// Cross-OS EXDEV constant (different errno values on different libcs); we
/// prefer to detect the cross-device-link error symbolically rather than
/// hard-coding magic numbers per platform. On platforms where the value is
/// known we use it; everywhere else we treat any rename failure as a fallback.
#[cfg(unix)]
fn libc_exdev() -> i32 {
    18 // EXDEV on Linux/macOS
}
#[cfg(not(unix))]
fn libc_exdev() -> i32 {
    -1
}

fn copy_dir_recursive(
    src: &std::path::Path,
    dst: &std::path::Path,
) -> Result<(), std::io::Error> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ftype = entry.file_type()?;
        if ftype.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Returns the currently deployed content version, by inspecting the marker
/// file. The default game id is `aracdia` (we do not yet support multiple
/// games — when we do this command will accept a gameid arg).
#[tauri::command]
pub fn content_status() -> Result<ContentStatus, String> {
    // Use the bundled game.conf to find what gameid we expect — this stays
    // correct even after a future rename. The bundle is always present in
    // a built launcher, but during `tauri dev` we fall back to the repo path.
    let gameid = bundled_gameid().unwrap_or_else(|| "aracdia".to_owned());
    let dir = deployed_dir(&gameid).map_err(|e| e.to_string())?;
    match read_installed_version(&gameid) {
        Some(version) if dir.exists() => Ok(ContentStatus::Installed {
            version,
            path: dir.to_string_lossy().into_owned(),
        }),
        _ => Ok(ContentStatus::NotInstalled),
    }
}

fn bundled_gameid() -> Option<String> {
    #[cfg(debug_assertions)]
    {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if let Some(repo) = manifest.parent().and_then(|p| p.parent()) {
            let conf = repo.join("game").join("game.conf");
            if conf.exists() {
                if let Ok(s) = fs::read_to_string(&conf) {
                    for line in s.lines() {
                        if let Some(v) = line.trim().strip_prefix("name") {
                            if let Some(v) = v.trim_start().strip_prefix('=') {
                                let id = v.trim();
                                if !id.is_empty() {
                                    return Some(id.to_owned());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

#[tauri::command]
pub async fn fetch_content_release() -> Result<ContentRelease, String> {
    resolve_latest_release().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_content(
    app: AppHandle,
    state: State<'_, ContentLock>,
    release: ContentRelease,
) -> Result<InstallComplete, String> {
    let lock = state.0.clone();
    let guard = lock
        .try_lock_owned()
        .map_err(|_| ContentError::AlreadyInstalling.to_string())?;

    let result = install_inner(&app, &release).await;
    drop(guard);

    match result {
        Ok(_gameid) => {
            let payload = InstallComplete {
                version: release.version.clone(),
            };
            let _ = app.emit(events::COMPLETE, payload.clone());
            Ok(payload)
        }
        Err(err) => {
            let message = err.to_string();
            let _ = app.emit(
                events::ERROR,
                InstallError {
                    message: message.clone(),
                },
            );
            Err(message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, published: Option<&str>) -> GithubRelease {
        GithubRelease {
            tag_name: tag.to_owned(),
            draft: false,
            prerelease: false,
            published_at: published.map(|s| s.to_owned()),
            assets: vec![],
        }
    }

    #[test]
    fn pick_release_skips_drafts_and_unrelated_tags() {
        let mut r = release("game-v0.1.0", Some("2026-05-23T10:00:00Z"));
        r.draft = true;
        let releases = vec![
            r,
            release("v0.1.0", Some("2026-05-23T11:00:00Z")), // wrong prefix
            release("game-v0.0.1", Some("2026-05-22T10:00:00Z")),
            release("game-v0.2.0", Some("2026-05-23T12:00:00Z")),
        ];
        let picked = pick_release(releases).unwrap();
        assert_eq!(picked.tag_name, "game-v0.2.0");
    }

    #[test]
    fn pick_release_returns_none_when_no_match() {
        let releases = vec![release("v1.0.0", None), release("hotfix-foo", None)];
        assert!(pick_release(releases).is_none());
    }

    #[test]
    fn pick_release_falls_back_to_tag_order_without_dates() {
        let releases = vec![
            release("game-v0.1.0", None),
            release("game-v0.2.0", None),
            release("game-v0.1.5", None),
        ];
        let picked = pick_release(releases).unwrap();
        assert_eq!(picked.tag_name, "game-v0.2.0");
    }
}
