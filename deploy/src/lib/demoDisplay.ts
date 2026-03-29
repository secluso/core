// SPDX-License-Identifier: GPL-3.0-or-later

const DEV_SETTINGS_STORAGE_KEY = "secluso-dev-settings";
const MACOS_HOME_PATH_PATTERN = /\/Users\/[^/]+(?=\/|$)/g;

type DemoDisplaySettings = {
  enabled?: boolean;
  maskUserPathsWithDemo?: boolean;
};

function shouldMaskUserPathsWithDemo(): boolean {
  if (typeof localStorage === "undefined") return false;

  try {
    const raw = localStorage.getItem(DEV_SETTINGS_STORAGE_KEY);
    if (!raw) return false;

    const settings = JSON.parse(raw) as DemoDisplaySettings;
    return !!(settings.enabled && settings.maskUserPathsWithDemo);
  } catch {
    return false;
  }
}

// Temporary demo-only redaction for paths rendered in the UI.
export function maskDemoText(value: string | null | undefined): string {
  if (!value) return "";
  if (!shouldMaskUserPathsWithDemo()) return value;

  return value.replace(MACOS_HOME_PATH_PATTERN, "/Users/demo");
}
