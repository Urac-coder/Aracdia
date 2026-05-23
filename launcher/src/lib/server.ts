import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface ServerSession {
  pid: number;
  logPath: string;
  startedAt: string; // ISO 8601
  bind: string;
  port: number;
  worldPath: string;
  binary: string;
  binaryName: string;
}

export type ServerStatus =
  | { kind: "stopped" }
  | { kind: "running"; pid: number; logPath: string; startedAt: string; bind: string; port: number; worldPath: string; binary: string; binaryName: string };

export interface ServerLine {
  stream: "stdout" | "stderr";
  line: string;
}

export const SERVER_EVENTS = {
  started: "server://started",
  stopped: "server://stopped",
  line: "server://line",
} as const;

export async function getServerStatus(): Promise<ServerStatus> {
  return await invoke<ServerStatus>("server_status");
}

export async function startServer(): Promise<ServerSession> {
  return await invoke<ServerSession>("start_server");
}

export async function stopServer(): Promise<void> {
  await invoke<void>("stop_server");
}

export interface ServerListeners {
  onStarted?: (session: ServerSession) => void;
  onStopped?: (session: ServerSession) => void;
  onLine?: (line: ServerLine) => void;
}

export async function listenToServer(listeners: ServerListeners): Promise<UnlistenFn> {
  const unlisten: UnlistenFn[] = [];
  if (listeners.onStarted) {
    unlisten.push(
      await listen<ServerSession>(SERVER_EVENTS.started, (e) =>
        listeners.onStarted!(e.payload),
      ),
    );
  }
  if (listeners.onStopped) {
    unlisten.push(
      await listen<ServerSession>(SERVER_EVENTS.stopped, (e) =>
        listeners.onStopped!(e.payload),
      ),
    );
  }
  if (listeners.onLine) {
    unlisten.push(
      await listen<ServerLine>(SERVER_EVENTS.line, (e) =>
        listeners.onLine!(e.payload),
      ),
    );
  }
  return () => {
    for (const fn of unlisten) fn();
  };
}
