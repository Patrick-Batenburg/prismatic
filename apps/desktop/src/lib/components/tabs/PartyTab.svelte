<script lang="ts">
  import type { Character, CurrencyInfo, NameMap } from "$lib/api";
  import { markModified, trackEdit } from "$lib/stores";

  let {
    party = $bindable(),
    currency = $bindable(),
    nameMap = null,
  }: {
    party: Character[];
    currency: CurrencyInfo | null;
    nameMap: NameMap | null;
  } = $props();

  function getEquipOptions(dataClass: string): { id: number; name: string }[] {
    if (!nameMap) return [];
    const map = dataClass === "weapon" ? nameMap.weapons : nameMap.armors;
    if (!map) return [];
    return Object.entries(map)
      .map(([id, name]) => ({ id: Number(id), name }))
      .sort((a, b) => a.id - b.id);
  }

  function onEquipChange(
    charIdx: number,
    eqIdx: number,
    eq: { item_id: number | null; item_name: string | null; data_class: string; slot_name: string },
    value: string,
  ) {
    const oldId = eq.item_id;
    const id = Number(value);
    eq.item_id = id || null;
    // Update the displayed name from nameMap
    if (id && nameMap) {
      const map = eq.data_class === "weapon" ? nameMap.weapons : nameMap.armors;
      eq.item_name = map?.[id] ?? null;
    } else {
      eq.item_name = null;
    }
    trackEdit(
      ['party', String(charIdx), 'equips', String(eqIdx), 'item_id'],
      oldId, eq.item_id,
      `Set ${eq.slot_name} to ${eq.item_name || 'None'}`
    );
    markModified(`party.${charIdx}.equips.${eqIdx}`);
  }
</script>

