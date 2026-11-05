<script lang="ts">
  export interface ProcessArtifact {
    pid: number;
    ppid: number;
    name: string;
    exe_path: string;
    command_line: string;
    sha256: string;
    user: string;
    cpu_percent: number;
    memory_mb: number;
    is_suspicious: bool;
    threat_indicator: string | null;
  }

  export let processes: ProcessArtifact[] = [];

  let searchQuery = '';
  let onlySuspicious = false;

  $: filteredProcesses = processes.filter((p) => {
    if (onlySuspicious && !p.is_suspicious) return false;
    if (!searchQuery) return true;
    const q = searchQuery.toLowerCase();
    return (
      p.name.toLowerCase().includes(q) ||
      p.pid.toString().includes(q) ||
      p.user.toLowerCase().includes(q) ||
      p.command_line.toLowerCase().includes(q)
    );
  });
</script>

<div class="proc-container">
  <div class="controls-bar">
    <div class="search-box">
      <span class="icon">🔍</span>
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Filter by PID, process name, command, or user..."
      />
      {#if searchQuery}
        <button class="clear-btn" on:click={() => (searchQuery = '')}>✕</button>
      {/if}
    </div>

    <div class="filter-actions">
      <label class="toggle-label">
        <input type="checkbox" bind:checked={onlySuspicious} />
        <span>Flagged / Suspicious Only ({processes.filter((p) => p.is_suspicious).length})</span>
      </label>
      <span class="count-tag">Showing {filteredProcesses.length} of {processes.length} processes</span>
    </div>
  </div>

  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th style="width: 80px;">PID</th>
          <th>Process Name</th>
          <th style="width: 140px;">User</th>
          <th style="width: 110px;">Memory</th>
          <th style="width: 90px;">CPU</th>
          <th style="width: 130px;">Threat Status</th>
          <th>Anomaly Indicators</th>
        </tr>
      </thead>
      <tbody>
        {#if filteredProcesses.length === 0}
          <tr>
            <td colspan="7" class="empty-cell">No matching processes found.</td>
          </tr>
        {:else}
          {#each filteredProcesses as proc}
            <tr class:suspicious-row={proc.is_suspicious}>
              <td class="mono">{proc.pid}</td>
              <td>
                <div class="name-cell">
                  <span class="proc-name">{proc.name}</span>
                  <span class="proc-path">{proc.exe_path}</span>
                </div>
              </td>
              <td>{proc.user}</td>
              <td class="mono">{proc.memory_mb} MB</td>
              <td class="mono">{proc.cpu_percent.toFixed(1)}%</td>
              <td>
                {#if proc.is_suspicious}
                  <span class="badge danger">⚠️ SUSPICIOUS</span>
                {:else}
                  <span class="badge normal">CLEAN</span>
                {/if}
              </td>
              <td class="threat-note">
                {#if proc.threat_indicator}
                  <span class="threat-text">{proc.threat_indicator}</span>
                {:else}
                  <span class="muted-dash">—</span>
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
  .proc-container {
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

  .suspicious-row td {
    background: rgba(239, 68, 68, 0.08);
  }

  .suspicious-row:hover td {
    background: rgba(239, 68, 68, 0.15);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .name-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .proc-name {
    color: var(--text-main);
    font-weight: 600;
  }

  .proc-path {
    font-size: 0.72rem;
    color: var(--text-sub);
    font-family: var(--font-mono);
  }

  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 700;
  }

  .badge.normal {
    background: rgba(16, 185, 129, 0.15);
    color: var(--accent-green);
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .badge.danger {
    background: rgba(239, 68, 68, 0.2);
    color: var(--accent-red);
    border: 1px solid rgba(239, 68, 68, 0.4);
  }

  .threat-note {
    font-size: 0.78rem;
  }

  .threat-text {
    color: #fca5a5;
    font-weight: 500;
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

