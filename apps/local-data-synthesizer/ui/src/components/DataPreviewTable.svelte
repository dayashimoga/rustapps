<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let tableName: string = '';
  export let rows: Record<string, any>[] = [];

  const dispatch = createEventDispatcher();

  $: columns = rows.length > 0 ? Object.keys(rows[0]) : [];

  function handleExportCsv() {
    dispatch('exportCsv', { tableName });
  }
</script>

<div class="data-preview-table-container">
  <div class="preview-header">
    <div class="title-wrap">
      <span class="preview-title">SYNTHETIC ROWS PREVIEW:</span>
      <span class="table-tag">{tableName}</span>
      <span class="count-tag">Showing {rows.length} rows</span>
    </div>

    <button class="export-csv-btn" on:click={handleExportCsv} title="Export Table as CSV">
      <span>💾</span> Export Table CSV
    </button>
  </div>

  <div class="table-scroll-wrap">
    {#if rows.length === 0}
      <div class="empty-state">
        <span>No synthetic rows generated yet. Click "Generate Synthetic Dataset" above.</span>
      </div>
    {:else}
      <table class="grid-table">
        <thead>
          <tr>
            {#each columns as col}
              <th>{col}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each rows as row}
            <tr>
              {#each columns as col}
                <td title={row[col] !== null ? String(row[col]) : 'NULL'}>
                  {#if row[col] === null}
                    <span class="null-val">NULL</span>
                  {:else}
                    {row[col]}
                  {/if}
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

<style>
  .data-preview-table-container {
    display: flex;
    flex-direction: column;
    background: #0d121a;
    border: 1px solid #1a2332;
    border-radius: 8px;
    overflow: hidden;
    flex: 1;
  }

  .preview-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: #141b26;
    border-bottom: 1px solid #1a2332;
    font-family: monospace;
  }

  .title-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .preview-title {
    font-size: 0.72rem;
    font-weight: 700;
    color: #64748b;
  }

  .table-tag {
    color: #38bdf8;
    font-size: 0.75rem;
    font-weight: 700;
  }

  .count-tag {
    font-size: 0.68rem;
    color: #94a3b8;
  }

  .export-csv-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: #1c2636;
    border: 1px solid #28374d;
    color: #38bdf8;
    font-family: inherit;
    font-size: 0.72rem;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .export-csv-btn:hover {
    background: #0284c7;
    color: #ffffff;
    border-color: #38bdf8;
  }

  .table-scroll-wrap {
    flex: 1;
    overflow: auto;
    max-height: 280px;
  }

  .grid-table {
    width: 100%;
    border-collapse: collapse;
    font-family: monospace;
    font-size: 0.72rem;
    text-align: left;
  }

  th {
    background: #111722;
    color: #94a3b8;
    padding: 6px 12px;
    font-weight: 600;
    border-bottom: 1px solid #1a2332;
    position: sticky;
    top: 0;
    z-index: 10;
  }

  td {
    padding: 5px 12px;
    border-bottom: 1px solid #141c28;
    color: #f1f5f9;
    white-space: nowrap;
    max-width: 240px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  tr:hover td {
    background: rgba(56, 189, 248, 0.05);
  }

  .null-val {
    color: #64748b;
    font-style: italic;
  }

  .empty-state {
    padding: 28px;
    text-align: center;
    color: #64748b;
    font-family: monospace;
    font-size: 0.8rem;
  }
</style>
