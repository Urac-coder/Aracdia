//! Engine spawn pipeline: locate the binary inside the installed engine
//! directory, launch it as a subprocess with the right arguments, stream its
//! stdout/stderr into a rolling log file, and emit Tauri events so the UI
//! can show the "game running" state.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::paths;
use crate::profile::{self};
use crate::settings::{self, LauncherSettings};

/// Tauri event names emitted while a game session is active.
mod events {
    pub const STARTED: &str = "engine://launch:started";
    pub const LINE: &str = "engine://launch:line";
    pub const EXITED: &str = "engine://launch:exited";
}

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("engine is not installed")]
    NotInstalled,
    #[error("engine binary not found in {0:?}")]
    BinaryNotFound(PathBuf),
    #[error("a game session is already running")]
    AlreadyRunning,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("settings error: {0}")]
    Settings(String),
    #[error("profile error: {0}")]
    Profile(String),
}

impl From<LaunchError> for String {
    fn from(err: LaunchError) -> Self {
        err.to_string()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchStarted {
    pub pid: u32,
    pub log_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchLine {
    pub stream: &'static str, // "stdout" | "stderr"
    pub line: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchExited {
    pub exit_code: Option<i32>,
    pub success: bool,
}

/// Application state: at most one running engine subprocess.
pub struct LaunchState {
    inner: Arc<Mutex<Option<Child>>>,
}

impl Default for LaunchState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
        }
    }
}

// ---------------------------------------------------------------------------
// Binary resolution
// ---------------------------------------------------------------------------

/// Locates the engine executable inside an extracted engine directory.
///
/// Tries the canonical layout first, then falls back to a one-level glob to
/// tolerate the older Windows zip layout that wraps everything under
/// `luanti-<version>-win64/`.
pub fn find_engine_binary(engine_dir: &Path) -> Option<PathBuf> {
    let candidates: &[&[&str]] = if cfg!(target_os = "macos") {
        &[
            &["luanti.app", "Contents", "MacOS", "luanti"],
            &["Aracdia.app", "Contents", "MacOS", "Aracdia"],
        ]
    } else if cfg!(target_os = "windows") {
        &[&["bin", "luanti.exe"], &["bin", "aracdia.exe"]]
    } else {
        &[
            &["bin", "luanti"],
            &["bin", "aracdia"],
            &["usr", "bin", "luanti"],
        ]
    };

    for parts in candidates {
        let mut p = engine_dir.to_path_buf();
        for part in *parts {
            p.push(part);
        }
        if p.is_file() {
            return Some(p);
        }
    }

    // Fallback: walk one directory level deep looking for any of the candidates
    // (handles zips that wrap the contents under a single subdirectory).
    if let Ok(entries) = std::fs::read_dir(engine_dir) {
        for entry in entries.flatten() {
            let inner = entry.path();
            if inner.is_dir() {
                if let Some(found) = find_engine_binary(&inner) {
                    return Some(found);
                }
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// macOS quarantine handling
// ---------------------------------------------------------------------------

/// macOS attaches a `com.apple.quarantine` extended attribute to anything
/// downloaded from the network. Without removing it, Gatekeeper refuses to
/// run an unsigned `.app`. Best-effort: ignore failures so the install
/// pipeline never blocks on this.
pub fn strip_quarantine(path: &Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("xattr")
            .args(["-dr", "com.apple.quarantine"])
            .arg(path)
            .status();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path; // no-op on other OSes
    }
}

// ---------------------------------------------------------------------------
// Argument building
// ---------------------------------------------------------------------------

fn build_args(settings: &LauncherSettings, username: &str) -> Vec<String> {
    let mut args = vec![
        // Username forwarded to the engine for both menu prefill and direct connect
        "--name".to_owned(),
        username.to_owned(),
    ];
    let address = settings.server_address.trim();
    if !address.is_empty() && settings.auto_connect {
        args.push("--address".to_owned());
        args.push(address.to_owned());
        args.push("--port".to_owned());
        args.push(settings.server_port.to_string());
        args.push("--go".to_owned());
    }
    args
}

// ---------------------------------------------------------------------------
// Logging
// ---------------------------------------------------------------------------

fn new_log_file() -> Result<(std::fs::File, PathBuf), std::io::Error> {
    let dir = paths::data_dir()?.join("logs");
    std::fs::create_dir_all(&dir)?;
    let stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("engine-{stamp}.log"));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    Ok((file, path))
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn is_engine_running(state: State<'_, LaunchState>) -> Result<bool, String> {
    let mut guard = state.inner.lock().await;
    let still_running = if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(Some(_)) => false, // process exited, will be reaped below
            Ok(None) => true,
            Err(_) => false,
        }
    } else {
        false
    };
    if !still_running && guard.is_some() {
        *guard = None;
    }
    Ok(still_running)
}

#[tauri::command]
pub async fn launch_engine(
    app: AppHandle,
    state: State<'_, LaunchState>,
) -> Result<LaunchStarted, String> {
    {
        // Refuse if a child is already alive
        let mut guard = state.inner.lock().await;
        if let Some(child) = guard.as_mut() {
            if matches!(child.try_wait(), Ok(None)) {
                return Err(LaunchError::AlreadyRunning.to_string());
            }
            *guard = None;
        }
    }

    let settings = settings::load_settings().map_err(|e| LaunchError::Settings(e))?;
    let custom = settings.install_dir.clone();
    let engine_dir = paths::engine_dir(custom.as_deref()).map_err(LaunchError::from)?;
    if !engine_dir.exists() {
        return Err(LaunchError::NotInstalled.to_string());
    }

    let binary = find_engine_binary(&engine_dir)
        .ok_or_else(|| LaunchError::BinaryNotFound(engine_dir.clone()))?;

    // Make sure the binary is allowed to run on macOS (idempotent best-effort)
    strip_quarantine(&binary);

    let username = match profile::load_profile().map_err(LaunchError::Profile)? {
        Some(p) => p.username,
        None => "Player".to_owned(),
    };
    let args = build_args(&settings, &username);

    // Working directory: the engine's parent so relative paths (games/, mods/)
    // resolve against `<engine>/` for non-`.app` layouts. For `.app` bundles
    // we still cd to the engine root so logs and config end up there.
    let cwd = engine_dir.clone();

    let (log_file, log_path) = new_log_file().map_err(LaunchError::from)?;
    let log_path_display = log_path.to_string_lossy().into_owned();
    drop(log_file); // we'll reopen in append mode in the streaming task

    let mut command = Command::new(&binary);
    command
        .args(&args)
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = command.spawn().map_err(LaunchError::from)?;
    let pid = child.id().unwrap_or(0);

    // Stream stdout + stderr to log file and emit per-line events
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let app_for_stdout = app.clone();
    let log_path_stdout = log_path.clone();
    if let Some(stdout) = stdout {
        tokio::spawn(async move {
            stream_to_log(stdout, "stdout", app_for_stdout, log_path_stdout).await;
        });
    }
    let app_for_stderr = app.clone();
    let log_path_stderr = log_path.clone();
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            stream_to_log(stderr, "stderr", app_for_stderr, log_path_stderr).await;
        });
    }

    {
        let mut guard = state.inner.lock().await;
        *guard = Some(child);
    }

    let started = LaunchStarted {
        pid,
        log_path: log_path_display.clone(),
    };
    let _ = app.emit(events::STARTED, started.clone());

    // Reap the process in a detached task so the command returns immediately
    let inner = state.inner.clone();
    let app_for_exit = app.clone();
    tokio::spawn(async move {
        let exit_status = {
            let mut guard = inner.lock().await;
            match guard.as_mut() {
                Some(child) => child.wait().await.ok(),
                None => None,
            }
        };
        {
            let mut guard = inner.lock().await;
            *guard = None;
        }
        let exit_code = exit_status.as_ref().and_then(|s| s.code());
        let success = exit_status.as_ref().map(|s| s.success()).unwrap_or(false);
        let _ = app_for_exit.emit(
            events::EXITED,
            LaunchExited {
                exit_code,
                success,
            },
        );
    });

    Ok(started)
}

