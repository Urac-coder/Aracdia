//! Persisted launcher settings.
//!
//! Stored as a single JSON file next to `profile.json`. Designed to be
//! forward-compatible: unknown fields in the file are preserved, missing
//! fields fall back to defaults — so adding new settings later is safe.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::paths;

/// Hard limits — kept in sync with frontend validation.
const MEMORY_MIN_MB: u32 = 512;
const MEMORY_MAX_MB: u32 = 32_768;
const PORT_MIN: u16 = 1;
const PORT_MAX: u16 = 65_535;
const SERVER_ADDR_MAX: usize = 253; // RFC 1035 max DNS name length

/// Default values used both as fallbacks and as the "reset" target.
const DEFAULT_MEMORY_MB: u32 = 2048;
/// Default address for a *remote* server (e.g. a future VPS). Empty means
/// "use the launcher-managed local server".
const DEFAULT_SERVER_ADDRESS: &str = "";
const DEFAULT_SERVER_PORT: u16 = 30_000;
/// The launcher always auto-connects to the configured server (local or
/// remote). The Luanti main menu is never shown to the player.
const DEFAULT_AUTO_CONNECT: bool = true;
/// Default port the launcher-managed local server listens on.
const DEFAULT_LOCAL_SERVER_PORT: u16 = 30_000;
/// Default bind address: all interfaces, so LAN friends can join immediately
/// (Internet still requires port forwarding or a tunnel — documented in the
/// "Server" panel of the launcher).
const DEFAULT_LOCAL_SERVER_BIND: &str = "0.0.0.0";
/// Default manifest URL: the GitHub Releases API "latest" endpoint of the
/// `aracdia-engine` repo. Configurable so users/devs can point the launcher
/// at a custom fork while iterating on engine builds.
const DEFAULT_MANIFEST_URL: &str =
    "https://api.github.com/repos/Urac-coder/aracdia-engine/releases/latest";
/// Default content manifest URL: the GitHub Releases API endpoint that lists
/// recent releases of the Aracdia monorepo. The launcher filters those
/// releases by tag prefix `game-v` and picks the newest. Configurable so
/// devs can point at a fork while iterating on game content.
const DEFAULT_CONTENT_MANIFEST_URL: &str =
    "https://api.github.com/repos/Urac-coder/Aracdia/releases?per_page=30";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSettings {
    /// Engine RAM allocation in MiB (passed through to the engine on launch).
    pub memory_mb: u32,
    /// Default server address used when clicking "Play" (empty = singleplayer).
    pub server_address: String,
    /// Default server port.
    pub server_port: u16,
    /// If true, "Play" auto-connects to the configured server. Otherwise it
    /// opens the engine's own server browser.
    pub auto_connect: bool,
    /// Custom install dir for engine + game content. `None` = use OS default.
    #[serde(default)]
    pub install_dir: Option<PathBuf>,
    /// URL where the launcher fetches the engine release manifest. Allows
    /// overriding the default `aracdia-engine` repo for testing.
    #[serde(default = "default_manifest_url")]
    pub manifest_url: String,
    /// URL where the launcher fetches the *content* (game) release manifest.
    /// Releases under this endpoint are filtered by tag prefix `game-v`.
    #[serde(default = "default_content_manifest_url")]
    pub content_manifest_url: String,
    /// Port for the launcher-managed local Aracdia server.
    #[serde(default = "default_local_server_port")]
    pub local_server_port: u16,
    /// Bind address for the local server. `0.0.0.0` exposes it on the LAN
    /// (and on Internet if the user forwards the port). `127.0.0.1` keeps
    /// it strictly local to the machine.
    #[serde(default = "default_local_server_bind")]
    pub local_server_bind: String,
}

fn default_manifest_url() -> String {
    DEFAULT_MANIFEST_URL.to_owned()
}

fn default_content_manifest_url() -> String {
    DEFAULT_CONTENT_MANIFEST_URL.to_owned()
}

fn default_local_server_port() -> u16 {
    DEFAULT_LOCAL_SERVER_PORT
}

