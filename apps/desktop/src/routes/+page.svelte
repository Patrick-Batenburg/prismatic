<script lang="ts">
  import { goto } from '$app/navigation';
  import { api, type EngineInfo } from '$lib/api';
  import { currentEngine, currentGameDir, statusMessage, addToast } from '$lib/stores';
  import SaveFolderPicker from '$lib/components/SaveFolderPicker.svelte';

  let engines = $state<EngineInfo[]>([]);
  let loading = $state(true);

  // Engine logos: static/engines/{iconKey}.svg
  const engineIconKey: Record<string, string> = {
    'rpg-maker-mv': 'rpg-maker-mv',
    'rpg-maker-vx-ace': 'rpg-maker-vx-ace',
    'pixel-game-maker-mv': 'pixel-game-maker-mv',
    'renpy': 'renpy',
    'wolf-rpg-editor': 'wolf-rpg-editor',
    'flash': 'flash',
    'unreal-engine': 'unreal-engine',
    'sugarcube': 'sugarcube',
  };

  const engineEmojis: Record<string, string> = {
    'rpg-maker-mv': '🎮',
    'rpg-maker-vx-ace': '⚔️',
    'pixel-game-maker-mv': '🕹️',
    'renpy': '📖',
    'wolf-rpg-editor': '🐺',
    'flash': '⚡',
    'unreal-engine': '🔷',
    'sugarcube': '🍬',
  };

  let logoFailed = $state<Record<string, boolean>>({});

  const engineColors: Record<string, string> = {
    'rpg-maker-mv': '#4fc3f7',
    'rpg-maker-vx-ace': '#66bb6a',
    'pixel-game-maker-mv': '#ff7043',
    'renpy': '#ce93d8',
    'wolf-rpg-editor': '#ff9800',
    'flash': '#f44336',
    'unreal-engine': '#1565c0',
    'sugarcube': '#8b5cf6',
  };

  function getPickerConfig(engine: EngineInfo) {
    switch (engine.id) {
      case 'flash':
        return { extension: 'sol', defaultDir: '%APPDATA%/Macromedia/Flash Player/#SharedObjects' as string | null, badgeColor: '#f44336', title: 'Select Flash Save Folder' };
      case 'unreal-engine':
        return { extension: 'sav', defaultDir: '%LOCALAPPDATA%' as string | null, badgeColor: '#1565c0', title: 'Select Unreal Engine Save Folder' };
      case 'sugarcube':
        return { extension: 'save', defaultDir: '%USERPROFILE%/Downloads' as string | null, badgeColor: '#8b5cf6', title: 'Select SugarCube Save Folder' };
      default:
        return { extension: 'sav', defaultDir: null as string | null, badgeColor: '#6c5ce7', title: 'Select Save Folder' };
    }
  }

  const debugInfo: Record<string, { keys: string[]; note: string }> = {
    'rpg-maker-mv': {
      keys: ['F9 — Variable/Switch editor', 'F8 — Console'],
      note: 'Injects a debug plugin into js/plugins/',
    },
    'rpg-maker-vx-ace': {
      keys: ['F9 — Variable/Switch editor', 'F8 — Console'],
      note: 'Creates a shortcut that launches with test mode',
    },
    'renpy': {
      keys: ['Shift+O — Developer console', 'Shift+R — Reload game'],
      note: 'Adds a config patch to game/',
    },
    'wolf-rpg-editor': {
      keys: ['F3 — Debug window (inspector)', 'F10 — Pause game'],
      note: 'Creates a shortcut that launches with debug flags',
    },
  };

  $effect(() => {
    loadEngines();
  });

  async function loadEngines() {
    try {
      engines = await api.listEngines();
    } catch (e) {
      addToast(`Failed to load engines: ${e}`, 'error');
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
    statusMessage.set(`${engine.name} — ${gameDir}`);
    addToast(`Loaded ${engine.name}`, 'success');
    goto('/editor');
  }

  async function autoDetect() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({ directory: true, title: 'Select game folder' });
      if (!selected) return;

      const gameDir = selected as string;
      detectingFolder = true;

      const detected = await api.detectEngine(gameDir);
      if (!detected) {
        addToast('Could not auto-detect engine. Try selecting one manually.', 'error');
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
      addToast(`Error: ${e}`, 'error');
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

      const { open } = await import('@tauri-apps/plugin-dialog');

      if (engine.pick_mode === 'file') {
        const selected = await open({
          title: `Select ${engine.name} save file`,
          filters: engine.save_extensions.length
            ? [{ name: 'Save files', extensions: engine.save_extensions }]
            : [],
        });
        if (!selected) return;
        const filePath = (selected as { path: string }).path ?? (selected as string);
        const lastSep = Math.max(filePath.lastIndexOf('/'), filePath.lastIndexOf('\\'));
        const gameDir = lastSep > 0 ? filePath.substring(0, lastSep) : filePath;
        await finishSetup(engine, gameDir);
        return;
      }

      const selected = await open({ directory: true, title: `Select ${engine.name} game folder` });
      if (!selected) return;

      await finishSetup(engine, selected as string);
    } catch (e) {
      addToast(`Error: ${e}`, 'error');
    }
  }

  async function onFlashSaveSelected(path: string) {
    const engine = flashPickerEngine!;
    flashPickerEngine = null;
    try {
      await finishSetup(engine, path);
    } catch (e) {
      addToast(`Error: ${e}`, 'error');
    }
  }
