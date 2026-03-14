<script lang="ts">
  import { api, type TableMeta, type TableRow, type CellChange } from "$lib/api";
  import { addToast, setStatus } from "$lib/stores";
  import { preferencesStore } from "$lib/preferences";
  import { SvelteMap, SvelteSet } from "svelte/reactivity";

  const { tables }: { tables: TableMeta[] } = $props();

  const pageSize = $derived($preferencesStore.tablePageSize);

  let selectedTable = $state<string | null>(null);
  let columns = $state<string[]>([]);
  let rows = $state<TableRow[]>([]);
  let totalRows = $state(0);
  let page = $state(0);
  let loading = $state(false);
  let editingCell = $state<{ rowIdx: number; colIdx: number } | null>(null);
  let editValue = $state("");
  const changes = new SvelteMap<string, CellChange>();
  const selectedRowids = new SvelteSet<number>();

  const totalPages = $derived(Math.max(1, Math.ceil(totalRows / pageSize)));
  const changeCount = $derived(changes.size);

  // Display columns: skip the first column (rowid)
  const displayColumns = $derived(columns.slice(1));

  $effect(() => {
    if (tables.length > 0 && !selectedTable) {
      void selectTable(tables[0].name);
    }
  });

  async function selectTable(name: string) {
    selectedTable = name;
    page = 0;
    selectedRowids.clear();
    changes.clear();
    await loadData();
  }

  async function loadData() {
    if (!selectedTable) return;
    loading = true;
    try {
      const result = await api.queryTable(selectedTable, page * pageSize, pageSize);
      columns = result.columns;
      rows = result.rows;
      totalRows = result.total_rows;
    } catch (e) {
      addToast(`Failed to load table: ${e}`, "error");
    } finally {
      loading = false;
    }
  }

  async function goToPage(p: number) {
    page = p;
    await loadData();
  }

  function focusOnMount(node: HTMLElement) {
    node.focus();
  }

  function startEdit(rowIdx: number, colIdx: number) {
    const displayColIdx = colIdx;
    const actualColIdx = colIdx + 1; // skip rowid
    const rowid = Number(rows[rowIdx].values[0]);
    const colName = columns[actualColIdx];
    const key = `${selectedTable}:${rowid}:${colName}`;

    // If there's a staged change, use that value; otherwise use current cell value
    const existing = changes.get(key);
    const currentVal = existing !== undefined ? existing.value : rows[rowIdx].values[actualColIdx];
    editValue = currentVal === null ? "" : String(currentVal);
    editingCell = { rowIdx, colIdx: displayColIdx };
  }

  function commitEdit() {
    if (!editingCell || !selectedTable) return;
    const { rowIdx, colIdx } = editingCell;
    const actualColIdx = colIdx + 1;
    const rowid = Number(rows[rowIdx].values[0]);
    const colName = columns[actualColIdx];
    const originalValue = rows[rowIdx].values[actualColIdx];
    const parsed = parseValue(editValue);
    const key = `${selectedTable}:${rowid}:${colName}`;

    // If the parsed value matches the original, remove any staged change
    if (parsed === originalValue || (parsed === null && originalValue === null)) {
      changes.delete(key);
    } else {
      const change: CellChange = { table: selectedTable, rowid, column: colName, value: parsed };
      changes.set(key, change);
    }
    editingCell = null;
  }

  function cancelEdit() {
    editingCell = null;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      commitEdit();
    } else if (e.key === "Escape") {
      cancelEdit();
    }
  }

  function parseValue(val: string): unknown {
    if (val === "") return null;
    if (val === "true") return true;
    if (val === "false") return false;
    const num = Number(val);
    if (!isNaN(num) && val.trim() !== "") return num;
    return val;
  }

  function getCellValue(rowIdx: number, displayColIdx: number): unknown {
    const actualColIdx = displayColIdx + 1;
    const rowid = rows[rowIdx].values[0];
    const colName = columns[actualColIdx];
    const key = `${selectedTable}:${rowid}:${colName}`;
    const staged = changes.get(key);
    if (staged !== undefined) return staged.value;
    return rows[rowIdx].values[actualColIdx];
  }

  function isCellChanged(rowIdx: number, displayColIdx: number): boolean {
    const actualColIdx = displayColIdx + 1;
    const rowid = rows[rowIdx].values[0];
    const colName = columns[actualColIdx];
    const key = `${selectedTable}:${rowid}:${colName}`;
    return changes.has(key);
  }

  function formatCell(value: unknown): string {
    if (value === null) return "NULL";
    if (typeof value === "boolean") return String(value);
    return String(value);
  }

  function discardChanges() {
    changes.clear();
  }

  function toggleRowSelection(rowid: number) {
    if (selectedRowids.has(rowid)) {
      selectedRowids.delete(rowid);
    } else {
      selectedRowids.add(rowid);
    }
    // SvelteSet is reactive, no reassignment needed
  }

  async function addRow() {
    if (!selectedTable) return;
    try {
      const newRowid = await api.insertRow(selectedTable);
      setStatus(`Inserted row ${newRowid}`, "success");
      await loadData();
    } catch (e) {
      addToast(`Failed to insert row: ${e}`, "error");
    }
  }

  async function deleteSelectedRows() {
    if (!selectedTable || selectedRowids.size === 0) return;
    try {
      const ids = Array.from(selectedRowids);
      const deleted = await api.deleteRows(selectedTable, ids);
      setStatus(`Deleted ${deleted} row(s)`, "success");
      selectedRowids.clear();
      await loadData();
    } catch (e) {
      addToast(`Failed to delete: ${e}`, "error");
    }
  }

  async function saveChanges() {
    if (changes.size === 0) return;
    try {
      const changeList = Array.from(changes.values());
      const updated = await api.updateRows(changeList);
      setStatus(`Updated ${updated} row(s)`, "success");
      changes.clear();
      await loadData();
    } catch (e) {
      addToast(`Failed to save: ${e}`, "error");
    }
  }
