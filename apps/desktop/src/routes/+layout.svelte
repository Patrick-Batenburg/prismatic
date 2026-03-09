<script lang="ts">
  import '../app.css';
  import { toasts } from '$lib/stores';
  import { statusMessage } from '$lib/stores';

  let { children } = $props();
  let toastList = $derived($toasts);
  let status = $derived($statusMessage);
</script>

<div class="app-shell">
  <main class="app-content">
    {@render children()}
  </main>

  <footer class="status-bar">
    <span>{status}</span>
  </footer>

  <!-- Toast notifications -->
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
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    padding: 0 12px;
    font-size: 11px;
    color: var(--text-muted);
    flex-shrink: 0;
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

  .toast-success { background: var(--success); color: white; }
  .toast-error { background: var(--danger); color: white; }
  .toast-info { background: var(--bg-card); border: 1px solid var(--border); color: var(--text-primary); }

  @keyframes slideIn {
    from { transform: translateX(100%); opacity: 0; }
    to { transform: translateX(0); opacity: 1; }
  }
</style>
