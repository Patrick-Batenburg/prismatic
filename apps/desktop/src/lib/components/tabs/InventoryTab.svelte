<script lang="ts">
  import type { Inventory } from "$lib/api";
  import { batchMode, batchSelected, toggleBatchItem, history, markModified, trackEdit } from '$lib/stores';
  import type { Change } from '$lib/stores/history';
  import BatchToolbar from '$lib/components/BatchToolbar.svelte';

  let { inventory = $bindable() }: { inventory: Inventory } = $props();
  let search = $state("");
  let activeSection = $state<"items" | "weapons" | "armors">("items");

  let filteredItems = $derived(
    (() => {
      const list = inventory[activeSection];
      if (!search) return list;
      const q = search.toLowerCase();
      return list.filter((i) => i.name.toLowerCase().includes(q) || i.id.toString().includes(q));
    })(),
  );

  function handleBatchAction(action: string, selectedIds: Set<string>, value?: string) {
    const changes: Change[] = [];
    const section = inventory[activeSection];
    const numVal = value ? Number(value) : 0;

    if (action === 'set_quantity') {
      for (let i = 0; i < section.length; i++) {
        if (!selectedIds.has(String(section[i].id))) continue;
        changes.push({ path: ['inventory', activeSection, String(i), 'quantity'], oldValue: section[i].quantity, newValue: numVal });
        section[i].quantity = numVal;
        markModified(`inventory.${activeSection}.${i}.quantity`);
      }
    } else if (action === 'remove_selected') {
      const oldArray = structuredClone(section);
      const newArray = section.filter(item => !selectedIds.has(String(item.id)));
      changes.push({ path: ['inventory', activeSection], oldValue: oldArray, newValue: newArray });
      inventory[activeSection] = newArray;
    }

    if (changes.length > 0) {
      history.push({ description: `Batch ${action} on ${selectedIds.size} items`, changes });
    }
  }
</script>

<div class="inventory-header">
  <div class="section-tabs">
    <button class:active={activeSection === "items"} onclick={() => (activeSection = "items")}>
      Items ({inventory.items.length})
    </button>
    <button class:active={activeSection === "weapons"} onclick={() => (activeSection = "weapons")}>
      Weapons ({inventory.weapons.length})
    </button>
    <button class:active={activeSection === "armors"} onclick={() => (activeSection = "armors")}>
      Armors ({inventory.armors.length})
    </button>
  </div>
  <input type="text" placeholder="Search items..." bind:value={search} class="search-input" />
</div>

<BatchToolbar
  items={filteredItems.map((item) => ({ id: String(item.id) }))}
  actions={[
    { label: 'Set quantity...', value: 'set_quantity' },
    { label: 'Remove selected', value: 'remove_selected' },
  ]}
  onapply={handleBatchAction}
/>

<div class="item-table">
  <div class="table-header">
    {#if $batchMode}
      <span class="col-check"></span>
    {/if}
    <span class="col-id">ID</span>
    <span class="col-name">Name</span>
    <span class="col-qty">Qty</span>
  </div>
  {#each filteredItems as item, idx (item.id)}
    <div class="table-row">
      {#if $batchMode}
        <input
          type="checkbox"
          class="batch-check"
          checked={$batchSelected.has(String(item.id))}
          onchange={() => toggleBatchItem(String(item.id))}
        />
      {/if}
      <span class="col-id">{item.id}</span>
      <span class="col-name">{item.name}</span>
      <input
        class="col-qty"
        type="number"
        value={item.quantity}
        onfocus={(e) => { e.currentTarget.dataset.old = String(item.quantity); }}
        onchange={(e) => {
          const oldVal = Number(e.currentTarget.dataset.old);
          const newVal = Number(e.currentTarget.value);
          item.quantity = newVal;
          trackEdit(
            ['inventory', activeSection, String(idx), 'quantity'],
            oldVal, newVal,
            `Set ${item.name} quantity to ${newVal}`
          );
          markModified(`inventory.${activeSection}.${idx}`);
        }}
        min="0"
      />
    </div>
  {/each}
  {#if filteredItems.length === 0}
    <div class="no-results">No items found</div>
  {/if}
</div>

<style>
  .inventory-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
    gap: 12px;
  }

  .section-tabs {
    display: flex;
    gap: 4px;
  }

  .section-tabs button {
    padding: 6px 12px;
    font-size: 12px;
    border-radius: var(--radius);
  }
  .section-tabs button.active {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
    color: white;
  }

  .search-input {
    width: 220px;
  }

  .item-table {
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
    padding: 6px 12px;
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
  }
  .col-name {
    flex: 1;
  }
  .col-qty {
    width: 80px;
  }

  .table-row .col-qty {
    text-align: right;
  }

  .no-results {
    padding: 20px;
    text-align: center;
    color: var(--text-muted);
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
