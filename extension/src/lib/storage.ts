import { api } from "./browser-api";

export type OsOverride = "auto" | "windows" | "macos" | "linux";

export interface Settings {
  sessionName: string;
  osOverride: OsOverride;
  includeUserAgent: boolean;
}

const DEFAULTS: Settings = {
  sessionName: "",
  osOverride: "auto",
  includeUserAgent: false,
};

/**
 * `storage.local` only — deliberately never `storage.sync`. A session name
 * is meaningful only for the one browser profile/installation it was set
 * up for, so settings must not follow the user to another device.
 */
export async function loadSettings(): Promise<Settings> {
  const stored = await api.storage.local.get(["sessionName", "osOverride", "includeUserAgent"]);
  return {
    sessionName: typeof stored.sessionName === "string" ? stored.sessionName : DEFAULTS.sessionName,
    osOverride: isOsOverride(stored.osOverride) ? stored.osOverride : DEFAULTS.osOverride,
    includeUserAgent:
      typeof stored.includeUserAgent === "boolean"
        ? stored.includeUserAgent
        : DEFAULTS.includeUserAgent,
  };
}

export async function saveSettings(patch: Partial<Settings>): Promise<void> {
  await api.storage.local.set(patch);
}

function isOsOverride(value: unknown): value is OsOverride {
  return value === "auto" || value === "windows" || value === "macos" || value === "linux";
}
