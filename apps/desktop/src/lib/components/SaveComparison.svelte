<script lang="ts">
  import type { SaveData, Character, Variable, Switch } from "$lib/api";
  import { getPreferences } from "$lib/preferences";
  import { patienceDiff, simpleLineDiff, type DiffLine } from "$lib/diff-utils";

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

  const { base, compare, baseName, compareName, onclose }: Props = $props();
  let showUnchanged = $state(getPreferences().showUnchangedDefault);

  // Only show tabs that have data in at least one save
  const availableSections = $derived(
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

  const partyDiffs = $derived(computePartyDiffs(base.party, compare.party));
  const variableDiffs = $derived(computeVariableDiffs(base.variables, compare.variables));
  const switchDiffs = $derived(computeSwitchDiffs(base.switches, compare.switches));
  const rawDiffLines = $derived(computeRawDiff(base.raw, compare.raw));

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
          if (s >= charA.stats.length || s >= charB.stats.length) continue;
          const sa = charA.stats[s],
            sb = charB.stats[s];
          if (sa.current !== sb.current || sa.max !== sb.max) {
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
          label: vb.name ?? `#${id}`,
          baseVal: null,
          compareVal: vb.value,
          status: "added",
        });
      else if (va && !vb)
        diffs.push({
          label: va.name ?? `#${id}`,
          baseVal: va.value,
          compareVal: null,
          status: "removed",
        });
      else if (va && vb && va.value !== vb.value)
        diffs.push({
          label: va.name ?? `#${id}`,
          baseVal: va.value,
          compareVal: vb.value,
          status: "changed",
        });
      else if (va && vb)
        diffs.push({
          label: va.name ?? `#${id}`,
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
          label: sb.name ?? `#${id}`,
          baseVal: null,
          compareVal: sb.value ? "ON" : "OFF",
          status: "added",
        });
      else if (sa && !sb)
        diffs.push({
          label: sa.name ?? `#${id}`,
          baseVal: sa.value ? "ON" : "OFF",
          compareVal: null,
          status: "removed",
        });
      else if (sa && sb && sa.value !== sb.value)
        diffs.push({
          label: sa.name ?? `#${id}`,
          baseVal: sa.value ? "ON" : "OFF",
          compareVal: sb.value ? "ON" : "OFF",
          status: "changed",
        });
      else if (sa && sb)
        diffs.push({
          label: sa.name ?? `#${id}`,
          baseVal: sa.value ? "ON" : "OFF",
          compareVal: sb.value ? "ON" : "OFF",
          status: "unchanged",
        });
    }
    return diffs;
  }

  /** Compute a git-diff style line-by-line diff of two JSON values. */
  function computeRawDiff(a: unknown, b: unknown): DiffLine[] {
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

  function filteredDiffs(diffs: DiffField[]): DiffField[] {
    return showUnchanged ? diffs : diffs.filter((d) => d.status !== "unchanged");
  }
</script>

<div
  class="modal-overlay"
  role="button"
  aria-label="Close dialog"
  tabindex="-1"
  onclick={onclose}
  onkeydown={(e) => e.key === "Escape" && onclose()}
>
  <div
    class="comparison-modal"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label="Save comparison"
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
                label: a?.name ?? `#${id}`,
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
