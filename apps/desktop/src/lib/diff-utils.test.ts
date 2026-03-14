import { describe, it, expect } from "vitest";
import { simpleLineDiff, patienceDiff, collapseContext } from "$lib/diff-utils";

// ---------------------------------------------------------------------------
// simpleLineDiff
// ---------------------------------------------------------------------------

describe("simpleLineDiff", () => {
  it("identical arrays — all context", () => {
    const lines = ["a", "b", "c"];
    const result = simpleLineDiff(lines, lines);
    expect(result).toEqual([
      { type: "context", line: "a" },
      { type: "context", line: "b" },
      { type: "context", line: "c" },
    ]);
  });

  it("single line changed — one removed, one added", () => {
    const result = simpleLineDiff(["a", "b", "c"], ["a", "X", "c"]);
    expect(result).toEqual([
      { type: "context", line: "a" },
      { type: "removed", line: "b" },
      { type: "added", line: "X" },
      { type: "context", line: "c" },
    ]);
  });

  it("empty arrays — no output", () => {
    expect(simpleLineDiff([], [])).toEqual([]);
  });

  it("addition only — empty a, non-empty b", () => {
    const result = simpleLineDiff([], ["x", "y"]);
    expect(result).toEqual([
      { type: "added", line: "x" },
      { type: "added", line: "y" },
    ]);
  });

  it("removal only — non-empty a, empty b", () => {
    const result = simpleLineDiff(["x", "y"], []);
    expect(result).toEqual([
      { type: "removed", line: "x" },
      { type: "removed", line: "y" },
    ]);
  });
});

// ---------------------------------------------------------------------------
// patienceDiff
// ---------------------------------------------------------------------------

describe("patienceDiff", () => {
  it("identical input — returns hunk 'No differences'", () => {
    const lines = ["a", "b", "c", "d", "e"];
    const result = patienceDiff(lines, lines);
    expect(result).toHaveLength(1);
    expect(result[0].type).toBe("hunk");
    expect(result[0].line).toContain("No differences");
  });

  it("single line added — has 'added' entries", () => {
    const a = ["a", "b", "c"];
    const b = ["a", "b", "NEW", "c"];
    const result = patienceDiff(a, b);
    const added = result.filter((l) => l.type === "added");
    expect(added).toHaveLength(1);
    expect(added[0].line).toBe("NEW");
  });

  it("single line removed — has 'removed' entries", () => {
    const a = ["a", "b", "c"];
    const b = ["a", "c"];
    const result = patienceDiff(a, b);
    const removed = result.filter((l) => l.type === "removed");
    expect(removed).toHaveLength(1);
    expect(removed[0].line).toBe("b");
  });

  it("context collapsing — large identical sections get collapsed into 'hunk' entries", () => {
    // 20 identical lines with a single change in the middle
    const shared = Array.from({ length: 20 }, (_, i) => `line${i}`);
    const a = [...shared, "OLD", ...shared];
    const b = [...shared, "NEW", ...shared];
    const result = patienceDiff(a, b);
    const hunks = result.filter((l) => l.type === "hunk");
    expect(hunks.length).toBeGreaterThan(0);
  });

  it("moved block — patience diff handles unique line matching", () => {
    // Lines that are unique in both sides should be matched as anchors
    const a = ["UNIQUE_A", "common1", "common2"];
    const b = ["common1", "common2", "UNIQUE_B"];
    const result = patienceDiff(a, b);
    const types = result.map((l) => l.type);
    // There should be removed and added entries (UNIQUE_A removed, UNIQUE_B added)
    expect(types).toContain("removed");
    expect(types).toContain("added");
  });
});

// ---------------------------------------------------------------------------
// collapseContext
// ---------------------------------------------------------------------------

describe("collapseContext", () => {
  it("no changes — returns 'No differences' hunk", () => {
    const edits = [
      { type: "context" as const, line: "a" },
      { type: "context" as const, line: "b" },
    ];
    const result = collapseContext(edits, 3);
    expect(result).toHaveLength(1);
    expect(result[0].type).toBe("hunk");
    expect(result[0].line).toContain("No differences");
  });

  it("change at start — shows context after", () => {
    const edits = [
      { type: "removed" as const, line: "OLD" },
      { type: "added" as const, line: "NEW" },
      { type: "context" as const, line: "c1" },
      { type: "context" as const, line: "c2" },
      // 10 more unchanged lines that should collapse
      ...Array.from({ length: 10 }, (_, i) => ({ type: "context" as const, line: `x${i}` })),
    ];
    const result = collapseContext(edits, 2);
    const hunks = result.filter((l) => l.type === "hunk");
    expect(hunks.length).toBeGreaterThan(0);
    // The removed/added should be present
    expect(result.some((l) => l.type === "removed")).toBe(true);
    expect(result.some((l) => l.type === "added")).toBe(true);
  });

  it("change at end — shows context before", () => {
    const edits = [
      // 10 unchanged lines then a change
      ...Array.from({ length: 10 }, (_, i) => ({ type: "context" as const, line: `x${i}` })),
      { type: "removed" as const, line: "OLD" },
      { type: "added" as const, line: "NEW" },
    ];
    const result = collapseContext(edits, 2);
    const hunks = result.filter((l) => l.type === "hunk");
    // The leading unchanged lines should be collapsed into a hunk
    expect(hunks.length).toBeGreaterThan(0);
    expect(result.some((l) => l.type === "removed")).toBe(true);
    expect(result.some((l) => l.type === "added")).toBe(true);
  });

  it("changes far apart — collapsed section between them", () => {
    const edits = [
      { type: "removed" as const, line: "FIRST_OLD" },
      { type: "added" as const, line: "FIRST_NEW" },
      ...Array.from({ length: 20 }, (_, i) => ({ type: "context" as const, line: `mid${i}` })),
      { type: "removed" as const, line: "SECOND_OLD" },
      { type: "added" as const, line: "SECOND_NEW" },
    ];
    const result = collapseContext(edits, 3);
    const hunks = result.filter((l) => l.type === "hunk");
    // The 20 middle lines (minus 3 ctx on each side) should collapse
    expect(hunks.length).toBeGreaterThan(0);
    expect(hunks[0].line).toMatch(/@@ \d+ unchanged lines @@/);
  });
});
