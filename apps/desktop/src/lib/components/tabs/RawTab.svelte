<script lang="ts">
  import JsonNode from '$lib/components/JsonNode.svelte';

  let { data }: { data: any } = $props();
  let search = $state('');
  let matchCount = $state(0);
  let expandAll = $state(false);
</script>

<div class="raw-controls">
  <input type="text" placeholder="Search keys or values..." bind:value={search} class="search-input" />
  {#if search}
    <span class="match-count">{matchCount} matches</span>
  {/if}
  <button onclick={() => expandAll = !expandAll}>
    {expandAll ? 'Collapse All' : 'Expand All'}
  </button>
</div>

<div class="json-tree">
  <JsonNode
    key="root"
    value={data}
    depth={0}
    {search}
    {expandAll}
    bind:matchCount
  />
</div>

<style>
  .raw-controls {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-bottom: 12px;
  }

  .search-input { width: 300px; }

  .match-count {
    font-size: 12px;
    color: var(--text-muted);
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
