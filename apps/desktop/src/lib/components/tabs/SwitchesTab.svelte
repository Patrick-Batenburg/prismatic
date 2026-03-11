<script lang="ts">
  import type { Switch } from "$lib/api";
  import { markModified } from "$lib/stores";

  let { switches = $bindable() }: { switches: Switch[] } = $props();
  let search = $state("");
  let filterState = $state<"all" | "on" | "off">("all");

  let filtered = $derived(
    (() => {
      let list = switches;
      if (filterState === "on") list = list.filter((s) => s.value);
      if (filterState === "off") list = list.filter((s) => !s.value);
      if (search) {
        const q = search.toLowerCase();
        list = list.filter(
          (s) => s.id.toString().includes(q) || (s.name && s.name.toLowerCase().includes(q)),
        );
      }
      return list;
    })(),
  );

  function toggleSwitch(sw: Switch, idx: number) {
    sw.value = !sw.value;
    markModified(`switches.${idx}`);
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

<div class="sw-table">
  <div class="table-header">
    <span class="col-id">ID</span>
    <span class="col-name">Name</span>
    <span class="col-toggle">State</span>
  </div>
  {#each filtered as sw, idx (sw.id)}
    <div class="table-row">
      <span class="col-id">{sw.id}</span>
      <span class="col-name">{sw.name || `#${sw.id}`}</span>
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
    font-family: monospace;
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
</style>
