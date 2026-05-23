//! Game content deployment.
//!
//! The launcher ships the Aracdia `game/` folder bundled as a Tauri resource.
//! Before the engine is spawned, this module copies (or refreshes) it into
//! the location Luanti scans for installable games — `<luanti_user>/games/<id>/`.
//!
//! The deploy is idempotent: a SHA-256 signature of the source tree is
//! written into a marker file in the destination, and re-deploys are skipped
//! when the destination signature matches the source. That keeps subsequent
//! launches instant while still picking up dev-time edits to `game/`.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};
use thiserror::Error;

use crate::paths;

const MARKER_FILE: &str = ".aracdia-game-version";

#[derive(Debug, Error)]
pub enum GameError {
    #[error("game source folder not found at any of: {0}")]
    SourceNotFound(String),
    #[error("malformed game.conf: {0}")]
    MalformedConf(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<GameError> for String {
    fn from(err: GameError) -> Self {
        err.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct DeployedGame {
    pub gameid: String,
    pub source_dir: PathBuf,
    pub deployed_dir: PathBuf,
}

// ---------------------------------------------------------------------------
// Source resolution: prefer Tauri resources, fall back to the repo `game/`
// when running `tauri dev` against the source tree.
// ---------------------------------------------------------------------------

fn resolve_source(app: &AppHandle) -> Result<PathBuf, GameError> {
    let mut tried = Vec::new();

    // Dev builds read `game/` straight from the repo so Lua/texture edits
    // show up on the next launch without rebuilding the Tauri bundle.
    #[cfg(debug_assertions)]
    if let Some(candidate) = repo_game_dir() {
        if candidate.is_dir() {
            return Ok(candidate);
        }
        tried.push(candidate.display().to_string());
    }

    // Production (and dev fallback): bundled Tauri resource.
    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join("game");
        if candidate.is_dir() {
            return Ok(candidate);
        }
        tried.push(candidate.display().to_string());
    }

    // Last-resort repo lookup for debug builds when the bundle is missing.
    #[cfg(debug_assertions)]
    if let Some(candidate) = repo_game_dir() {
        if candidate.is_dir() {
            return Ok(candidate);
        }
        if !tried.iter().any(|t| t == &candidate.display().to_string()) {
            tried.push(candidate.display().to_string());
        }
    }

    Err(GameError::SourceNotFound(tried.join(", ")))
}

#[cfg(debug_assertions)]
fn repo_game_dir() -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .map(|repo_root| repo_root.join("game"))
}

// ---------------------------------------------------------------------------
// game.conf parsing — only the `name = ...` line really matters here.
// ---------------------------------------------------------------------------

fn read_gameid(source: &Path) -> Result<String, GameError> {
    let conf = source.join("game.conf");
    let content = std::fs::read_to_string(&conf)
        .map_err(|e| GameError::MalformedConf(format!("could not read {conf:?}: {e}")))?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("name") {
            let value = value.trim_start();
            if let Some(stripped) = value.strip_prefix('=') {
                let id = stripped.trim();
                if !id.is_empty() {
                    return Ok(id.to_owned());
                }
            }
        }
    }
    Err(GameError::MalformedConf(
        "missing required `name = <id>` line".to_owned(),
    ))
}

// ---------------------------------------------------------------------------
// Tree signature: SHA-256 over (sorted) (relative path, file contents).
// Symlinks are followed; the marker file itself is excluded.
// ---------------------------------------------------------------------------

fn signature_of_tree(root: &Path) -> Result<String, GameError> {
    let mut entries: Vec<PathBuf> = Vec::new();
    collect_files(root, root, &mut entries)?;
    entries.sort();

    let mut hasher = Sha256::new();
    for rel in &entries {
        let abs = root.join(rel);
        let bytes = std::fs::read(&abs)?;
        // Length-prefix the relative path so two files with concatenable
        // contents can't collide.
        let path_bytes = rel.to_string_lossy().into_owned();
        hasher.update((path_bytes.len() as u64).to_le_bytes());
        hasher.update(path_bytes.as_bytes());
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn collect_files(
    root: &Path,
    cur: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(cur)? {
        let entry = entry?;
        let path = entry.path();
        // Skip the marker file (we'll write it after copy)
        if path.file_name().map(|n| n == MARKER_FILE).unwrap_or(false) {
            continue;
        }
        let ftype = entry.file_type()?;
        if ftype.is_dir() {
            collect_files(root, &path, out)?;
        } else if ftype.is_file() || ftype.is_symlink() {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
                .to_path_buf();
            out.push(rel);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Recursive copy
// ---------------------------------------------------------------------------

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ftype = entry.file_type()?;
        if ftype.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if ftype.is_file() {
            std::fs::copy(&from, &to)?;
        } else if ftype.is_symlink() {
            // Follow the symlink and copy the target as a regular file
            let real = std::fs::read_link(&from)?;
            let real_full = if real.is_absolute() {
                real
            } else {
                from.parent().map(|p| p.join(&real)).unwrap_or(real)
            };
            if real_full.is_dir() {
                copy_dir_recursive(&real_full, &to)?;
            } else {
                std::fs::copy(&real_full, &to)?;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Public deploy entry point
// ---------------------------------------------------------------------------

/// Copies the bundled `game/` to Luanti's `<user>/games/<gameid>/`. Skips the
/// copy when:
/// - the destination already matches the source signature, or
/// - the destination was populated by a content download (presence of
///   `crate::content::CONTENT_MARKER_FILE`). In that case the downloaded
///   tree is authoritative and the bundle is only a fallback for first runs
///   without internet.
///
/// Debug builds (`tauri dev`) always deploy from the local `game/` tree so
/// content edits are visible without publishing a GitHub release.
pub fn deploy_game(app: &AppHandle) -> Result<DeployedGame, GameError> {
    let source = resolve_source(app)?;
    let gameid = read_gameid(&source)?;

    let games_root = paths::luanti_user_games_dir()?;
    std::fs::create_dir_all(&games_root)?;

    let dest = games_root.join(&gameid);

    // Downloaded releases win in production; dev builds override them so
    // iterating on `game/mods/` does not require wiping App Support by hand.
    let content_downloaded = dest.join(crate::content::CONTENT_MARKER_FILE).exists();
    if content_downloaded && !cfg!(debug_assertions) {
        return Ok(DeployedGame {
            gameid,
            source_dir: source,
            deployed_dir: dest,
        });
    }

    let marker = dest.join(MARKER_FILE);
    let sig = signature_of_tree(&source)?;

    let already = std::fs::read_to_string(&marker)
        .ok()
        .map(|s| s.trim().to_owned());
    if already.as_deref() == Some(sig.as_str()) {
        return Ok(DeployedGame {
            gameid,
            source_dir: source,
            deployed_dir: dest,
        });
    }

    if dest.exists() {
        std::fs::remove_dir_all(&dest)?;
    }
    copy_dir_recursive(&source, &dest)?;
    std::fs::write(&marker, sig)?;

    Ok(DeployedGame {
        gameid,
        source_dir: source,
        deployed_dir: dest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    #[test]
    fn read_gameid_parses_name_line() {
        let dir = tempdir();
        std::fs::write(dir.join("game.conf"), "name = aracdia\ntitle = Aracdia\n").unwrap();
        let id = read_gameid(&dir).unwrap();
        assert_eq!(id, "aracdia");
    }

    #[test]
    fn read_gameid_ignores_comments_and_blank_lines() {
        let dir = tempdir();
        std::fs::write(
            dir.join("game.conf"),
            "# leading comment\n\n   name = aracdia_xp \n",
        )
        .unwrap();
        let id = read_gameid(&dir).unwrap();
        assert_eq!(id, "aracdia_xp");
    }

    #[test]
    fn signature_changes_when_a_file_changes() {
        let dir = tempdir();
        std::fs::write(dir.join("a.txt"), b"hello").unwrap();
        std::fs::write(dir.join("b.txt"), b"world").unwrap();
        let s1 = signature_of_tree(&dir).unwrap();

        std::fs::write(dir.join("a.txt"), b"hello!").unwrap();
        let s2 = signature_of_tree(&dir).unwrap();

        assert_ne!(s1, s2);
    }

    #[test]
    fn signature_stable_across_runs_with_same_content() {
        let dir = tempdir();
        std::fs::create_dir_all(dir.join("nested")).unwrap();
        std::fs::write(dir.join("nested/x.txt"), b"abc").unwrap();
        std::fs::write(dir.join("y.txt"), b"def").unwrap();
        let s1 = signature_of_tree(&dir).unwrap();
        let s2 = signature_of_tree(&dir).unwrap();
        assert_eq!(s1, s2);
    }

    #[test]
    fn signature_excludes_marker_file() {
        let dir = tempdir();
        std::fs::write(dir.join("a.txt"), b"hello").unwrap();
        let s1 = signature_of_tree(&dir).unwrap();
        // Writing the marker file should not change the signature
        let mut f = std::fs::File::create(dir.join(MARKER_FILE)).unwrap();
        writeln!(f, "{s1}").unwrap();
        let s2 = signature_of_tree(&dir).unwrap();
        assert_eq!(s1, s2);
    }

    fn tempdir() -> PathBuf {
        // Combine a process id, a timestamp and a per-call counter so two
        // parallel test threads cannot accidentally land in the same dir.
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let mut p = std::env::temp_dir();
        let n = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let pid = std::process::id();
        let c = COUNTER.fetch_add(1, Ordering::Relaxed);
        p.push(format!("aracdia_test_{pid}_{n}_{c}"));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
