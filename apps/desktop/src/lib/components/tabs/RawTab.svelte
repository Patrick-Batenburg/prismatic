<script lang="ts">
  import JsonNode from '$lib/components/JsonNode.svelte';

  let { data, engineId = '' }: { data: any; engineId?: string } = $props();
  let searchInput = $state('');
  let search = $state('');
  let matchCount = $state(0);
  let expandAll = $state(false);
  let copyLabel = $state('Copy raw');

  // Ren'Py-specific filters
  let isRenpy = $derived(engineId === 'renpy');
  let hideRenpyDialogs = $state(true);
  let hideRenpyVars = $state(true);

  // Debounce search input — prevents freezing on large JSON trees
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;
  function onSearchInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    searchInput = val;
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      matchCount = 0;
      search = val;
    }, 400);
  }

  // "Hide Dialogs": any store.* entry that is a complex object with a __class__
  // These are game event trees, quest nodes, gallery entries, dialog objects, etc.
  function isRenpyDialog(_key: string, value: any): boolean {
    if (typeof value !== 'object' || value === null || !('__class__' in value)) return false;
    const cls: string = value.__class__;
    // Any store.* class that's an event/dialog/quest/gallery/root object
    if (cls.startsWith('store.')) return true;
    // renpy display/layout objects
    if (cls.startsWith('renpy.display.') || cls.startsWith('renpy.execution.')) return true;
    return false;
  }

  // "Hide Ren'Py variables": internal engine state keys
  function isRenpyInternal(key: string, value: any): boolean {
    // Internal renpy state: keys starting with _ (except _save_data, _metadata)
    if (key.startsWith('_') && !key.startsWith('_save') && !key.startsWith('_metadata')) return true;
    // store._ prefixed keys (internal renpy store vars)
    if (key.startsWith('store._')) return true;
    // renpy.* keyed entries
    if (key.startsWith('renpy.')) return true;
    // __class__, __version__ and other dunder keys inside objects
    if (key.startsWith('__') && key.endsWith('__')) return true;
    // The rollback log object (item [1] in _save_data array)
    if (typeof value === 'object' && value !== null && '__class__' in value) {
      const cls: string = value.__class__;
      if (cls.startsWith('renpy.')) return true;
    }
    return false;
  }

  let filterFn = $derived.by(() => {
    if (!isRenpy) return undefined;
    if (!hideRenpyDialogs && !hideRenpyVars) return undefined;
    return (key: string, value: any): boolean => {
      if (hideRenpyDialogs && isRenpyDialog(key, value)) return false;
      if (hideRenpyVars && isRenpyInternal(key, value)) return false;
      return true;
    };
  });

  async function copyToClipboard() {
    try {
      const json = JSON.stringify(data, null, 2);
      await navigator.clipboard.writeText(json);
      copyLabel = 'Copied!';
      setTimeout(() => copyLabel = 'Copy raw', 2000);
    } catch {
      copyLabel = 'Failed';
      setTimeout(() => copyLabel = 'Copy raw', 2000);
    }
  }
</script>

<div class="raw-controls">
  <input type="text" placeholder="Search keys or values..." value={searchInput} oninput={onSearchInput} class="search-input" />
  {#if search}
    <span class="match-count">{matchCount} matches</span>
  {/if}
  <button onclick={() => expandAll = !expandAll}>
    {expandAll ? 'Collapse All' : 'Expand All'}
  </button>
  <button onclick={copyToClipboard} class="copy-btn">
    {copyLabel}
  </button>
</div>

{#if isRenpy}
  <div class="filter-controls">
    <label class="filter-toggle">
      <input type="checkbox" bind:checked={hideRenpyDialogs} />
      Hide Ren'Py Dialogs
    </label>
    <label class="filter-toggle">
      <input type="checkbox" bind:checked={hideRenpyVars} />
      Hide Ren'Py variables
    </label>
  </div>
{/if}

<div class="json-tree">
  <JsonNode
    key="root"
    value={data}
    depth={0}
    {search}
    {expandAll}
    {filterFn}
    bind:matchCount
  />
</div>

<style>
  .raw-controls {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-bottom: 8px;
  }

  .search-input { width: 300px; }

  .match-count {
    font-size: 12px;
    color: var(--text-muted);
  }

  .copy-btn {
    margin-left: auto;
  }

  .filter-controls {
    display: flex;
    gap: 16px;
    margin-bottom: 10px;
  }

  .filter-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text-secondary);
    cursor: pointer;
    user-select: none;
  }

  .filter-toggle input[type="checkbox"] {
    accent-color: var(--accent);
    cursor: pointer;
  }

  .json-tree {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px;
    overflow: auto;
    max-height: calc(100vh - 200px);
    font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
    font-size: 13px;
    line-height: 1.6;
  }
</style>
