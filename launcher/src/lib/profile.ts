import { invoke } from "@tauri-apps/api/core";

/**
 * Local offline player profile.
 * `id` is a stable UUID v4 generated on first launch and persisted on disk.
 * It is the player identity used by the game until we plug a real auth server.
 */
export interface PlayerProfile {
  id: string;
  username: string;
  createdAt: string;
  updatedAt: string;
}

/** Loads the persisted profile from the user data dir, or `null` if none exists yet. */
export async function loadProfile(): Promise<PlayerProfile | null> {
  return await invoke<PlayerProfile | null>("load_profile");
}

/** Creates or updates the persisted profile with the given username. */
export async function saveProfile(username: string): Promise<PlayerProfile> {
  return await invoke<PlayerProfile>("save_profile", { username });
}

/** Removes the persisted profile (used for "log out"). */
export async function clearProfile(): Promise<void> {
  await invoke<void>("clear_profile");
}

/** Username validation rules — kept identical on Rust side for consistency. */
export const USERNAME_RULES = {
  min: 3,
  max: 16,
  pattern: /^[A-Za-z0-9_]+$/,
} as const;

export function validateUsername(value: string): string | null {
  const trimmed = value.trim();
  if (trimmed.length < USERNAME_RULES.min) {
    return `Au moins ${USERNAME_RULES.min} caractères.`;
  }
  if (trimmed.length > USERNAME_RULES.max) {
    return `Au maximum ${USERNAME_RULES.max} caractères.`;
  }
  if (!USERNAME_RULES.pattern.test(trimmed)) {
    return "Lettres, chiffres et _ uniquement.";
  }
  return null;
}
