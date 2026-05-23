//! Cross-platform directories used by the launcher.
//!
//! All resolved paths follow the OS conventions via the `directories` crate:
//! - macOS:   `~/Library/Application Support/com.aracdia.launcher/`
//! - Windows: `%APPDATA%\Aracdia\Launcher\`
//! - Linux:   `~/.local/share/aracdia-launcher/` (XDG_DATA_HOME)

use std::path::PathBuf;

use directories::ProjectDirs;
use once_cell::sync::Lazy;

/// Application identifiers used to compute platform-specific directories.
const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "Aracdia";
const APPLICATION: &str = "Launcher";

static PROJECT_DIRS: Lazy<Option<ProjectDirs>> =
    Lazy::new(|| ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION));

/// Returns the directory where launcher state (profile, config) is persisted.
pub fn data_dir() -> Result<PathBuf, std::io::Error> {
    PROJECT_DIRS
        .as_ref()
        .map(|d| d.data_dir().to_path_buf())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "could not resolve OS data directory",
            )
        })
}

/// Returns the path of the persisted offline profile JSON file.
pub fn profile_file() -> Result<PathBuf, std::io::Error> {
    Ok(data_dir()?.join("profile.json"))
}

/// Ensures `data_dir()` exists, creating it (and parents) if necessary.
pub fn ensure_data_dir() -> Result<PathBuf, std::io::Error> {
    let dir = data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
