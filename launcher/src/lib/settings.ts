import { invoke } from "@tauri-apps/api/core";

export interface LauncherSettings {
  memoryMb: number;
  /** Optional remote server override (e.g. a future Aracdia VPS). Empty
   * means: use the launcher-managed local server. */
  serverAddress: string;
  serverPort: number;
  /** Always-true now (the launcher never shows the Luanti menu). Kept for
   * forward compatibility with persisted settings files. */
  autoConnect: boolean;
  installDir: string | null;
  manifestUrl: string;
  contentManifestUrl: string;
  /** Port the launcher-managed local server listens on. */
  localServerPort: number;
  /** Bind address (`0.0.0.0` exposes on LAN, `127.0.0.1` machine-only). */
  localServerBind: string;
}

/** Mirror of Rust-side limits — keep in sync with `settings.rs`. */
export const SETTINGS_RULES = {
  memoryMin: 512,
  memoryMax: 32_768,
  portMin: 1,
  portMax: 65_535,
  serverAddressMax: 253,
} as const;

export const DEFAULT_MANIFEST_URL =
  "https://api.github.com/repos/Urac-coder/aracdia-engine/releases/latest";

export const DEFAULT_CONTENT_MANIFEST_URL =
  "https://api.github.com/repos/Urac-coder/Aracdia/releases?per_page=30";

export const DEFAULT_SETTINGS: LauncherSettings = {
  memoryMb: 2048,
  serverAddress: "",
  serverPort: 30_000,
  autoConnect: true,
  installDir: null,
  manifestUrl: DEFAULT_MANIFEST_URL,
  contentManifestUrl: DEFAULT_CONTENT_MANIFEST_URL,
  localServerPort: 30_000,
  localServerBind: "0.0.0.0",
};

export async function loadSettings(): Promise<LauncherSettings> {
  return await invoke<LauncherSettings>("load_settings");
}

export async function saveSettings(
  settings: LauncherSettings,
): Promise<LauncherSettings> {
  return await invoke<LauncherSettings>("save_settings", { settings });
}

export async function resetSettings(): Promise<LauncherSettings> {
  return await invoke<LauncherSettings>("reset_settings");
}

export interface SettingsValidationErrors {
  memoryMb?: string;
  serverAddress?: string;
  serverPort?: string;
  manifestUrl?: string;
  contentManifestUrl?: string;
  localServerPort?: string;
  localServerBind?: string;
}

export function validateSettings(
  settings: LauncherSettings,
): SettingsValidationErrors {
  const errors: SettingsValidationErrors = {};

  if (
    !Number.isFinite(settings.memoryMb) ||
    settings.memoryMb < SETTINGS_RULES.memoryMin
  ) {
    errors.memoryMb = `Minimum ${SETTINGS_RULES.memoryMin} Mio.`;
  } else if (settings.memoryMb > SETTINGS_RULES.memoryMax) {
    errors.memoryMb = `Maximum ${SETTINGS_RULES.memoryMax} Mio.`;
  }

  if (
    !Number.isInteger(settings.serverPort) ||
    settings.serverPort < SETTINGS_RULES.portMin ||
    settings.serverPort > SETTINGS_RULES.portMax
  ) {
    errors.serverPort = `Port entre ${SETTINGS_RULES.portMin} et ${SETTINGS_RULES.portMax}.`;
  }

  if (settings.serverAddress.length > SETTINGS_RULES.serverAddressMax) {
    errors.serverAddress = `Adresse trop longue (max ${SETTINGS_RULES.serverAddressMax}).`;
  } else if (
    settings.serverAddress.length > 0 &&
    settings.serverAddress.trim().length === 0
  ) {
    errors.serverAddress = "Adresse invalide.";
  }

  const manifestUrl = settings.manifestUrl.trim();
  if (manifestUrl.length === 0) {
    errors.manifestUrl = "URL requise.";
  } else if (!/^https?:\/\//i.test(manifestUrl)) {
    errors.manifestUrl = "Doit commencer par http(s)://";
  }

  const contentManifestUrl = settings.contentManifestUrl.trim();
  if (contentManifestUrl.length === 0) {
    errors.contentManifestUrl = "URL requise.";
  } else if (!/^https?:\/\//i.test(contentManifestUrl)) {
    errors.contentManifestUrl = "Doit commencer par http(s)://";
  }

  if (
    !Number.isInteger(settings.localServerPort) ||
    settings.localServerPort < SETTINGS_RULES.portMin ||
    settings.localServerPort > SETTINGS_RULES.portMax
  ) {
    errors.localServerPort = `Port entre ${SETTINGS_RULES.portMin} et ${SETTINGS_RULES.portMax}.`;
  }

  const bind = settings.localServerBind.trim();
  if (bind.length === 0) {
    errors.localServerBind = "Adresse requise.";
  } else if (!isValidIp(bind)) {
    errors.localServerBind = "Doit être une adresse IP (ex : 0.0.0.0 ou 127.0.0.1).";
  }

  return errors;
}

function isValidIp(s: string): boolean {
  // IPv4
  const parts = s.split(".");
  if (parts.length === 4 && parts.every((p) => /^\d+$/.test(p) && Number(p) <= 255)) {
    return true;
  }
  // very loose IPv6 acceptance — Rust validates strictly server-side
  return /^[0-9a-fA-F:]+$/.test(s) && s.includes(":");
}

export function hasErrors(errors: SettingsValidationErrors): boolean {
  return Object.values(errors).some((v) => v !== undefined);
}
