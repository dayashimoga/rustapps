<script lang="ts">
  export interface LogEventRecord {
    timestamp: number;
    source: string;
    event_id: number;
    level: 'Debug' | 'Info' | 'Warning' | 'Error' | 'Critical';
    message: string;
    user_id: string | null;
  }

  export let logs: LogEventRecord[] = [];

  let selectedLevel = 'ALL';
  let searchQuery = '';

  function getLevelBadgeClass(level: string): string {
    switch (level) {
      case 'Critical':
        return 'badge-critical';
      case 'Error':
        return 'badge-error';
      case 'Warning':
        return 'badge-warning';
      default:
        return 'badge-info';
    }
  }

  function formatTimestamp(ts: number): string {
    return new Date(ts * 1000).toLocaleString();
  }

  $: filteredLogs = logs.filter((l) => {
    if (selectedLevel !== 'ALL' && l.level.toUpperCase() !== selectedLevel) {
      return false;
    }
    if (!searchQuery) return true;
    const q = searchQuery.toLowerCase();
    return (
      l.source.toLowerCase().includes(q) ||
      l.message.toLowerCase().includes(q) ||
      l.event_id.toString().includes(q)
    );
  });
</script>

<div class="logs-container">
  <div class="controls-bar">
    <div class="search-box">
      <span class="icon">📜</span>
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Search event logs by source, message, event ID..."
      />
      {#if searchQuery}
        <button class="clear-btn" on:click={() => (searchQuery = '')}>✕</button>
      {/if}
    </div>

    <div class="filter-actions">
      <div class="level-tabs">
        <button
          class="level-tab"
          class:active={selectedLevel === 'ALL'}
          on:click={() => (selectedLevel = 'ALL')}
        >
          All ({logs.len || logs.length})
        </button>
        <button
          class="level-tab crit"
          class:active={selectedLevel === 'CRITICAL'}
          on:click={() => (selectedLevel = 'CRITICAL')}
        >
          Critical
        </button>
        <button
          class="level-tab err"
          class:active={selectedLevel === 'ERROR'}
          on:click={() => (selectedLevel = 'ERROR')}
        >
          Error
        </button>
        <button
          class="level-tab warn"
          class:active={selectedLevel === 'WARNING'}
          on:click={() => (selectedLevel = 'WARNING')}
        >
          Warning
        </button>
        <button
          class="level-tab inf"
          class:active={selectedLevel === 'INFO'}
          on:click={() => (selectedLevel = 'INFO')}
        >
          Info
        </button>
      </div>
      <span class="count-tag">Showing {filteredLogs.length} events</span>
    </div>
  </div>

  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th style="width: 170px;">Timestamp</th>
          <th style="width: 90px;">Level</th>
          <th style="width: 140px;">Source</th>
          <th style="width: 90px;">Event ID</th>
          <th style="width: 120px;">User</th>
          <th>Event Description</th>
        </tr>
      </thead>
      <tbody>
        {#if filteredLogs.length === 0}
          <tr>
            <td colspan="6" class="empty-cell">No event logs matching current criteria.</td>
          </tr>
        {:else}
          {#each filteredLogs as log}
            <tr class="level-row-{log.level.toLowerCase()}">
              <td class="mono">{formatTimestamp(log.timestamp)}</td>
              <td>
                <span class="badge {getLevelBadgeClass(log.level)}">{log.level.toUpperCase()}</span>
              </td>
              <td><span class="source-tag">{log.source}</span></td>
              <td class="mono">{log.event_id}</td>
              <td class="user-cell">{log.user_id || 'SYSTEM'}</td>
              <td class="msg-cell">{log.message}</td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>
</div>

<style>
  .logs-container {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .controls-bar {
    padding: 12px 16px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
  }

  .search-box {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 10px;
    width: 360px;
    max-width: 100%;
  }

  .search-box input {
    background: transparent;
    border: none;
    color: var(--text-main);
    font-size: 0.85rem;
    width: 100%;
    outline: none;
  }

  .clear-btn {
    background: transparent;
    border: none;
    color: var(--text-sub);
    font-size: 0.8rem;
  }

  .filter-actions {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .level-tabs {
    display: flex;
    gap: 4px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 3px;
  }

  .level-tab {
    background: transparent;
    border: none;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 0.75rem;
    color: var(--text-muted);
    font-weight: 600;
  }

  .level-tab:hover {
    color: var(--text-main);
  }

  .level-tab.active {
    background: var(--border);
    color: var(--text-main);
  }

  .level-tab.crit.active {
    background: rgba(239, 68, 68, 0.25);
    color: var(--accent-red);
  }

  .level-tab.err.active {
    background: rgba(249, 115, 22, 0.25);
    color: #f97316;
  }

  .level-tab.warn.active {
    background: rgba(245, 158, 11, 0.25);
    color: var(--accent-yellow);
  }

  .count-tag {
    font-size: 0.78rem;
    color: var(--text-sub);
  }

  .table-wrap {
    overflow-x: auto;
    max-height: calc(100vh - 360px);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.83rem;
  }

  th {
    position: sticky;
    top: 0;
    background: #0f1422;
    padding: 10px 14px;
    text-align: left;
    color: var(--text-sub);
    font-size: 0.72rem;
    text-transform: uppercase;
    font-weight: 600;
    border-bottom: 1px solid var(--border);
    z-index: 10;
  }

  td {
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    color: var(--text-muted);
  }

  tr:hover td {
    background: var(--bg-card-hover);
  }

  .level-row-critical td {
    background: rgba(239, 68, 68, 0.08);
  }

  .level-row-error td {
    background: rgba(249, 115, 22, 0.06);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .source-tag {
    font-weight: 600;
    color: var(--text-main);
  }

  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 700;
  }

  .badge-critical {
    background: rgba(239, 68, 68, 0.25);
    color: var(--accent-red);
    border: 1px solid rgba(239, 68, 68, 0.5);
  }

  .badge-error {
    background: rgba(249, 115, 22, 0.25);
    color: #f97316;
  }

  .badge-warning {
    background: rgba(245, 158, 11, 0.2);
    color: var(--accent-yellow);
  }

  .badge-info {
    background: rgba(59, 130, 246, 0.15);
    color: var(--accent-blue);
  }

  .user-cell {
    font-family: var(--font-mono);
    font-size: 0.78rem;
  }

  .msg-cell {
    color: var(--text-main);
  }

  .empty-cell {
    text-align: center;
    padding: 30px;
    color: var(--text-sub);
  }
</style>
