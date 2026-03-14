import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";

// ---------------------------------------------------------------------------
// localStorage mock — history.ts imports getPreferences which reads localStorage
// ---------------------------------------------------------------------------
const localStorageMock = {
  getItem: vi.fn(() => null),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};
Object.defineProperty(globalThis, "localStorage", {
  value: localStorageMock,
  writable: true,
});

import { history, type Command } from "$lib/stores/history";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeCommand(desc: string): Command {
  return {
    description: desc,
    changes: [{ path: ["field"], oldValue: "old", newValue: "new" }],
  };
}

function getState() {
  return get(history);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("history store", () => {
  beforeEach(() => {
    history.clear();
    // Reset to a known max depth
    history.setMaxDepth(100);
  });

  it("push adds to undo stack", () => {
    const cmd = makeCommand("edit field");
    history.push(cmd);
    expect(getState().undoStack).toHaveLength(1);
    expect(getState().undoStack[0].description).toBe("edit field");
  });

  it("undo pops from undo and pushes to redo", () => {
    const cmd = makeCommand("edit");
    history.push(cmd);
    const result = history.undo();
    expect(result).toEqual(cmd);
    expect(getState().undoStack).toHaveLength(0);
    expect(getState().redoStack).toHaveLength(1);
    expect(getState().redoStack[0]).toEqual(cmd);
  });

  it("redo pops from redo and pushes to undo", () => {
    const cmd = makeCommand("edit");
    history.push(cmd);
    history.undo();
    const result = history.redo();
    expect(result).toEqual(cmd);
    expect(getState().redoStack).toHaveLength(0);
    expect(getState().undoStack).toHaveLength(1);
    expect(getState().undoStack[0]).toEqual(cmd);
  });

  it("push clears redo stack", () => {
    const cmd1 = makeCommand("first");
    const cmd2 = makeCommand("second");
    history.push(cmd1);
    history.undo();
    // There's now something in redo
    expect(getState().redoStack).toHaveLength(1);
    // Pushing a new command should wipe redo
    history.push(cmd2);
    expect(getState().redoStack).toHaveLength(0);
  });

  it("undo on empty stack returns null", () => {
    expect(getState().undoStack).toHaveLength(0);
    const result = history.undo();
    expect(result).toBeNull();
  });

  it("redo on empty stack returns null", () => {
    expect(getState().redoStack).toHaveLength(0);
    const result = history.redo();
    expect(result).toBeNull();
  });

  it("clear empties both stacks", () => {
    history.push(makeCommand("a"));
    history.push(makeCommand("b"));
    history.undo();
    expect(getState().undoStack.length).toBeGreaterThan(0);
    expect(getState().redoStack.length).toBeGreaterThan(0);
    history.clear();
    expect(getState().undoStack).toHaveLength(0);
    expect(getState().redoStack).toHaveLength(0);
  });

  it("max depth limits stack size", () => {
    history.setMaxDepth(3);
    history.push(makeCommand("cmd1"));
    history.push(makeCommand("cmd2"));
    history.push(makeCommand("cmd3"));
    history.push(makeCommand("cmd4")); // should evict cmd1
    const stack = getState().undoStack;
    expect(stack).toHaveLength(3);
    // Oldest entry should have been dropped
    expect(stack[0].description).toBe("cmd2");
    expect(stack[2].description).toBe("cmd4");
  });

  it("peekUndo returns description of top undo item, or null when empty", () => {
    expect(history.peekUndo()).toBeNull();
    history.push(makeCommand("top"));
    expect(history.peekUndo()).toBe("top");
  });

  it("peekRedo returns description of top redo item, or null when empty", () => {
    expect(history.peekRedo()).toBeNull();
    history.push(makeCommand("item"));
    history.undo();
    expect(history.peekRedo()).toBe("item");
  });
});
