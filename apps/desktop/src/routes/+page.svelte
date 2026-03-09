<script lang="ts">
  import { goto } from '$app/navigation';
  import { api, type EngineInfo } from '$lib/api';
  import { currentEngine, currentGameDir, statusMessage, addToast } from '$lib/stores';

  let engines = $state<EngineInfo[]>([]);
  let loading = $state(true);

  const engineIcons: Record<string, string> = {
    'rpg-maker-mv': '🎮',
    'pixel-game-maker-mv': '🕹️',
    'renpy': '📖',
  };

  const engineColors: Record<string, string> = {
    'rpg-maker-mv': '#4fc3f7',
    'pixel-game-maker-mv': '#ff7043',
    'renpy': '#ce93d8',
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

  async function selectEngine(engine: EngineInfo) {
    try {
      // Open folder picker
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({ directory: true, title: `Select ${engine.name} game folder` });

      if (!selected) return;

      const gameDir = selected as string;

      // Try auto-detect first
      const detected = await api.detectEngine(gameDir);
      const finalEngine = detected || engine;

      await api.setGame(finalEngine.id, gameDir);
      currentEngine.set(finalEngine);
      currentGameDir.set(gameDir);
      statusMessage.set(`${finalEngine.name} — ${gameDir}`);
      addToast(`Loaded ${finalEngine.name} game`, 'success');

      goto('/editor');
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
      {#each engines as engine}
        <button
          class="engine-card"
          style="--engine-color: {engineColors[engine.id] || '#6c5ce7'}"
          onclick={() => selectEngine(engine)}
        >
          <div class="engine-icon">{engineIcons[engine.id] || '🎲'}</div>
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
        </button>
      {/each}
    </div>
  {/if}
</div>

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
    font-size: 48px;
    margin-bottom: 12px;
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
</style>
