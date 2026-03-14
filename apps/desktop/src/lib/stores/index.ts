import { writable } from "svelte/store";
import type { EngineInfo, SaveFile, SaveData, NameMap, PatchInfo } from "$lib/api";
import { getPreferences } from "$lib/preferences";

export const currentEngine = writable<EngineInfo | null>(null);
export const currentGameDir = writable<string | null>(null);
export const saves = writable<SaveFile[]>([]);
export const currentSave = writable<SaveData | null>(null);
export const currentSavePath = writable<string | null>(null);
export const nameMap = writable<NameMap | null>(null);
export const modifiedFields = writable<Set<string>>(new Set());
/** Temporary status flash (e.g. "Saved!", "Backup restored"). Clears after duration. */
export const statusFlash = writable<{ text: string; type: "success" | "error" | "info" } | null>(
  null,
);

let statusTimer: ReturnType<typeof setTimeout> | null = null;
export function setStatus(
  text: string,
  type: "success" | "error" | "info" = "info",
  duration?: number,
) {
  if (statusTimer !== null) clearTimeout(statusTimer);
  statusFlash.set({ text, type });
  const ms = duration ?? getPreferences().statusFlashDuration * 1000;
  statusTimer = setTimeout(() => {
    statusFlash.set(null);
    statusTimer = null;
  }, ms);
}
export const activePatch = writable<PatchInfo | null>(null);
export const toasts = writable<
  { id: number; message: string; type: "success" | "error" | "info" }[]
>([]);

let toastId = 0;
export function addToast(
  message: string,
  type: "success" | "error" | "info" = "info",
  duration?: number,
) {
  const id = toastId++;
  const ms = duration ?? getPreferences().notificationDuration * 1000;
  toasts.update((t) => [...t, { id, message, type }]);
  if (ms > 0) {
    setTimeout(() => {
      toasts.update((t) => t.filter((toast) => toast.id !== id));
    }, ms);
  }
}

export { history, trackEdit } from "./history";
export type { Command, Change } from "./history";

export function markModified(path: string) {
  modifiedFields.update((s) => {
    s.add(path);
    return new Set(s);
  });
}

export const batchMode = writable<boolean>(false);
export const batchSelected = writable<Set<string>>(new Set());

export function toggleBatchItem(id: string) {
  batchSelected.update((s) => {
    const next = new Set(s);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    return next;
  });
}

export function clearBatchSelection() {
  batchSelected.set(new Set());
}
