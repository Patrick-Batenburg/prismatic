<script lang="ts">
  let {
    key,
    value,
    depth = 0,
    search = '',
    expandAll = false,
    matchCount = $bindable(0)
  }: {
    key: string;
    value: any;
    depth?: number;
    search?: string;
    expandAll?: boolean;
    matchCount?: number;
  } = $props();

  let expanded = $state(depth < 1);
  let editing = $state(false);
  let editValue = $state('');

  // React to expandAll changes
  $effect(() => {
    if (expandAll) expanded = true;
    if (!expandAll && depth > 0) expanded = false;
  });

  let isObject = $derived(value !== null && typeof value === 'object' && !Array.isArray(value));
  let isArray = $derived(Array.isArray(value));
  let isExpandable = $derived(isObject || isArray);
  let childEntries = $derived(isObject ? Object.entries(value) : isArray ? value.map((v: any, i: number) => [String(i), v]) : []);
  let childCount = $derived(childEntries.length);

  let isMatch = $derived((() => {
    if (!search) return false;
    const q = search.toLowerCase();
    return key.toLowerCase().includes(q) ||
      (!isExpandable && String(value).toLowerCase().includes(q));
  })());

  // Count matches (approximate — each leaf node counts itself)
  $effect(() => {
    if (isMatch) {
      matchCount++;
    }
  });

  function valueColor(val: any): string {
    if (val === null) return 'var(--text-muted)';
    if (typeof val === 'string') return '#ce93d8';
    if (typeof val === 'number') return '#4fc3f7';
    if (typeof val === 'boolean') return val ? 'var(--success)' : 'var(--danger)';
    return 'var(--text-primary)';
  }

  function formatValue(val: any): string {
    if (val === null) return 'null';
    if (typeof val === 'string') return `"${val}"`;
    return String(val);
  }

  function startEdit() {
    editValue = typeof value === 'string' ? value : JSON.stringify(value);
    editing = true;
  }

  function commitEdit() {
    editing = false;
    try {
      value = JSON.parse(editValue);
    } catch {
      value = editValue;
    }
  }
</script>

<div class="node" class:match={isMatch} style="padding-left: {depth * 16}px">
  {#if isExpandable}
    <button class="toggle" onclick={() => expanded = !expanded}>
      {expanded ? '▼' : '▶'}
    </button>
    <span class="key">{key}</span>
    <span class="bracket">{isArray ? '[' : '{'}</span>
    {#if !expanded}
      <span class="collapsed-hint">{childCount} {isArray ? 'items' : 'keys'}</span>
      <span class="bracket">{isArray ? ']' : '}'}</span>
    {/if}
  {:else}
    <span class="spacer"></span>
    <span class="key">{key}:</span>
    {#if editing}
      <input class="edit-input" bind:value={editValue}
        onblur={commitEdit}
        onkeydown={(e) => { if (e.key === 'Enter') commitEdit(); if (e.key === 'Escape') editing = false; }}
      />
    {:else}
      <span class="value" style="color: {valueColor(value)}" ondblclick={startEdit}
        title="Double-click to edit">
        {formatValue(value)}
      </span>
    {/if}
  {/if}
</div>

{#if isExpandable && expanded}
  {#each childEntries as [childKey, childVal]}
    <svelte:self
      key={childKey}
      value={childVal}
      depth={depth + 1}
      {search}
      {expandAll}
      bind:matchCount
    />
  {/each}
  <div class="node" style="padding-left: {depth * 16}px">
    <span class="spacer"></span>
    <span class="bracket">{isArray ? ']' : '}'}</span>
  </div>
{/if}

<style>
  .node {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 24px;
    white-space: nowrap;
  }

  .node.match {
    background: rgba(245, 158, 11, 0.15);
    border-radius: 2px;
  }

  .toggle {
    width: 16px;
    height: 16px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 10px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .toggle:hover { color: var(--text-primary); }

  .spacer { width: 16px; flex-shrink: 0; }

  .key {
    color: #81d4fa;
    font-weight: 500;
  }

  .value {
    cursor: pointer;
  }
  .value:hover {
    text-decoration: underline dotted;
  }

  .bracket { color: var(--text-muted); }

  .collapsed-hint {
    color: var(--text-muted);
    font-style: italic;
    font-size: 12px;
  }

  .edit-input {
    font-family: inherit;
    font-size: inherit;
    padding: 1px 4px;
    min-width: 100px;
  }
</style>
