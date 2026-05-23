//! Local Aracdia server lifecycle.
//!
//! The launcher spawns a Luanti subprocess in `--server` mode so the user can
//! play directly inside Aracdia without ever seeing the engine's vanilla
//! menu. The server lives as long as the launcher is open; clicking JOUER
//! waits until the TCP port is listening, then connects a separate client
//! subprocess via `--address 127.0.0.1 --port <p> --go`.
//!
//! Conceptually parallel to `launch.rs`, but the lifecycle is different:
//! - the server is started *automatically* when the launcher boots,
//! - it is stopped when the launcher quits (best-effort),
//! - sessions are persisted to disk so a launcher crash does not leak orphan
//!   server processes that would block the next start.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, Notify};

use crate::game;
use crate::launch::{find_engine_binary, kill_pid, pid_alive_with_name, strip_quarantine};
use crate::paths;
use crate::settings;

/// Tauri events emitted while the local server is alive.
mod events {
    pub const STARTED: &str = "server://started";
    pub const STOPPED: &str = "server://stopped";
    pub const LINE: &str = "server://line";
}

/// Maximum time we wait for the server's "listening" log line before failing
/// the boot. The Luanti server boots in well under a second on a modern Mac
/// so 20 s is generous.
const READY_TIMEOUT: Duration = Duration::from_secs(20);

/// Substring we look for in the server's log to consider it ready. Format:
/// `ACTION[Main]: Server for gameid="<id>" listening on <bind>:<port>.`
const READY_MARKER: &str = "Server for gameid=";

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("engine is not installed")]
    NotInstalled,
    #[error("engine binary not found in {0:?}")]
    BinaryNotFound(PathBuf),
    #[error("server is already running")]
    AlreadyRunning,
    #[error("server failed to start listening within {0:?}")]
    NotReady(Duration),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("settings error: {0}")]
    Settings(String),
    #[error("game deploy error: {0}")]
    Game(String),
}

impl From<ServerError> for String {
    fn from(err: ServerError) -> Self {
        err.to_string()
    }
}

/// On-disk record of the running server. Survives launcher crashes via
/// `<data>/server-session.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSession {
    pub pid: u32,
    pub log_path: PathBuf,
    pub started_at: DateTime<Utc>,
    pub bind: String,
    pub port: u16,
    pub world_path: PathBuf,
    /// Full path to the engine binary; used as anti-PID-recycling check.
    pub binary: PathBuf,
    pub binary_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerLine {
    pub stream: &'static str,
    pub line: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ServerStatus {
    Stopped,
    Running(ServerSession),
}

// ---------------------------------------------------------------------------
// In-memory state
// ---------------------------------------------------------------------------

enum SessionState {
    None,
    Owned { child: Child, info: ServerSession },
    Recovered { info: ServerSession },
}

impl SessionState {
    fn info(&self) -> Option<&ServerSession> {
        match self {
            SessionState::None => None,
            SessionState::Owned { info, .. } | SessionState::Recovered { info } => Some(info),
        }
    }
}

pub struct ServerState {
    inner: Arc<Mutex<SessionState>>,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionState::None)),
        }
    }
}

// ---------------------------------------------------------------------------
// World setup — make sure the world directory has a sane `world.mt` so the
// server doesn't error on first start.
// ---------------------------------------------------------------------------

fn ensure_world_initialised(world: &Path, gameid: &str) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(world)?;
    let world_mt = world.join("world.mt");
    if world_mt.exists() {
        return Ok(());
    }
    let content = format!(
        "gameid = {gameid}\n\
        backend = sqlite3\n\
        auth_backend = sqlite3\n\
        player_backend = sqlite3\n\
        mod_storage_backend = sqlite3\n\
        world_name = Aracdia\n\
        creative_mode = false\n\
        enable_damage = true\n",
    );
    std::fs::write(world_mt, content)
}

// ---------------------------------------------------------------------------
// Logging
// ---------------------------------------------------------------------------

fn new_log_file() -> Result<(std::fs::File, PathBuf), std::io::Error> {
    let dir = paths::server_log_dir()?;
    let stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("server-{stamp}.log"));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    Ok((file, path))
}

// ---------------------------------------------------------------------------
// Session persistence
// ---------------------------------------------------------------------------

