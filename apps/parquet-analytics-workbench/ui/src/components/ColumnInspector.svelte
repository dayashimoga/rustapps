<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export interface ColumnStatistics {
    column: string;
    data_type: string;
    total_count: number;
    null_count: number;
    distinct_count: number;
    min_value: string | null;
    max_value: string | null;
    mean_numeric: number | null;
  }

  export let stats: ColumnStatistics | null = null;
  export let isOpen: boolean = false;

  const dispatch = createEventDispatcher<{
    close: void;
    addFilter: { column: string; value: string };
    setChartAxis: { column: string };
  }>();

  function handleClose() {
    dispatch('close');
  }
</script>

{#if isOpen && stats}
  <!-- Modal Backdrop -->
  <div
    class="backdrop"
    on:click={handleClose}
    role="button"
    tabindex="0"
    on:keydown={(e) => e.key === 'Escape' && handleClose()}
  >
    <div
      class="panel"
      on:click|stopPropagation
      on:keydown|stopPropagation
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <div class="panel-header">
        <div class="title-wrap">
          <span class="icon">📊</span>
          <h3>Column: <code>{stats.column}</code></h3>
          <span class="type-pill">{stats.data_type}</span>
        </div>
        <button class="btn-close" on:click={handleClose}>✕</button>
      </div>

      <div class="panel-body">
        <div class="stats-grid">
          <div class="stat-box">
            <span class="val">{stats.total_count.toLocaleString()}</span>
            <span class="lbl">Total Rows</span>
          </div>
          <div class="stat-box">
            <span class="val">{stats.distinct_count.toLocaleString()}</span>
            <span class="lbl">Distinct Values</span>
          </div>
          <div class="stat-box">
            <span class="val">{stats.null_count.toLocaleString()}</span>
            <span class="lbl">Null Values ({((stats.null_count / (stats.total_count || 1)) * 100).toFixed(1)}%)</span>
          </div>
          {#if stats.mean_numeric !== null}
            <div class="stat-box stat-highlight">
              <span class="val">{stats.mean_numeric.toFixed(2)}</span>
              <span class="lbl">Mean Average</span>
            </div>
          {/if}
        </div>

        <div class="range-card">
          <div class="range-row">
            <span class="range-lbl">MINIMUM:</span>
            <span class="range-val">{stats.min_value ?? '—'}</span>
          </div>
          <div class="range-row">
            <span class="range-lbl">MAXIMUM:</span>
            <span class="range-val">{stats.max_value ?? '—'}</span>
          </div>
        </div>

        <div class="actions-row">
          <button
            class="btn btn-secondary"
            on:click={() => dispatch('setChartAxis', { column: stats?.column || '' })}
          >
            📈 Plot in Chart
          </button>
          <button class="btn btn-primary" on:click={handleClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .panel {
    background: #111827;
    border: 1px solid rgba(75, 85, 99, 0.4);
    border-radius: 12px;
    width: 90%;
    max-width: 480px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
  }
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 18px;
    border-bottom: 1px solid rgba(75, 85, 99, 0.3);
  }
  .title-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title-wrap h3 {
    margin: 0;
    font-size: 1rem;
    color: #f8fafc;
  }
  .type-pill {
    background: rgba(99, 102, 241, 0.2);
    color: #818cf8;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 700;
  }
  .btn-close {
    background: none;
    border: none;
    color: #94a3b8;
    font-size: 1.1rem;
    cursor: pointer;
  }
  .panel-body {
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .stats-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .stat-box {
    background: #1e293b;
    padding: 12px;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    border: 1px solid rgba(51, 65, 85, 0.4);
  }
  .stat-highlight {
    border-color: rgba(6, 182, 212, 0.4);
    background: rgba(6, 182, 212, 0.1);
  }
  .val {
    font-size: 1.2rem;
    font-weight: 700;
    color: #f8fafc;
    font-family: var(--font-mono);
  }
  .lbl {
    font-size: 0.75rem;
    color: #94a3b8;
  }
  .range-card {
    background: #1e293b;
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border: 1px solid rgba(51, 65, 85, 0.3);
  }
  .range-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.8rem;
    font-family: var(--font-mono);
  }
  .range-lbl {
    color: #64748b;
  }
  .range-val {
    color: #cbd5e1;
  }
  .actions-row {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    border: none;
  }
  .btn-primary {
    background: #6366f1;
    color: #ffffff;
  }
  .btn-secondary {
    background: #334155;
    color: #cbd5e1;
  }
</style>