</script>

<div class="engine-selector">
  <div class="header">
    <h1 class="title">Prismatic</h1>
    <p class="subtitle">Select your game engine to get started</p>
  </div>

  {#if loading}
    <div class="loading">Loading engines...</div>
  {:else}
    <div class="engine-grid">
      <button
        class="engine-card auto-detect-card"
        style="--engine-color: #ffd54f"
        onclick={() => autoDetect()}
        disabled={detectingFolder}
      >
        <div class="engine-icon"><span class="engine-emoji">{detectingFolder ? '⏳' : '📂'}</span></div>
        <div class="engine-name">{detectingFolder ? 'Detecting...' : 'Auto-Detect'}</div>
        <div class="engine-desc">Select a game folder and automatically detect the engine</div>
      </button>
      {#each engines as engine}
        <button
          class="engine-card"
          style="--engine-color: {engineColors[engine.id] || '#6c5ce7'}"
          onclick={() => selectEngine(engine)}
        >
          <div class="engine-icon">
            {#if logoFailed[engine.id]}
              <span class="engine-emoji">{engineEmojis[engine.id] || '🎲'}</span>
            {:else}
              <img
                src="/engines/{engineIconKey[engine.id] || engine.id}.svg"
                alt={engine.name}
                onerror={() => logoFailed[engine.id] = true}
              />
            {/if}
          </div>
          <div class="engine-name">{engine.name}</div>
          <div class="engine-desc">{engine.description}</div>
          <div class="engine-meta">
            {#if engine.supports_debug}
              <span class="badge" style="background: var(--success); color: white;">Debug Mode</span>
            {/if}
            <span class="badge" style="background: var(--bg-tertiary); color: var(--text-secondary);">
              {engine.save_extensions.map(e => `.${e}`).join(', ')}
            </span>
          </div>
          {#if engine.supports_debug && debugInfo[engine.id]}
            <div class="debug-details">
              <div class="debug-keys">
                {#each debugInfo[engine.id].keys as key}
                  <span class="debug-key">{key}</span>
                {/each}
              </div>
              <div class="debug-note">{debugInfo[engine.id].note}</div>
            </div>
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
    oncancel={() => flashPickerEngine = null}
    title={pickerConfig.title}
    hint={flashPickerEngine.save_dir_hint ?? 'Navigate to the folder containing your save files.'}
    extension={pickerConfig.extension}
    defaultDir={pickerConfig.defaultDir}
    badgeColor={pickerConfig.badgeColor}
  />
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
  }

  .title {
    font-size: 32px;
    font-weight: 700;
    background: linear-gradient(135deg, var(--accent-primary), var(--accent-secondary));
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
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
  }

  .engine-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: var(--engine-color);
    opacity: 0;
    transition: opacity 0.3s;
  }

  .engine-card:hover {
    border-color: var(--engine-color);
    transform: translateY(-4px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3), 0 0 20px color-mix(in srgb, var(--engine-color) 20%, transparent);
  }

  .engine-card:hover::before {
    opacity: 1;
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
    font-family: monospace;
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
