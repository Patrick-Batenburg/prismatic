<script lang="ts">
  import "../app.css";
  import {
    toasts,
    statusFlash,
    currentEngine,
    currentSavePath,
    modifiedFields,
    history,
  } from "$lib/stores";
  import { preferencesStore } from "$lib/preferences";

  const FONT_STACKS: Record<string, string> = {
    "system-default": '-apple-system, "Segoe UI", system-ui, sans-serif',
    Inter: '"Inter", system-ui, sans-serif',
    "Segoe UI": '"Segoe UI", system-ui, sans-serif',
    Roboto: '"Roboto", system-ui, sans-serif',
    "Noto Sans": '"Noto Sans", system-ui, sans-serif',
  };

  const MONO_STACKS: Record<string, string> = {
    "Cascadia Code": '"Cascadia Code", "Fira Code", "Consolas", monospace',
    "Fira Code": '"Fira Code", "Cascadia Code", "Consolas", monospace',
    "JetBrains Mono": '"JetBrains Mono", "Fira Code", "Consolas", monospace',
    Consolas: '"Consolas", "Courier New", monospace',
    "Source Code Pro": '"Source Code Pro", "Fira Code", "Consolas", monospace',
  };

  // Apply appearance preferences to <html>
  $effect(() => {
    const prefs = $preferencesStore;
    const html = document.documentElement;

    // Theme
    html.classList.toggle("light", prefs.theme === "light");

    // Accent color
    html.style.setProperty("--accent-primary", prefs.accentColor);
    html.style.setProperty(
      "--accent-secondary",
      `color-mix(in srgb, ${prefs.accentColor} 80%, white)`,
    );
    html.style.setProperty(
      "--accent-glow",
      `color-mix(in srgb, ${prefs.accentColor} 20%, transparent)`,
    );
    html.style.setProperty("--border-focus", prefs.accentColor);

    // Fonts
    const fontStack = FONT_STACKS[prefs.appFont] ?? FONT_STACKS["system-default"];
    html.style.setProperty("font-family", fontStack);
    const monoStack = MONO_STACKS[prefs.monoFont] ?? MONO_STACKS["Cascadia Code"];
    html.style.setProperty("--font-mono", monoStack);
  });

  // Apply editor preferences
  $effect(() => {
    const prefs = $preferencesStore;
    history.setMaxDepth(prefs.undoHistoryDepth);
  });

  let { children } = $props();
  let toastList = $derived($toasts);
  let flash = $derived($statusFlash);

  let baseStatus = $derived(
    (() => {
      const engine = $currentEngine;
      const savePath = $currentSavePath;
      const modified = $modifiedFields;

      if (!engine) return "Select a game engine to get started";

      const saveName = savePath?.split(/[/\\]/).pop();
      if (!saveName) return `${engine.name} — No save loaded`;

      const modCount = modified.size;
      if (modCount > 0)
        return `${engine.name} — ${saveName} — ${modCount} unsaved change${modCount === 1 ? "" : "s"}`;

      return `${engine.name} — ${saveName}`;
    })(),
  );

  let statusText = $derived(flash?.text ?? baseStatus);
  let statusType = $derived(flash?.type ?? "idle");
</script>

<div class="app-shell">
  <main class="app-content">
    {@render children()}
  </main>

  <footer class="status-bar">
    <span class="status-text status-{statusType}">{statusText}</span>
  </footer>

  <!-- Toast notifications (errors only) -->
  {#if toastList.length > 0}
    <div class="toast-container">
      {#each toastList as toast (toast.id)}
        <div class="toast toast-{toast.type}">
          {toast.message}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .app-content {
    flex: 1;
    overflow: auto;
  }

  .status-bar {
    height: 24px;
    background: var(--bg-secondary);
    border-top: none;
    display: flex;
    align-items: center;
    padding: 0 12px;
    font-size: 11px;
    color: var(--text-muted);
    flex-shrink: 0;
    position: relative;
  }
  .status-text.status-success {
    color: var(--success);
  }
  .status-text.status-error {
    color: var(--danger);
  }
  .status-text.status-info {
    color: var(--accent-primary);
  }
  .status-bar::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: var(--border-spectral);
  }

  .toast-container {
    position: fixed;
    top: 16px;
    right: 16px;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .toast {
    padding: 10px 16px;
    border-radius: var(--radius);
    font-size: 13px;
    animation: slideIn 0.3s ease;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    max-width: 400px;
  }

  .toast-success {
    background: var(--success);
    color: white;
  }
  .toast-error {
    background: var(--danger);
    color: white;
  }
  .toast-info {
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-primary);
  }

  @keyframes slideIn {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }
</style>
