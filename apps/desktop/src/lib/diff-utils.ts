export type DiffLineType = "context" | "added" | "removed" | "hunk";

const DIFF_FALLBACK_THRESHOLD = 2_000_000;
const DIFF_CONTEXT_LINES = 3;

export interface DiffLine {
  type: DiffLineType;
  line: string;
}

type Edit = { type: "context" | "added" | "removed"; line: string };

export function simpleLineDiff(a: string[], b: string[]): Edit[] {
  const result: Edit[] = [];
  const max = Math.max(a.length, b.length);
  for (let i = 0; i < max; i++) {
    if (i < a.length && i < b.length && a[i] === b[i]) {
      result.push({ type: "context", line: a[i] });
    } else {
      if (i < a.length) result.push({ type: "removed", line: a[i] });
      if (i < b.length) result.push({ type: "added", line: b[i] });
    }
  }
  return result;
}

/** Fallback LCS for small segments without unique anchors. */
function fallbackLCS(
  a: string[],
  aStart: number,
  aEnd: number,
  b: string[],
  bStart: number,
  bEnd: number,
): Edit[] {
  const n = aEnd - aStart,
    m = bEnd - bStart;

  // For very large segments, use simple line diff
  if (n * m > DIFF_FALLBACK_THRESHOLD) {
    return simpleLineDiff(a.slice(aStart, aEnd), b.slice(bStart, bEnd));
  }

  // Standard O(nm) LCS
  const table: number[][] = [];
  for (let i = 0; i <= n; i++) table[i] = new Array(m + 1).fill(0);
  for (let i = 1; i <= n; i++) {
    for (let j = 1; j <= m; j++) {
      if (a[aStart + i - 1] === b[bStart + j - 1]) {
        table[i][j] = table[i - 1][j - 1] + 1;
      } else {
        table[i][j] = Math.max(table[i - 1][j], table[i][j - 1]);
      }
    }
  }

  const edits: Edit[] = [];
  let i = n,
    j = m;
  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && a[aStart + i - 1] === b[bStart + j - 1]) {
      edits.push({ type: "context", line: a[aStart + i - 1] });
      i--;
      j--;
    } else if (j > 0 && (i === 0 || table[i][j - 1] >= table[i - 1][j])) {
      edits.push({ type: "added", line: b[bStart + j - 1] });
      j--;
    } else {
      edits.push({ type: "removed", line: a[aStart + i - 1] });
      i--;
    }
  }
  edits.reverse();
  return edits;
}

/** Patience diff: finds unique matching lines as anchors, then recursively diffs gaps.
 *  Produces cleaner diffs for structured data like JSON. */
