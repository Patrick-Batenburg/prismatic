<script lang="ts">
  import type { Variable } from "$lib/api";
  import { batchMode, batchSelected, toggleBatchItem, history, markModified, trackEdit } from '$lib/stores';
  import type { Change } from '$lib/stores/history';
  import BatchToolbar from '$lib/components/BatchToolbar.svelte';

  let { variables = $bindable() }: { variables: Variable[] } = $props();
  let search = $state("");
  let filterGroup = $state<string | null>(null);
  let showUnnamed = $state(true);

  let groups = $derived(
    (() => {
      const g: string[] = [];
      variables.forEach((v) => {
        if (v.group && !g.includes(v.group)) {
          g.push(v.group);
        }
      });
      return g.sort();
    })(),
  );

  let filtered = $derived(
    (() => {
      let list = variables;
      if (!showUnnamed) list = list.filter((v) => v.name);
      if (filterGroup) list = list.filter((v) => v.group === filterGroup);
      if (search) {
        const q = search.toLowerCase();
        list = list.filter(
          (v) =>
            v.id.toString().includes(q) ||
            (v.name && v.name.toLowerCase().includes(q)) ||
            JSON.stringify(v.value).toLowerCase().includes(q),
        );
      }
      return list;
    })(),
  );

  function updateValue(variable: Variable, newVal: string, idx: number) {
    const oldValue = variable.value;
    // Try to parse as number first
    const num = Number(newVal);
    let newValue: string | number | boolean = newVal;
    if (!isNaN(num) && newVal.trim() !== "") {
      newValue = num;
    } else if (newVal === "true") {
      newValue = true;
    } else if (newVal === "false") {
      newValue = false;
    }
    variable.value = newValue;
    // Find the actual array index for undo path resolution
    const arrayIdx = variables.indexOf(variable);
    trackEdit(
      ['variables', String(arrayIdx), 'value'],
      oldValue, newValue,
      `Set variable "${variable.name || variable.id}" to ${newValue}`
    );
    markModified(`variables.${arrayIdx}`);
  }

  function handleBatchAction(action: string, selectedIds: Set<string>, value?: string) {
    const changes: Change[] = [];
    const targetValue = action === 'reset_to_zero' ? 0 : (value ? (isNaN(Number(value)) ? value : Number(value)) : 0);

    for (let i = 0; i < variables.length; i++) {
      if (!selectedIds.has(String(variables[i].id))) continue;
      changes.push({ path: ['variables', String(i), 'value'], oldValue: variables[i].value, newValue: targetValue });
      variables[i].value = targetValue;
      markModified(`variables.${i}`);
    }

    if (changes.length > 0) {
      history.push({ description: `Batch ${action} on ${selectedIds.size} variables`, changes });
    }
  }

  function handleBatchAction(action: string, selectedIds: Set<string>, value?: string) {
    const changes: Change[] = [];
    const targetValue = action === 'reset_to_zero' ? 0 : (value ? (isNaN(Number(value)) ? value : Number(value)) : 0);

    for (let i = 0; i < variables.length; i++) {
      if (!selectedIds.has(String(variables[i].id))) continue;
      changes.push({ path: ['variables', String(variables[i].id), 'value'], oldValue: variables[i].value, newValue: targetValue });
      variables[i].value = targetValue;
      markModified(`variables.${i}`);
    }

    if (changes.length > 0) {
      history.push({ description: `Batch ${action} on ${selectedIds.size} variables`, changes });
    }
  }
</script>

<div class="var-controls">
  <input type="text" placeholder="Search variables..." bind:value={search} class="search-input" />

  {#if groups.length > 0}
    <select bind:value={filterGroup}>
      <option value={null}>All groups</option>
      {#each groups as group (group)}
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

<BatchToolbar
  items={filtered.map((v) => ({ id: String(v.id) }))}
  actions={[
    { label: 'Set value...', value: 'set_value' },
    { label: 'Reset to 0', value: 'reset_to_zero' },
  ]}
  onapply={handleBatchAction}
/>

<div class="var-table">
  <div class="table-header">
    {#if $batchMode}
      <span class="col-check"></span>
    {/if}
    <span class="col-id">ID</span>
    <span class="col-name">Name</span>
    <span class="col-value">Value</span>
  </div>
  {#each filtered as variable, idx (variable.id)}
    <div class="table-row" class:has-name={!!variable.name}>
      {#if $batchMode}
        <input
          type="checkbox"
          class="batch-check"
          checked={$batchSelected.has(String(variable.id))}
          onchange={() => toggleBatchItem(String(variable.id))}
        />
      {/if}
      <span class="col-id">{variable.id}</span>
      <span class="col-name" title={variable.name || `Variable #${variable.id}`}>
        {variable.name || `#${variable.id}`}
      </span>
      <input
        class="col-value"
        type="text"
        value={JSON.stringify(variable.value)}
        onchange={(e) => updateValue(variable, (e.target as HTMLInputElement).value, idx)}
      />
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

  .search-input {
    width: 240px;
  }

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
  .table-row:hover {
    background: var(--bg-hover);
  }
  .table-row.has-name .col-name {
    color: var(--text-primary);
  }

  .col-id {
    width: 60px;
    color: var(--text-muted);
    font-family: monospace;
    font-size: 12px;
  }
  .col-name {
    flex: 1;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .col-value {
    width: 200px;
    font-family: monospace;
    font-size: 12px;
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
