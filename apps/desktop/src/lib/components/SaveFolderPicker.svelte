<script lang="ts">
  import { api, type SaveDirEntry, type ScanProgressEvent } from '$lib/api';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  let {
    onselect,
    oncancel,
    title = 'Select Save Folder',
    hint = 'Navigate to the folder containing your save files.',
    extension = 'sav',
    defaultDir = null as string | null,
    badgeColor = '#6c5ce7',
  }: {
    onselect: (path: string) => void;
    oncancel: () => void;
    title?: string;
    hint?: string;
    extension?: string;
    defaultDir?: string | null;
    badgeColor?: string;
  } = $props();

  let currentPath = $state('');
  let entries = $state<SaveDirEntry[]>([]);
  let loading = $state(true);
  let error = $state('');
  let pathInput = $state('');
  let deepScan = $state(false);
  let scanning = $state(false);
  let scanProgress = $state({ done: 0, total: 0 });

  let unlistenProgress: UnlistenFn | null = null;
  let unlistenComplete: UnlistenFn | null = null;

  let filesHere = $derived(entries.filter(e => !e.is_dir).length);

  $effect(() => {
    browse();
    return () => {
      cleanupListeners();
    };
  });

  function cleanupListeners() {
    unlistenProgress?.();
    unlistenComplete?.();
    unlistenProgress = null;
    unlistenComplete = null;
  }

  async function browse(dir?: string) {
    loading = true;
    error = '';
    scanning = false;
    cleanupListeners();
    try {
      const [resolvedPath, items] = await api.browseSaveDir(dir ?? null, defaultDir, extension);
      currentPath = resolvedPath;
      pathInput = resolvedPath;
      entries = items;

      if (deepScan) {
        startDeepScan();
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function startDeepScan() {
    cleanupListeners();
    scanning = true;
    scanProgress = { done: 0, total: 0 };

    unlistenProgress = await listen<ScanProgressEvent>('scan-progress', (event) => {
      const { path, file_count, folders_done, folders_total } = event.payload;
      scanProgress = { done: folders_done, total: folders_total };

      entries = entries.map(e =>
        e.is_dir && e.path === path ? { ...e, file_count } : e
      );
    });

    unlistenComplete = await listen('scan-complete', () => {
      scanning = false;
      cleanupListeners();
    });

    try {
      await api.deepScanDir(currentPath, extension);
    } catch (e) {
      scanning = false;
      cleanupListeners();
    }
  }

  function toggleDeepScan() {
    deepScan = !deepScan;
    if (deepScan) {
      startDeepScan();
    } else {
      scanning = false;
      cleanupListeners();
    }
  }

  function navigateUp() {
    const parent = currentPath.replace(/[\\/][^\\/]+$/, '');
    if (parent && parent !== currentPath) {
      browse(parent);
    }
  }

  function navigateTo(entry: SaveDirEntry) {
    if (entry.is_dir) {
      browse(entry.path);
    }
  }

  function handlePathSubmit() {
    if (pathInput.trim()) {
      browse(pathInput.trim());
    }
  }

  function selectCurrent() {
    onselect(currentPath);
  }

  function simulateScan() {
    scanning = true;
    const total = 20;
    scanProgress = { done: 0, total };
    const interval = setInterval(() => {
      scanProgress = { done: scanProgress.done + 1, total };
      if (scanProgress.done >= total) {
        clearInterval(interval);
        scanning = false;
      }
    }, 200);
  }
</script>

<div class="modal-overlay" role="button" tabindex="-1" onclick={oncancel} onkeydown={(e) => { if (e.key === 'Escape') oncancel(); }}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <h3>{title}</h3>
      <p class="hint">{hint} Look for <code>.{extension}</code> files.</p>
    </div>

    <div class="path-bar">
      <button class="btn-up" onclick={navigateUp} title="Go up">
        ..
      </button>
      <input
        class="path-input"
        type="text"
        bind:value={pathInput}
        onkeydown={(e) => { if (e.key === 'Enter') handlePathSubmit(); }}
      />
      <button class="btn-go" onclick={handlePathSubmit}>Go</button>
      {#if import.meta.env.DEV}
        <button class="btn-go" onclick={simulateScan} title="DEV: simulate scan">▶</button>
      {/if}
      <button
        class="btn-deep"
        class:active={deepScan}
        onclick={toggleDeepScan}
        title={deepScan ? 'Switch to shallow scan (immediate files only)' : 'Deep scan (count files in all subfolders)'}
      >
        {deepScan ? '🔍' : '📁'}
      </button>
    </div>

    <div class="scan-bar" class:visible={scanning}>
      <div class="scan-progress" style="width: {scanProgress.total > 0 ? (scanProgress.done / scanProgress.total * 100) : 0}%"></div>
      <span class="scan-text">Scanning {scanProgress.done}/{scanProgress.total} folders...</span>
    </div>

    {#if loading}
      <div class="loading-state">Loading...</div>
    {:else if error}
      <div class="error-state">{error}</div>
    {:else}
      <div class="file-list">
        {#each entries as entry}
          {#if entry.is_dir}
            <button class="file-entry dir" onclick={() => navigateTo(entry)}>
              <span class="entry-icon">📁</span>
              <span class="entry-name">{entry.name}</span>
              {#if entry.file_count > 0}
                <span class="file-badge" style="background: color-mix(in srgb, {badgeColor} 15%, transparent); color: {badgeColor};">{entry.file_count} .{extension}</span>
              {/if}
            </button>
          {:else}
            <div class="file-entry file">
              <span class="entry-icon">📄</span>
              <span class="entry-name">{entry.name}</span>
            </div>
          {/if}
        {/each}
        {#if entries.length === 0}
          <div class="empty">No files or folders found</div>
        {/if}
      </div>
    {/if}

    <div class="modal-footer">
      {#if filesHere > 0}
        <span class="file-info" style="color: {badgeColor};">{filesHere} .{extension} file{filesHere !== 1 ? 's' : ''} in this folder</span>
      {/if}
      <div class="footer-actions">
        <button onclick={oncancel}>Cancel</button>
        <button
          class="btn-primary"
          disabled={filesHere === 0}
          onclick={selectCurrent}
          title={filesHere === 0 ? `Navigate to a folder containing .${extension} files` : ''}
        >
          Select this folder
        </button>
      </div>
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

  .modal {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 24px;
    width: 600px;
    max-width: 90vw;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
  }

  .modal-header {
    margin-bottom: 16px;
  }

  .modal-header h3 {
    margin-bottom: 4px;
  }

  .hint {
    color: var(--text-secondary);
    font-size: 13px;
  }

  .hint code {
    background: var(--bg-tertiary);
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 12px;
  }

  .path-bar {
    display: flex;
    gap: 4px;
    margin-bottom: 12px;
  }

  .btn-up {
    padding: 6px 10px;
    font-size: 13px;
    font-weight: 600;
    border-radius: var(--radius);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    flex-shrink: 0;
  }

  .btn-up:hover {
    background: var(--bg-hover);
  }

  .path-input {
    flex: 1;
    padding: 6px 10px;
    font-size: 12px;
    font-family: monospace;
    border-radius: var(--radius);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    color: var(--text-primary);
  }

  .btn-go {
    padding: 6px 12px;
    font-size: 13px;
    border-radius: var(--radius);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    flex-shrink: 0;
  }

  .btn-go:hover {
    background: var(--bg-hover);
  }

  .btn-deep {
    padding: 6px 10px;
    font-size: 13px;
    border-radius: var(--radius);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    flex-shrink: 0;
    cursor: pointer;
  }

  .btn-deep:hover {
    background: var(--bg-hover);
  }

  .btn-deep.active {
    background: color-mix(in srgb, var(--accent-primary) 15%, transparent);
    border-color: var(--accent-primary);
  }

  .scan-bar {
    position: relative;
    height: 20px;
    background: var(--bg-tertiary);
    border-radius: var(--radius);
    margin-bottom: 8px;
    overflow: hidden;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.15s ease;
  }

  .scan-bar.visible {
    opacity: 1;
    pointer-events: auto;
  }

  .scan-progress {
    position: absolute;
    top: 0;
    left: 0;
    height: 100%;
    background: color-mix(in srgb, var(--accent-primary) 30%, transparent);
    transition: width 0.15s ease;
    border-radius: var(--radius);
  }

  .scan-text {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .file-list {
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-primary);
    height: 400px;
  }

  .file-entry {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    font-size: 13px;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border);
    color: var(--text-primary);
  }

  .file-entry:last-child {
    border-bottom: none;
  }

  .file-entry.dir {
    cursor: pointer;
  }

  .file-entry.dir:hover {
    background: var(--bg-hover);
  }

  .entry-icon {
    flex-shrink: 0;
  }

  .entry-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-badge {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 10px;
    font-weight: 500;
    flex-shrink: 0;
  }

  .file-entry.file {
    color: var(--text-muted);
  }

  .loading-state, .error-state, .empty {
    padding: 32px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }

  .error-state {
    color: var(--danger);
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 16px;
    gap: 12px;
  }

  .file-info {
    font-size: 12px;
    font-weight: 500;
  }

  .footer-actions {
    display: flex;
    gap: 8px;
    margin-left: auto;
  }
</style>