async fn stream_to_log<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    stream_name: &'static str,
    app: AppHandle,
    log_path: PathBuf,
) {
    let mut buf = BufReader::new(reader).lines();
    while let Ok(Some(line)) = buf.next_line().await {
        // Append to the rolling log file (best effort; never block on errors)
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            use std::io::Write as _;
            let _ = writeln!(file, "[{stream_name}] {line}");
        }
        let _ = app.emit(
            events::LINE,
            LaunchLine {
                stream: stream_name,
                line,
            },
        );
    }
}

#[tauri::command]
pub async fn stop_engine(state: State<'_, LaunchState>) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    if let Some(child) = guard.as_mut() {
        let _ = child.start_kill();
    }
    *guard = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_args_includes_name() {
        let s = LauncherSettings::default();
        let args = build_args(&s, "Aragorn");
        assert!(args.windows(2).any(|w| w == ["--name", "Aragorn"]));
        assert!(!args.iter().any(|a| a == "--address"));
    }

    #[test]
    fn build_args_with_auto_connect() {
        let mut s = LauncherSettings::default();
        s.server_address = "play.example.com".to_owned();
        s.server_port = 30000;
        s.auto_connect = true;
        let args = build_args(&s, "U");
        assert!(args.iter().any(|a| a == "--go"));
        assert!(args.windows(2).any(|w| w == ["--address", "play.example.com"]));
        assert!(args.windows(2).any(|w| w == ["--port", "30000"]));
    }

    #[test]
    fn build_args_no_go_without_auto_connect() {
        let mut s = LauncherSettings::default();
        s.server_address = "play.example.com".to_owned();
        s.auto_connect = false;
        let args = build_args(&s, "U");
        assert!(!args.iter().any(|a| a == "--address"));
        assert!(!args.iter().any(|a| a == "--go"));
    }
}
