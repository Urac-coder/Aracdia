//! Cross-platform directories used by the launcher.
//!
//! All resolved paths follow the OS conventions via the `directories` crate:
//! - macOS:   `~/Library/Application Support/com.aracdia.launcher/`
//! - Windows: `%APPDATA%\Aracdia\Launcher\`
//! - Linux:   `~/.local/share/aracdia-launcher/` (XDG_DATA_HOME)

use std::path::{Path, PathBuf};

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

/// Resolves the install root: a custom path if the user set one, otherwise
/// the OS data dir. The directory is created on demand.
pub fn install_root(custom: Option<&Path>) -> Result<PathBuf, std::io::Error> {
    let root = match custom {
        Some(p) => p.to_path_buf(),
        None => data_dir()?,
    };
    std::fs::create_dir_all(&root)?;
    Ok(root)
}

/// Returns the directory where the engine is extracted (single-version policy).
pub fn engine_dir(custom_install: Option<&Path>) -> Result<PathBuf, std::io::Error> {
    Ok(install_root(custom_install)?.join("engine"))
}

/// Returns the path of the marker file written after a successful engine
/// install. Its presence + version means "the engine is ready to launch".
pub fn engine_version_file(custom_install: Option<&Path>) -> Result<PathBuf, std::io::Error> {
    Ok(engine_dir(custom_install)?.join(".aracdia-version"))
}

/// Returns the directory used for staging downloads before they are verified
/// and extracted into their final location.
pub fn cache_dir() -> Result<PathBuf, std::io::Error> {
    let dir = data_dir()?.join("cache");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
