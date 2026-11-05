<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let columns: string[] = [];
  export let rows: string[][] = [];
  export let totalRows: number = 0;
  export let pageSize: number = 25;
  export let pageIndex: number = 0;
  export let sortColumn: string | null = null;
  export let sortDirection: 'Asc' | 'Desc' = 'Asc';
  export let isLoading: boolean = false;

  const dispatch = createEventDispatcher<{
    pageChange: number;
    sortChange: { column: string; direction: 'Asc' | 'Desc' };
    columnSelect: string;
  }>();

  $: totalPages = Math.ceil(totalRows / (pageSize || 1)) || 1;

  function handleHeaderClick(col: string) {
    let nextDir: 'Asc' | 'Desc' = 'Asc';
    if (sortColumn === col) {
      nextDir = sortDirection === 'Asc' ? 'Desc' : 'Asc';
    }
    dispatch('sortChange', { column: col, direction: nextDir });
  }

  function handlePrev() {
    if (pageIndex > 0) {
      dispatch('pageChange', pageIndex - 1);
    }
  }

  function handleNext() {
    if (pageIndex + 1 < totalPages) {
      dispatch('pageChange', pageIndex + 1);
    }
  }
</script>

<div class="table-container">
  <div class="table-viewport">
    {#if isLoading}
      <div class="loading-overlay">
        <span class="spinner">⏳</span> Querying local analytical cache...
      </div>
    {/if}

    <table class="data-grid">
      <thead>
        <tr>
          <th class="col-index">#</th>
          {#each columns as col}
            <th
              class="col-header"
              class:sorted={sortColumn === col}
              on:click={() => handleHeaderClick(col)}
              title="Click to sort by {col}"
            >
              <div class="header-inner">
                <span class="col-title">{col}</span>
                {#if sortColumn === col}
                  <span class="sort-indicator">{sortDirection === 'Asc' ? '▲' : '▼'}</span>
                {/if}
                <button
                  class="btn-inspect"
                  on:click|stopPropagation={() => dispatch('columnSelect', col)}
                  title="Inspect Column Stats"
                >
                  📊
                </button>
              </div>
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#if rows.length === 0 && !isLoading}
          <tr>
            <td colspan={columns.length + 1} class="empty-cell">
              No matching rows found in this dataset view.
            </td>
          </tr>
        {:else}
          {#each rows as row, rIdx}
            <tr>
              <td class="col-index">{pageIndex * pageSize + rIdx + 1}</td>
              {#each row as cell}
                <td class="data-cell" title={cell}>
                  {cell !== '' ? cell : '—'}
                </td>
              {/each}
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>

  <!-- Pagination Controls -->
  <div class="pagination-bar">
    <div class="page-metrics">
      Showing <strong>{rows.length}</strong> of <strong>{totalRows.toLocaleString()}</strong> rows
    </div>

    <div class="page-controls">
      <button class="btn-page" on:click={handlePrev} disabled={pageIndex === 0 || isLoading}>
        ◀ Prev
      </button>
      <span class="page-label">
        Page {pageIndex + 1} of {totalPages}
      </span>
      <button class="btn-page" on:click={handleNext} disabled={pageIndex + 1 >= totalPages || isLoading}>
        Next ▶
      </button>
    </div>
  </div>
</div>

<style>
  .table-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #0b0f19;
    position: relative;
    overflow: hidden;
  }
  .table-viewport {
    flex: 1;
    overflow: auto;
    position: relative;
  }
  .loading-overlay {
    position: absolute;
    inset: 0;
    background: rgba(11, 15, 25, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    font-size: 0.9rem;
    color: #38bdf8;
    z-index: 10;
  }
  .data-grid {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.82rem;
    font-family: var(--font-mono);
  }
  thead {
    position: sticky;
    top: 0;
    background: #1e293b;
    z-index: 5;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
  }
  th {
    padding: 8px 12px;
    text-align: left;
    font-weight: 600;
    color: #94a3b8;
    border-bottom: 2px solid #334155;
    border-right: 1px solid #334155;
    user-select: none;
    white-space: nowrap;
  }
  .col-header {
    cursor: pointer;
  }
  .col-header:hover {
    background: #334155;
    color: #f8fafc;
  }
  .col-header.sorted {
    color: #38bdf8;
  }
  .header-inner {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .col-title {
    flex: 1;
  }
  .sort-indicator {
    font-size: 0.7rem;
    color: #38bdf8;
  }
  .btn-inspect {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.75rem;
    opacity: 0.4;
    padding: 2px;
  }
  .btn-inspect:hover {
    opacity: 1;
  }
  .col-index {
    width: 50px;
    text-align: center;
    color: #64748b;
    border-right: 1px solid #1e293b;
  }
  tbody tr {
    border-bottom: 1px solid #1e293b;
    transition: background 0.05s;
  }
  tbody tr:hover {
    background: rgba(30, 41, 59, 0.4);
  }
  td {
    padding: 6px 12px;
    color: #cbd5e1;
    border-right: 1px solid rgba(51, 65, 85, 0.3);
    max-width: 250px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty-cell {
    text-align: center;
    padding: 40px;
    color: #64748b;
  }
  .pagination-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: #111827;
    border-top: 1px solid rgba(75, 85, 99, 0.3);
    font-size: 0.8rem;
  }
  .page-metrics {
    color: #94a3b8;
  }
  .page-metrics strong {
    color: #f8fafc;
  }
  .page-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .btn-page {
    background: #1e293b;
    border: 1px solid #334155;
    color: #cbd5e1;
    padding: 4px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .btn-page:hover:not(:disabled) {
    background: #334155;
    color: #ffffff;
  }
  .btn-page:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .page-label {
    color: #94a3b8;
  }
</style>
