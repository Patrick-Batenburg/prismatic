<script lang="ts">
  import { goto } from '$app/navigation';
  import { api, type SaveFile, type SaveData, type DiffEntry, type BackupEntry } from '$lib/api';
  import {
    currentEngine, currentGameDir, saves, currentSave, currentSavePath,
    nameMap, statusMessage, addToast, activePatch, modifiedFields
  } from '$lib/stores';
  import PartyTab from '$lib/components/tabs/PartyTab.svelte';
  import InventoryTab from '$lib/components/tabs/InventoryTab.svelte';
  import VariablesTab from '$lib/components/tabs/VariablesTab.svelte';
  import SwitchesTab from '$lib/components/tabs/SwitchesTab.svelte';
  import CurrencyTab from '$lib/components/tabs/CurrencyTab.svelte';
  import RawTab from '$lib/components/tabs/RawTab.svelte';
  import DiffView from '$lib/components/DiffView.svelte';

  let engine = $derived($currentEngine);
  let gameDir = $derived($currentGameDir);
  let saveList = $derived($saves);
  let save = $derived($currentSave);
  let savePath = $derived($currentSavePath);
  let patch = $derived($activePatch);

  let activeTab = $state('party');
  let loading = $state(false);
  let showDiff = $state(false);
  let diffEntries = $state<DiffEntry[]>([]);
  let backups = $state<BackupEntry[]>([]);
  let showBackups = $state(false);

  // Available tabs based on save data
  let tabs = $derived((() => {
    const t: { id: string; label: string; available: boolean }[] = [
      { id: 'party', label: 'Party', available: !!save?.party },
      { id: 'inventory', label: 'Inventory', available: !!save?.inventory },
      { id: 'currency', label: 'Currency', available: !!save?.currency },
      { id: 'variables', label: 'Variables', available: !!save?.variables },
      { id: 'switches', label: 'Switches', available: !!save?.switches },
      { id: 'raw', label: 'Raw', available: !!save },
    ];
    return t.filter(tab => tab.available);
  })());

  $effect(() => {
    if (!engine || !gameDir) {
      goto('/');
      return;
    }
    loadSaves();
  });

  async function loadSaves() {
    try {
      const list = await api.listSaves();
      saves.set(list);
      // Also load name map
      try {
        const names = await api.getNames();
        nameMap.set(names);
      } catch { /* name resolution is optional */ }
    } catch (e) {
      addToast(`Failed to list saves: ${e}`, 'error');
    }
  }

  async function selectSave(sf: SaveFile) {
    loading = true;
    try {
      const data = await api.loadSave(sf.path);
      currentSave.set(data);
      currentSavePath.set(sf.path);
      modifiedFields.set(new Set());
      statusMessage.set(`${engine?.name} — ${sf.name}`);

      // Auto-select first available tab
      if (tabs.length > 0 && !tabs.find(t => t.id === activeTab)) {
        activeTab = tabs[0].id;
      }
    } catch (e) {
      addToast(`Failed to load save: ${e}`, 'error');
    } finally {
      loading = false;
    }
  }

  async function handleSave() {
    if (!savePath || !save) return;
    try {
      const msg = await api.saveFile(savePath, save);
      addToast(msg, 'success');
      modifiedFields.set(new Set());
    } catch (e) {
      addToast(`Save failed: ${e}`, 'error');
    }
  }

  async function handleReload() {
    if (!savePath) return;
    try {
      diffEntries = await api.getDiff(savePath);
      if (diffEntries.length > 0) {
        showDiff = true;
      } else {
        addToast('No changes detected', 'info');
      }
      // Reload the save
      const data = await api.loadSave(savePath);
      currentSave.set(data);
      modifiedFields.set(new Set());
    } catch (e) {
      addToast(`Reload failed: ${e}`, 'error');
    }
  }

  async function handleDebugToggle() {
    if (!engine?.supports_debug) return;
    try {
      if (patch) {
        await api.revertDebugPatch(patch);
        activePatch.set(null);
        addToast('Debug mode reverted', 'success');
      } else {
        const p = await api.applyDebugPatch();
        activePatch.set(p);
        addToast('Debug mode enabled', 'success');
      }
    } catch (e) {
      addToast(`Debug patch failed: ${e}`, 'error');
    }
  }

  async function showBackupList() {
    if (!savePath) return;
    try {
      backups = await api.listBackups(savePath);
      showBackups = true;
    } catch (e) {
      addToast(`Failed to list backups: ${e}`, 'error');
    }
  }

  async function restoreBackup(backup: BackupEntry) {
    if (!savePath) return;
    try {
      await api.restoreBackup(backup.path, savePath);
      addToast('Backup restored', 'success');
      showBackups = false;
      // Reload
      const data = await api.loadSave(savePath);
      currentSave.set(data);
    } catch (e) {
      addToast(`Restore failed: ${e}`, 'error');
    }
  }

  function goBack() {
    currentSave.set(null);
    currentSavePath.set(null);
    currentEngine.set(null);
    currentGameDir.set(null);
    goto('/');
  }
</script>

