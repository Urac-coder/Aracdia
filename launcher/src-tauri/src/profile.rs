//! Offline player profile: a stable username + UUID persisted on disk.
//!
//! This is the identity layer used until we plug a real auth server.
//! The profile lives in `paths::profile_file()` as a JSON file.

use std::fs;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::paths;

/// Username constraints — kept in sync with the frontend rules.
const USERNAME_MIN: usize = 3;
const USERNAME_MAX: usize = 16;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfile {
    pub id: Uuid,
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("invalid username: {0}")]
    InvalidUsername(&'static str),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid profile file: {0}")]
    Parse(#[from] serde_json::Error),
}

// Tauri commands return errors as strings to keep the JS side simple.
impl From<ProfileError> for String {
    fn from(err: ProfileError) -> Self {
        err.to_string()
    }
}

fn validate_username(value: &str) -> Result<String, ProfileError> {
    let trimmed = value.trim();
    if trimmed.len() < USERNAME_MIN {
        return Err(ProfileError::InvalidUsername("username too short"));
    }
    if trimmed.len() > USERNAME_MAX {
        return Err(ProfileError::InvalidUsername("username too long"));
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(ProfileError::InvalidUsername(
            "username contains invalid characters",
        ));
    }
    Ok(trimmed.to_owned())
}

/// Reads the persisted profile from disk, or returns `None` if none exists.
#[tauri::command]
pub fn load_profile() -> Result<Option<PlayerProfile>, String> {
    let path = paths::profile_file().map_err(ProfileError::from)?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(ProfileError::from)?;
    let profile: PlayerProfile =
        serde_json::from_slice(&bytes).map_err(ProfileError::from)?;
    Ok(Some(profile))
}

/// Creates a new profile or updates the existing one with a new username.
/// The internal UUID is preserved across renames so the player keeps their identity.
#[tauri::command]
pub fn save_profile(username: String) -> Result<PlayerProfile, String> {
    let username = validate_username(&username)?;
    paths::ensure_data_dir().map_err(ProfileError::from)?;

    let path = paths::profile_file().map_err(ProfileError::from)?;
    let now = Utc::now();

    let existing = if path.exists() {
        let bytes = fs::read(&path).map_err(ProfileError::from)?;
        serde_json::from_slice::<PlayerProfile>(&bytes).ok()
    } else {
        None
    };

    let profile = match existing {
        Some(prev) => PlayerProfile {
            id: prev.id,
            username,
            created_at: prev.created_at,
            updated_at: now,
        },
        None => PlayerProfile {
            id: Uuid::new_v4(),
            username,
            created_at: now,
            updated_at: now,
        },
    };

    let json = serde_json::to_vec_pretty(&profile).map_err(ProfileError::from)?;
    fs::write(&path, json).map_err(ProfileError::from)?;

    Ok(profile)
}

/// Removes the persisted profile (used for "log out").
#[tauri::command]
pub fn clear_profile() -> Result<(), String> {
    let path = paths::profile_file().map_err(ProfileError::from)?;
    if path.exists() {
        fs::remove_file(&path).map_err(ProfileError::from)?;
    }
    Ok(())
}