fn write_session(info: &ServerSession) -> Result<(), std::io::Error> {
    let path = paths::server_session_file()?;
    let json = serde_json::to_string_pretty(info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}

fn clear_session_file() {
    if let Ok(path) = paths::server_session_file() {
        let _ = std::fs::remove_file(path);
    }
}

fn read_session_file() -> Option<ServerSession> {
    let path = paths::server_session_file().ok()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

// ---------------------------------------------------------------------------
// Internal session reconciliation
// ---------------------------------------------------------------------------

async fn reconcile(state: &ServerState) -> Option<ServerSession> {
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
// Wait until the server logs the "listening" line.
//
// Luanti speaks UDP, so a TCP probe never succeeds. The most reliable signal
// is the engine's own log line `Server for gameid="..." listening on ...`,
// which we watch for in the streaming task and forward via `Notify`.
// ---------------------------------------------------------------------------

async fn wait_until_ready(ready: Arc<Notify>) -> Result<(), ServerError> {
    let waited = ready.notified();
    tokio::select! {
        _ = waited => Ok(()),
        _ = tokio::time::sleep(READY_TIMEOUT) => Err(ServerError::NotReady(READY_TIMEOUT)),
    }
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

/// Luanti has no `--bind` flag; it picks up `bind_address` from a config file
/// instead. We materialise a per-launcher config holding just the network
/// settings and pass it via `--config`. Anything else stays at engine
/// defaults.
fn write_server_config(bind: &str, port: u16) -> Result<PathBuf, std::io::Error> {
    let path = paths::server_conf_file()?;
    let body = format!(
        "# Auto-generated by Aracdia launcher. Do not edit — your changes will be overwritten.\n\
        bind_address = {bind}\n\
        port = {port}\n\
        time_speed = 0\n\
        ",
    );
    std::fs::write(&path, body)?;
    Ok(path)
}

fn build_args(world: &Path, gameid: &str, conf: &Path, port: u16) -> Vec<String> {
    vec![
        "--server".to_owned(),
        "--world".to_owned(),
        world.to_string_lossy().into_owned(),
        "--gameid".to_owned(),
        gameid.to_owned(),
        "--config".to_owned(),
        conf.to_string_lossy().into_owned(),
        // `--port` is also honored when present in the config, but passing
        // it on the CLI keeps the actual listen port visible in `ps`.
        "--port".to_owned(),
        port.to_string(),
    ]
}

/// Streams a child stream into a log file + Tauri events. When `ready` is
/// `Some`, a one-shot `notify_one` is fired the first time `READY_MARKER` is
/// observed in the output.
async fn stream_to_log(
    app: AppHandle,
    label: &'static str,
    reader: impl tokio::io::AsyncRead + Unpin,
    log_path: PathBuf,
    ready: Option<Arc<Notify>>,
) {
    use std::io::Write;
    let mut buf = BufReader::new(reader).lines();
    let mut signalled = false;
    while let Ok(Some(line)) = buf.next_line().await {
        if !signalled {
            if let Some(ref n) = ready {
                if line.contains(READY_MARKER) {
                    n.notify_one();
                    signalled = true;
                }
            }
        }
        let _ = app.emit(
            events::LINE,
            ServerLine {
                stream: label,
                line: line.clone(),
            },
        );
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = writeln!(f, "[{label}] {line}");
        }
    }
}

async fn spawn_server(app: &AppHandle) -> Result<ServerSession, ServerError> {
    // 1. Resolve installed engine binary
    let custom_install = settings::load_settings()
        .map_err(ServerError::Settings)?
        .install_dir;
    let engine_dir = paths::engine_dir(custom_install.as_deref())?;
    if !engine_dir.exists() {
        return Err(ServerError::NotInstalled);
    }
    let binary =
        find_engine_binary(&engine_dir).ok_or_else(|| ServerError::BinaryNotFound(engine_dir.clone()))?;
    strip_quarantine(&binary);

    // 2. Resolve settings
    let settings = settings::load_settings().map_err(ServerError::Settings)?;
    let bind = settings.local_server_bind.clone();
    let port = settings.local_server_port;

    // 3. Make sure the bundled / downloaded game is in place — the server
    //    needs `--gameid aracdia` to be installable.
    let deployed = game::deploy_game(app).map_err(|e| ServerError::Game(e.to_string()))?;

    // 4. World prep
    let world = paths::world_dir()?;
    ensure_world_initialised(&world, &deployed.gameid)?;

    // 5. Server config (bind_address goes here, not on the CLI)
    let conf_path = write_server_config(&bind, port)?;

    // 6. Log file
    let (log_file, log_path) = new_log_file()?;
    let stdout_log = log_path.clone();
    let stderr_log = log_path.clone();

    // 7. Spawn
    let args = build_args(&world, &deployed.gameid, &conf_path, port);
    eprintln!(
        "[server] spawning {:?} {} (world={}, bind={}:{})",
        binary,
        args.join(" "),
        world.display(),
        bind,
        port
    );

    let mut command = Command::new(&binary);
    command
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(false); // we manage lifecycle explicitly

    let mut child = command.spawn()?;
    let pid = child
        .id()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "child has no PID"))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let binary_name = binary
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "luanti".to_owned());

    let info = ServerSession {
        pid,
        log_path: log_path.clone(),
        started_at: Utc::now(),
        bind: bind.clone(),
        port,
        world_path: world.clone(),
        binary: binary.clone(),
        binary_name,
    };
    write_session(&info)?;
    drop(log_file);

    let ready = Arc::new(Notify::new());
    if let Some(stdout) = stdout {
        let app = app.clone();
        tokio::spawn(stream_to_log(
            app,
            "stdout",
            stdout,
            stdout_log,
            Some(ready.clone()),
        ));
    }
    if let Some(stderr) = stderr {
        let app = app.clone();
        tokio::spawn(stream_to_log(app, "stderr", stderr, stderr_log, None));
    }

    // 8. Wait until the server logs its listening line
    wait_until_ready(ready).await?;

    let _ = app.emit(events::STARTED, info.clone());

    // Move into Owned state
    {
        let state: State<'_, ServerState> = app.state();
        let mut guard = state.inner.lock().await;
        *guard = SessionState::Owned { child, info: info.clone() };
    }

    Ok(info)
}

