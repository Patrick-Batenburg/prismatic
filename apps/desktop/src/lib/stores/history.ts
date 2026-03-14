import { writable, get } from "svelte/store";
import { getPreferences } from "$lib/preferences";

export interface Change {
  path: string[];
  oldValue: unknown;
  newValue: unknown;
}

export interface Command {
  description: string;
  changes: Change[];
}

interface HistoryState {
  undoStack: Command[];
  redoStack: Command[];
  maxDepth: number;
}

function createHistoryStore() {
  const { subscribe, update } = writable<HistoryState>({
    undoStack: [],
    redoStack: [],
    maxDepth: getPreferences().undoHistoryDepth,
  });

  return {
    subscribe,

    push(command: Command) {
      update((state) => {
        const undoStack = [...state.undoStack, command];
        if (undoStack.length > state.maxDepth) {
          undoStack.shift();
        }
        return { ...state, undoStack, redoStack: [] };
      });
    },

    undo(): Command | null {
      let undone: Command | null = null;
      update((state) => {
        if (state.undoStack.length === 0) return state;
        const undoStack = [...state.undoStack];
        const command = undoStack.pop()!;
        undone = command;
        return {
          ...state,
          undoStack,
          redoStack: [...state.redoStack, command],
        };
      });
      return undone;
    },

    redo(): Command | null {
      let redone: Command | null = null;
      update((state) => {
        if (state.redoStack.length === 0) return state;
        const redoStack = [...state.redoStack];
        const command = redoStack.pop()!;
        redone = command;
        return {
          ...state,
          undoStack: [...state.undoStack, command],
          redoStack,
        };
      });
      return redone;
    },

    clear() {
      update((state) => ({ ...state, undoStack: [], redoStack: [] }));
    },

    /** Imperative-only helpers (use $history.undoStack.length for reactive checks) */
    peekUndo(): string | null {
      const state = get({ subscribe });
      if (state.undoStack.length === 0) return null;
      return state.undoStack[state.undoStack.length - 1].description;
    },

    peekRedo(): string | null {
      const state = get({ subscribe });
      if (state.redoStack.length === 0) return null;
      return state.redoStack[state.redoStack.length - 1].description;
    },

    setMaxDepth(depth: number) {
      update((state) => ({ ...state, maxDepth: depth }));
    },
  };
}

export const history = createHistoryStore();

export function trackEdit(
  path: string[],
  oldValue: unknown,
  newValue: unknown,
  description: string,
) {
  if (oldValue === newValue) return;
  history.push({
    description,
    changes: [{ path, oldValue, newValue }],
  });
}
