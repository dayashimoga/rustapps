<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from '../lib/ipc';

  export let onSelectPayload: (payload: string) => void = () => {};

  let searchQuery = '';
  let favoritesOnly = false;
  let historyList: Array<{
    id: string;
    payload_type: string;
    raw_payload: string;
    label: string | null;
    source: 'Generated' | 'Scanned';
    is_favorite: boolean;
    security_risk_level: string | null;
    is_encrypted: boolean;
    has_signature: boolean;
    created_at: number;
  }> = [];

  let stats: {
    total_generated: number;
    total_scanned: number;
    favorites_count: number;
    security_threats_blocked: number;
  } | null = null;

  let isLoading = false;
  let errorMessage = '';

  onMount(() => {
    loadStats();
    loadHistory();
  });

  async function loadStats() {
    try {
      stats = await invokeIpc<any>('get_studio_stats', {});
    } catch (e: any) {
      console.warn('Failed to fetch stats:', e);
    }
  }

  async function loadHistory() {
    isLoading = true;
    errorMessage = '';
    try {
      const records = await invokeIpc<any>('list_history', {
        search_query: searchQuery.trim() ? searchQuery.trim() : null,
        favorites_only: favoritesOnly,
        limit: 100,
      });
      historyList = records;
    } catch (e: any) {
      errorMessage = e?.message || String(e);
    } finally {
      isLoading = false;
    }
  }

  async function toggleFavorite(id: string) {
    try {
      await invokeIpc('toggle_favorite', { id });
      await loadHistory();
      await loadStats();
    } catch (e: any) {
      errorMessage = e?.message || String(e);
    }
  }

  async function deleteItem(id: string) {
    try {
      await invokeIpc('delete_history_item', { id });
      await loadHistory();
      await loadStats();
    } catch (e: any) {
      errorMessage = e?.message || String(e);
    }
  }

  function formatDate(epochSecs: number): string {
    return new Date(epochSecs * 1000).toLocaleString();
  }
</script>

