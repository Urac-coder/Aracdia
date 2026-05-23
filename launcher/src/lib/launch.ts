import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface LaunchStarted {
  pid: number;
  logPath: string;
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

export async function launchEngine(): Promise<LaunchStarted> {
  return await invoke<LaunchStarted>("launch_engine");
}

export async function stopEngine(): Promise<void> {
  await invoke<void>("stop_engine");
}

export async function isEngineRunning(): Promise<boolean> {
  return await invoke<boolean>("is_engine_running");
}

export interface LaunchListeners {
  onStarted?: (event: LaunchStarted) => void;
  onLine?: (event: LaunchLine) => void;
  onExited?: (event: LaunchExited) => void;
}

export async function listenToLaunch(
  listeners: LaunchListeners,
): Promise<UnlistenFn> {
  const handles: UnlistenFn[] = [];
  if (listeners.onStarted) {
    handles.push(
      await listen<LaunchStarted>(LAUNCH_EVENTS.started, (e) =>
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
