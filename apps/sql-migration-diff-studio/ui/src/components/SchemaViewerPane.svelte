<script lang="ts">
  import type { DatabaseSchema, TableDefinition } from '../lib/types';

  export let schema: DatabaseSchema | null = null;

  let selectedTable: TableDefinition | null = null;
  let activeTab: 'tables' | 'raw_ddl' = 'tables';

  $: if (schema && (!selectedTable || !schema.tables.some((t) => t.name === selectedTable?.name))) {
    selectedTable = schema.tables[0] || null;
  }
</script>

{#if schema}
  <div class="schema-pane-shell">
    <div class="pane-top-bar">
      <div class="schema-meta">
        <h2 class="title">{schema.title}</h2>
        <div class="tags">
          <span class="dialect-badge">{schema.dialect.toUpperCase()}</span>
          <span class="count-badge">{schema.tables.length} Tables</span>
        </div>
      </div>

      <div class="view-toggles">
        <button
          type="button"
          class="v-btn"
          class:active={activeTab === 'tables'}
          on:click={() => (activeTab = 'tables')}
        >
          Visual Tables
        </button>
        <button
          type="button"
          class="v-btn"
          class:active={activeTab === 'raw_ddl'}
          on:click={() => (activeTab = 'raw_ddl')}
        >
          Raw DDL
        </button>
      </div>
    </div>

    {#if activeTab === 'tables'}
      <div class="tables-workspace">
        <!-- Tables sidebar list -->
        <div class="tbl-list-col">
          <div class="col-hdr">RELATIONAL TABLES</div>
          <div class="tbl-items">
            {#each schema.tables as tbl}
              <div
                class="tbl-row"
                class:selected={selectedTable?.name === tbl.name}
                on:click={() => (selectedTable = tbl)}
                on:keydown={(e) => e.key === 'Enter' && (selectedTable = tbl)}
                role="button"
                tabindex="0"
              >
                <div class="tbl-name">{tbl.name}</div>
                <div class="tbl-badges">
                  <span class="col-cnt">{tbl.columns.length} cols</span>
                  {#if tbl.indexes.length > 0}
                    <span class="idx-cnt">{tbl.indexes.length} idx</span>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>

        <!-- Table deep dive view -->
        <div class="tbl-detail-col">
          {#if selectedTable}
            <div class="detail-card">
              <div class="detail-hdr">
                <div class="dh-left">
                  <span class="table-icon">📋</span>
                  <span class="table-title">{selectedTable.name}</span>
                </div>
                {#if selectedTable.primary_key.length > 0}
                  <div class="pk-badge">
                    🔑 PK: ({selectedTable.primary_key.join(', ')})
                  </div>
                {/if}
              </div>

              <!-- Columns Table -->
              <div class="table-section">
                <div class="sec-title">COLUMNS</div>
                <div class="data-table-wrapper">
                  <table class="columns-table">
                    <thead>
                      <tr>
                        <th>Name</th>
                        <th>Type</th>
                        <th>Nullability</th>
                        <th>Default</th>
                        <th>Attributes</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each selectedTable.columns as col}
                        <tr>
                          <td class="col-name mono font-semibold">{col.name}</td>
                          <td class="col-type mono">{col.data_type}</td>
                          <td>
                            <span class="null-pill" class:not-null={!col.is_nullable}>
                              {col.is_nullable ? 'NULL' : 'NOT NULL'}
                            </span>
                          </td>
                          <td class="mono default-val">{col.default_value || '—'}</td>
                          <td>
                            {#if col.is_primary_key}
                              <span class="attr-pill pk">PRIMARY KEY</span>
                            {/if}
                            {#if col.is_autoincrement}
                              <span class="attr-pill auto">AUTO_INC</span>
                            {/if}
                          </td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              </div>

              <!-- Indexes -->
              {#if selectedTable.indexes.length > 0}
                <div class="table-section">
                  <div class="sec-title">INDEXES ({selectedTable.indexes.length})</div>
                  <div class="indexes-grid">
                    {#each selectedTable.indexes as idx}
                      <div class="index-card">
                        <div class="idx-top">
                          <span class="idx-name mono">{idx.name}</span>
                          {#if idx.is_unique}
                            <span class="unique-pill">UNIQUE</span>
                          {/if}
                        </div>
                        <div class="idx-cols mono">Columns: ({idx.columns.join(', ')})</div>
                        <div class="idx-type">Method: {idx.index_type}</div>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}

              <!-- Foreign Keys -->
              {#if selectedTable.foreign_keys.length > 0}
                <div class="table-section">
                  <div class="sec-title">FOREIGN KEY CONSTRAINTS ({selectedTable.foreign_keys.length})</div>
                  <div class="fk-list">
                    {#each selectedTable.foreign_keys as fk}
                      <div class="fk-card">
                        <span class="fk-name mono font-semibold">{fk.name}:</span>
                        <span class="fk-desc mono">
                          ({fk.constrained_columns.join(', ')}) → {fk.referenced_table}({fk.referenced_columns.join(', ')})
                        </span>
                        {#if fk.on_delete}
                          <span class="cascade-pill">ON DELETE {fk.on_delete}</span>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          {:else}
            <div class="empty-selection">Select a table to inspect columns and indexes.</div>
          {/if}
        </div>
      </div>
    {:else}
      <!-- Raw DDL Text View -->
      <div class="raw-ddl-view">
        <pre>{schema.raw_ddl}</pre>
      </div>
    {/if}
  </div>
{:else}
  <div class="empty-state">
    <div class="empty-icon">🗄️</div>
    <h3>No Schema Selected</h3>
    <p>Choose an existing database schema or import a new SQL DDL script to begin.</p>
  </div>
{/if}

<style>
  .schema-pane-shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #030712;
  }

  .pane-top-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 20px;
    background: #090e18;
    border-bottom: 1px solid #1a2336;
  }

  .schema-meta {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .title {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 700;
    color: #f8fafc;
  }

  .tags {
    display: flex;
    gap: 8px;
  }

  .dialect-badge {
    background: #1e3a8a;
    color: #93c5fd;
    font-size: 0.72rem;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 4px;
    letter-spacing: 0.05em;
  }

  .count-badge {
    background: #1e293b;
    color: #94a3b8;
    font-size: 0.72rem;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 4px;
  }

  .view-toggles {
    display: flex;
    background: #060a12;
    border: 1px solid #1e293b;
    border-radius: 6px;
    padding: 2px;
  }

  .v-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 0.8rem;
    font-weight: 600;
    padding: 4px 12px;
    border-radius: 4px;
    cursor: pointer;
  }

  .v-btn.active {
    background: #1e293b;
    color: #f1f5f9;
  }

  .tables-workspace {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .tbl-list-col {
    width: 260px;
    min-width: 220px;
    background: #070b14;
    border-right: 1px solid #151f33;
    display: flex;
    flex-direction: column;
  }

  .col-hdr {
    padding: 10px 14px;
    font-size: 0.72rem;
    font-weight: 700;
    color: #64748b;
    border-bottom: 1px solid #151f33;
    letter-spacing: 0.05em;
  }

  .tbl-items {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .tbl-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
    background: #0a0f1d;
    border: 1px solid #151f33;
    transition: background 0.15s ease;
  }

  .tbl-row:hover {
    background: #101726;
  }

  .tbl-row.selected {
    background: #131c30;
    border-color: #2563eb;
  }

  .tbl-name {
    font-size: 0.84rem;
    font-weight: 600;
    color: #f1f5f9;
  }

  .tbl-badges {
    display: flex;
    gap: 4px;
  }

  .col-cnt {
    font-size: 0.68rem;
    background: #1e293b;
    color: #94a3b8;
    padding: 1px 5px;
    border-radius: 3px;
  }

  .idx-cnt {
    font-size: 0.68rem;
    background: #172554;
    color: #60a5fa;
    padding: 1px 5px;
    border-radius: 3px;
  }

  .tbl-detail-col {
    flex: 1;
    overflow-y: auto;
    padding: 18px;
  }

  .detail-card {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .detail-hdr {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 12px;
    border-bottom: 1px solid #1e293b;
  }

  .dh-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .table-icon {
    font-size: 1.2rem;
  }

  .table-title {
    font-size: 1.2rem;
    font-weight: 700;
    color: #f8fafc;
  }

  .pk-badge {
    background: rgba(245, 158, 11, 0.15);
    color: #fcd34d;
    border: 1px solid rgba(245, 158, 11, 0.3);
    font-size: 0.76rem;
    font-weight: 600;
    padding: 3px 10px;
    border-radius: 4px;
    font-family: monospace;
  }

  .table-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .sec-title {
    font-size: 0.72rem;
    font-weight: 700;
    color: #64748b;
    letter-spacing: 0.05em;
  }

  .data-table-wrapper {
    background: #060a12;
    border: 1px solid #1a2336;
    border-radius: 8px;
    overflow: hidden;
  }

  .columns-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.84rem;
  }

  .columns-table th {
    text-align: left;
    padding: 9px 12px;
    background: #090e18;
    color: #94a3b8;
    font-weight: 600;
    border-bottom: 1px solid #1a2336;
    font-size: 0.75rem;
    text-transform: uppercase;
  }

  .columns-table td {
    padding: 9px 12px;
    border-bottom: 1px solid #111827;
    color: #cbd5e1;
  }

  .columns-table tr:last-child td {
    border-bottom: none;
  }

  .mono {
    font-family: monospace;
  }

  .font-semibold {
    font-weight: 600;
  }

  .col-name {
    color: #f8fafc;
  }

  .col-type {
    color: #38bdf8;
  }

  .default-val {
    color: #94a3b8;
  }

  .null-pill {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 700;
    background: #1e293b;
    color: #94a3b8;
  }

  .null-pill.not-null {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
  }

  .attr-pill {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.68rem;
    font-weight: 700;
    margin-right: 4px;
  }

  .attr-pill.pk {
    background: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
  }

  .attr-pill.auto {
    background: rgba(16, 185, 129, 0.2);
    color: #34d399;
  }

  .indexes-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 10px;
  }

  .index-card {
    background: #060a12;
    border: 1px solid #1a2336;
    border-radius: 6px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .idx-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .idx-name {
    font-size: 0.8rem;
    font-weight: 600;
    color: #38bdf8;
  }

  .unique-pill {
    background: #1e3a8a;
    color: #93c5fd;
    font-size: 0.65rem;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: 3px;
  }

  .idx-cols {
    font-size: 0.78rem;
    color: #cbd5e1;
  }

  .idx-type {
    font-size: 0.7rem;
    color: #64748b;
  }

  .fk-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .fk-card {
    display: flex;
    align-items: center;
    gap: 8px;
    background: #060a12;
    border: 1px solid #1a2336;
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 0.8rem;
  }

  .fk-name {
    color: #a78bfa;
  }

  .fk-desc {
    color: #cbd5e1;
  }

  .cascade-pill {
    background: #312e81;
    color: #c7d2fe;
    font-size: 0.68rem;
    padding: 1px 6px;
    border-radius: 3px;
  }

  .raw-ddl-view {
    flex: 1;
    padding: 16px;
    overflow-y: auto;
    background: #060a12;
  }

  .raw-ddl-view pre {
    margin: 0;
    font-family: monospace;
    font-size: 0.85rem;
    line-height: 1.5;
    color: #e2e8f0;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    color: #64748b;
  }

  .empty-icon {
    font-size: 3rem;
    opacity: 0.4;
    margin-bottom: 12px;
  }

  .empty-selection {
    text-align: center;
    padding: 40px;
    color: #64748b;
  }
</style>
