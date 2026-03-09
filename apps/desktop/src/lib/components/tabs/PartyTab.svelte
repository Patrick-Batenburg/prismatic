<script lang="ts">
  import type { Character } from '$lib/api';
  import { markModified } from '$lib/stores';

  let { party = $bindable() }: { party: Character[] } = $props();
</script>

<div class="party-grid">
  {#each party as char, idx}
    <div class="character-card">
      <div class="char-header">
        <input class="char-name" bind:value={char.name} oninput={() => markModified(`party.${idx}.name`)} />
        {#if char.class_name}
          <span class="char-class">{char.class_name}</span>
        {/if}
      </div>

      <div class="char-level">
        <label>
          Level
          <input type="number" bind:value={char.level} oninput={() => markModified(`party.${idx}.level`)} />
        </label>
        <label>
          EXP
          <input type="number" bind:value={char.exp} oninput={() => markModified(`party.${idx}.exp`)} />
        </label>
      </div>

      <div class="stat-grid">
        {#each char.stats as stat, si}
          <label class="stat-item">
            <span class="stat-label">{stat.label}</span>
            <input type="number" bind:value={stat.current}
              oninput={() => markModified(`party.${idx}.stats.${si}`)} />
          </label>
        {/each}
      </div>

      {#if char.equipment.length > 0}
        <div class="equip-section">
          <h4>Equipment</h4>
          {#each char.equipment as eq}
            <div class="equip-row">
              <span class="equip-slot">{eq.slot_name}</span>
              <span class="equip-name">{eq.item_name || (eq.item_id ? `#${eq.item_id}` : '—')}</span>
            </div>
          {/each}
        </div>
      {/if}

      {#if char.skills.length > 0}
        <details class="skills-section">
          <summary>Skills ({char.skills.length})</summary>
          <div class="skill-list">
            {#each char.skills as skill}
              <span class="skill-badge">{skill.name}</span>
            {/each}
          </div>
        </details>
      {/if}
    </div>
  {/each}
</div>

<style>
  .party-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 16px;
  }

  .character-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 20px;
  }

  .char-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 14px;
  }

  .char-name {
    font-size: 18px;
    font-weight: 600;
    background: transparent;
    border: 1px solid transparent;
    padding: 2px 6px;
    flex: 1;
  }
  .char-name:hover { border-color: var(--border); }
  .char-name:focus { border-color: var(--border-focus); }

  .char-class {
    font-size: 12px;
    color: var(--accent-primary);
    background: var(--accent-glow);
    padding: 2px 8px;
    border-radius: 10px;
  }

  .char-level {
    display: flex;
    gap: 12px;
    margin-bottom: 14px;
  }

  .char-level label {
    display: flex;
    flex-direction: column;
    font-size: 11px;
    color: var(--text-secondary);
    gap: 4px;
  }

  .char-level input {
    width: 100px;
  }

  .stat-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
    gap: 8px;
    margin-bottom: 14px;
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .stat-label {
    font-size: 10px;
    text-transform: uppercase;
    color: var(--text-muted);
    letter-spacing: 0.5px;
  }

  .stat-item input {
    width: 100%;
  }

  .equip-section h4 {
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  .equip-row {
    display: flex;
    justify-content: space-between;
    padding: 4px 0;
    font-size: 13px;
    border-bottom: 1px solid var(--border);
  }

  .equip-slot { color: var(--text-muted); }
  .equip-name { color: var(--text-primary); }

  .skills-section {
    margin-top: 12px;
  }

  .skills-section summary {
    font-size: 12px;
    color: var(--text-secondary);
    cursor: pointer;
    margin-bottom: 6px;
  }

  .skill-list {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .skill-badge {
    font-size: 11px;
    padding: 2px 8px;
    background: var(--bg-tertiary);
    border-radius: 10px;
    color: var(--text-secondary);
  }
</style>
