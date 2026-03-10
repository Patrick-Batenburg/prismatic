<script lang="ts">
  import JsonNode from './JsonNode.svelte';

  let {
    key,
    value,
    depth = 0,
    search = '',
    expandAll = false,
    matchCount = $bindable(0),
    filterFn = undefined as ((key: string, value: any) => boolean) | undefined
  }: {
    key: string;
    value: any;
    depth?: number;
    search?: string;
    expandAll?: boolean;
    matchCount?: number;
    filterFn?: ((key: string, value: any) => boolean) | undefined;
  } = $props();

  // Capture initial depth (not reactive — intentional, depth is fixed per instance)
  // svelte-ignore state_referenced_locally
  const initialDepth = depth;
  let expanded = $state(initialDepth < 1);
  let editing = $state(false);
  let editValue = $state('');

  // React to expandAll changes
  $effect(() => {
    if (expandAll) expanded = true;
    if (!expandAll && initialDepth > 0) expanded = false;
  });

  let isObject = $derived(value !== null && typeof value === 'object' && !Array.isArray(value));
  let isArray = $derived(Array.isArray(value));
  let isExpandable = $derived(isObject || isArray);

  let childEntries = $derived.by(() => {
    let entries: [string, any][] = isObject
      ? Object.entries(value)
      : isArray
        ? value.map((v: any, i: number) => [String(i), v])
        : [];
    if (filterFn) {
      entries = entries.filter(([k, v]) => filterFn!(k, v));
    }
    return entries;
  });

  let childCount = $derived(childEntries.length);

  // Search match — only check this node's key/value, not descendants
  let isMatch = $derived((() => {
    if (!search) return false;
    const q = search.toLowerCase();
    return key.toLowerCase().includes(q) ||
      (!isExpandable && String(value).toLowerCase().includes(q));
  })());

  // When searching, auto-expand nodes that contain matches
  let hasDescendantMatch = $derived.by(() => {
    if (!search) return false;
    if (!isExpandable) return false;
    // Quick string check on JSON.stringify is much faster than recursive component rendering
    const str = JSON.stringify(value).toLowerCase();
    return str.includes(search.toLowerCase());
  });

  // Search-driven visibility: if searching, only render children that might match
  let visibleChildren = $derived.by(() => {
    if (!search) return childEntries;
    const q = search.toLowerCase();
    return childEntries.filter(([k, v]) => {
      // Always show if key matches
      if (k.toLowerCase().includes(q)) return true;
      // For primitives, check value
      if (v === null || typeof v !== 'object') {
        return String(v).toLowerCase().includes(q);
      }
      // For objects/arrays, do a quick string check
      return JSON.stringify(v).toLowerCase().includes(q);
    });
  });

  // Auto-expand when searching and this node has matches
  $effect(() => {
    if (search && hasDescendantMatch) {
      expanded = true;
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
      <button class="value" style="color: {valueColor(value)}" ondblclick={startEdit}
        title="Double-click to edit">
        {formatValue(value)}
      </button>
    {/if}
  {/if}
</div>

{#if isExpandable && expanded}
  {#each visibleChildren as [childKey, childVal] (childKey)}
    <JsonNode
      key={childKey}
      value={childVal}
      depth={depth + 1}
      {search}
      {expandAll}
      {filterFn}
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
    background: none;
    border: none;
    padding: 0;
    font-family: inherit;
    font-size: inherit;
    text-align: left;
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
