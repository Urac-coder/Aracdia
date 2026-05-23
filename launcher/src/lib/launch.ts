import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/// Mirrors the Rust `RunningSession` struct (camelCased on the wire).
export interface RunningSession {
  pid: number;
  logPath: string;
  startedAt: string; // ISO 8601
  binary: string;
  binaryName: string;
}

export interface LaunchLine {
  stream: "stdout" | "stderr";
  line: string;
}

export interface LaunchExited {
  exitCode: number | null;
  success: boolean;
}

export const LAUNCH_EVENTS = {
  started: "engine://launch:started",
  line: "engine://launch:line",
  exited: "engine://launch:exited",
} as const;

export async function launchEngine(): Promise<RunningSession> {
  return await invoke<RunningSession>("launch_engine");
}

export async function stopEngine(): Promise<void> {
  await invoke<void>("stop_engine");
}

export async function isEngineRunning(): Promise<boolean> {
  return await invoke<boolean>("is_engine_running");
}

/// Returns the live engine session if any (own child OR recovered after a
/// launcher restart while the engine is still alive).
export async function currentSession(): Promise<RunningSession | null> {
  return await invoke<RunningSession | null>("current_session");
}

export interface LaunchListeners {
  onStarted?: (event: RunningSession) => void;
  onLine?: (event: LaunchLine) => void;
  onExited?: (event: LaunchExited) => void;
}

export async function listenToLaunch(
  listeners: LaunchListeners,
): Promise<UnlistenFn> {
  const handles: UnlistenFn[] = [];
  if (listeners.onStarted) {
    handles.push(
      await listen<RunningSession>(LAUNCH_EVENTS.started, (e) =>
        listeners.onStarted!(e.payload),
      ),
    );
  }
  if (listeners.onLine) {
    handles.push(
      await listen<LaunchLine>(LAUNCH_EVENTS.line, (e) =>
        listeners.onLine!(e.payload),
      ),
    );
  }
  if (listeners.onExited) {
    handles.push(
      await listen<LaunchExited>(LAUNCH_EVENTS.exited, (e) =>
        listeners.onExited!(e.payload),
      ),
    );
  }
  return () => {
    for (const fn of handles) fn();
  };
}
