<script lang="ts">
  import type { SaveData, Character, Variable, Switch } from "$lib/api";
  import { getPreferences } from "$lib/preferences";

  interface DiffField {
    label: string;
    baseVal: unknown;
    compareVal: unknown;
    status: "added" | "removed" | "changed" | "unchanged";
  }

  interface Props {
    base: SaveData;
    compare: SaveData;
    baseName: string;
    compareName: string;
    onclose: () => void;
  }

  let { base, compare, baseName, compareName, onclose }: Props = $props();
  let showUnchanged = $state(getPreferences().showUnchangedDefault);

  // Only show tabs that have data in at least one save
  let availableSections = $derived(
    (() => {
      const sections: { id: string; label: string }[] = [];
      if (base.party || compare.party) sections.push({ id: "party", label: "Party" });
      if (base.inventory || compare.inventory)
        sections.push({ id: "inventory", label: "Inventory" });
      if (base.variables || compare.variables)
        sections.push({ id: "variables", label: "Variables" });
      if (base.switches || compare.switches) sections.push({ id: "switches", label: "Switches" });
      sections.push({ id: "raw", label: "Raw" });
      return sections;
    })(),
  );

  let activeSection = $state("");
  // Auto-select first available section
  $effect(() => {
    if (!activeSection || !availableSections.find((s) => s.id === activeSection)) {
      activeSection = availableSections[0]?.id ?? "raw";
    }
  });

  let partyDiffs = $derived(computePartyDiffs(base.party, compare.party));
  let variableDiffs = $derived(computeVariableDiffs(base.variables, compare.variables));
  let switchDiffs = $derived(computeSwitchDiffs(base.switches, compare.switches));
  let rawDiffLines = $derived(computeRawDiff(base.raw, compare.raw));

  function computePartyDiffs(
    a: Character[] | null,
    b: Character[] | null,
  ): { name: string; fields: DiffField[] }[] {
    const result: { name: string; fields: DiffField[] }[] = [];
    const maxLen = Math.max(a?.length ?? 0, b?.length ?? 0);
    for (let i = 0; i < maxLen; i++) {
      const charA = a?.[i];
      const charB = b?.[i];
      const fields: DiffField[] = [];
      if (!charA && charB) {
        fields.push({
          label: "(entire character)",
          baseVal: null,
          compareVal: charB.name,
          status: "added",
        });
      } else if (charA && !charB) {
        fields.push({
          label: "(entire character)",
          baseVal: charA.name,
          compareVal: null,
          status: "removed",
        });
      } else if (charA && charB) {
        if (charA.name !== charB.name)
          fields.push({
            label: "Name",
            baseVal: charA.name,
            compareVal: charB.name,
            status: "changed",
          });
        if (charA.level !== charB.level)
          fields.push({
            label: "Level",
            baseVal: charA.level,
            compareVal: charB.level,
            status: "changed",
          });
        if (charA.exp !== charB.exp)
          fields.push({
            label: "EXP",
            baseVal: charA.exp,
            compareVal: charB.exp,
            status: "changed",
          });
        for (let s = 0; s < Math.max(charA.stats.length, charB.stats.length); s++) {
          const sa = charA.stats[s],
            sb = charB.stats[s];
          if (sa && sb && (sa.current !== sb.current || sa.max !== sb.max)) {
            fields.push({
              label: sa.label,
              baseVal: `${sa.current}/${sa.max}`,
              compareVal: `${sb.current}/${sb.max}`,
              status: "changed",
            });
          }
        }
      }
      result.push({ name: charA?.name ?? charB?.name ?? `Character ${i}`, fields });
    }
    return result;
  }

  function computeVariableDiffs(a: Variable[] | null, b: Variable[] | null): DiffField[] {
    const mapA = new Map((a ?? []).map((v) => [v.id, v]));
    const mapB = new Map((b ?? []).map((v) => [v.id, v]));
    const allIds = new Set([...mapA.keys(), ...mapB.keys()]);
    const diffs: DiffField[] = [];
    for (const id of allIds) {
      const va = mapA.get(id),
        vb = mapB.get(id);
      if (!va && vb)
        diffs.push({
          label: vb.name || `#${id}`,
          baseVal: null,
          compareVal: vb.value,
          status: "added",
        });
      else if (va && !vb)
        diffs.push({
          label: va.name || `#${id}`,
          baseVal: va.value,
          compareVal: null,
          status: "removed",
        });
      else if (va && vb && va.value !== vb.value)
        diffs.push({
          label: va.name || `#${id}`,
          baseVal: va.value,
          compareVal: vb.value,
          status: "changed",
        });
      else if (va && vb)
        diffs.push({
          label: va.name || `#${id}`,
          baseVal: va.value,
          compareVal: vb.value,
          status: "unchanged",
        });
    }
    return diffs;
  }

  function computeSwitchDiffs(a: Switch[] | null, b: Switch[] | null): DiffField[] {
    const mapA = new Map((a ?? []).map((s) => [s.id, s]));
    const mapB = new Map((b ?? []).map((s) => [s.id, s]));
    const allIds = new Set([...mapA.keys(), ...mapB.keys()]);
    const diffs: DiffField[] = [];
    for (const id of allIds) {
      const sa = mapA.get(id),
        sb = mapB.get(id);
      if (!sa && sb)
        diffs.push({
          label: sb.name || `#${id}`,
          baseVal: null,
          compareVal: sb.value ? "ON" : "OFF",
          status: "added",
        });
      else if (sa && !sb)
        diffs.push({
          label: sa.name || `#${id}`,
          baseVal: sa.value ? "ON" : "OFF",
          compareVal: null,
          status: "removed",
        });
      else if (sa && sb && sa.value !== sb.value)
        diffs.push({
          label: sa.name || `#${id}`,
          baseVal: sa.value ? "ON" : "OFF",
          compareVal: sb.value ? "ON" : "OFF",
          status: "changed",
        });
      else if (sa && sb)
        diffs.push({
          label: sa.name || `#${id}`,
          baseVal: sa.value ? "ON" : "OFF",
          compareVal: sb.value ? "ON" : "OFF",
          status: "unchanged",
        });
    }
    return diffs;
  }

  /** Compute a git-diff style line-by-line diff of two JSON values. */
  function computeRawDiff(
    a: unknown,
    b: unknown,
  ): { type: "context" | "added" | "removed" | "hunk"; line: string }[] {
    const linesA = JSON.stringify(a, null, 2).split("\n");
    const linesB = JSON.stringify(b, null, 2).split("\n");

    // Simple LCS-based diff for reasonable-size outputs
    const MAX_LINES = 5000;
    if (linesA.length > MAX_LINES || linesB.length > MAX_LINES) {
      // Fallback to simple line-by-line for very large files
      return simpleLineDiff(linesA, linesB);
    }

    return patienceDiff(linesA, linesB);
  }

  function simpleLineDiff(
    a: string[],
    b: string[],
  ): { type: "context" | "added" | "removed"; line: string }[] {
    const result: { type: "context" | "added" | "removed"; line: string }[] = [];
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

  /** Patience diff: finds unique matching lines as anchors, then recursively diffs gaps.
   *  Produces cleaner diffs for structured data like JSON. */
  function patienceDiff(
    a: string[],
    b: string[],
  ): { type: "context" | "added" | "removed" | "hunk"; line: string }[] {
    const CONTEXT = 3;
    type Edit = { type: "context" | "added" | "removed"; line: string };

    function diff(
      a: string[],
      aStart: number,
      aEnd: number,
      b: string[],
      bStart: number,
      bEnd: number,
    ): Edit[] {
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
        // eslint-disable-next-line svelte/prefer-svelte-reactivity -- non-reactive local
        const uniqueA = new Map<string, number[]>();
        for (let i = aStart; i < aEnd; i++) {
          const arr = uniqueA.get(a[i]);
          if (arr) arr.push(i);
          else uniqueA.set(a[i], [i]);
        }
        // eslint-disable-next-line svelte/prefer-svelte-reactivity -- non-reactive local
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
          if (!bIndices || bIndices.length !== 1) continue;
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
      if (n * m > 2_000_000) {
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

    const edits = diff(a, 0, a.length, b, 0, b.length);
    return collapseContext(edits, CONTEXT);
  }

  /** Collapse unchanged context lines into hunks, keeping CONTEXT lines around changes. */
  function collapseContext(
    edits: { type: "context" | "added" | "removed"; line: string }[],
    ctx: number,
  ): { type: "context" | "added" | "removed" | "hunk"; line: string }[] {
    // Find indices of changed lines
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- non-reactive local
    const changed = new Set<number>();
    edits.forEach((e, i) => {
      if (e.type !== "context") changed.add(i);
    });

    if (changed.size === 0) return [{ type: "hunk", line: "No differences in raw data" }];

    // Mark context lines to keep (within ctx lines of a change)
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- non-reactive local
    const keep = new Set<number>();
    for (const idx of changed) {
      for (let k = Math.max(0, idx - ctx); k <= Math.min(edits.length - 1, idx + ctx); k++) {
        keep.add(k);
      }
      keep.add(idx);
    }

    const result: { type: "context" | "added" | "removed" | "hunk"; line: string }[] = [];
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

  function filteredDiffs(diffs: DiffField[]): DiffField[] {
    return showUnchanged ? diffs : diffs.filter((d) => d.status !== "unchanged");
  }
</script>

<div
  class="comparison-overlay"
  role="dialog"
  tabindex="-1"
  onclick={onclose}
  onkeydown={(e) => e.key === "Escape" && onclose()}
>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="comparison-modal"
    role="document"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="comparison-header">
      <h2>Save Comparison</h2>
      <div class="comparison-names">
        <span class="name-base">{baseName}</span>
        <span class="vs">vs</span>
        <span class="name-compare">{compareName}</span>
      </div>
      <label class="show-unchanged">
        <input type="checkbox" bind:checked={showUnchanged} /> Show unchanged
      </label>
      <button onclick={onclose} class="close-btn">✕</button>
    </div>

    <div class="comparison-tabs">
      {#each availableSections as section (section.id)}
        <button
          class="tab"
          class:active={activeSection === section.id}
          onclick={() => (activeSection = section.id)}
        >
          {section.label}
        </button>
      {/each}
    </div>

    <div class="comparison-body">
      {#if activeSection === "party"}
        {#each partyDiffs as charDiff (charDiff.name)}
          {#if charDiff.fields.length > 0 || showUnchanged}
            <div class="party-diff-card">
              <h3>{charDiff.name}</h3>
              {#each charDiff.fields as diff (diff.label)}
                <div class="diff-row {diff.status}">
                  <span class="diff-label">{diff.label}</span>
                  <span class="diff-base">{diff.baseVal ?? "—"}</span>
                  <span class="diff-compare">{diff.compareVal ?? "—"}</span>
                </div>
              {/each}
            </div>
          {/if}
        {/each}
      {:else if activeSection === "inventory"}
        {@const baseSections = ["items", "weapons", "armors"] as const}
        {#each baseSections as section (section)}
          {@const baseItems = base.inventory?.[section] ?? []}
          {@const compItems = compare.inventory?.[section] ?? []}
          {@const mapBase = new Map(baseItems.map((i) => [i.id, i]))}
          {@const mapComp = new Map(compItems.map((i) => [i.id, i]))}
          {@const allIds = [...new Set([...mapBase.keys(), ...mapComp.keys()])]}
          {@const diffs = allIds
            .map((id) => {
              const a = mapBase.get(id),
                b = mapComp.get(id);
              if (!a && b)
                return {
                  label: b.name || `#${id}`,
                  baseVal: null,
                  compareVal: b.quantity,
                  status: "added" as const,
                };
              if (a && !b)
                return {
                  label: a.name || `#${id}`,
                  baseVal: a.quantity,
                  compareVal: null,
                  status: "removed" as const,
                };
              if (a && b && a.quantity !== b.quantity)
                return {
                  label: a.name || `#${id}`,
                  baseVal: a.quantity,
                  compareVal: b.quantity,
                  status: "changed" as const,
                };
              return {
                label: a?.name || `#${id}`,
                baseVal: a?.quantity,
                compareVal: b?.quantity,
                status: "unchanged" as const,
              };
            })
            .filter((d) => showUnchanged || d.status !== "unchanged")}
          {#if diffs.length > 0}
            <h3 style="text-transform: capitalize; margin: 12px 0 6px;">{section}</h3>
            <div class="diff-table">
              <div class="diff-header">
                <span>Item</span><span>{baseName}</span><span>{compareName}</span>
              </div>
              {#each diffs as diff (diff.label)}
                <div class="diff-row {diff.status}">
                  <span class="diff-label">{diff.label}</span>
                  <span class="diff-base">{diff.baseVal ?? "—"}</span>
                  <span class="diff-compare">{diff.compareVal ?? "—"}</span>
                </div>
              {/each}
            </div>
          {/if}
        {/each}
      {:else if activeSection === "variables"}
        <div class="diff-table">
          <div class="diff-header">
            <span>Field</span><span>{baseName}</span><span>{compareName}</span>
          </div>
          {#each filteredDiffs(variableDiffs) as diff (diff.label)}
            <div class="diff-row {diff.status}">
              <span class="diff-label">{diff.label}</span>
              <span class="diff-base">{diff.baseVal ?? "—"}</span>
              <span class="diff-compare">{diff.compareVal ?? "—"}</span>
            </div>
          {/each}
        </div>
      {:else if activeSection === "switches"}
        <div class="diff-table">
          <div class="diff-header">
            <span>Switch</span><span>{baseName}</span><span>{compareName}</span>
          </div>
          {#each filteredDiffs(switchDiffs) as diff (diff.label)}
            <div class="diff-row {diff.status}">
              <span class="diff-label">{diff.label}</span>
              <span class="diff-base">{diff.baseVal ?? "—"}</span>
              <span class="diff-compare">{diff.compareVal ?? "—"}</span>
            </div>
          {/each}
        </div>
      {:else if activeSection === "raw"}
        <div class="raw-diff">
          {#each rawDiffLines as line, i (i)}
            {#if line.type === "hunk"}
              <div class="diff-line diff-hunk">{line.line}</div>
            {:else if line.type === "removed"}
              <div class="diff-line diff-removed">- {line.line}</div>
            {:else if line.type === "added"}
              <div class="diff-line diff-added">+ {line.line}</div>
            {:else}
              <div class="diff-line diff-context">{line.line}</div>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .comparison-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .comparison-modal {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    width: 90vw;
    max-width: 900px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-elevated);
  }
  .comparison-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    border-bottom: 1px solid var(--border);
  }
  .comparison-header h2 {
    font-size: 16px;
    font-weight: 400;
    letter-spacing: 0.5px;
  }
  .comparison-names {
    margin-left: auto;
    font-size: 13px;
  }
  .name-base {
    color: var(--text-secondary);
  }
  .vs {
    color: var(--text-muted);
    margin: 0 8px;
  }
  .name-compare {
    color: var(--accent-primary);
  }
  .show-unchanged {
    font-size: 12px;
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 18px;
    cursor: pointer;
    padding: 4px 8px;
  }
  .close-btn:hover {
    color: var(--text-primary);
  }
  .comparison-tabs {
    display: flex;
    gap: 2px;
    padding: 0 16px;
    border-bottom: 1px solid var(--border);
  }
  .tab {
    padding: 8px 16px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .tab:hover {
    color: var(--text-primary);
  }
  .tab.active {
    color: var(--accent-primary);
    border-bottom-color: var(--accent-primary);
  }
  .comparison-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }
  .diff-table {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .diff-header {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    padding: 8px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--text-muted);
  }
  .diff-row {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    font-size: 13px;
  }
  .diff-row.changed {
    background: rgba(245, 158, 11, 0.08);
    border-left: 3px solid var(--warning);
  }
  .diff-row.added {
    background: rgba(16, 185, 129, 0.08);
    border-left: 3px solid var(--success);
  }
  .diff-row.removed {
    background: rgba(239, 68, 68, 0.08);
    border-left: 3px solid var(--danger);
  }
  .diff-row.unchanged {
    color: var(--text-muted);
  }
  .diff-label {
    color: var(--text-secondary);
  }
  .diff-base {
    color: var(--text-muted);
  }
  .diff-compare {
    color: var(--text-primary);
  }
  .party-diff-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px;
    margin-bottom: 8px;
  }
  .party-diff-card h3 {
    font-size: 14px;
    font-weight: 400;
    margin-bottom: 8px;
  }

  /* Git-diff style raw view */
  .raw-diff {
    font-family: var(--font-mono);
    font-size: 12px;
    overflow: auto;
    max-height: 500px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-card);
  }
  .diff-line {
    padding: 1px 12px;
    white-space: pre;
    line-height: 1.5;
  }
  .diff-added {
    background: rgba(16, 185, 129, 0.12);
    color: var(--success);
  }
  .diff-removed {
    background: rgba(239, 68, 68, 0.12);
    color: var(--danger);
  }
  .diff-context {
    color: var(--text-muted);
  }
  .diff-hunk {
    background: rgba(108, 92, 231, 0.1);
    color: var(--accent-primary);
    font-style: italic;
    padding: 4px 12px;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }
</style>
