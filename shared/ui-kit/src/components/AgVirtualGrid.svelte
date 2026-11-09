<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { VirtualColumn } from './types';

  export let columns: VirtualColumn[] = [];
  export let rows: any[] = [];
  export let rowHeight: number = 32;
  export let selectedRowIndex: number = -1;

  const dispatch = createEventDispatcher<{
    selectRow: { row: any; index: number };
    sort: { columnId: string; direction: 'asc' | 'desc' };
  }>();

  let containerEl: HTMLDivElement;
  let scrollTop: number = 0;
  let containerHeight: number = 400;

  let sortColumnId: string = '';
  let sortDirection: 'asc' | 'desc' = 'asc';

  $: totalRows = rows.length;
  $: totalHeight = totalRows * rowHeight;

  // Virtual windowing math
  $: startIndex = Math.max(0, Math.floor(scrollTop / rowHeight) - 4);
  $: endIndex = Math.min(totalRows, Math.ceil((scrollTop + containerHeight) / rowHeight) + 4);
  $: visibleRows = rows.slice(startIndex, endIndex);
  $: offsetY = startIndex * rowHeight;

  function handleScroll(e: Event) {
    const target = e.currentTarget as HTMLDivElement;
    scrollTop = target.scrollTop;
  }

  function handleResize() {
    if (containerEl) {
      containerHeight = containerEl.clientHeight;
    }
  }

  function handleSort(col: VirtualColumn) {
    if (!col.sortable) return;
    if (sortColumnId === col.id) {
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      sortColumnId = col.id;
      sortDirection = 'asc';
    }
    dispatch('sort', { columnId: sortColumnId, direction: sortDirection });
  }

  function selectRow(row: any, index: number) {
    selectedRowIndex = index;
    dispatch('selectRow', { row, index });
  }
</script>

<svelte:window on:resize={handleResize} />

<div class="ag-grid-wrapper">
  <!-- Table Header -->
  <div class="ag-grid-header">
    {#each columns as col}
      <button
        type="button"
        class="ag-grid-th"
        class:sortable={col.sortable}
        style={col.width ? `width: ${col.width}px; flex: none;` : 'flex: 1;'}
        on:click={() => handleSort(col)}
      >
        <span class="ag-grid-th-text">{col.header}</span>
        {#if col.sortable && sortColumnId === col.id}
          <span class="ag-sort-indicator">{sortDirection === 'asc' ? '▲' : '▼'}</span>
        {/if}
      </button>
    {/each}
  </div>

  <!-- Virtualized Scroll Viewport -->
  <div
    bind:this={containerEl}
    class="ag-grid-viewport"
    on:scroll={handleScroll}
  >
    <div class="ag-grid-spacer" style="height: {totalHeight}px;">
      <div class="ag-grid-content" style="transform: translateY({offsetY}px);">
        {#each visibleRows as row, i}
          {@const actualIndex = startIndex + i}
          <div
            class="ag-grid-row"
            class:selected={actualIndex === selectedRowIndex}
            style="height: {rowHeight}px;"
            on:click={() => selectRow(row, actualIndex)}
            role="row"
            tabindex="0"
            on:keydown={(e) => {
              if (e.key === 'Enter') selectRow(row, actualIndex);
            }}
          >
            {#each columns as col}
              <div
                class="ag-grid-cell"
                class:ag-tabular={col.isMonospace}
                style="{col.width ? `width: ${col.width}px; flex: none;` : 'flex: 1;'} text-align: {col.align || 'left'};"
              >
                {col.render ? col.render(row, actualIndex) : (row[col.id] ?? '')}
              </div>
            {/each}
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .ag-grid-wrapper {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    background: var(--ag-bg-canvas);
    border: 1px solid var(--ag-border);
    border-radius: var(--ag-radius-md);
    overflow: hidden;
  }

  .ag-grid-header {
    display: flex;
    align-items: center;
    background: var(--ag-bg-panel);
    border-bottom: 1px solid var(--ag-border);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--ag-text-muted);
    user-select: none;
    z-index: 2;
  }

  .ag-grid-th {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px 12px;
    background: transparent;
    border: none;
    color: inherit;
    font: inherit;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ag-grid-th.sortable {
    cursor: pointer;
  }

  .ag-grid-th.sortable:hover {
    color: var(--ag-text-primary);
  }

  .ag-sort-indicator {
    font-size: 9px;
    color: var(--ag-accent-primary);
  }

  .ag-grid-viewport {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    position: relative;
  }

  .ag-grid-spacer {
    position: relative;
    width: 100%;
  }

  .ag-grid-content {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
  }

  .ag-grid-row {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--ag-border-subtle);
    cursor: pointer;
    transition: background var(--ag-transition-fast);
  }

  .ag-grid-row:hover {
    background: var(--ag-bg-hover);
  }

  .ag-grid-row.selected {
    background: var(--ag-bg-selected);
  }

  .ag-grid-cell {
    padding: 0 12px;
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>

