<script lang="ts">
  import { preferencesStore, setPreferences } from "$lib/preferences";

  let prefs = $derived($preferencesStore);

  // eslint-disable-next-line svelte/prefer-writable-derived -- intentional: bound to input, validated on blur
  let depthInput = $state("");

  $effect(() => {
    depthInput = String(prefs.undoHistoryDepth);
  });

  function handleDepthBlur() {
    const val = Math.round(Number(depthInput));
    if (!isNaN(val) && val >= 10 && val <= 1000) {
      setPreferences({ undoHistoryDepth: val });
    } else {
      depthInput = String(prefs.undoHistoryDepth);
    }
  }
</script>

<div class="settings-section">
  <h3>Editor</h3>

  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">Undo history depth</span>
      <span class="setting-desc">Maximum number of undo steps to keep (10-1000)</span>
    </div>
    <input
      class="number-input"
      type="number"
      min="10"
      max="1000"
      bind:value={depthInput}
      onblur={handleDepthBlur}
      onkeydown={(e) => {
        if (e.key === "Enter") handleDepthBlur();
      }}
    />
  </div>

  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">Search debounce</span>
      <span class="setting-desc">Delay before search starts after typing in the Raw tab</span>
    </div>
    <select
      value={prefs.searchDebounce}
      onchange={(e) =>
        setPreferences({ searchDebounce: Number((e.target as HTMLSelectElement).value) })}
    >
      <option value={200}>200ms</option>
      <option value={400}>400ms</option>
      <option value={600}>600ms</option>
      <option value={800}>800ms</option>
    </select>
  </div>

  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">Show unchanged in comparisons</span>
      <span class="setting-desc"
        >Default state of the "show unchanged" toggle in save comparison</span
      >
    </div>
    <button
      class="toggle-switch"
      class:on={prefs.showUnchangedDefault}
      onclick={() => setPreferences({ showUnchangedDefault: !prefs.showUnchangedDefault })}
      aria-pressed={prefs.showUnchangedDefault}
      aria-label="Toggle show unchanged default"
    >
      <span class="toggle-knob"></span>
    </button>
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

  .number-input {
    width: 80px;
    text-align: center;
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