<!-- Currency at the top -->
{#if currency}
  <div class="currency-bar">
    <label class="currency-label" for="party-currency">
      <span class="currency-icon">💰</span>
      <span class="currency-name">{currency.label}</span>
    </label>
    <input
      id="party-currency"
      type="number"
      class="currency-input"
      value={currency.amount}
      onfocus={(e) => { e.currentTarget.dataset.old = String(currency.amount); }}
      onchange={(e) => {
        const oldVal = Number(e.currentTarget.dataset.old);
        const newVal = Number(e.currentTarget.value);
        currency.amount = newVal;
        trackEdit(
          ['currency', 'amount'],
          oldVal, newVal,
          `Set ${currency.label} to ${newVal}`
        );
        markModified("currency.amount");
      }}
    />
  </div>
{/if}

<div class="party-grid">
  {#each party as char, idx (char.id)}
    <div class="character-card">
      <div class="char-header">
        <input
          class="char-name"
          value={char.name}
          onfocus={(e) => { e.currentTarget.dataset.old = char.name; }}
          onchange={(e) => {
            const oldVal = e.currentTarget.dataset.old ?? '';
            const newVal = e.currentTarget.value;
            char.name = newVal;
            trackEdit(
              ['party', String(idx), 'name'],
              oldVal, newVal,
              `Rename character to "${newVal}"`
            );
            markModified(`party.${idx}.name`);
          }}
        />
        {#if char.class_name}
          <span class="char-class">{char.class_name}</span>
        {/if}
      </div>

      <div class="char-level">
        <label>
          Level
          <input
            type="number"
            value={char.level}
            onfocus={(e) => { e.currentTarget.dataset.old = String(char.level); }}
            onchange={(e) => {
              const oldVal = Number(e.currentTarget.dataset.old);
              const newVal = Number(e.currentTarget.value);
              char.level = newVal;
              trackEdit(
                ['party', String(idx), 'level'],
                oldVal, newVal,
                `Set ${char.name} level to ${newVal}`
              );
              markModified(`party.${idx}.level`);
            }}
          />
        </label>
        <label>
          EXP
          <input
            type="number"
            value={char.exp}
            onfocus={(e) => { e.currentTarget.dataset.old = String(char.exp); }}
            onchange={(e) => {
              const oldVal = Number(e.currentTarget.dataset.old);
              const newVal = Number(e.currentTarget.value);
              char.exp = newVal;
              trackEdit(
                ['party', String(idx), 'exp'],
                oldVal, newVal,
                `Set ${char.name} EXP to ${newVal}`
              );
              markModified(`party.${idx}.exp`);
            }}
          />
        </label>
      </div>

      <div class="stat-grid">
        {#each char.stats as stat, si (stat.key)}
          <label class="stat-item">
            <span class="stat-label">{stat.label}</span>
            <input
              type="number"
              value={stat.current}
              onfocus={(e) => { e.currentTarget.dataset.old = String(stat.current); }}
              onchange={(e) => {
                const oldVal = Number(e.currentTarget.dataset.old);
                const newVal = Number(e.currentTarget.value);
                stat.current = newVal;
                trackEdit(
                  ['party', String(idx), 'stats', String(si), 'current'],
                  oldVal, newVal,
                  `Set ${char.name} ${stat.label} to ${newVal}`
                );
                markModified(`party.${idx}.stats.${si}.current`);
              }}
            />
          </label>
        {/each}
      </div>

      {#if char.equipment.length > 0}
        <div class="equip-section">
          <h4>Equipment</h4>
          {#each char.equipment as eq, ei (eq.slot_name)}
            {@const options = getEquipOptions(eq.data_class)}
            <div class="equip-row">
              <span class="equip-slot">{eq.slot_name}</span>
              {#if options.length > 0}
                <select
                  class="equip-select"
                  value={eq.item_id ?? 0}
                  onchange={(e) => onEquipChange(idx, ei, eq, e.currentTarget.value)}
                >
                  <option value={0}>— None —</option>
                  {#each options as opt (opt.id)}
                    <option value={opt.id}>{opt.name}</option>
                  {/each}
                </select>
              {:else}
                <span class="equip-name"
                  >{eq.item_name || (eq.item_id ? `#${eq.item_id}` : "—")}</span
                >
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      {#if char.skills.length > 0}
        <details class="skills-section">
          <summary>Skills ({char.skills.length})</summary>
          <div class="skill-list">
            {#each char.skills as skill (skill.id)}
              <span class="skill-badge">{skill.name}</span>
            {/each}
          </div>
        </details>
      {/if}
    </div>
  {/each}
</div>

<style>
  .currency-bar {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 16px 24px;
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 16px;
  }

  .currency-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 18px;
    font-weight: 600;
  }

  .currency-icon {
    font-size: 24px;
  }
  .currency-name {
    color: var(--text-primary);
  }

  .currency-input {
    font-size: 18px;
    font-weight: 600;
    padding: 6px 12px;
    width: 180px;
    text-align: right;
    background: var(--bg-input);
  }

  .party-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 16px;
  }

  .character-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px;
    box-shadow: var(--shadow-card);
    transition: border-color var(--transition), box-shadow var(--transition);
  }
  .character-card:hover {
    border-color: rgba(108, 92, 231, 0.3);
    box-shadow: var(--shadow-elevated);
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
  .char-name:hover {
    border-color: var(--border);
  }
  .char-name:focus {
    border-color: var(--border-focus);
  }

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
    align-items: center;
    padding: 5px 0;
    font-size: 13px;
    border-bottom: 1px solid var(--border);
    gap: 12px;
  }

  .equip-slot {
    color: var(--text-muted);
    min-width: 80px;
    flex-shrink: 0;
  }

  .equip-select {
    flex: 1;
    padding: 4px 8px;
    font-size: 13px;
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    min-width: 0;
  }
  .equip-select:hover {
    border-color: var(--border-focus);
  }
  .equip-select:focus {
    border-color: var(--accent-primary);
    outline: none;
  }

  .equip-name {
    color: var(--text-primary);
  }

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