<div class="editor-layout">
  <!-- Sidebar: save file list -->
  <aside class="sidebar">
    <div class="sidebar-header">
      <button class="btn-back" onclick={goBack}>← Back</button>
      <h2 class="sidebar-title">{engine?.name || 'Editor'}</h2>
    </div>

    <div class="save-list">
      {#each saveList as sf}
        <button
          class="save-item"
          class:active={savePath === sf.path}
          onclick={() => selectSave(sf)}
        >
          <div class="save-name">{sf.name}</div>
          <div class="save-meta">
            {(sf.size / 1024).toFixed(0)} KB
          </div>
        </button>
      {/each}
      {#if saveList.length === 0}
        <div class="no-saves">No save files found</div>
      {/if}
    </div>

    {#if engine?.supports_debug}
      <div class="debug-section">
        <button
          class="debug-toggle"
          class:active={!!patch}
          onclick={handleDebugToggle}
        >
          {patch ? '🔧 Revert Debug Mode' : '🔧 Enable Debug Mode'}
        </button>
      </div>
    {/if}
  </aside>

  <!-- Main content -->
  <div class="main-area">
    {#if loading}
      <div class="loading-state">Loading save...</div>
    {:else if save}
      <!-- Toolbar -->
      <div class="toolbar">
        <div class="tab-bar">
          {#each tabs as tab}
            <button
              class="tab"
              class:active={activeTab === tab.id}
              onclick={() => activeTab = tab.id}
            >
              {tab.label}
            </button>
          {/each}
        </div>

        <div class="toolbar-actions">
          <button onclick={handleReload} title="Reload & diff">↻ Reload</button>
          <button onclick={showBackupList} title="Backups">📦 Backups</button>
          <button class="btn-primary" onclick={handleSave}>💾 Save</button>
        </div>
      </div>

      <!-- Tab content -->
      <div class="tab-content">
        {#if activeTab === 'party' && save.party}
          <PartyTab party={save.party} />
        {:else if activeTab === 'inventory' && save.inventory}
          <InventoryTab inventory={save.inventory} />
        {:else if activeTab === 'currency' && save.currency}
          <CurrencyTab currency={save.currency} />
        {:else if activeTab === 'variables' && save.variables}
          <VariablesTab variables={save.variables} />
        {:else if activeTab === 'switches' && save.switches}
          <SwitchesTab switches={save.switches} />
        {:else if activeTab === 'raw'}
          <RawTab data={save.raw} />
        {/if}
      </div>
    {:else}
      <div class="empty-state">
        <div class="empty-icon">📂</div>
        <div class="empty-text">Select a save file from the sidebar</div>
      </div>
    {/if}
  </div>

  <!-- Diff modal -->
  {#if showDiff}
    <DiffView entries={diffEntries} onclose={() => showDiff = false} />
  {/if}

  <!-- Backups modal -->
  {#if showBackups}
    <div class="modal-overlay" onclick={() => showBackups = false}>
      <div class="modal" onclick={(e) => e.stopPropagation()}>
        <h3>Backups</h3>
        {#if backups.length === 0}
          <p class="no-data">No backups found</p>
        {:else}
          <div class="backup-list">
            {#each backups as backup}
              <div class="backup-item">
                <div>
                  <div class="backup-name">{backup.name}</div>
                  <div class="backup-meta">{new Date(backup.modified).toLocaleString()} — {(backup.size / 1024).toFixed(0)} KB</div>
                </div>
                <button class="btn-primary" onclick={() => restoreBackup(backup)}>Restore</button>
              </div>
            {/each}
          </div>
        {/if}
        <button onclick={() => showBackups = false}>Close</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .editor-layout {
    display: flex;
    height: 100%;
  }

  .sidebar {
    width: 240px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .sidebar-header {
    padding: 12px;
    border-bottom: 1px solid var(--border);
  }

  .btn-back {
    font-size: 12px;
    padding: 4px 8px;
    margin-bottom: 8px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
  }
  .btn-back:hover { color: var(--text-primary); }

  .sidebar-title {
    font-size: 15px;
    font-weight: 600;
  }

  .save-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .save-item {
    width: 100%;
    text-align: left;
    padding: 10px 12px;
    border-radius: var(--radius);
    margin-bottom: 4px;
    background: transparent;
    border: 1px solid transparent;
  }
  .save-item:hover { background: var(--bg-hover); }
  .save-item.active {
    background: var(--bg-tertiary);
    border-color: var(--accent-primary);
  }

  .save-name { font-size: 13px; font-weight: 500; }
  .save-meta { font-size: 11px; color: var(--text-muted); margin-top: 2px; }

  .no-saves { padding: 20px; text-align: center; color: var(--text-muted); font-size: 13px; }

  .debug-section {
    padding: 12px;
    border-top: 1px solid var(--border);
  }

  .debug-toggle {
    width: 100%;
    padding: 8px;
    font-size: 12px;
    border-radius: var(--radius);
  }
  .debug-toggle.active {
    background: rgba(239, 68, 68, 0.1);
    border-color: var(--danger);
    color: var(--danger);
  }

  .main-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .tab-bar {
    display: flex;
    gap: 2px;
  }

  .tab {
    padding: 6px 16px;
    border-radius: var(--radius) var(--radius) 0 0;
    font-size: 13px;
    background: transparent;
    border: 1px solid transparent;
    border-bottom: none;
    color: var(--text-secondary);
  }
  .tab:hover { color: var(--text-primary); background: var(--bg-tertiary); }
  .tab.active {
    background: var(--bg-primary);
    color: var(--accent-primary);
    border-color: var(--border);
    font-weight: 500;
  }

  .toolbar-actions {
    display: flex;
    gap: 8px;
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  .loading-state, .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
  }

  .empty-icon { font-size: 48px; margin-bottom: 12px; }
  .empty-text { font-size: 16px; }

  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 24px;
    min-width: 400px;
    max-width: 600px;
    max-height: 80vh;
    overflow-y: auto;
  }

  .modal h3 { margin-bottom: 16px; }
  .no-data { color: var(--text-muted); padding: 16px; text-align: center; }

  .backup-list { display: flex; flex-direction: column; gap: 8px; margin-bottom: 16px; }

  .backup-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    background: var(--bg-tertiary);
    border-radius: var(--radius);
  }

  .backup-name { font-size: 13px; font-weight: 500; }
  .backup-meta { font-size: 11px; color: var(--text-muted); }
</style>
