<script lang="ts">
  import { preferencesStore, setPreferences, DEFAULT_KEYBINDINGS } from "$lib/preferences";
  import { eventToCombo, isReserved, ACTION_LABELS } from "$lib/keybindings";

  let prefs = $derived($preferencesStore);
  let bindings = $derived(prefs.keybindings);

  let recordingAction = $state<string | null>(null);
  let reservedWarning = $state(false);
  let lastAttempt = $state("");

  // Find conflicts: actions sharing the same combo
  let conflicts = $derived(
    (() => {
      // eslint-disable-next-line svelte/prefer-svelte-reactivity -- pure derived computation
      const map = new Map<string, string[]>();
      for (const [action, combo] of Object.entries(bindings)) {
        const list = map.get(combo) ?? [];
        list.push(action);
        map.set(combo, list);
      }
      // eslint-disable-next-line svelte/prefer-svelte-reactivity -- pure derived computation
      const result = new Set<string>();
      for (const actions of map.values()) {
        if (actions.length > 1) actions.forEach((a) => result.add(a));
      }
      return result;
    })(),
  );

  function startRecording(action: string) {
    recordingAction = action;
    reservedWarning = false;
    lastAttempt = "";
  }

  function cancelRecording() {
    recordingAction = null;
    reservedWarning = false;
    lastAttempt = "";
  }

  function handleRecordKey(e: KeyboardEvent) {
    // Block ALL keyboard events while recording
    e.preventDefault();
    e.stopPropagation();
    e.stopImmediatePropagation();

    if (e.key === "Escape") {
      cancelRecording();
      return;
    }

    const combo = eventToCombo(e);
    if (!combo) return;

    lastAttempt = combo;

    if (isReserved(combo)) {
      reservedWarning = true;
      return;
    }

    reservedWarning = false;
    setPreferences({
      keybindings: { ...bindings, [recordingAction!]: combo },
    });
    cancelRecording();
  }

  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) cancelRecording();
  }

  function focusOnMount(node: HTMLElement) {
    node.focus();
  }

  function resetBinding(action: string) {
    const def = DEFAULT_KEYBINDINGS[action];
    if (def) {
      setPreferences({
        keybindings: { ...bindings, [action]: def },
      });
    }
  }

  function resetAll() {
    setPreferences({ keybindings: { ...DEFAULT_KEYBINDINGS } });
  }
</script>

<div class="settings-section">
  <h3>Keyboard Shortcuts</h3>

  <div class="shortcut-table">
    {#each Object.entries(bindings) as [action, combo] (action)}
      <div class="shortcut-row">
        <span class="shortcut-label">{ACTION_LABELS[action] ?? action}</span>
        <div class="shortcut-control">
          <span
            class="keybinding-chip"
            class:conflict={conflicts.has(action)}
            onclick={() => startRecording(action)}
            onkeydown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                startRecording(action);
              }
            }}
            tabindex="0"
            role="button"
            title={conflicts.has(action)
              ? "Conflict: another action uses this binding"
              : "Click to rebind"}
          >
            {combo}
          </span>
          {#if combo !== DEFAULT_KEYBINDINGS[action]}
            <button
              class="reset-btn"
              title="Reset to default ({DEFAULT_KEYBINDINGS[action]})"
              onclick={() => resetBinding(action)}>✕</button
            >
          {/if}
        </div>
      </div>
    {/each}
  </div>

  <button class="reset-all-btn" onclick={resetAll}>Reset All to Defaults</button>
</div>

{#if recordingAction}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="record-overlay"
    onclick={handleOverlayClick}
    onkeydown={handleRecordKey}
    tabindex="-1"
    use:focusOnMount
  >
    <div class="record-modal">
      <div class="record-action">{ACTION_LABELS[recordingAction] ?? recordingAction}</div>
      <div class="record-prompt">
        {#if reservedWarning}
          <span class="record-reserved">{lastAttempt} is a reserved shortcut</span>
        {:else}
          Press a key combination...
        {/if}
      </div>
      <div class="record-current">
        Current: <span class="record-combo">{bindings[recordingAction]}</span>
      </div>
      <div class="record-hint">Press <kbd>Esc</kbd> or click outside to cancel</div>
    </div>
  </div>
{/if}

<style>
  .settings-section h3 {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 16px;
  }

  .shortcut-table {
    display: flex;
    flex-direction: column;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
  }

  .shortcut-row:last-child {
    border-bottom: none;
  }

  .shortcut-label {
    font-size: 14px;
    color: var(--text-primary);
  }

  .shortcut-control {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .keybinding-chip {
    display: inline-block;
    padding: 4px 12px;
    font-size: 12px;
    font-family: var(--font-mono);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
    min-width: 80px;
    text-align: center;
    user-select: none;
  }

  .keybinding-chip:hover {
    border-color: var(--accent-primary);
  }

  .keybinding-chip.conflict {
    border-color: var(--warning);
    background: color-mix(in srgb, var(--warning) 10%, transparent);
  }

  .reset-btn {
    padding: 2px 6px;
    font-size: 11px;
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
  }

  .reset-btn:hover {
    color: var(--danger);
    border-color: var(--danger);
  }

  .reset-all-btn {
    margin-top: 16px;
    font-size: 13px;
    color: var(--text-secondary);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    padding: 8px 16px;
    border-radius: var(--radius);
    cursor: pointer;
  }

  .reset-all-btn:hover {
    color: var(--text-primary);
    border-color: var(--text-secondary);
  }

  /* Recording overlay & modal */
  .record-overlay {
    position: fixed;
    inset: 0;
    z-index: 10000;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    backdrop-filter: blur(2px);
    outline: none;
  }

  .record-modal {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 28px 36px;
    min-width: 320px;
    text-align: center;
    box-shadow: var(--shadow-elevated);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }

  .record-action {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .record-prompt {
    font-size: 14px;
    color: var(--accent-primary);
    min-height: 22px;
  }

  .record-reserved {
    color: var(--danger);
  }

  .record-current {
    font-size: 12px;
    color: var(--text-muted);
  }

  .record-combo {
    font-family: var(--font-mono);
    background: var(--bg-tertiary);
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
  }

  .record-hint {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 4px;
  }

  .record-hint kbd {
    font-family: var(--font-mono);
    background: var(--bg-tertiary);
    padding: 1px 5px;
    border-radius: 3px;
    border: 1px solid var(--border);
    font-size: 10px;
  }
</style>
