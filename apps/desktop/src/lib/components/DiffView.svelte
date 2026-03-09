<script lang="ts">
  import type { DiffEntry } from '$lib/api';

  let { entries, onclose }: { entries: DiffEntry[]; onclose: () => void } = $props();
  let search = $state('');

  let filtered = $derived((() => {
    if (!search) return entries;
    const q = search.toLowerCase();
    return entries.filter(e =>
      e.path.toLowerCase().includes(q) ||
      JSON.stringify(e.old_value).toLowerCase().includes(q) ||
      JSON.stringify(e.new_value).toLowerCase().includes(q)
    );
  })());
</script>

<div class="modal-overlay" onclick={onclose}>
  <div class="diff-modal" onclick={(e) => e.stopPropagation()}>
    <div class="diff-header">
      <h3>Changes Detected ({entries.length})</h3>
      <input type="text" placeholder="Filter changes..." bind:value={search} class="search-input" />
      <button onclick={onclose}>✕</button>
    </div>

    <div class="diff-list">
      {#each filtered as entry}
        <div class="diff-entry"
          class:added={entry.old_value === null}
          class:removed={entry.new_value === null}
          class:changed={entry.old_value !== null && entry.new_value !== null}
        >
          <div class="diff-path">{entry.path}</div>
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

  .diff-header h3 { flex: 1; font-size: 16px; }
  .search-input { width: 200px; }

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

  .diff-entry.added { border-left-color: var(--success); }
  .diff-entry.removed { border-left-color: var(--danger); }
  .diff-entry.changed { border-left-color: var(--warning); }

  .diff-path {
    font-family: monospace;
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .diff-values {
    display: flex;
    align-items: center;
    gap: 8px;
    font-family: monospace;
    font-size: 13px;
  }

  .old-val { color: var(--danger); }
  .new-val { color: var(--success); }
  .arrow { color: var(--text-muted); }

  .no-changes {
    padding: 20px;
    text-align: center;
    color: var(--text-muted);
  }
</style>
