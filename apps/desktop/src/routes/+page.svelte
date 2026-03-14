<script lang="ts">
  import { goto } from "$app/navigation";
  import { api, type EngineInfo } from "$lib/api";
  import { currentEngine, currentGameDir, setStatus, addToast } from "$lib/stores";
  import SaveFolderPicker from "$lib/components/SaveFolderPicker.svelte";
  import SettingsModal from "$lib/components/SettingsModal.svelte";
  import { preferencesStore, sortEngines, setEngineOrder } from "$lib/preferences";
  import { getEngineDefaults } from "$lib/engine-config";

  let engines = $state<EngineInfo[]>([]);
  let loading = $state(true);
  let showSettings = $state(false);
  let reorderMode = $state(false);

  // Drag state (only active in reorder mode)
  let dragIdx = $state<number | null>(null);
  let dropIdx = $state<number | null>(null);
  let pointerStartX = 0;
  let pointerStartY = 0;
  let ghostOffsetX = 0;
  let ghostOffsetY = 0;
  let isDragging = $state(false);
  let suppressClick = $state(false);
  let ghostEl: HTMLElement | null = null;

  // Engine logos: static/engines/{iconKey}.svg
  const engineIconKey: Record<string, string> = {
    "rpg-maker-mv": "rpg-maker-mv",
    "rpg-maker-vx-ace": "rpg-maker-vx-ace",
    "pixel-game-maker-mv": "pixel-game-maker-mv",
    renpy: "renpy",
    "wolf-rpg-editor": "wolf-rpg-editor",
    flash: "flash",
    "unreal-engine": "unreal-engine",
    sugarcube: "sugarcube",
    sqlite: "sqlite",
    unity: "unity",
  };

  const engineEmojis: Record<string, string> = {
    "rpg-maker-mv": "🎮",
    "rpg-maker-vx-ace": "⚔️",
    "pixel-game-maker-mv": "🕹️",
    renpy: "📖",
    "wolf-rpg-editor": "🐺",
    flash: "⚡",
    "unreal-engine": "🔷",
    sugarcube: "🍬",
    sqlite: "🗄️",
    unity: "🎮",
  };

  const logoFailed = $state<Record<string, boolean>>({});

  const engineColors: Record<string, string> = {
    "rpg-maker-mv": "#4fc3f7",
    "rpg-maker-vx-ace": "#66bb6a",
    "pixel-game-maker-mv": "#ff7043",
    renpy: "#ce93d8",
    "wolf-rpg-editor": "#ff9800",
    flash: "#f44336",
    "unreal-engine": "#1565c0",
    sugarcube: "#8b5cf6",
    sqlite: "#003b57",
    unity: "#222c37",
  };

  function getLastPickerDir(engineId: string): string | null {
    try {
      return localStorage.getItem(`picker_dir_${engineId}`);
    } catch {
      return null;
    }
  }
  function setLastPickerDir(engineId: string, dir: string) {
    try {
      localStorage.setItem(`picker_dir_${engineId}`, dir);
    } catch {
      /* ignore */
    }
  }

  function getPickerConfig(engine: EngineInfo) {
    const engineDefaults = getEngineDefaults();

    const config = engineDefaults[engine.id] ?? {
      extension: engine.save_extensions[0] ?? "sav",
      defaultDir: "%USERPROFILE%\\Downloads",
      badgeColor: "#6c5ce7",
      title: "Select Save Folder",
    };

    // Use persisted last-used directory if available, then fall back to Downloads
    const lastDir = getLastPickerDir(engine.id);
    if (lastDir) config.defaultDir = lastDir;
    config.defaultDir ??= "%USERPROFILE%\\Downloads";

    return config;
  }

  const debugInfo: Record<string, { keys: string[]; note: string }> = {
    "rpg-maker-mv": {
      keys: ["F9 — Variable/Switch editor", "F8 — Console"],
      note: "Injects a debug plugin into js/plugins/",
    },
    "rpg-maker-vx-ace": {
      keys: ["F9 — Variable/Switch editor", "F8 — Console"],
      note: "Creates a shortcut that launches with test mode",
    },
    renpy: {
      keys: ["Shift+O — Developer console", "Shift+R — Reload game"],
      note: "Adds a config patch to game/",
    },
    "wolf-rpg-editor": {
      keys: ["F3 — Debug window (inspector)", "F10 — Pause game"],
      note: "Creates a shortcut that launches with debug flags",
    },
  };

  $effect(() => {
    void loadEngines();
  });

  async function loadEngines() {
    try {
      engines = sortEngines(await api.listEngines());
    } catch (e) {
      addToast(`Failed to load engines: ${e}`, "error");
    } finally {
      loading = false;
    }
  }

  let detectingFolder = $state(false);
  let flashPickerEngine = $state<EngineInfo | null>(null);

  async function finishSetup(engine: EngineInfo, gameDir: string) {
    await api.setGame(engine.id, gameDir);
    currentEngine.set(engine);
    currentGameDir.set(gameDir);
    setStatus(`Loaded ${engine.name} — ${gameDir}`, "success");
    void goto("/editor");
  }

  async function autoDetect() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: true,
        title: "Select game folder",
      });
      if (!selected) return;

      const gameDir = String(selected);
      detectingFolder = true;

      const detected = await api.detectEngine(gameDir);
      if (!detected) {
        addToast("Could not auto-detect engine. Try selecting one manually.", "error");
        detectingFolder = false;
        return;
      }

      if (detected.save_dir_hint) {
        // Show the Flash save picker instead of going directly to editor
        flashPickerEngine = detected;
        detectingFolder = false;
        return;
      }

      await finishSetup(detected, gameDir);
    } catch (e) {
      addToast(`Error: ${e}`, "error");
    } finally {
      detectingFolder = false;
    }
  }

  async function selectEngine(engine: EngineInfo) {
    try {
      if (engine.save_dir_hint) {
        flashPickerEngine = engine;
        return;
      }

      const { open } = await import("@tauri-apps/plugin-dialog");

      if (engine.pick_mode === "file") {
        const selected = await open({
          title: `Select ${engine.name} save file`,
          filters: engine.save_extensions.length
            ? [{ name: "Save files", extensions: engine.save_extensions }]
            : [],
        });
        if (!selected) return;
        const filePath = String(selected);
        const lastSep = Math.max(filePath.lastIndexOf("/"), filePath.lastIndexOf("\\"));
        const gameDir = lastSep > 0 ? filePath.substring(0, lastSep) : filePath;
        await finishSetup(engine, gameDir);
        return;
      }

      const selected = await open({
        directory: true,
        title: `Select ${engine.name} game folder`,
      });
      if (!selected) return;

      await finishSetup(engine, String(selected));
    } catch (e) {
      addToast(`Error: ${e}`, "error");
    }
  }

  function handlePointerDown(e: PointerEvent, idx: number) {
    if (!reorderMode) return;
    e.preventDefault();
    pointerStartX = e.clientX;
    pointerStartY = e.clientY;
    dragIdx = idx;
    isDragging = false;

    // Store offset from pointer to card top-left so ghost doesn't snap
    const target = e.currentTarget;
    if (!(target instanceof HTMLElement)) return;
    const rect = target.getBoundingClientRect();
    ghostOffsetX = e.clientX - rect.left;
    ghostOffsetY = e.clientY - rect.top;

    document.addEventListener("pointermove", handleDocPointerMove);
    document.addEventListener("pointerup", handleDocPointerUp);
  }

  function createGhost(sourceEl: HTMLElement, x: number, y: number) {
    const rect = sourceEl.getBoundingClientRect();
    const cloned = sourceEl.cloneNode(true);
    if (!(cloned instanceof HTMLElement)) return;
    ghostEl = cloned;
    ghostEl.style.position = "fixed";
    ghostEl.style.width = rect.width + "px";
    ghostEl.style.height = rect.height + "px";
    ghostEl.style.left = x - ghostOffsetX + "px";
    ghostEl.style.top = y - ghostOffsetY + "px";
    ghostEl.style.pointerEvents = "none";
    ghostEl.style.zIndex = "1000";
    ghostEl.style.opacity = "0.85";
    ghostEl.style.transform = "scale(1.04) rotate(1.5deg)";
    ghostEl.style.boxShadow = "0 12px 32px rgba(0,0,0,0.35)";
    ghostEl.style.transition = "none";
    ghostEl.style.cursor = "grabbing";
    document.body.appendChild(ghostEl);
  }

  function handleDocPointerMove(e: PointerEvent) {
    if (dragIdx === null) return;
    const dx = e.clientX - pointerStartX;
    const dy = e.clientY - pointerStartY;

    if (!isDragging && Math.abs(dx) + Math.abs(dy) > 8) {
      isDragging = true;
      // Find the source card element and create ghost from it
      const sourceCard = document.querySelector<HTMLElement>(`[data-engine-idx="${dragIdx}"]`);
      if (sourceCard) createGhost(sourceCard, e.clientX, e.clientY);
    }
    if (!isDragging) return;

    // Move ghost to follow cursor
    if (ghostEl) {
      ghostEl.style.left = e.clientX - ghostOffsetX + "px";
      ghostEl.style.top = e.clientY - ghostOffsetY + "px";
    }

    // Detect drop target (ghost has pointerEvents:none so elementFromPoint sees through it)
    const el = document.elementFromPoint(e.clientX, e.clientY);
    const card = el?.closest<HTMLElement>("[data-engine-idx]") ?? null;
    dropIdx = card ? Number(card.dataset.engineIdx) : null;
  }

  function handleDocPointerUp() {
    // Remove ghost
    if (ghostEl) {
      ghostEl.remove();
      ghostEl = null;
    }
    document.removeEventListener("pointermove", handleDocPointerMove);
    document.removeEventListener("pointerup", handleDocPointerUp);

    if (isDragging && dragIdx !== null && dropIdx !== null && dragIdx !== dropIdx) {
      const reordered = [...engines];
      const [moved] = reordered.splice(dragIdx, 1);
      reordered.splice(dropIdx, 0, moved);
      engines = reordered;
      setEngineOrder(reordered.map((en) => en.id));
      suppressClick = true;
    }
    dragIdx = null;
    dropIdx = null;
    isDragging = false;
  }

  async function resetEngineOrder() {
    setEngineOrder([]);
    engines = await api.listEngines();
  }

  async function onFlashSaveSelected(path: string) {
    const engine = flashPickerEngine!;
    flashPickerEngine = null;
    setLastPickerDir(engine.id, path);
    try {
      await finishSetup(engine, path);
    } catch (e) {
      addToast(`Error: ${e}`, "error");
    }
  }
