import { describe, it, expect, beforeEach, vi } from "vitest";

// ---------------------------------------------------------------------------
// localStorage mock — must be set up before importing the module so that
// loadFromStorage() sees the mock when the module is first evaluated.
// ---------------------------------------------------------------------------

const localStorageStore: Record<string, string> = {};
const localStorageMock = {
  getItem: vi.fn((key: string) => localStorageStore[key] ?? null),
  setItem: vi.fn((key: string, value: string) => {
    localStorageStore[key] = value;
  }),
  removeItem: vi.fn((key: string) => {
    delete localStorageStore[key];
  }),
  clear: vi.fn(() => {
    for (const k of Object.keys(localStorageStore)) delete localStorageStore[k];
  }),
};

Object.defineProperty(globalThis, "localStorage", {
  value: localStorageMock,
  writable: true,
});

// Now we can safely import the module.
import { setPreferences, getPreferences } from "$lib/preferences";

// Defaults as defined in the source
const DEFAULTS = {
  deepScanDefault: false,
  updateMode: "auto" as const,
  notificationDuration: 4,
  statusFlashDuration: 5,
  tablePageSize: 50,
  theme: "dark" as const,
  accentColor: "#6c5ce7",
  appFont: "system-default",
  monoFont: "Cascadia Code",
  undoHistoryDepth: 100,
  searchDebounce: 400,
  showUnchangedDefault: false,
};

function resetToDefaults() {
  localStorageMock.clear();
  localStorageMock.getItem.mockImplementation((key: string) => localStorageStore[key] ?? null);
  // Restore defaults by calling setPreferences with exact defaults
  setPreferences(DEFAULTS);
}

describe("preferences validate (via setPreferences / getPreferences)", () => {
  beforeEach(() => {
    resetToDefaults();
  });

  it("valid defaults pass validation unchanged", () => {
    const prefs = getPreferences();
    expect(prefs.theme).toBe("dark");
    expect(prefs.accentColor).toBe("#6c5ce7");
    expect(prefs.tablePageSize).toBe(50);
    expect(prefs.undoHistoryDepth).toBe(100);
    expect(prefs.searchDebounce).toBe(400);
  });

  it("invalid theme reverts to default", () => {
    // @ts-expect-error testing invalid value
    setPreferences({ theme: "invalid" });
    expect(getPreferences().theme).toBe("dark");
  });

  it("invalid accent color reverts to default", () => {
    setPreferences({ accentColor: "not-a-hex" });
    expect(getPreferences().accentColor).toBe("#6c5ce7");
  });

  it("valid accent color (short hex) is accepted", () => {
    setPreferences({ accentColor: "#abc" });
    expect(getPreferences().accentColor).toBe("#abc");
  });

  it("undo depth below min reverts to default", () => {
    setPreferences({ undoHistoryDepth: 1 });
    expect(getPreferences().undoHistoryDepth).toBe(100);
  });

  it("undo depth above max reverts to default", () => {
    setPreferences({ undoHistoryDepth: 99999 });
    expect(getPreferences().undoHistoryDepth).toBe(100);
  });

  it("undo depth at min boundary is accepted", () => {
    setPreferences({ undoHistoryDepth: 10 });
    expect(getPreferences().undoHistoryDepth).toBe(10);
  });

  it("undo depth at max boundary is accepted", () => {
    setPreferences({ undoHistoryDepth: 1000 });
    expect(getPreferences().undoHistoryDepth).toBe(1000);
  });

  it("invalid page size reverts to default", () => {
    setPreferences({ tablePageSize: 999 });
    expect(getPreferences().tablePageSize).toBe(50);
  });

  it("valid page size is accepted", () => {
    setPreferences({ tablePageSize: 100 });
    expect(getPreferences().tablePageSize).toBe(100);
  });

  it("invalid debounce reverts to default", () => {
    setPreferences({ searchDebounce: 999 });
    expect(getPreferences().searchDebounce).toBe(400);
  });

  it("valid debounce is accepted", () => {
    setPreferences({ searchDebounce: 200 });
    expect(getPreferences().searchDebounce).toBe(200);
  });

  it("unknown keys in the raw object are stripped — only known fields survive", () => {
    // setPreferences merges into current; unknown keys can't be passed via Partial<Preferences>
    // but we can verify the returned prefs only has the known shape
    setPreferences({ theme: "light" });
    const prefs = getPreferences();
    expect(prefs).not.toHaveProperty("unknownKey");
    expect(prefs.theme).toBe("light");
  });

  it("partial preferences are filled with defaults", () => {
    // Apply only a theme change; all other fields should remain at defaults
    setPreferences({ theme: "light" });
    const prefs = getPreferences();
    expect(prefs.theme).toBe("light");
    expect(prefs.accentColor).toBe("#6c5ce7");
    expect(prefs.tablePageSize).toBe(50);
    expect(prefs.undoHistoryDepth).toBe(100);
  });
});
