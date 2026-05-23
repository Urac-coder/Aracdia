import { invoke } from "@tauri-apps/api/core";

export interface LauncherSettings {
  memoryMb: number;
  serverAddress: string;
  serverPort: number;
  autoConnect: boolean;
  installDir: string | null;
  manifestUrl: string;
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
  "https://api.github.com/repos/aracdia/aracdia-engine/releases/latest";

export const DEFAULT_SETTINGS: LauncherSettings = {
  memoryMb: 2048,
  serverAddress: "",
  serverPort: 30_000,
  autoConnect: false,
  installDir: null,
  manifestUrl: DEFAULT_MANIFEST_URL,
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

  return errors;
}

export function hasErrors(errors: SettingsValidationErrors): boolean {
  return Object.values(errors).some((v) => v !== undefined);
}
