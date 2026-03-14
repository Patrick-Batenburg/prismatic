<script lang="ts">
  import type { DiffEntry, NameMap } from "$lib/api";

  let {
    entries,
    onclose,
    nameMap = null,
  }: { entries: DiffEntry[]; onclose: () => void; nameMap?: NameMap | null } = $props();
  let search = $state("");

  /** Convert raw marshal-rs JSON paths to human-readable form.
   *  e.g. __value.{"__id":662,"__value":"actors","__type":7}.__value.@data.__value[1].__value.@tp
   *    → Actor "Eric" (#1) > tp
   */
  function humanizePath(path: string): string {
    let p = path
      // Extract symbol names from marshal-rs JSON key objects
      .replace(/\{"__id":\d+,"__value":"([^"]+)","__type":\d+\}/g, "$1")
      // Remove .__value wrapper segments (before ., [, or end of string)
      .replace(/\.__value(?=\.|$|\[)/g, "")
      // Remove leading __value.
      .replace(/^__value\./, "")
      // Strip @ prefix from Ruby instance variables
      .replace(/@(\w)/g, "$1")
      // Clean up .[ → [
      .replace(/\.\[/g, "[")
      // Clean up leading/trailing dots
      .replace(/^\.+|\.+$/g, "")
      // Clean up double dots
      .replace(/\.{2,}/g, ".");

    if (nameMap) {
      p = enrichWithNames(p, nameMap);
    }

    return p;
  }

  /** RPG Maker VX Ace param_plus indices → human-readable stat names. */
  const PARAM_NAMES: Record<number, string> = {
    0: "Max HP",
    1: "Max MP",
    2: "ATK",
    3: "DEF",
    4: "MAT",
    5: "MDF",
    6: "AGI",
    7: "LUK",
  };

  /** Map internal field names to readable labels. */
  function humanizeField(field: string): string {
    return (
      field
        // param_plus[N] → "ATK bonus" etc.
        .replace(/param_plus\[(\d+)\]/, (_, i) => {
          const name = PARAM_NAMES[Number(i)];
          return name ? `${name} bonus` : `param_plus[${i}]`;
        })
        // Common RPG Maker stat fields
        .replace(/^hp$/, "HP")
        .replace(/^mp$/, "MP")
        .replace(/^tp$/, "TP")
        .replace(/^level$/, "Level")
        .replace(/^exp$/, "EXP")
        .replace(/^gold$/, "Gold")
        .replace(/^name$/, "Name")
    );
  }

  /** Look up known entity names and annotate path segments. */
  function enrichWithNames(path: string, nm: NameMap): string {
    const lookups: Record<string, Record<number, string> | undefined> = {
      "actors.data": nm.actors,
      "variables.data": nm.variables,
      "switches.data": nm.switches,
    };

    for (const [prefix, map] of Object.entries(lookups)) {
      if (!map) continue;
      const re = new RegExp(`^${prefix.replace(".", "\\.")}\\[(\\d+)\\](.*)`);
      const m = path.match(re);
      if (m) {
        const id = Number(m[1]);
        const rest = m[2];
        const category = prefix.split(".")[0];
        // Capitalize: actors → Actor, variables → Variable, switches → Switch
        const label = category.charAt(0).toUpperCase() + category.slice(1).replace(/s$/, "");
        const name = map[id];
        const nameStr = name ? ` "${name}"` : "";
        const suffix = rest ? ` > ${humanizeField(rest.replace(/^\./, ""))}` : "";
        return `${label}${nameStr} (#${id})${suffix}`;
      }
    }

    // Inventory items in party hash: party.items.{symbol_id}, party.weapons.{...}, party.armors.{...}
    const invMatch = path.match(/^party\.(items|weapons|armors)\.(\d+)(.*)/);
    if (invMatch) {
      const [, section, idStr, rest] = invMatch;
      const id = Number(idStr);
      const sectionMap: Record<string, Record<number, string> | undefined> = {
        items: nm.items,
        weapons: nm.weapons,
        armors: nm.armors,
      };
      const map = sectionMap[section];
      const name = map?.[id];
      const label = section.charAt(0).toUpperCase() + section.slice(1).replace(/s$/, "");
      const nameStr = name ? ` "${name}"` : "";
      const suffix = rest ? ` > ${humanizeField(rest.replace(/^\./, ""))}` : "";
      return `${label}${nameStr} (#${id})${suffix}`;
    }

    return path;
  }

  let filtered = $derived(
    (() => {
      if (!search) return entries;
      const q = search.toLowerCase();
      return entries.filter(
        (e) =>
          humanizePath(e.path).toLowerCase().includes(q) ||
          e.path.toLowerCase().includes(q) ||
          JSON.stringify(e.old_value).toLowerCase().includes(q) ||
          JSON.stringify(e.new_value).toLowerCase().includes(q),
      );
    })(),
  );
</script>

<div
  class="modal-overlay"
  role="button"
  tabindex="-1"
  onclick={onclose}
  onkeydown={(e) => {
    if (e.key === "Escape") onclose();
  }}
>
  <div
    class="diff-modal"
    role="dialog"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="diff-header">
      <h3>Changes Detected ({entries.length})</h3>
      <input type="text" placeholder="Filter changes..." bind:value={search} class="search-input" />
      <button onclick={onclose}>✕</button>
    </div>

    <div class="diff-list">
      {#each filtered as entry (entry.path)}
        <div
          class="diff-entry"
          class:added={entry.old_value === null}
          class:removed={entry.new_value === null}
          class:changed={entry.old_value !== null && entry.new_value !== null}
        >
          <div class="diff-path" title={entry.path}>{humanizePath(entry.path)}</div>
          <div class="diff-values">
            {#if entry.old_value !== null}
              <span class="old-val">{JSON.stringify(entry.old_value)}</span>
            {/if}
            {#if entry.old_value !== null && entry.new_value !== null}
              <span class="arrow">→</span>
            {/if}
            {#if entry.new_value !== null}
              <span class="new-val">{JSON.stringify(entry.new_value)}</span>
            {/if}
          </div>
        </div>
      {/each}
      {#if filtered.length === 0}
        <div class="no-changes">No matching changes</div>
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .diff-modal {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    width: 700px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
  }

  .diff-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .diff-header h3 {
    flex: 1;
    font-size: 16px;
  }
  .search-input {
    width: 200px;
  }

  .diff-list {
    overflow-y: auto;
    padding: 12px;
  }

  .diff-entry {
    padding: 8px 12px;
    margin-bottom: 6px;
    border-radius: var(--radius);
    border-left: 3px solid var(--border);
    background: var(--bg-tertiary);
  }

  .diff-entry.added {
    border-left-color: var(--success);
  }
  .diff-entry.removed {
    border-left-color: var(--danger);
  }
  .diff-entry.changed {
    border-left-color: var(--warning);
  }

  .diff-path {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .diff-values {
    display: flex;
    align-items: center;
    gap: 8px;
    font-family: var(--font-mono);
    font-size: 13px;
  }

  .old-val {
    color: var(--danger);
  }
  .new-val {
    color: var(--success);
  }
  .arrow {
    color: var(--text-muted);
  }

  .no-changes {
    padding: 20px;
    text-align: center;
    color: var(--text-muted);
  }
</style>
