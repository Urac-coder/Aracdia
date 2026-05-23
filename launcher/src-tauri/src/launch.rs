//! Engine spawn pipeline: locate the binary inside the installed engine
//! directory, launch it as a subprocess with the right arguments, stream its
//! stdout/stderr into a rolling log file, and emit Tauri events so the UI
//! can show the "game running" state.
//!
//! Sessions are persisted to `<data>/session.json` so that if the launcher
//! is killed (crash, SIGKILL, …) while the engine is still alive, the next
//! launcher run can detect the running process and offer "Quitter le jeu"
//! pointing at the right PID and log file.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};
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

/// Persisted record of the running engine subprocess. Survives launcher
/// crashes via the on-disk `session.json` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningSession {
    pub pid: u32,
    pub log_path: PathBuf,
    pub started_at: DateTime<Utc>,
    /// Full path to the engine binary that was spawned. Used as anti-PID-recycling
    /// check: a recovered session is only trusted if the live process at that PID
    /// has a matching executable name.
    pub binary: PathBuf,
    pub binary_name: String,
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

/// In-memory representation of "what session, if any, is alive right now".
enum SessionState {
    None,
    Owned {
        child: Child,
        info: RunningSession,
    },
    Recovered {
        info: RunningSession,
    },
}

impl SessionState {
    fn info(&self) -> Option<&RunningSession> {
        match self {
            SessionState::None => None,
            SessionState::Owned { info, .. } | SessionState::Recovered { info } => Some(info),
        }
    }
}

/// Application state: at most one running engine subprocess.
pub struct LaunchState {
    inner: Arc<Mutex<SessionState>>,
}

impl Default for LaunchState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionState::None)),
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
// Session persistence
// ---------------------------------------------------------------------------

fn session_file_path() -> Result<PathBuf, std::io::Error> {
    Ok(paths::data_dir()?.join("session.json"))
}

