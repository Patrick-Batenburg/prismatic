<script lang="ts">
  import { preferencesStore, setPreferences } from "$lib/preferences";
  import { checkForUpdates, relaunchApp } from "$lib/updater";
  import { addToast } from "$lib/stores";

  const prefs = $derived($preferencesStore);
  let checking = $state(false);
  let updateReady = $state(false);

  async function handleCheckUpdate() {
    checking = true;
    try {
      const result = await checkForUpdates();
      if (result?.available) {
        updateReady = true;
        addToast(`Update ${result.version} downloaded — restart to install`, "info", 0);
      } else {
        addToast("You're on the latest version", "success");
      }
    } finally {
      checking = false;
    }
  }
</script>

<div class="settings-section">
  <h3>General</h3>

  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">Update behavior</span>
      <span class="setting-desc">How the app handles new versions</span>
    </div>
    <select
      value={prefs.updateMode}
      onchange={(e: Event & { currentTarget: HTMLSelectElement }) => {
        const val = e.currentTarget.value;
        if (val === "auto" || val === "notify" || val === "off") {
          setPreferences({ updateMode: val });
        }
      }}
    >
      <option value="auto">Automatic</option>
      <option value="notify">Notify only</option>
      <option value="off">Disabled</option>
    </select>
  </div>

  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">Updates</span>
      <span class="setting-desc"
        >{updateReady
          ? "A new version is ready to install"
          : "Check for new versions of Prismatic"}</span
      >
    </div>
    {#if updateReady}
      <button class="btn-update restart" onclick={relaunchApp}>Restart to update</button>
    {:else}
      <button class="btn-update" onclick={handleCheckUpdate} disabled={checking}>
        {checking ? "Checking..." : "Check for updates"}
      </button>
    {/if}
  </div>

  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">Deep scan by default</span>
      <span class="setting-desc">Scan subfolders for save files when opening folder picker</span>
    </div>
    <button
      class="toggle-switch"
      class:on={prefs.deepScanDefault}
      onclick={() => setPreferences({ deepScanDefault: !prefs.deepScanDefault })}
      aria-pressed={prefs.deepScanDefault}
      aria-label="Toggle deep scan default"
    >
      <span class="toggle-knob"></span>
    </button>
  </div>

  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">Notification duration</span>
      <span class="setting-desc">How long toast notifications stay visible</span>
    </div>
    <select
      value={prefs.notificationDuration}
      onchange={(e: Event & { currentTarget: HTMLSelectElement }) =>
        setPreferences({ notificationDuration: Number(e.currentTarget.value) })}
    >
      <option value={2}>2s</option>
      <option value={4}>4s</option>
      <option value={6}>6s</option>
      <option value={8}>8s</option>
    </select>
  </div>

  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">Status flash duration</span>
      <span class="setting-desc">How long status bar messages stay visible</span>
    </div>
    <select
      value={prefs.statusFlashDuration}
      onchange={(e: Event & { currentTarget: HTMLSelectElement }) =>
        setPreferences({ statusFlashDuration: Number(e.currentTarget.value) })}
    >
      <option value={3}>3s</option>
      <option value={5}>5s</option>
      <option value={8}>8s</option>
      <option value={10}>10s</option>
    </select>
  </div>

  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">Table page size</span>
      <span class="setting-desc">Number of rows per page in the SQLite table browser</span>
    </div>
    <select
      value={prefs.tablePageSize}
      onchange={(e: Event & { currentTarget: HTMLSelectElement }) =>
        setPreferences({ tablePageSize: Number(e.currentTarget.value) })}
    >
      <option value={25}>25</option>
      <option value={50}>50</option>
      <option value={100}>100</option>
      <option value={200}>200</option>
    </select>
  </div>
</div>

<style>
  .settings-section h3 {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 16px;
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }

  .setting-row:last-child {
    border-bottom: none;
  }

  .setting-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .setting-label {
    font-size: 14px;
    color: var(--text-primary);
  }

  .setting-desc {
    font-size: 12px;
    color: var(--text-muted);
  }

  select {
    min-width: 80px;
  }

  .btn-update {
    font-size: 13px;
    padding: 6px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    cursor: pointer;
    white-space: nowrap;
  }

  .btn-update:hover:not(:disabled) {
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }

  .btn-update:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .btn-update.restart {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
    color: white;
  }

  .btn-update.restart:hover {
    opacity: 0.9;
  }

  .toggle-switch {
    position: relative;
    width: 44px;
    height: 24px;
    border-radius: 12px;
    border: none;
    background: var(--bg-tertiary);
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
    transition: background 0.2s ease;
  }

  .toggle-switch.on {
    background: var(--accent-primary);
  }

  .toggle-knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    transition: transform 0.2s ease;
  }

  .toggle-switch.on .toggle-knob {
    transform: translateX(20px);
  }
</style>
