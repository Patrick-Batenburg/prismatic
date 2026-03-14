const RESERVED: Set<string> = new Set([
  "Ctrl+W",
  "Ctrl+T",
  "Ctrl+N",
  "Ctrl+Tab",
  "Ctrl+Shift+Tab",
  "Alt+F4",
  "F5",
  "F12",
]);

const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);

/** Normalize a KeyboardEvent into the canonical combo string. Returns null for modifier-only presses. */
export function eventToCombo(e: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null;

  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");

  let key = e.key;
  if (key === " ") key = "Space";
  if (key.length === 1) key = key.toUpperCase();

  parts.push(key);
  return parts.join("+");
}

/** Check if a combo string is reserved (browser/OS shortcut). */
export function isReserved(combo: string): boolean {
  return RESERVED.has(combo);
}

/** Check if a KeyboardEvent matches a combo string. */
export function matchesCombo(e: KeyboardEvent, combo: string): boolean {
  return eventToCombo(e) === combo;
}

/** Human-readable label for an action id. */
export const ACTION_LABELS: Record<string, string> = {
  undo: "Undo",
  redo: "Redo",
  redoAlt: "Redo (alt)",
  save: "Save",
  reload: "Reload",
  toggleBatch: "Toggle Batch Mode",
};