</script>

<div class="table-browser">
  <aside class="sidebar">
    <div class="sidebar-header">Tables</div>
    {#each tables as table (table.name)}
      <button
        class="table-item"
        class:active={selectedTable === table.name}
        onclick={() => selectTable(table.name)}
      >
        <span class="table-name">{table.name}</span>
        <span class="row-count">{table.row_count}</span>
      </button>
    {/each}
  </aside>

  <div class="main-area">
    {#if changeCount > 0}
      <div class="changes-bar">
        <span>{changeCount} unsaved change{changeCount !== 1 ? "s" : ""}</span>
        <div class="changes-actions">
          <button class="btn-discard" onclick={discardChanges}>Discard</button>
          <button class="btn-save" onclick={saveChanges}>Save</button>
        </div>
      </div>
    {/if}

    {#if loading}
      <div class="loading">Loading...</div>
    {:else if !selectedTable}
      <div class="empty">Select a table to browse</div>
    {:else}
      <div class="toolbar">
        <button class="btn-action" onclick={addRow}>+ Add Row</button>
        <button
          class="btn-action btn-danger"
          onclick={deleteSelectedRows}
          disabled={selectedRowids.size === 0}
        >
          Delete ({selectedRowids.size})
        </button>
      </div>
      <div class="grid-container">
        <table class="data-grid">
          <thead>
            <tr>
              <th class="checkbox-col"></th>
              {#each displayColumns as col (col)}
                <th>{col}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each rows as row, rowIdx (row.values[0])}
              {@const rowid = Number(row.values[0])}
              <tr class:row-selected={selectedRowids.has(rowid)}>
                <td class="checkbox-col">
                  <input
                    type="checkbox"
                    checked={selectedRowids.has(rowid)}
                    onchange={() => toggleRowSelection(rowid)}
                  />
                </td>
                {#each displayColumns as _, colIdx (colIdx)}
                  <td
                    class:changed={isCellChanged(rowIdx, colIdx)}
                    onclick={() => startEdit(rowIdx, colIdx)}
                  >
                    {#if editingCell?.rowIdx === rowIdx && editingCell.colIdx === colIdx}
                      <input
                        class="cell-edit"
                        type="text"
                        bind:value={editValue}
                        onblur={commitEdit}
                        onkeydown={handleKeydown}
                        use:focusOnMount
                      />
                    {:else}
                      <span
                        class="cell-value"
                        class:null-value={getCellValue(rowIdx, colIdx) === null}
                      >
                        {formatCell(getCellValue(rowIdx, colIdx))}
                      </span>
                    {/if}
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      {#if totalPages > 1}
        <div class="pagination">
          <button disabled={page === 0} onclick={() => goToPage(page - 1)}>Prev</button>
          <span>Page {page + 1} of {totalPages}</span>
          <button disabled={page >= totalPages - 1} onclick={() => goToPage(page + 1)}>Next</button>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .table-browser {
    display: flex;
    height: 100%;
    gap: 0;
  }

  /* Sidebar */
  .sidebar {
    width: 200px;
    min-width: 200px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .sidebar-header {
    padding: 10px 12px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
  }

  .table-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    border: none;
    background: none;
    color: var(--text-secondary);
    font-size: 13px;
    cursor: pointer;
    text-align: left;
    width: 100%;
    border-bottom: 1px solid var(--border);
  }
  .table-item:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }
  .table-item.active {
    background: var(--bg-tertiary);
    color: var(--accent-primary);
  }

  .table-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .row-count {
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    margin-left: 8px;
    flex-shrink: 0;
  }

  /* Main area */
  .main-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  .changes-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: var(--bg-tertiary);
    border-bottom: 1px solid var(--border);
    font-size: 13px;
    color: var(--text-primary);
  }

  .changes-actions {
    display: flex;
    gap: 8px;
  }

  .btn-discard {
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-secondary);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
  }
  .btn-discard:hover {
    color: var(--text-primary);
  }

  .btn-save {
    padding: 4px 12px;
    border: 1px solid var(--accent-primary);
    border-radius: var(--radius-sm);
    background: var(--accent-primary);
    color: #fff;
    cursor: pointer;
    font-size: 12px;
  }
  .btn-save:hover {
    opacity: 0.9;
  }

  .toolbar {
    display: flex;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .btn-action {
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
  }
  .btn-action:hover:not(:disabled) {
    color: var(--text-primary);
    background: var(--bg-tertiary);
  }
  .btn-action:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-danger {
    color: #e74c3c;
    border-color: #e74c3c40;
  }
  .btn-danger:hover:not(:disabled) {
    background: #e74c3c20;
    color: #e74c3c;
  }

  .checkbox-col {
    width: 32px;
    min-width: 32px;
    max-width: 32px;
    text-align: center;
    padding: 4px !important;
  }

  .row-selected td {
    background: rgba(99, 102, 241, 0.1);
  }

  .loading,
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    font-size: 14px;
  }

  /* Data grid */
  .grid-container {
    flex: 1;
    overflow: auto;
  }

  .data-grid {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    font-family: var(--font-mono);
  }

  .data-grid thead {
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .data-grid th {
    background: var(--bg-tertiary);
    padding: 8px 12px;
    text-align: left;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
  }

  .data-grid td {
    padding: 4px 12px;
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-primary);
  }

  .data-grid tr:hover td {
    background: var(--bg-tertiary);
  }

  .data-grid td.changed {
    background: rgba(255, 165, 0, 0.15);
  }

  .data-grid tr:hover td.changed {
    background: rgba(255, 165, 0, 0.25);
  }

  .cell-value.null-value {
    color: var(--text-muted);
    font-style: italic;
  }

  .cell-edit {
    width: 100%;
    padding: 2px 4px;
    border: 1px solid var(--accent-primary);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 13px;
    outline: none;
  }

  /* Pagination */
  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 10px 12px;
    border-top: 1px solid var(--border);
    font-size: 13px;
    color: var(--text-secondary);
  }

  .pagination button {
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-secondary);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
  }
  .pagination button:hover:not(:disabled) {
    color: var(--text-primary);
    background: var(--bg-tertiary);
  }
  .pagination button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
