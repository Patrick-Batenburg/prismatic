<script lang="ts">
  import { batchMode, batchSelected, clearBatchSelection } from '$lib/stores';

  interface Props {
    items: { id: string }[];
    actions: { label: string; value: string }[];
    onapply: (action: string, selectedIds: Set<string>, value?: string) => void;
  }

  let { items, actions, onapply }: Props = $props();
  let selectedAction = $state('');

  function selectAll() {
    batchSelected.set(new Set(items.map((i) => i.id)));
  }

  function selectNone() {
    clearBatchSelection();
  }

  let promptValue = $state('');
  let showPrompt = $state(false);

  function apply() {
    if (!selectedAction || $batchSelected.size === 0) return;
    const action = actions.find(a => a.value === selectedAction);
    if (action?.label.endsWith('...') && !showPrompt) {
      showPrompt = true;
      return;
    }
    onapply(selectedAction, $batchSelected, promptValue || undefined);
    selectedAction = '';
    promptValue = '';
    showPrompt = false;
  }

  function cancelPrompt() {
    showPrompt = false;
    promptValue = '';
  }
</script>

{#if $batchMode}
  <div class="batch-toolbar">
    <span class="batch-count">{$batchSelected.size} of {items.length} selected</span>
    <button onclick={selectAll} class="batch-btn">Select All</button>
    <button onclick={selectNone} class="batch-btn">Select None</button>
    <div class="batch-action">
      <select bind:value={selectedAction}>
        <option value="">Choose action...</option>
        {#each actions as action}
          <option value={action.value}>{action.label}</option>
        {/each}
      </select>
      <button onclick={apply} disabled={!selectedAction || $batchSelected.size === 0} class="btn-primary">
        Apply
      </button>
    </div>
    {#if showPrompt}
      <div class="batch-prompt">
        <input
          type="text"
          bind:value={promptValue}
          placeholder="Enter value..."
          onkeydown={(e) => { if (e.key === 'Enter') apply(); if (e.key === 'Escape') cancelPrompt(); }}
        />
        <button onclick={apply} class="btn-primary" disabled={!promptValue}>OK</button>
        <button onclick={cancelPrompt}>Cancel</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .batch-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    margin-bottom: 12px;
  }
  .batch-count {
    font-size: 12px;
    color: var(--text-secondary);
    margin-right: auto;
  }
  .batch-btn {
    font-size: 12px;
    padding: 4px 10px;
  }
  .batch-action {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .batch-action select {
    font-size: 12px;
    padding: 4px 8px;
  }
  .batch-prompt {
    display: flex;
    gap: 6px;
    align-items: center;
    margin-left: 8px;
  }
  .batch-prompt input {
    width: 120px;
    font-size: 12px;
    padding: 4px 8px;
  }
</style>