fn default_local_server_bind() -> String {
    DEFAULT_LOCAL_SERVER_BIND.to_owned()
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            memory_mb: DEFAULT_MEMORY_MB,
            server_address: DEFAULT_SERVER_ADDRESS.to_owned(),
            server_port: DEFAULT_SERVER_PORT,
            auto_connect: DEFAULT_AUTO_CONNECT,
            install_dir: None,
            manifest_url: default_manifest_url(),
            content_manifest_url: default_content_manifest_url(),
            local_server_port: default_local_server_port(),
            local_server_bind: default_local_server_bind(),
        }
    }
}

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("invalid setting: {0}")]
    Invalid(&'static str),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid settings file: {0}")]
    Parse(#[from] serde_json::Error),
}

impl From<SettingsError> for String {
    fn from(err: SettingsError) -> Self {
        err.to_string()
    }
}

fn settings_file() -> Result<PathBuf, std::io::Error> {
    Ok(paths::data_dir()?.join("settings.json"))
}

fn validate(settings: &LauncherSettings) -> Result<(), SettingsError> {
    if settings.memory_mb < MEMORY_MIN_MB {
        return Err(SettingsError::Invalid("memory_mb is below the minimum"));
    }
    if settings.memory_mb > MEMORY_MAX_MB {
        return Err(SettingsError::Invalid("memory_mb is above the maximum"));
    }
    if settings.server_port < PORT_MIN || settings.server_port > PORT_MAX {
        return Err(SettingsError::Invalid("server_port is out of range"));
    }
    if settings.server_address.len() > SERVER_ADDR_MAX {
        return Err(SettingsError::Invalid("server_address is too long"));
    }
    // Allow empty (means: no default server). Otherwise require non-whitespace.
    if !settings.server_address.is_empty()
        && settings.server_address.trim().is_empty()
    {
        return Err(SettingsError::Invalid("server_address is whitespace only"));
    }
    if settings.manifest_url.trim().is_empty() {
        return Err(SettingsError::Invalid("manifest_url is empty"));
    }
    if !(settings.manifest_url.starts_with("https://")
        || settings.manifest_url.starts_with("http://"))
    {
        return Err(SettingsError::Invalid("manifest_url must be http(s)"));
    }
    if settings.content_manifest_url.trim().is_empty() {
        return Err(SettingsError::Invalid("content_manifest_url is empty"));
    }
    if !(settings.content_manifest_url.starts_with("https://")
        || settings.content_manifest_url.starts_with("http://"))
    {
        return Err(SettingsError::Invalid("content_manifest_url must be http(s)"));
    }
    if settings.local_server_port < PORT_MIN || settings.local_server_port > PORT_MAX {
        return Err(SettingsError::Invalid("local_server_port is out of range"));
    }
    let bind = settings.local_server_bind.trim();
    if bind.is_empty() {
        return Err(SettingsError::Invalid("local_server_bind is empty"));
    }
    if bind.parse::<std::net::IpAddr>().is_err() {
        return Err(SettingsError::Invalid("local_server_bind must be a valid IP address"));
    }
    Ok(())
}

/// Reads the persisted settings, returning defaults if the file does not exist
/// (or is unreadable — we never want a bad file to lock the user out).
#[tauri::command]
pub fn load_settings() -> Result<LauncherSettings, String> {
    let path = settings_file().map_err(SettingsError::from)?;
    if !path.exists() {
        return Ok(LauncherSettings::default());
    }
    let bytes = fs::read(&path).map_err(SettingsError::from)?;
    match serde_json::from_slice::<LauncherSettings>(&bytes) {
        Ok(settings) => Ok(settings),
        Err(err) => {
            // Corrupt file: log and return defaults rather than crashing the launcher.
            eprintln!(
                "[settings] failed to parse {}: {err}; falling back to defaults",
                path.display()
            );
            Ok(LauncherSettings::default())
        }
    }
}

/// Persists the given settings after validating them.
#[tauri::command]
pub fn save_settings(settings: LauncherSettings) -> Result<LauncherSettings, String> {
    validate(&settings)?;
    paths::ensure_data_dir().map_err(SettingsError::from)?;

    let path = settings_file().map_err(SettingsError::from)?;
    let json = serde_json::to_vec_pretty(&settings).map_err(SettingsError::from)?;
    fs::write(&path, json).map_err(SettingsError::from)?;
    Ok(settings)
}

/// Resets settings to the built-in defaults and persists the result.
#[tauri::command]
pub fn reset_settings() -> Result<LauncherSettings, String> {
    save_settings(LauncherSettings::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_pass_validation() {
        validate(&LauncherSettings::default()).unwrap();
    }

    #[test]
    fn rejects_low_memory() {
        let mut s = LauncherSettings::default();
        s.memory_mb = 0;
        assert!(validate(&s).is_err());
    }

    #[test]
    fn rejects_zero_port() {
        let mut s = LauncherSettings::default();
        s.server_port = 0;
        assert!(validate(&s).is_err());
    }
}
