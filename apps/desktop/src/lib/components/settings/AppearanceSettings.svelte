<script lang="ts">
  import { preferencesStore, setPreferences } from "$lib/preferences";

  const prefs = $derived($preferencesStore);

  const COLOR_PRESETS = [
    { name: "Purple", hex: "#6c5ce7" },
    { name: "Blue", hex: "#3b82f6" },
    { name: "Teal", hex: "#14b8a6" },
    { name: "Green", hex: "#10b981" },
    { name: "Orange", hex: "#f59e0b" },
    { name: "Pink", hex: "#ec4899" },
    { name: "Red", hex: "#ef4444" },
    { name: "Indigo", hex: "#6366f1" },
  ];

  const HEX_RE = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;

  // eslint-disable-next-line svelte/prefer-writable-derived -- intentional: bound to input, reset on blur
  let customHex = $state("");
  const isCustomColor = $derived(!COLOR_PRESETS.some((p) => p.hex === prefs.accentColor));
  let showCustomInput = $state(false);

  $effect(() => {
    customHex = prefs.accentColor;
  });

  function handleCustomBlur() {
    if (HEX_RE.test(customHex)) {
      setPreferences({ accentColor: customHex });
    } else {
      customHex = prefs.accentColor;
    }
  }

  const APP_FONTS = ["system-default", "Inter", "Segoe UI", "Roboto", "Noto Sans"];

  const MONO_FONTS = [
    "Cascadia Code",
    "Fira Code",
    "JetBrains Mono",
    "Consolas",
    "Source Code Pro",
  ];
</script>

<div class="settings-section">
  <h3>Appearance</h3>

  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">Theme</span>
      <span class="setting-desc">Switch between light and dark mode</span>
    </div>
    <div class="segmented-control">
      <button
        class:active={prefs.theme === "dark"}
        onclick={() => setPreferences({ theme: "dark" })}>Dark</button
      >
      <button
        class:active={prefs.theme === "light"}
        onclick={() => setPreferences({ theme: "light" })}>Light</button
      >
    </div>
  </div>

  <div class="setting-row setting-row-column">
    <div class="setting-info">
      <span class="setting-label">Accent color</span>
      <span class="setting-desc">Primary color used for highlights and focus indicators</span>
    </div>
    <div class="color-row">
      {#each COLOR_PRESETS as preset (preset.hex)}
        <button
          class="color-swatch"
          class:active={prefs.accentColor === preset.hex}
          style="background: {preset.hex};"
          title={preset.name}
          onclick={() => {
            setPreferences({ accentColor: preset.hex });
            showCustomInput = false;
          }}
        ></button>
      {/each}
      <button
        class="color-swatch custom-swatch"
        class:active={isCustomColor || showCustomInput}
        style="background: {isCustomColor ? prefs.accentColor : 'var(--bg-tertiary)'};"
        title="Custom"
        onclick={() => (showCustomInput = !showCustomInput)}
      >
        {#if !isCustomColor}?{/if}
      </button>
    </div>
    {#if showCustomInput || isCustomColor}
      <input
        class="hex-input"
        type="text"
        bind:value={customHex}
        onblur={handleCustomBlur}
        onkeydown={(e) => {
          if (e.key === "Enter") handleCustomBlur();
        }}
        placeholder="#6c5ce7"
        maxlength={7}
      />
    {/if}
  </div>

  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">App font</span>
      <span class="setting-desc">Font used for the application interface</span>
    </div>
    <select
      value={prefs.appFont}
      onchange={(e: Event & { currentTarget: HTMLSelectElement }) =>
        setPreferences({ appFont: e.currentTarget.value })}
    >
      {#each APP_FONTS as font (font)}
        <option value={font}>{font === "system-default" ? "System Default" : font}</option>
      {/each}
    </select>
  </div>

  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">Monospace font</span>
      <span class="setting-desc">Font used for code, JSON, and table data</span>
    </div>
    <select
      value={prefs.monoFont}
      onchange={(e: Event & { currentTarget: HTMLSelectElement }) =>
        setPreferences({ monoFont: e.currentTarget.value })}
    >
      {#each MONO_FONTS as font (font)}
        <option value={font}>{font}</option>
      {/each}
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

  .segmented-control {
    display: flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .segmented-control button {
    padding: 6px 16px;
    font-size: 13px;
    border: none;
    border-radius: 0;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .segmented-control button:not(:last-child) {
    border-right: 1px solid var(--border);
  }

  .segmented-control button.active {
    background: var(--accent-primary);
    color: white;
  }

  .setting-row-column {
    flex-direction: column;
    align-items: flex-start;
  }

  .color-row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 8px;
  }

  .color-swatch {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    padding: 0;
    transition: all 0.15s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    color: var(--text-muted);
  }

  .color-swatch:hover {
    transform: scale(1.15);
    border-color: var(--text-muted);
    box-shadow: none;
    background: inherit;
  }

  .color-swatch.active {
    border-color: var(--text-primary);
    box-shadow:
      0 0 0 2px var(--bg-primary),
      0 0 0 4px currentColor;
  }

  .hex-input {
    margin-top: 8px;
    width: 100px;
    font-family: var(--font-mono);
    font-size: 13px;
  }
</style>
