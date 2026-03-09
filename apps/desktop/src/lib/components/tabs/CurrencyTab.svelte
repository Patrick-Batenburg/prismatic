<script lang="ts">
  import type { CurrencyInfo } from '$lib/api';
  import { markModified } from '$lib/stores';

  let { currency = $bindable() }: { currency: CurrencyInfo } = $props();
</script>

<div class="currency-card">
  <label class="currency-label">
    <span class="currency-icon">💰</span>
    <span class="currency-name">{currency.label}</span>
  </label>
  <input type="number" class="currency-input" bind:value={currency.amount}
    oninput={() => markModified('currency.amount')} />
</div>

<style>
  .currency-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 24px;
    display: flex;
    align-items: center;
    gap: 16px;
    max-width: 400px;
  }

  .currency-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 18px;
    font-weight: 600;
  }

  .currency-icon { font-size: 28px; }
  .currency-name { color: var(--text-primary); }

  .currency-input {
    font-size: 20px;
    font-weight: 600;
    padding: 8px 14px;
    width: 180px;
    text-align: right;
    background: var(--bg-input);
  }
</style>
