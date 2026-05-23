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
const DEFAULT_SERVER_ADDRESS: &str = "";
const DEFAULT_SERVER_PORT: u16 = 30_000;
const DEFAULT_AUTO_CONNECT: bool = false;

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
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            memory_mb: DEFAULT_MEMORY_MB,
            server_address: DEFAULT_SERVER_ADDRESS.to_owned(),
            server_port: DEFAULT_SERVER_PORT,
            auto_connect: DEFAULT_AUTO_CONNECT,
            install_dir: None,
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