</script>

<div class="engine-selector">
  <div class="header">
    <img src="/logo.svg" alt="Prismatic" class="app-logo" />
    <p class="subtitle">Select your game engine to get started</p>
    <button class="settings-btn" onclick={() => (showSettings = true)} title="Settings">⚙</button>
  </div>

  {#if loading}
    <div class="loading">Loading engines...</div>
  {:else}
    <div class="grid-toolbar">
      {#if reorderMode}
        <button class="toolbar-btn" onclick={resetEngineOrder}>Reset to Default</button>
        <button class="toolbar-btn toolbar-btn-primary" onclick={() => (reorderMode = false)}
          >Done</button
        >
      {:else}
        <button class="toolbar-btn" onclick={() => (reorderMode = true)}>Reorder</button>
      {/if}
    </div>
    <div class="engine-grid" class:reorder-mode={reorderMode}>
      <button
        class="engine-card auto-detect-card"
        class:reorder-locked={reorderMode}
        style="--engine-color: #ffd54f"
        onclick={() => !reorderMode && autoDetect()}
        disabled={detectingFolder || reorderMode}
      >
        <div class="engine-icon">
          <span class="engine-emoji">{detectingFolder ? "⏳" : "📂"}</span>
        </div>
        <div class="engine-name">
          {detectingFolder ? "Detecting..." : "Auto-Detect"}
        </div>
        {#if !reorderMode}
          <div class="engine-desc">Select a game folder and automatically detect the engine</div>
        {/if}
      </button>
      {#each engines as engine, idx (engine.id)}
        <button
          class="engine-card"
          class:drag-over={reorderMode && dropIdx === idx && dragIdx !== idx}
          class:dragging={reorderMode && dragIdx === idx && isDragging}
          style="--engine-color: {engineColors[engine.id] || '#6c5ce7'}"
          data-engine-idx={idx}
          onclick={() => {
            if (reorderMode) return;
            if (suppressClick) {
              suppressClick = false;
              return;
            }
            void selectEngine(engine);
          }}
          onpointerdown={reorderMode ? (e) => handlePointerDown(e, idx) : undefined}
        >
          <div
            class="engine-icon"
            class:engine-icon-banner={engine.id === "wolf-rpg-editor" ||
              engine.id === "pixel-game-maker-mv"}
          >
            {#if logoFailed[engine.id]}
              <span class="engine-emoji">{engineEmojis[engine.id] || "🎲"}</span>
            {:else}
              <img
                src="/engines/{engineIconKey[engine.id] || engine.id}.svg"
                alt={engine.name}
                onerror={() => (logoFailed[engine.id] = true)}
              />
            {/if}
          </div>
          <div class="engine-name">{engine.name}</div>
          {#if !reorderMode}
            <div class="engine-desc">{engine.description}</div>
            <div class="engine-meta">
              {#if engine.supports_debug}
                <span class="badge" style="background: var(--success); color: white;"
                  >Debug Mode</span
                >
              {/if}
              <span
                class="badge"
                style="background: var(--bg-tertiary); color: var(--text-secondary);"
              >
                {engine.save_extensions.map((e) => `.${e}`).join(", ")}
              </span>
            </div>
            {#if engine.supports_debug && debugInfo[engine.id]}
              <div class="debug-details">
                <div class="debug-keys">
                  {#each debugInfo[engine.id].keys as key (key)}
                    <span class="debug-key">{key}</span>
                  {/each}
                </div>
                <div class="debug-note">{debugInfo[engine.id].note}</div>
              </div>
            {/if}
          {/if}
          {#if reorderMode}
            <span class="drag-hint">Drag to reorder</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

{#if flashPickerEngine}
  {@const pickerConfig = getPickerConfig(flashPickerEngine)}
  <SaveFolderPicker
    onselect={onFlashSaveSelected}
    oncancel={() => (flashPickerEngine = null)}
    title={pickerConfig.title}
    hint={flashPickerEngine.save_dir_hint ?? "Navigate to the folder containing your save files."}
    extension={pickerConfig.extension}
    defaultDir={pickerConfig.defaultDir}
    badgeColor={pickerConfig.badgeColor}
    deepScanDefault={$preferencesStore.deepScanDefault}
  />
{/if}

{#if showSettings}
  <SettingsModal onclose={() => (showSettings = false)} />
{/if}

<style>
  .engine-selector {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100%;
    padding: 40px;
  }

  .header {
    text-align: center;
    margin-bottom: 48px;
    position: relative;
  }

  .settings-btn {
    position: absolute;
    top: 0;
    right: -60px;
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-muted);
    font-size: 20px;
    width: 40px;
    height: 40px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
  }
  .settings-btn:hover {
    color: var(--text-primary);
    border-color: var(--text-secondary);
  }

  .app-logo {
    height: 128px;
    margin-bottom: 8px;
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 16px;
  }

  .loading {
    color: var(--text-muted);
    font-size: 16px;
  }

  .engine-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 20px;
    max-width: 900px;
    width: 100%;
  }

  .grid-toolbar {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    max-width: 900px;
    width: 100%;
    margin-bottom: 12px;
  }

  .toolbar-btn {
    padding: 6px 14px;
    font-size: 13px;
    border-radius: var(--radius);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .toolbar-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .toolbar-btn-primary {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
    color: white;
  }

  .toolbar-btn-primary:hover {
    opacity: 0.9;
  }

  .engine-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 28px;
    text-align: center;
    cursor: pointer;
    transition: all 0.3s ease;
    position: relative;
    overflow: hidden;
    box-shadow: var(--shadow-card);
  }

  .engine-card::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: var(--engine-color);
    opacity: 1;
  }

  .engine-card:hover {
    border-color: var(--engine-color);
    transform: translateY(-2px);
    box-shadow: var(--shadow-elevated);
  }

  .engine-card:hover::before {
    height: 2px;
  }

  /* Reorder mode styles */
  .reorder-mode .engine-card {
    cursor: grab;
    touch-action: none;
    user-select: none;
  }

  .reorder-mode .engine-card:hover {
    transform: none;
    border-color: var(--border);
    box-shadow: var(--shadow-card);
  }

  .reorder-mode .engine-card:active {
    cursor: grabbing;
  }

  .engine-card.dragging {
    opacity: 0.25;
    background: var(--bg-tertiary);
    border-style: dashed;
    border-color: var(--text-muted);
    box-shadow: none;
    cursor: grabbing;
  }

  .engine-card.dragging * {
    visibility: hidden;
  }

  .engine-card.drag-over {
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 2px var(--accent-primary);
    background: color-mix(in srgb, var(--accent-primary) 8%, var(--bg-card));
    transform: scale(1.03);
  }

  .reorder-locked {
    opacity: 0.4;
    cursor: default !important;
    pointer-events: none;
  }

  .drag-hint {
    display: block;
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 8px;
  }

  .engine-icon {
    margin-bottom: 12px;
    height: 64px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .engine-icon img {
    height: 64px;
    width: 64px;
    object-fit: contain;
  }

  .engine-icon-banner {
    height: auto;
  }

  .engine-icon-banner img {
    width: 100%;
    height: auto;
    max-height: 64px;
  }

  .engine-emoji {
    font-size: 64px;
    line-height: 1;
  }

  .engine-name {
    font-size: 18px;
    font-weight: 600;
    margin-bottom: 6px;
  }

  .engine-desc {
    color: var(--text-secondary);
    font-size: 13px;
    margin-bottom: 14px;
  }

  .engine-meta {
    display: flex;
    gap: 6px;
    justify-content: center;
    flex-wrap: wrap;
  }

  .debug-details {
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
    text-align: left;
  }

  .debug-keys {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-bottom: 6px;
  }

  .debug-key {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    padding: 2px 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    display: inline-block;
  }

  .debug-note {
    font-size: 11px;
    color: var(--text-muted);
    font-style: italic;
  }
</style>
