import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface EngineAsset {
  url: string;
  sha256: string;
  sizeBytes: number;
}

export interface EngineRelease {
  version: string;
  target: string;
  asset: EngineAsset;
}

export type EngineStatus =
  | { kind: "notInstalled" }
  | { kind: "installed"; version: string; path: string };

export type InstallPhase = "downloading" | "verifying" | "extracting";

export interface InstallProgress {
  phase: InstallPhase;
  bytesDone: number;
  bytesTotal: number | null;
}

export interface InstallComplete {
  version: string;
}

export interface InstallError {
  message: string;
}

export const ENGINE_EVENTS = {
  progress: "engine://progress",
  complete: "engine://complete",
  error: "engine://error",
} as const;

export async function getEngineStatus(): Promise<EngineStatus> {
  return await invoke<EngineStatus>("engine_status");
}

export async function getEngineCurrentTarget(): Promise<string> {
  return await invoke<string>("engine_current_target");
}

export async function fetchEngineRelease(): Promise<EngineRelease> {
  return await invoke<EngineRelease>("fetch_engine_release");
}

export async function installEngine(release: EngineRelease): Promise<void> {
  await invoke<void>("install_engine", { release });
}

export async function uninstallEngine(): Promise<void> {
  await invoke<void>("uninstall_engine");
}

export interface InstallListeners {
  onProgress?: (progress: InstallProgress) => void;
  onComplete?: (event: InstallComplete) => void;
  onError?: (event: InstallError) => void;
}

/**
 * Subscribes to all install events at once. Returns a single unlisten
 * function that detaches every handler.
 */
export async function listenToInstall(
  listeners: InstallListeners,
): Promise<UnlistenFn> {
  const unlisten: UnlistenFn[] = [];
  if (listeners.onProgress) {
    unlisten.push(
      await listen<InstallProgress>(ENGINE_EVENTS.progress, (e) =>
        listeners.onProgress!(e.payload),
      ),
    );
  }
  if (listeners.onComplete) {
    unlisten.push(
      await listen<InstallComplete>(ENGINE_EVENTS.complete, (e) =>
        listeners.onComplete!(e.payload),
      ),
    );
  }
  if (listeners.onError) {
    unlisten.push(
      await listen<InstallError>(ENGINE_EVENTS.error, (e) =>
        listeners.onError!(e.payload),
      ),
    );
  }
  return () => {
    for (const fn of unlisten) fn();
  };
}

const SIZE_UNITS = ["B", "KiB", "MiB", "GiB", "TiB"];

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < SIZE_UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  const decimals = unit === 0 ? 0 : value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(decimals)} ${SIZE_UNITS[unit]}`;
}

export function describePhase(phase: InstallPhase): string {
  switch (phase) {
    case "downloading":
      return "Téléchargement";
    case "verifying":
      return "Vérification";
    case "extracting":
      return "Installation";
  }
}
