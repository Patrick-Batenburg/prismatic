import { writable } from "svelte/store";
import type { EngineInfo, SaveFile, SaveData, NameMap, PatchInfo } from "$lib/api";

export const currentEngine = writable<EngineInfo | null>(null);
export const currentGameDir = writable<string | null>(null);
export const saves = writable<SaveFile[]>([]);
export const currentSave = writable<SaveData | null>(null);
export const currentSavePath = writable<string | null>(null);
export const nameMap = writable<NameMap | null>(null);
export const modifiedFields = writable<Set<string>>(new Set());
export const statusMessage = writable<string>("Ready");
export const activePatch = writable<PatchInfo | null>(null);
export const toasts = writable<
  { id: number; message: string; type: "success" | "error" | "info" }[]
>([]);

let toastId = 0;
export function addToast(
  message: string,
  type: "success" | "error" | "info" = "info",
  duration = 4000,
) {
  const id = toastId++;
  toasts.update((t) => [...t, { id, message, type }]);
  setTimeout(() => {
    toasts.update((t) => t.filter((toast) => toast.id !== id));
  }, duration);
}

export { history, trackEdit } from './history';
export type { Command, Change } from './history';

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
