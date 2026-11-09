<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let tables: any[] = [];
  export let activeTableName: string = '';

  const dispatch = createEventDispatcher();

  function selectTab(tableName: string) {
    activeTableName = tableName;
    dispatch('tableSelect', { tableName });
  }

  function getStrategyBadge(strategy: any): string {
    if (typeof strategy === 'string') return strategy;
    if (strategy?.strategy) return strategy.strategy.replace('synthetic_', '');
    return 'custom';
  }
</script>

<div class="schema-inspector">
  <div class="table-tabs">
    {#each tables as table}
      <button
        class="tab-btn"
        class:active={table.table_name === activeTableName}
        on:click={() => selectTab(table.table_name)}
      >
        <span class="table-icon">🗄️</span>
        <span class="table-name">{table.table_name}</span>
        <span class="row-count-tag">{table.row_count} rows</span>
      </button>
    {/each}
  </div>

  <div class="columns-table-container">
    {#if tables.find((t) => t.table_name === activeTableName)}
      {@const activeTable = tables.find((t) => t.table_name === activeTableName)}
      <table class="columns-table">
        <thead>
          <tr>
            <th>Column Name</th>
            <th>Type</th>
            <th>Constraints</th>
            <th>Synthesis / Anonymization Strategy</th>
            <th>Nullable</th>
          </tr>
        </thead>
        <tbody>
          {#each activeTable.columns as col}
            <tr>
              <td class="col-name">
                {#if col.is_primary_key}
                  <span class="pk-icon" title="Primary Key">🔑</span>
                {/if}
                {col.column_name}
              </td>
              <td>
                <span class="type-pill">{col.data_type}</span>
              </td>
              <td>
                {#if col.foreign_key_ref}
                  <span class="fk-pill" title="Foreign Key Reference">
                    ↗ {col.foreign_key_ref.target_table}.{col.foreign_key_ref.target_column}
                  </span>
                {:else if col.is_primary_key}
                  <span class="pk-pill">PRIMARY KEY</span>
                {:else}
                  <span class="none-pill">—</span>
                {/if}
              </td>
              <td>
                <span class="strategy-badge">
                  {getStrategyBadge(col.strategy)}
                </span>
              </td>
              <td class="nullable-col">
                {col.nullable_probability > 0 ? `${(col.nullable_probability * 100).toFixed(0)}%` : 'No'}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <div class="empty-schema">No schema selected</div>
    {/if}
  </div>
</div>

<style>
  .schema-inspector {
    display: flex;
    flex-direction: column;
    background: #0d121a;
    border: 1px solid #1a2332;
    border-radius: 8px;
    overflow: hidden;
  }

  .table-tabs {
    display: flex;
    background: #121924;
    border-bottom: 1px solid #1a2332;
    padding: 4px 8px;
    gap: 6px;
  }

  .tab-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #172130;
    border: 1px solid #233147;
    color: #94a3b8;
    padding: 4px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-family: monospace;
    font-size: 0.75rem;
    transition: all 0.15s ease;
  }

  .tab-btn:hover {
    color: #f1f5f9;
    border-color: #38bdf8;
  }

  .tab-btn.active {
    background: #0284c7;
    border-color: #38bdf8;
    color: #ffffff;
    font-weight: 700;
  }

  .row-count-tag {
    font-size: 0.65rem;
    background: rgba(0, 0, 0, 0.3);
    padding: 1px 4px;
    border-radius: 3px;
  }

  .columns-table-container {
    max-height: 220px;
    overflow-y: auto;
  }

  .columns-table {
    width: 100%;
    border-collapse: collapse;
    font-family: monospace;
    font-size: 0.72rem;
    text-align: left;
  }

  th {
    background: #141b26;
    color: #64748b;
    padding: 6px 12px;
    font-weight: 600;
    border-bottom: 1px solid #1e2838;
  }

  td {
    padding: 6px 12px;
    border-bottom: 1px solid #141c28;
    color: #cbd5e1;
  }

  .col-name {
    color: #f1f5f9;
    font-weight: 600;
  }

  .type-pill {
    background: #16202e;
    border: 1px solid #28374d;
    padding: 1px 5px;
    border-radius: 3px;
    color: #38bdf8;
    font-size: 0.68rem;
  }

  .pk-pill {
    background: rgba(234, 179, 8, 0.15);
    border: 1px solid #eab30866;
    color: #facc15;
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 0.65rem;
    font-weight: 700;
  }

  .fk-pill {
    background: rgba(168, 85, 247, 0.15);
    border: 1px solid #a855f766;
    color: #c084fc;
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 0.68rem;
  }

  .none-pill {
    color: #475569;
  }

  .strategy-badge {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid #10b98166;
    color: #34d399;
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 0.68rem;
  }

  .empty-schema {
    padding: 24px;
    text-align: center;
    color: #64748b;
    font-family: monospace;
    font-size: 0.8rem;
  }
</style>
