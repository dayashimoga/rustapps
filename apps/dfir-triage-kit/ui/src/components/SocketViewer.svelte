<script lang="ts">
  export interface SocketArtifact {
    protocol: 'Tcp' | 'Udp';
    local_addr: string;
    remote_addr: string | null;
    state: 'Listening' | 'Established' | 'TimeWait' | 'CloseWait' | 'Unknown';
    pid: number;
    process_name: string;
    is_external: boolean;
  }

  export let sockets: SocketArtifact[] = [];

  let filterExternalOnly = false;
  let searchFilter = '';

  $: filteredSockets = sockets.filter((s) => {
    if (filterExternalOnly && !s.is_external) return false;
    if (!searchFilter) return true;
    const q = searchFilter.toLowerCase();
    return (
      s.local_addr.toLowerCase().includes(q) ||
      (s.remote_addr && s.remote_addr.toLowerCase().includes(q)) ||
      s.process_name.toLowerCase().includes(q) ||
      s.pid.toString().includes(q)
    );
  });
</script>

<div class="socket-container">
  <div class="controls-bar">
    <div class="search-box">
      <span class="icon">🌐</span>
      <input
        type="text"
        bind:value={searchFilter}
        placeholder="Filter by local/remote address, state, or PID..."
      />
      {#if searchFilter}
        <button class="clear-btn" on:click={() => (searchFilter = '')}>✕</button>
      {/if}
    </div>

    <div class="filter-actions">
      <label class="toggle-label">
        <input type="checkbox" bind:checked={filterExternalOnly} />
        <span>External Egress Points Only ({sockets.filter((s) => s.is_external).length})</span>
      </label>
      <span class="count-tag">Showing {filteredSockets.length} of {sockets.length} active sockets</span>
    </div>
  </div>

  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th style="width: 70px;">Proto</th>
          <th>Local Address</th>
          <th>Remote Address</th>
          <th style="width: 120px;">State</th>
          <th style="width: 90px;">PID</th>
          <th style="width: 140px;">Process</th>
          <th style="width: 120px;">Classification</th>
        </tr>
      </thead>
      <tbody>
        {#if filteredSockets.length === 0}
          <tr>
            <td colspan="7" class="empty-cell">No network sockets matching current filter.</td>
          </tr>
        {:else}
          {#each filteredSockets as s}
            <tr class:external-row={s.is_external}>
              <td>
                <span class="proto-badge" class:tcp={s.protocol === 'Tcp'}>
                  {s.protocol.toUpperCase()}
                </span>
              </td>
              <td class="mono">{s.local_addr}</td>
              <td class="mono">
                {#if s.remote_addr}
                  <span class="remote-addr" class:remote-ext={s.is_external}>{s.remote_addr}</span>
                {:else}
                  <span class="muted-dash">*</span>
                {/if}
              </td>
              <td>
                <span class="state-pill state-{s.state.toLowerCase()}">{s.state}</span>
              </td>
              <td class="mono">{s.pid}</td>
              <td>{s.process_name}</td>
              <td>
                {#if s.is_external}
                  <span class="badge warning">⚠️ EXTERNAL</span>
                {:else}
                  <span class="badge internal">INTERNAL</span>
                {/if}
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>
</div>

<style>
  .socket-container {
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

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.82rem;
    color: var(--text-muted);
    cursor: pointer;
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

  .external-row td {
    background: rgba(245, 158, 11, 0.06);
  }

  .external-row:hover td {
    background: rgba(245, 158, 11, 0.12);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .proto-badge {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 700;
    background: var(--border);
    color: var(--text-muted);
  }

  .proto-badge.tcp {
    background: rgba(59, 130, 246, 0.2);
    color: var(--accent-blue);
  }

  .remote-addr.remote-ext {
    color: var(--accent-yellow);
    font-weight: 600;
  }

  .state-pill {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 600;
  }

  .state-listening {
    background: rgba(6, 182, 212, 0.15);
    color: var(--accent-cyan);
  }

  .state-established {
    background: rgba(16, 185, 129, 0.15);
    color: var(--accent-green);
  }

  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 700;
  }

  .badge.internal {
    background: rgba(148, 163, 184, 0.1);
    color: var(--text-sub);
  }

  .badge.warning {
    background: rgba(245, 158, 11, 0.2);
    color: var(--accent-yellow);
    border: 1px solid rgba(245, 158, 11, 0.35);
  }

  .muted-dash {
    color: var(--border);
  }

  .empty-cell {
    text-align: center;
    padding: 30px;
    color: var(--text-sub);
  }
</style>

