<script lang="ts">
  import type { Inventory } from '$lib/api';
  import { markModified } from '$lib/stores';

  let { inventory = $bindable() }: { inventory: Inventory } = $props();
  let search = $state('');
  let activeSection = $state<'items' | 'weapons' | 'armors'>('items');

  let filteredItems = $derived((() => {
    const list = inventory[activeSection];
    if (!search) return list;
    const q = search.toLowerCase();
    return list.filter(i =>
      i.name.toLowerCase().includes(q) || i.id.toString().includes(q)
    );
  })());
</script>

<div class="inventory-header">
  <div class="section-tabs">
    <button class:active={activeSection === 'items'} onclick={() => activeSection = 'items'}>
      Items ({inventory.items.length})
    </button>
    <button class:active={activeSection === 'weapons'} onclick={() => activeSection = 'weapons'}>
      Weapons ({inventory.weapons.length})
    </button>
    <button class:active={activeSection === 'armors'} onclick={() => activeSection = 'armors'}>
      Armors ({inventory.armors.length})
    </button>
  </div>
  <input type="text" placeholder="Search items..." bind:value={search} class="search-input" />
</div>

<div class="item-table">
  <div class="table-header">
    <span class="col-id">ID</span>
    <span class="col-name">Name</span>
    <span class="col-qty">Qty</span>
  </div>
  {#each filteredItems as item, idx}
    <div class="table-row">
      <span class="col-id">{item.id}</span>
      <span class="col-name">{item.name}</span>
      <input class="col-qty" type="number" bind:value={item.quantity}
        oninput={() => markModified(`inventory.${activeSection}.${idx}`)} min="0" />
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

  .search-input { width: 220px; }

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
  .table-row:hover { background: var(--bg-hover); }

  .col-id { width: 60px; color: var(--text-muted); }
  .col-name { flex: 1; }
  .col-qty { width: 80px; }

  .table-row .col-qty {
    text-align: right;
  }

  .no-results {
    padding: 20px;
    text-align: center;
    color: var(--text-muted);
  }
</style>
