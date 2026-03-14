<script lang="ts">
  import type { Switch } from "$lib/api";
  import {
    batchMode,
    batchSelected,
    toggleBatchItem,
    history,
    markModified,
    trackEdit,
  } from "$lib/stores";
  import type { Change } from "$lib/stores/history";
  import BatchToolbar from "$lib/components/BatchToolbar.svelte";

  let { switches = $bindable() }: { switches: Switch[] } = $props();
  let search = $state("");
  let filterState = $state<"all" | "on" | "off">("all");

  const filtered = $derived(
    (() => {
      let list = switches;
      if (filterState === "on") list = list.filter((s) => s.value);
      if (filterState === "off") list = list.filter((s) => !s.value);
      if (search) {
        const q = search.toLowerCase();
        list = list.filter(
          (s) => s.id.toString().includes(q) || s.name?.toLowerCase().includes(q) === true,
        );
      }
      return list;
    })(),
  );

  function toggleSwitch(sw: Switch, _idx: number) {
    const oldValue = sw.value;
    sw.value = !sw.value;
    // Find the actual array index for undo path resolution
    const arrayIdx = switches.indexOf(sw);
    trackEdit(
      ["switches", String(arrayIdx), "value"],
      oldValue,
      sw.value,
      `Toggle switch "${sw.name ?? sw.id}" ${sw.value ? "ON" : "OFF"}`,
    );
    markModified(`switches.${arrayIdx}`);
  }

  function handleBatchAction(action: string, selectedIds: Set<string>) {
    const changes: Change[] = [];

    for (let i = 0; i < switches.length; i++) {
      if (!selectedIds.has(String(switches[i].id))) continue;
      const oldVal = switches[i].value;
      let newVal: boolean;
      if (action === "turn_all_on") newVal = true;
      else if (action === "turn_all_off") newVal = false;
      else newVal = !oldVal;
      if (oldVal !== newVal) {
        changes.push({
          path: ["switches", String(i), "value"],
          oldValue: oldVal,
          newValue: newVal,
        });
        switches[i].value = newVal;
        markModified(`switches.${i}`);
      }
    }

    if (changes.length > 0) {
      history.push({ description: `Batch ${action} on ${selectedIds.size} switches`, changes });
      switches = switches.slice(); // trigger reactivity for batch mutations
    }
  }
</script>

<div class="sw-controls">
  <input type="text" placeholder="Search switches..." bind:value={search} class="search-input" />

  <div class="filter-tabs">
    <button class:active={filterState === "all"} onclick={() => (filterState = "all")}>All</button>
    <button class:active={filterState === "on"} onclick={() => (filterState = "on")}>ON</button>
    <button class:active={filterState === "off"} onclick={() => (filterState = "off")}>OFF</button>
  </div>

  <span class="count">{filtered.length} / {switches.length}</span>
</div>

<BatchToolbar
  items={filtered.map((s) => ({ id: String(s.id) }))}
  actions={[
    { label: "Turn all ON", value: "turn_all_on" },
    { label: "Turn all OFF", value: "turn_all_off" },
    { label: "Toggle selected", value: "toggle_selected" },
  ]}
  onapply={handleBatchAction}
/>

<div class="sw-table">
  <div class="table-header">
    {#if $batchMode}
      <span class="col-check"></span>
    {/if}
    <span class="col-id">ID</span>
    <span class="col-name">Name</span>
    <span class="col-toggle">State</span>
  </div>
  {#each filtered as sw, idx (sw.id)}
    <div class="table-row">
      {#if $batchMode}
        <input
          type="checkbox"
          class="batch-check"
          checked={$batchSelected.has(String(sw.id))}
          onchange={() => toggleBatchItem(String(sw.id))}
        />
      {/if}
      <span class="col-id">{sw.id}</span>
      <span class="col-name">{sw.name ?? `#${sw.id}`}</span>
      <button
        class="col-toggle toggle-btn"
        class:on={sw.value}
        onclick={() => toggleSwitch(sw, idx)}
      >
        {sw.value ? "ON" : "OFF"}
      </button>
    </div>
  {/each}
</div>

<style>
  .sw-controls {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-bottom: 12px;
  }

  .search-input {
    width: 240px;
  }

  .filter-tabs {
    display: flex;
    gap: 2px;
  }
  .filter-tabs button {
    padding: 4px 10px;
    font-size: 12px;
    border-radius: var(--radius-sm);
  }
  .filter-tabs button.active {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
    color: white;
  }

  .count {
    font-size: 12px;
    color: var(--text-muted);
    margin-left: auto;
  }

  .sw-table {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .table-header {
    display: flex;
    padding: 8px 12px;
    background: var(--bg-tertiary);
    font-size: 11px;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .table-row {
    display: flex;
    padding: 5px 12px;
    border-top: 1px solid var(--border);
    align-items: center;
    font-size: 13px;
  }
  .table-row:hover {
    background: var(--bg-hover);
  }

  .col-id {
    width: 60px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .col-name {
    flex: 1;
  }
  .col-toggle {
    width: 60px;
    text-align: center;
  }

  .toggle-btn {
    padding: 2px 10px;
    font-size: 11px;
    font-weight: 600;
    border-radius: 10px;
    background: rgba(239, 68, 68, 0.15);
    border-color: var(--danger);
    color: var(--danger);
  }
  .toggle-btn.on {
    background: rgba(16, 185, 129, 0.15);
    border-color: var(--success);
    color: var(--success);
  }

  .batch-check {
    width: 16px;
    height: 16px;
    cursor: pointer;
    margin-right: 8px;
  }
  .col-check {
    width: 24px;
  }
</style>
