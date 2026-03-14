<script lang="ts">
  import { batchMode, batchSelected, clearBatchSelection } from "$lib/stores";

  interface Props {
    items: { id: string }[];
    actions: { label: string; value: string }[];
    onapply: (action: string, selectedIds: Set<string>, value?: string) => void;
  }

  let { items, actions, onapply }: Props = $props();
  let selectedAction = $state("");
  let promptValue = $state("");

  // Actions ending with '...' need a value input
  let needsValue = $derived(
    actions.find((a) => a.value === selectedAction)?.label.endsWith("...") ?? false,
  );

  function apply() {
    if (!selectedAction || $batchSelected.size === 0) return;
    if (needsValue && !promptValue) return;
    onapply(selectedAction, $batchSelected, promptValue || undefined);
    selectedAction = "";
    promptValue = "";
  }

  function selectAll() {
    batchSelected.set(new Set(items.map((i) => i.id)));
  }

  function selectNone() {
    clearBatchSelection();
  }
</script>

{#if $batchMode}
  <div class="batch-toolbar">
    <span class="batch-count">{$batchSelected.size} of {items.length} selected</span>
    <button onclick={selectAll} class="batch-btn">Select All</button>
    <button onclick={selectNone} class="batch-btn">Select None</button>
    <div class="batch-action">
      <select
        bind:value={selectedAction}
        onchange={() => {
          promptValue = "";
        }}
      >
        <option value="">Choose action...</option>
        {#each actions as action (action.value)}
          <option value={action.value}>{action.label}</option>
        {/each}
      </select>
      {#if needsValue}
        <input
          type="text"
          class="batch-value-input"
          bind:value={promptValue}
          placeholder="Enter value..."
          onkeydown={(e) => {
            if (e.key === "Enter") apply();
          }}
        />
      {/if}
      <button
        onclick={apply}
        disabled={!selectedAction || $batchSelected.size === 0 || (needsValue && !promptValue)}
        class="btn-primary"
      >
        Apply
      </button>
    </div>
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
    flex-wrap: wrap;
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
  .batch-value-input {
    width: 120px;
    font-size: 12px;
    padding: 4px 8px;
  }
</style>