<div class="history-container">
  <div class="history-header">
    <h2>History & Offline Storage Analytics</h2>
    <p class="subtitle">SQLite WAL Database with Zero-Cloud Air-Gapped Persistence</p>
  </div>

  <!-- Analytics Cards -->
  <div class="stats-row">
    <div class="stat-card">
      <span class="stat-title">Generated Codes</span>
      <span class="stat-val">{stats?.total_generated ?? '—'}</span>
    </div>
    <div class="stat-card">
      <span class="stat-title">Scanned Codes</span>
      <span class="stat-val">{stats?.total_scanned ?? '—'}</span>
    </div>
    <div class="stat-card">
      <span class="stat-title">Starred Favorites</span>
      <span class="stat-val star-val">{stats?.favorites_count ?? '—'}</span>
    </div>
    <div class="stat-card">
      <span class="stat-title">Threats Quarantined</span>
      <span class="stat-val threat-val">{stats?.security_threats_blocked ?? '—'}</span>
    </div>
  </div>

  <!-- Search & Filters -->
  <div class="filter-bar">
    <div class="search-wrap">
      <input
        type="text"
        placeholder="Search history by payload content or custom label..."
        bind:value={searchQuery}
        on:input={loadHistory}
        class="search-input"
      />
    </div>
    <label class="fav-toggle">
      <input type="checkbox" bind:checked={favoritesOnly} on:change={loadHistory} />
      <span>★ Favorites Only</span>
    </label>
  </div>

  {#if errorMessage}
    <div class="error-banner">⚠️ {errorMessage}</div>
  {/if}

  <!-- History Records List -->
  <div class="history-list-wrap">
    {#if historyList.length > 0}
      <table class="history-table">
        <thead>
          <tr>
            <th>★</th>
            <th>Type</th>
            <th>Source</th>
            <th>Label / Preview</th>
            <th>Security</th>
            <th>Created</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each historyList as record}
            <tr>
              <td class="fav-cell">
                <button
                  class="btn-star"
                  class:starred={record.is_favorite}
                  on:click={() => toggleFavorite(record.id)}
                >
                  {record.is_favorite ? '★' : '☆'}
                </button>
              </td>
              <td><span class="type-pill">{record.payload_type}</span></td>
              <td><span class="source-pill" class:scan={record.source === 'Scanned'}>{record.source}</span></td>
              <td class="content-cell">
                {#if record.label}
                  <div class="record-label">{record.label}</div>
                {/if}
                <div class="record-preview">{record.raw_payload}</div>
              </td>
              <td>
                {#if record.security_risk_level}
                  <span class="risk-badge risk-{record.security_risk_level.toLowerCase()}">
                    {record.security_risk_level}
                  </span>
                {:else}
                  <span class="risk-badge risk-safe">Safe</span>
                {/if}
              </td>
              <td class="date-cell">{formatDate(record.created_at)}</td>
              <td class="actions-cell">
                <button class="btn-load" on:click={() => onSelectPayload(record.raw_payload)}>
                  Load
                </button>
                <button class="btn-del" on:click={() => deleteItem(record.id)}>
                  ✕
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <div class="empty-state">
        <p>No records match the current filter or history is empty.</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .history-container {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    height: 100%;
    overflow-y: auto;
  }

  .history-header h2 {
    font-size: 1.25rem;
    font-weight: 700;
  }

  .subtitle {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-top: 0.25rem;
  }

  .stats-row {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
  }

  .stat-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .stat-title {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .stat-val {
    font-size: 1.75rem;
    font-weight: 800;
    font-family: var(--font-mono);
    color: var(--accent-cyan);
  }

  .star-val {
    color: var(--accent-amber);
  }

  .threat-val {
    color: var(--accent-rose);
  }

  .filter-bar {
    display: flex;
    gap: 1rem;
    align-items: center;
  }

  .search-wrap {
    flex: 1;
  }

  .search-input {
    width: 100%;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: 8px;
    padding: 0.6rem 0.85rem;
    font-size: 0.85rem;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent-cyan);
  }

  .fav-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
    color: var(--accent-amber);
    cursor: pointer;
    white-space: nowrap;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    padding: 0.55rem 0.85rem;
    border-radius: 8px;
  }

  .error-banner {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: #fca5a5;
    padding: 0.75rem 1rem;
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .history-list-wrap {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    overflow: hidden;
  }

  .history-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .history-table th {
    background: var(--bg-elevated);
    padding: 0.75rem 1rem;
    text-align: left;
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    border-bottom: 1px solid var(--border-color);
  }

  .history-table td {
    padding: 0.65rem 1rem;
    border-bottom: 1px solid rgba(71, 85, 105, 0.2);
    vertical-align: middle;
  }

  .fav-cell {
    width: 32px;
  }

  .btn-star {
    background: transparent;
    border: none;
    font-size: 1.25rem;
    cursor: pointer;
    color: var(--text-muted);
  }

  .btn-star.starred {
    color: var(--accent-amber);
  }

  .type-pill {
    background: rgba(6, 182, 212, 0.15);
    color: var(--accent-cyan);
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
  }

  .source-pill {
    background: rgba(99, 102, 241, 0.15);
    color: var(--accent-indigo);
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
  }

  .source-pill.scan {
    background: rgba(16, 185, 129, 0.15);
    color: var(--accent-emerald);
  }

  .content-cell {
    max-width: 380px;
  }

  .record-label {
    font-weight: 600;
    color: var(--text-primary);
  }

  .record-preview {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .risk-badge {
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .risk-safe {
    background: rgba(16, 185, 129, 0.15);
    color: var(--accent-emerald);
  }

  .risk-critical {
    background: rgba(239, 68, 68, 0.15);
    color: var(--accent-rose);
  }

  .date-cell {
    font-size: 0.75rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .actions-cell {
    display: flex;
    gap: 0.4rem;
  }

  .btn-load {
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 0.3rem 0.6rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.75rem;
  }

  .btn-del {
    background: transparent;
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: var(--accent-rose);
    padding: 0.3rem 0.6rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.75rem;
  }

  .btn-del:hover {
    background: rgba(239, 68, 68, 0.15);
  }

  .empty-state {
    padding: 3rem;
    text-align: center;
    color: var(--text-muted);
  }
</style>