// ---------------------------------------------------------------------------
// Public API + Tauri commands
// ---------------------------------------------------------------------------

/// Returns the live server session, starting it on demand if needed.
pub async fn ensure_started(app: &AppHandle) -> Result<ServerSession, ServerError> {
    let state: State<'_, ServerState> = app.state();
    if let Some(info) = reconcile(&state).await {
        return Ok(info);
    }
    spawn_server(app).await
}

/// Stop the running server (best effort).
pub async fn stop(app: &AppHandle) -> Result<(), ServerError> {
    let state: State<'_, ServerState> = app.state();
    let mut guard = state.inner.lock().await;
    match std::mem::replace(&mut *guard, SessionState::None) {
        SessionState::None => {}
        SessionState::Owned { mut child, info } => {
            let _ = child.start_kill();
            // Reap so we don't leave a zombie.
            let _ = child.wait().await;
            clear_session_file();
            let _ = app.emit(events::STOPPED, info);
        }
        SessionState::Recovered { info } => {
            kill_pid(info.pid);
            clear_session_file();
            let _ = app.emit(events::STOPPED, info);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn start_server(
    app: AppHandle,
    state: State<'_, ServerState>,
) -> Result<ServerSession, String> {
    if let Some(info) = reconcile(&state).await {
        return Ok(info);
    }
    spawn_server(&app).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_server(app: AppHandle) -> Result<(), String> {
    stop(&app).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn server_status(state: State<'_, ServerState>) -> Result<ServerStatus, String> {
    Ok(match reconcile(&state).await {
        Some(info) => ServerStatus::Running(info),
        None => ServerStatus::Stopped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_args_minimal() {
        let args = build_args(
            Path::new("/tmp/world"),
            "aracdia",
            Path::new("/tmp/server.conf"),
            30000,
        );
        assert!(args.iter().any(|a| a == "--server"));
        assert!(args.windows(2).any(|w| w == ["--gameid", "aracdia"]));
        assert!(args.windows(2).any(|w| w == ["--config", "/tmp/server.conf"]));
        assert!(args.windows(2).any(|w| w == ["--port", "30000"]));
        assert!(args.windows(2).any(|w| w == ["--world", "/tmp/world"]));
        assert!(!args.iter().any(|a| a == "--bind"));
    }

    #[test]
    fn ensure_world_initialised_creates_world_mt() {
        let dir = std::env::temp_dir().join(format!(
            "aracdia_world_test_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        ensure_world_initialised(&dir, "aracdia").unwrap();
        let mt = std::fs::read_to_string(dir.join("world.mt")).unwrap();
        assert!(mt.contains("gameid = aracdia"));
        assert!(mt.contains("backend = sqlite3"));
        // Idempotent: second call leaves the file untouched.
        std::fs::write(dir.join("world.mt"), "MARKER").unwrap();
        ensure_world_initialised(&dir, "aracdia").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("world.mt")).unwrap(), "MARKER");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
