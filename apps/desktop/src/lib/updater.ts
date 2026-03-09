import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export interface UpdateResult {
  available: boolean;
  version: string;
}

/**
 * Check for updates.
 * @param checkOnly If true, only checks availability without downloading.
 */
export async function checkForUpdates(checkOnly = false): Promise<UpdateResult | null> {
  try {
    const update = await check();
    if (!update) return { available: false, version: "" };

    if (update.available && !checkOnly) {
      await update.downloadAndInstall();
    }

    return { available: update.available, version: update.version };
  } catch {
    // Silently fail — updater may not be configured (dev builds, no endpoint)
    return null;
  }
}

/** Relaunch the app to apply an installed update. */
export async function relaunchApp(): Promise<void> {
  await relaunch();
}
