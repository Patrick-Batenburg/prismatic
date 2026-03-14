import { writable, get } from "svelte/store";

const PREFS_KEY = "preferences";
const ENGINE_ORDER_KEY = "engine_order";

export type UpdateMode = "auto" | "notify" | "off";

export interface Preferences {
  // General
  deepScanDefault: boolean;
  updateMode: UpdateMode;
  notificationDuration: number;
  statusFlashDuration: number;
  tablePageSize: number;
  // Appearance
  theme: "light" | "dark";
  accentColor: string;
  appFont: string;
  monoFont: string;
  // Editor
  undoHistoryDepth: number;
  searchDebounce: number;
  showUnchangedDefault: boolean;
  // Keyboard Shortcuts
  keybindings: Record<string, string>;
}

export const DEFAULT_KEYBINDINGS: Record<string, string> = {
  undo: "Ctrl+Z",
  redo: "Ctrl+Shift+Z",
  redoAlt: "Ctrl+Y",
  save: "Ctrl+S",
  reload: "Ctrl+R",
  toggleBatch: "Ctrl+B",
};

const DEFAULTS: Preferences = {
  deepScanDefault: false,
  updateMode: "auto",
  notificationDuration: 4,
  statusFlashDuration: 5,
  tablePageSize: 50,
  theme: "dark",
  accentColor: "#6c5ce7",
  appFont: "system-default",
  monoFont: "Cascadia Code",
  undoHistoryDepth: 100,
  searchDebounce: 400,
  showUnchangedDefault: false,
  keybindings: { ...DEFAULT_KEYBINDINGS },
};

const VALID_PAGE_SIZES = [25, 50, 100, 200];
const VALID_DEBOUNCES = [200, 400, 600, 800];
const VALID_NOTIFICATION_DURATIONS = [2, 4, 6, 8];
const VALID_STATUS_FLASH_DURATIONS = [3, 5, 8, 10];
const MIN_UNDO_DEPTH = 10;
const MAX_UNDO_DEPTH = 1000;
const HEX_RE = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;

function validate(raw: Record<string, unknown>): Preferences {
  const merged = { ...DEFAULTS, ...raw };
  // General
  const deepScanDefault =
    typeof merged.deepScanDefault === "boolean" ? merged.deepScanDefault : DEFAULTS.deepScanDefault;
  const updateModeStr = String(merged.updateMode);
  const updateMode: UpdateMode =
    updateModeStr === "auto" || updateModeStr === "notify" || updateModeStr === "off"
      ? updateModeStr
      : DEFAULTS.updateMode;
  const notificationDuration = VALID_NOTIFICATION_DURATIONS.includes(
    Number(merged.notificationDuration),
  )
    ? Number(merged.notificationDuration)
    : DEFAULTS.notificationDuration;
  const statusFlashDuration = VALID_STATUS_FLASH_DURATIONS.includes(
    Number(merged.statusFlashDuration),
  )
    ? Number(merged.statusFlashDuration)
    : DEFAULTS.statusFlashDuration;
  const tablePageSize = VALID_PAGE_SIZES.includes(Number(merged.tablePageSize))
    ? Number(merged.tablePageSize)
    : DEFAULTS.tablePageSize;
  // Appearance
  const themeStr = String(merged.theme);
  const theme: "light" | "dark" =
    themeStr === "light" || themeStr === "dark" ? themeStr : DEFAULTS.theme;
  const accentColor =
    typeof merged.accentColor === "string" && HEX_RE.test(merged.accentColor)
      ? merged.accentColor
      : DEFAULTS.accentColor;
  const appFont = typeof merged.appFont === "string" ? merged.appFont : DEFAULTS.appFont;
  const monoFont = typeof merged.monoFont === "string" ? merged.monoFont : DEFAULTS.monoFont;
  // Editor
  const depthNum = Number(merged.undoHistoryDepth);
  const undoHistoryDepth =
    isNaN(depthNum) || depthNum < MIN_UNDO_DEPTH || depthNum > MAX_UNDO_DEPTH
      ? DEFAULTS.undoHistoryDepth
      : Math.round(depthNum);
  const searchDebounce = VALID_DEBOUNCES.includes(Number(merged.searchDebounce))
    ? Number(merged.searchDebounce)
    : DEFAULTS.searchDebounce;
  const showUnchangedDefault =
    typeof merged.showUnchangedDefault === "boolean"
      ? merged.showUnchangedDefault
      : DEFAULTS.showUnchangedDefault;
  // Keybindings
  const rawBindings =
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- runtime data from localStorage may be null
    typeof merged.keybindings === "object" && merged.keybindings !== null ? merged.keybindings : {};
  const keybindings: Record<string, string> = { ...DEFAULT_KEYBINDINGS };
  for (const [k, v] of Object.entries(rawBindings)) {
    if (typeof v === "string") keybindings[k] = v;
  }

  return {
    deepScanDefault,
    updateMode,
    notificationDuration,
    statusFlashDuration,
    tablePageSize,
    theme,
    accentColor,
    appFont,
    monoFont,
    undoHistoryDepth,
    searchDebounce,
    showUnchangedDefault,
    keybindings,
  };
}

function loadFromStorage(): Preferences {
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    if (!raw) return { ...DEFAULTS };
    return validate(JSON.parse(raw));
  } catch {
    return { ...DEFAULTS };
  }
}

export const preferencesStore = writable<Preferences>(loadFromStorage());

export function getPreferences(): Preferences {
  return get(preferencesStore);
}

export function setPreferences(update: Partial<Preferences>) {
  preferencesStore.update((current) => {
    const merged = validate({ ...current, ...update });
    try {
      localStorage.setItem(PREFS_KEY, JSON.stringify(merged));
    } catch {
      /* ignore */
    }
    return merged;
  });
}

// --- Engine order (unchanged) ---

export function getEngineOrder(): string[] {
  try {
    const raw = localStorage.getItem(ENGINE_ORDER_KEY);
    if (!raw) return [];
    return JSON.parse(raw);
  } catch {
    return [];
  }
}

export function setEngineOrder(order: string[]) {
  try {
    localStorage.setItem(ENGINE_ORDER_KEY, JSON.stringify(order));
  } catch {
    /* ignore */
  }
}

/** Sort engines by saved order. Unknown engines go at the end. */
export function sortEngines<T extends { id: string }>(engines: T[]): T[] {
  const order = getEngineOrder();
  if (order.length === 0) return engines;
  const indexMap = new Map(order.map((id, i) => [id, i]));
  return [...engines].sort((a, b) => {
    const ai = indexMap.get(a.id) ?? Infinity;
    const bi = indexMap.get(b.id) ?? Infinity;
    return ai - bi;
  });
}
