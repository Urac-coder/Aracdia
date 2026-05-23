import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface ContentAsset {
  url: string;
  sha256: string;
  sizeBytes: number;
}

export interface ContentRelease {
  /** Version stripped of the `game-v` prefix (e.g. `0.1.0`). */
  version: string;
  /** Original git tag (e.g. `game-v0.1.0`). */
  tag: string;
  asset: ContentAsset;
}

export type ContentStatus =
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

export const CONTENT_EVENTS = {
  progress: "content://progress",
  complete: "content://complete",
  error: "content://error",
} as const;

export async function getContentStatus(): Promise<ContentStatus> {
  return await invoke<ContentStatus>("content_status");
}

export async function fetchContentRelease(): Promise<ContentRelease> {
  return await invoke<ContentRelease>("fetch_content_release");
}

export async function installContent(release: ContentRelease): Promise<void> {
  await invoke<void>("install_content", { release });
}

export interface ContentInstallListeners {
  onProgress?: (progress: InstallProgress) => void;
  onComplete?: (event: InstallComplete) => void;
  onError?: (event: InstallError) => void;
}

export async function listenToContentInstall(
  listeners: ContentInstallListeners,
): Promise<UnlistenFn> {
  const unlisten: UnlistenFn[] = [];
  if (listeners.onProgress) {
    unlisten.push(
      await listen<InstallProgress>(CONTENT_EVENTS.progress, (e) =>
        listeners.onProgress!(e.payload),
      ),
    );
  }
  if (listeners.onComplete) {
    unlisten.push(
      await listen<InstallComplete>(CONTENT_EVENTS.complete, (e) =>
        listeners.onComplete!(e.payload),
      ),
    );
  }
  if (listeners.onError) {
    unlisten.push(
      await listen<InstallError>(CONTENT_EVENTS.error, (e) =>
        listeners.onError!(e.payload),
      ),
    );
  }
  return () => {
    for (const fn of unlisten) fn();
  };
}