fn write_session(info: &RunningSession) -> Result<(), std::io::Error> {
    let path = session_file_path()?;
    let json = serde_json::to_string_pretty(info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}

fn clear_session_file() {
    if let Ok(path) = session_file_path() {
        let _ = std::fs::remove_file(path);
    }
}

fn read_session_file() -> Option<RunningSession> {
    let path = session_file_path().ok()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

// ---------------------------------------------------------------------------
// PID introspection (anti-collision: must match expected binary name)
// ---------------------------------------------------------------------------

/// Strips an executable suffix and lower-cases the basename so that names like
/// "luanti", "luanti.exe", "Luanti" all compare equal.
fn normalize_exec_name(name: &str) -> String {
    let lower = name.to_lowercase();
    lower
        .strip_suffix(".exe")
        .unwrap_or(&lower)
        .to_string()
}

/// Returns true iff a process at `pid` is alive AND its executable basename
/// matches `expected_name` (case-insensitively, after stripping `.exe`).
fn pid_alive_with_name(pid: u32, expected_name: &str) -> bool {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        true,
        sysinfo::ProcessRefreshKind::new(),
    );
    let Some(p) = sys.process(Pid::from_u32(pid)) else {
        return false;
    };
    let raw = p.name().to_string_lossy().to_string();
    let actual = normalize_exec_name(&raw);
    let expected = normalize_exec_name(expected_name);
    actual == expected
}

/// Best-effort kill via sysinfo. Returns true on apparent success.
fn kill_pid(pid: u32) -> bool {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        true,
        sysinfo::ProcessRefreshKind::new(),
    );
    sys.process(Pid::from_u32(pid))
        .map(|p| p.kill())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Internal session reconciliation
// ---------------------------------------------------------------------------

/// Inspects the current state and returns the live session if any. Cleans
/// up after a process that has died since we last looked.
async fn reconcile_session(state: &LaunchState) -> Option<RunningSession> {
    let mut guard = state.inner.lock().await;
    match &mut *guard {
        SessionState::None => {}
        SessionState::Owned { child, info } => match child.try_wait() {
            Ok(None) => return Some(info.clone()),
            _ => {
                clear_session_file();
                *guard = SessionState::None;
                return None;
            }
        },
        SessionState::Recovered { info } => {
            if pid_alive_with_name(info.pid, &info.binary_name) {
                return Some(info.clone());
            }
            clear_session_file();
            *guard = SessionState::None;
            return None;
        }
    }

    // Nothing in memory — try recovering from disk (after a launcher crash).
    if let Some(info) = read_session_file() {
        if pid_alive_with_name(info.pid, &info.binary_name) {
            *guard = SessionState::Recovered { info: info.clone() };
            return Some(info);
        }
        clear_session_file();
    }
    None
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn is_engine_running(state: State<'_, LaunchState>) -> Result<bool, String> {
    Ok(reconcile_session(&state).await.is_some())
}

#[tauri::command]
pub async fn current_session(
    state: State<'_, LaunchState>,
) -> Result<Option<RunningSession>, String> {
    Ok(reconcile_session(&state).await)
}

#[tauri::command]
pub async fn launch_engine(
    app: AppHandle,
    state: State<'_, LaunchState>,
) -> Result<RunningSession, String> {
    if reconcile_session(&state).await.is_some() {
        return Err(LaunchError::AlreadyRunning.to_string());
    }

    let settings = settings::load_settings().map_err(|e| LaunchError::Settings(e))?;
    let custom = settings.install_dir.clone();
    let engine_dir = paths::engine_dir(custom.as_deref()).map_err(LaunchError::from)?;
    if !engine_dir.exists() {
        return Err(LaunchError::NotInstalled.to_string());
    }

    let binary = find_engine_binary(&engine_dir)
        .ok_or_else(|| LaunchError::BinaryNotFound(engine_dir.clone()))?;

    // Best-effort: clear quarantine on macOS before each launch
    strip_quarantine(&binary);

    let username = match profile::load_profile().map_err(LaunchError::Profile)? {
        Some(p) => p.username,
        None => "Player".to_owned(),
    };
    let args = build_args(&settings, &username);

    let cwd = engine_dir.clone();

    let (log_file, log_path) = new_log_file().map_err(LaunchError::from)?;
    drop(log_file); // we'll reopen in append mode in the streaming task

    let binary_name = binary
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "luanti".to_string());

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

    let info = RunningSession {
        pid,
        log_path: log_path.clone(),
        started_at: Utc::now(),
        binary: binary.clone(),
        binary_name,
    };

    // Persist to disk so a launcher crash doesn't lose track of the child
    if let Err(err) = write_session(&info) {
        eprintln!("[launch] warning: could not write session.json: {err}");
    }

    // Stream stdout + stderr → log file + per-line UI events
    if let Some(stdout) = child.stdout.take() {
        let app2 = app.clone();
        let path2 = log_path.clone();
        tokio::spawn(async move {
            stream_to_log(stdout, "stdout", app2, path2).await;
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let app2 = app.clone();
        let path2 = log_path.clone();
        tokio::spawn(async move {
            stream_to_log(stderr, "stderr", app2, path2).await;
        });
    }

    {
        let mut guard = state.inner.lock().await;
        *guard = SessionState::Owned {
            child,
            info: info.clone(),
        };
    }

    let _ = app.emit(events::STARTED, info.clone());

    // Reaper task: wait for the child to exit, then notify the UI and clear state.
    let inner = state.inner.clone();
    let app_for_exit = app.clone();
    tokio::spawn(async move {
        let exit_status = {
            let mut guard = inner.lock().await;
            match &mut *guard {
                SessionState::Owned { child, .. } => child.wait().await.ok(),
                _ => None,
            }
        };
        {
            let mut guard = inner.lock().await;
            *guard = SessionState::None;
        }
        clear_session_file();
        let exit_code = exit_status.as_ref().and_then(|s| s.code());
        let success = exit_status.as_ref().map(|s| s.success()).unwrap_or(false);
        let _ = app_for_exit.emit(events::EXITED, LaunchExited { exit_code, success });
    });

    Ok(info)
}

async fn stream_to_log<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    stream_name: &'static str,
    app: AppHandle,
    log_path: PathBuf,
) {
    let mut buf = BufReader::new(reader).lines();
    while let Ok(Some(line)) = buf.next_line().await {
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
pub async fn stop_engine(
    app: AppHandle,
    state: State<'_, LaunchState>,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    let info = guard.info().cloned();
    match &mut *guard {
        SessionState::Owned { child, .. } => {
            let _ = child.start_kill();
        }
        SessionState::Recovered { info } => {
            // We don't own the Child handle, kill via the OS
            let _ = kill_pid(info.pid);
            // Also notify the UI ourselves since no reaper task is watching.
            let _ = app.emit(
                events::EXITED,
                LaunchExited {
                    exit_code: None,
                    success: true,
                },
            );
        }
        SessionState::None => {}
    }
    clear_session_file();
    *guard = SessionState::None;
    drop(info); // explicit
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

    #[test]
    fn normalize_exec_name_strips_exe_and_lowercases() {
        assert_eq!(normalize_exec_name("luanti"), "luanti");
        assert_eq!(normalize_exec_name("Luanti"), "luanti");
        assert_eq!(normalize_exec_name("luanti.exe"), "luanti");
        assert_eq!(normalize_exec_name("LUANTI.EXE"), "luanti");
    }

    #[test]
    fn pid_alive_with_name_rejects_unrelated_pid() {
        // PID 1 is init/launchd on Unix and "System Idle"/"System" on Windows;
        // none of those should pass the `luanti` name check.
        assert!(!pid_alive_with_name(1, "luanti"));
    }
}