export function patienceDiff(a: string[], b: string[]): DiffLine[] {
  function diff(
    a: string[],
    aStartParam: number,
    aEndParam: number,
    b: string[],
    bStartParam: number,
    bEndParam: number,
  ): Edit[] {
    let aStart = aStartParam;
    let aEnd = aEndParam;
    let bStart = bStartParam;
    let bEnd = bEndParam;
    const edits: Edit[] = [];

    // Skip common prefix
    while (aStart < aEnd && bStart < bEnd && a[aStart] === b[bStart]) {
      edits.push({ type: "context", line: a[aStart] });
      aStart++;
      bStart++;
    }
    // Skip common suffix
    const suffix: Edit[] = [];
    while (aStart < aEnd && bStart < bEnd && a[aEnd - 1] === b[bEnd - 1]) {
      suffix.push({ type: "context", line: a[aEnd - 1] });
      aEnd--;
      bEnd--;
    }
    suffix.reverse();

    if (aStart === aEnd) {
      // Only additions remain
      for (let i = bStart; i < bEnd; i++) edits.push({ type: "added", line: b[i] });
    } else if (bStart === bEnd) {
      // Only removals remain
      for (let i = aStart; i < aEnd; i++) edits.push({ type: "removed", line: a[i] });
    } else {
      // Find unique lines in both sides
      const uniqueA = new Map<string, number[]>();
      for (let i = aStart; i < aEnd; i++) {
        const arr = uniqueA.get(a[i]);
        if (arr) arr.push(i);
        else uniqueA.set(a[i], [i]);
      }
      const uniqueB = new Map<string, number[]>();
      for (let i = bStart; i < bEnd; i++) {
        const arr = uniqueB.get(b[i]);
        if (arr) arr.push(i);
        else uniqueB.set(b[i], [i]);
      }

      // Collect lines unique in both A and B, keyed by their position in A
      const matches: { ai: number; bi: number }[] = [];
      for (const [line, aIndices] of uniqueA) {
        if (aIndices.length !== 1) continue;
        const bIndices = uniqueB.get(line);
        if (bIndices?.length !== 1) continue;
        matches.push({ ai: aIndices[0], bi: bIndices[0] });
      }

      // Sort by position in A
      matches.sort((x, y) => x.ai - y.ai);

      // Find longest increasing subsequence of B-indices (patience sorting)
      const piles: { bi: number; ai: number; prev: number }[] = [];
      const pileTops: number[] = []; // index into piles[] for each pile's top card

      for (const m of matches) {
        // Binary search for the leftmost pile whose top bi >= m.bi
        let lo = 0,
          hi = pileTops.length;
        while (lo < hi) {
          const mid = (lo + hi) >> 1;
          if (piles[pileTops[mid]].bi < m.bi) lo = mid + 1;
          else hi = mid;
        }
        const prev = lo > 0 ? pileTops[lo - 1] : -1;
        const card = { bi: m.bi, ai: m.ai, prev };
        const idx = piles.length;
        piles.push(card);
        if (lo === pileTops.length) pileTops.push(idx);
        else pileTops[lo] = idx;
      }

      // Backtrack to recover the LIS
      const anchors: { ai: number; bi: number }[] = [];
      if (pileTops.length > 0) {
        let k = pileTops[pileTops.length - 1];
        while (k >= 0) {
          anchors.push({ ai: piles[k].ai, bi: piles[k].bi });
          k = piles[k].prev;
        }
        anchors.reverse();
      }

      if (anchors.length === 0) {
        // No unique anchors found — fall back to simple LCS for this segment
        edits.push(...fallbackLCS(a, aStart, aEnd, b, bStart, bEnd));
      } else {
        // Recursively diff gaps between anchors
        let prevAi = aStart,
          prevBi = bStart;
        for (const anchor of anchors) {
          edits.push(...diff(a, prevAi, anchor.ai, b, prevBi, anchor.bi));
          edits.push({ type: "context", line: a[anchor.ai] });
          prevAi = anchor.ai + 1;
          prevBi = anchor.bi + 1;
        }
        edits.push(...diff(a, prevAi, aEnd, b, prevBi, bEnd));
      }
    }

    edits.push(...suffix);
    return edits;
  }

  const edits = diff(a, 0, a.length, b, 0, b.length);
  return collapseContext(edits, DIFF_CONTEXT_LINES);
}

/** Collapse unchanged context lines into hunks, keeping ctx lines around changes. */
export function collapseContext(edits: Edit[], ctx: number): DiffLine[] {
  // Find indices of changed lines
  const changed = new Set<number>();
  edits.forEach((e, i) => {
    if (e.type !== "context") changed.add(i);
  });

  if (changed.size === 0) return [{ type: "hunk", line: "No differences in raw data" }];

  // Mark context lines to keep (within ctx lines of a change)
  const keep = new Set<number>();
  for (const idx of changed) {
    for (let k = Math.max(0, idx - ctx); k <= Math.min(edits.length - 1, idx + ctx); k++) {
      keep.add(k);
    }
    keep.add(idx);
  }

  const result: DiffLine[] = [];
  let skipped = 0;

  for (let i = 0; i < edits.length; i++) {
    if (keep.has(i)) {
      if (skipped > 0) {
        result.push({ type: "hunk", line: `@@ ${skipped} unchanged lines @@` });
        skipped = 0;
      }
      result.push(edits[i]);
    } else {
      skipped++;
    }
  }
  if (skipped > 0) {
    result.push({ type: "hunk", line: `@@ ${skipped} unchanged lines @@` });
  }

  return result;
}
