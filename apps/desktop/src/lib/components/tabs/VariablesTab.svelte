<script lang="ts">
  import type { Variable } from '$lib/api';
  import { markModified } from '$lib/stores';

  let { variables = $bindable() }: { variables: Variable[] } = $props();
  let search = $state('');
  let filterGroup = $state<string | null>(null);
  let showUnnamed = $state(true);

  let groups = $derived((() => {
    const g = new Set<string>();
    variables.forEach(v => { if (v.group) g.add(v.group); });
    return Array.from(g).sort();
  })());

  let filtered = $derived((() => {
    let list = variables;
    if (!showUnnamed) list = list.filter(v => v.name);
    if (filterGroup) list = list.filter(v => v.group === filterGroup);
    if (search) {
      const q = search.toLowerCase();
      list = list.filter(v =>
        v.id.toString().includes(q) ||
        (v.name && v.name.toLowerCase().includes(q)) ||
        JSON.stringify(v.value).toLowerCase().includes(q)
      );
    }
    return list;
  })());

  function updateValue(variable: Variable, newVal: string, idx: number) {
    // Try to parse as number first
    const num = Number(newVal);
    if (!isNaN(num) && newVal.trim() !== '') {
      variable.value = num;
    } else if (newVal === 'true') {
      variable.value = true;
    } else if (newVal === 'false') {
      variable.value = false;
    } else {
      variable.value = newVal;
    }
    markModified(`variables.${idx}`);
  }
</script>

<div class="var-controls">
  <input type="text" placeholder="Search variables..." bind:value={search} class="search-input" />

  {#if groups.length > 0}
    <select bind:value={filterGroup}>
      <option value={null}>All groups</option>
      {#each groups as group}
        <option value={group}>{group}</option>
      {/each}
    </select>
  {/if}

  <label class="toggle-label">
    <input type="checkbox" bind:checked={showUnnamed} />
    Show unnamed
  </label>

  <span class="count">{filtered.length} / {variables.length}</span>
</div>

<div class="var-table">
  <div class="table-header">
    <span class="col-id">ID</span>
    <span class="col-name">Name</span>
    <span class="col-value">Value</span>
  </div>
  {#each filtered as variable, idx}
    <div class="table-row" class:has-name={!!variable.name}>
      <span class="col-id">{variable.id}</span>
      <span class="col-name" title={variable.name || `Variable #${variable.id}`}>
        {variable.name || `#${variable.id}`}
      </span>
      <input class="col-value" type="text"
        value={JSON.stringify(variable.value)}
        onchange={(e) => updateValue(variable, (e.target as HTMLInputElement).value, idx)} />
    </div>
  {/each}
</div>

<style>
  .var-controls {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }

  .search-input { width: 240px; }

  .toggle-label {
    font-size: 12px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
  }

  .count {
    font-size: 12px;
    color: var(--text-muted);
    margin-left: auto;
  }

  .var-table {
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
    letter-spacing: 0.5px;
  }

  .table-row {
    display: flex;
    padding: 5px 12px;
    border-top: 1px solid var(--border);
    align-items: center;
    font-size: 13px;
  }
  .table-row:hover { background: var(--bg-hover); }
  .table-row.has-name .col-name { color: var(--text-primary); }

  .col-id { width: 60px; color: var(--text-muted); font-family: monospace; font-size: 12px; }
  .col-name { flex: 1; color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .col-value { width: 200px; font-family: monospace; font-size: 12px; }
</style>
