<script lang="ts">
  import GeneralSettings from "./settings/GeneralSettings.svelte";
  import AppearanceSettings from "./settings/AppearanceSettings.svelte";
  import EditorSettings from "./settings/EditorSettings.svelte";
  import KeyboardSettings from "./settings/KeyboardSettings.svelte";

  const { onclose }: { onclose: () => void } = $props();

  const categories = [
    { id: "general", label: "General" },
    { id: "appearance", label: "Appearance" },
    { id: "editor", label: "Editor" },
    { id: "shortcuts", label: "Shortcuts" },
  ] as const;

  let activeCategory = $state<string>("general");
</script>

<div
  class="settings-overlay"
  role="dialog"
  tabindex="-1"
  onclick={onclose}
  onkeydown={(e) => e.key === "Escape" && onclose()}
>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="settings-modal"
    role="document"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="settings-header">
      <h2>Settings</h2>
      <button onclick={onclose} class="close-btn">✕</button>
    </div>
    <div class="settings-body">
      <nav class="settings-sidebar">
        {#each categories as cat (cat.id)}
          <button
            class="sidebar-item"
            class:active={activeCategory === cat.id}
            onclick={() => (activeCategory = cat.id)}
          >
            {cat.label}
          </button>
        {/each}
      </nav>
      <div class="settings-panel">
        {#if activeCategory === "general"}
          <GeneralSettings />
        {:else if activeCategory === "appearance"}
          <AppearanceSettings />
        {:else if activeCategory === "editor"}
          <EditorSettings />
        {:else if activeCategory === "shortcuts"}
          <KeyboardSettings />
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .settings-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    animation: fadeIn 0.15s ease;
  }

  .settings-modal {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    width: 620px;
    max-width: 90vw;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-elevated);
  }

  .settings-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .settings-header h2 {
    font-size: 16px;
    font-weight: 500;
    letter-spacing: 0.3px;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 18px;
    cursor: pointer;
    padding: 4px 8px;
  }
  .close-btn:hover {
    color: var(--text-primary);
  }

  .settings-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .settings-sidebar {
    width: 160px;
    border-right: 1px solid var(--border);
    padding: 12px 0;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .sidebar-item {
    padding: 8px 20px;
    font-size: 13px;
    text-align: left;
    background: none;
    border: none;
    border-radius: 0;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .sidebar-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .sidebar-item.active {
    color: var(--accent-primary);
    background: color-mix(in srgb, var(--accent-primary) 8%, transparent);
    font-weight: 500;
  }

  .settings-panel {
    flex: 1;
    padding: 20px;
    overflow-y: auto;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
