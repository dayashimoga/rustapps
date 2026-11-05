<script lang="ts">
  import { invokeIpc } from '../lib/ipc';

  export interface FuzzingResult {
    payload_index: number;
    payload: string;
    status: number;
    latency_ms: number;
    response_length: number;
    is_anomaly: boolean;
    anomaly_reason: string | null;
  }

  export let requestId: string;

  let strategy: string = 'sql_injection';
  let paramName: string = 'q';
  let isFuzzing: boolean = false;
  let results: FuzzingResult[] = [];
  let errorMsg: string = '';

  const strategies = [
    { id: 'sql_injection', name: 'SQL Injection Vectors' },
    { id: 'command_injection', name: 'Command & Shell Injection' },
    { id: 'buffer_overflow', name: 'Buffer Overflow Payloads' },
    { id: 'special_characters', name: 'Special / Unicode Sequences' },
    { id: 'boundary_values', name: 'Boundary Values & Numeric Extremes' },
  ];

  async function startFuzzing() {
    if (!requestId) return;
    isFuzzing = true;
    errorMsg = '';
    results = [];
    try {
      const res = await invokeIpc<FuzzingResult[]>('fuzz_request', {
        request_id: requestId,
        strategy,
        param_name: paramName,
      });
      results = res || [];
    } catch (e: any) {
      errorMsg = e?.message || 'Fuzzing campaign execution failed';
    } finally {
      isFuzzing = false;
    }
  }

  $: anomalyCount = results.filter((r) => r.is_anomaly).length;
</script>

<div class="fuzzer-container">
  <!-- Fuzzer Controls Header -->
  <div class="fuzzer-controls-card">
    <div class="control-col">
      <label for="strat-sel">Fuzzing Strategy</label>
      <select id="strat-sel" bind:value={strategy} class="styled-select" disabled={isFuzzing}>
        {#each strategies as s}
          <option value={s.id}>{s.name}</option>
        {/each}
      </select>
    </div>

    <div class="control-col">
      <label for="param-input">Target Parameter</label>
      <input
        id="param-input"
        type="text"
        bind:value={paramName}
        placeholder="Parameter name (e.g., id, q, token)"
        class="styled-input"
        disabled={isFuzzing}
      />
    </div>

    <div class="control-btn-col">
      <button class="btn btn-fuzz" on:click={startFuzzing} disabled={isFuzzing || !requestId}>
        {#if isFuzzing}
          <span class="spinner">⏳</span> Fuzzing Target...
        {:else}
          <span class="icon">⚡</span> Run Security Fuzzer
        {/if}
      </button>
    </div>
  </div>

  {#if errorMsg}
    <div class="alert error">{errorMsg}</div>
  {/if}

  <!-- Stats Bar -->
  {#if results.length > 0}
    <div class="fuzzer-stats-bar">
      <div class="stat-item">
        <span class="stat-num">{results.length}</span>
        <span class="stat-lbl">Vectors Tested</span>
      </div>
      <div class="stat-item {anomalyCount > 0 ? 'stat-alert' : 'stat-pass'}">
        <span class="stat-num">{anomalyCount}</span>
        <span class="stat-lbl">Anomalies Detected</span>
      </div>
      <div class="stat-item">
        <span class="stat-num">
          {Math.round(results.reduce((acc, r) => acc + r.latency_ms, 0) / (results.length || 1))} ms
        </span>
        <span class="stat-lbl">Avg Response Time</span>
      </div>
    </div>
  {/if}

  <!-- Results Table -->
  <div class="fuzzer-results-view">
    {#if isFuzzing}
      <div class="empty-state">
        <div class="spinner-large">⚡</div>
        <p>Air-Gapped fuzzer mutating payloads against local endpoint...</p>
      </div>
    {:else if results.length === 0}
      <div class="empty-state">
        <div class="empty-icon">🛡️</div>
        <p>Select a fuzzing strategy and target parameter to probe API robustness offline.</p>
      </div>
    {:else}
      <div class="table-wrap">
        <table class="fuzz-table">
          <thead>
            <tr>
              <th style="width: 50px;">#</th>
              <th>Payload</th>
              <th style="width: 80px;">Status</th>
              <th style="width: 90px;">Latency</th>
              <th style="width: 90px;">Size</th>
              <th style="width: 180px;">Security Finding</th>
            </tr>
          </thead>
          <tbody>
            {#each results as row}
              <tr class:row-anomaly={row.is_anomaly}>
                <td class="col-center">{row.payload_index + 1}</td>
                <td class="col-payload">
                  <code>{row.payload}</code>
                </td>
                <td class="col-center">
                  <span class="status-pill status-{row.status}">{row.status}</span>
                </td>
                <td class="col-center">{row.latency_ms} ms</td>
                <td class="col-center">{row.response_length} B</td>
                <td>
                  {#if row.is_anomaly}
                    <span class="anomaly-tag">⚠️ {row.anomaly_reason || 'Anomaly'}</span>
                  {:else}
                    <span class="pass-tag">✓ Normal</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
</div>

<style>
  .fuzzer-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #0f172a;
    padding: 16px;
    gap: 16px;
    overflow-y: auto;
  }
  .fuzzer-controls-card {
    display: flex;
    align-items: flex-end;
    gap: 16px;
    background: #1e293b;
    padding: 16px;
    border-radius: 8px;
    border: 1px solid rgba(75, 85, 99, 0.4);
  }
  .control-col {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }
  .control-col label {
    font-size: 0.8rem;
    font-weight: 500;
    color: #94a3b8;
  }
  .control-btn-col {
    flex-shrink: 0;
  }
  .styled-select, .styled-input {
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 6px;
    padding: 8px 12px;
    color: #f8fafc;
    font-size: 0.85rem;
  }
  .styled-select:focus, .styled-input:focus {
    outline: none;
    border-color: #8b5cf6;
  }
  .btn-fuzz {
    background: #8b5cf6;
    color: #ffffff;
    font-weight: 600;
    padding: 9px 18px;
    border-radius: 6px;
    border: none;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .btn-fuzz:hover:not(:disabled) {
    background: #7c3aed;
  }
  .btn-fuzz:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .fuzzer-stats-bar {
    display: flex;
    gap: 16px;
  }
  .stat-item {
    flex: 1;
    background: #1e293b;
    padding: 12px 16px;
    border-radius: 8px;
    border: 1px solid rgba(75, 85, 99, 0.3);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }
  .stat-num {
    font-size: 1.4rem;
    font-weight: 700;
    color: #f8fafc;
  }
  .stat-lbl {
    font-size: 0.75rem;
    color: #94a3b8;
  }
  .stat-alert .stat-num {
    color: #ef4444;
  }
  .stat-pass .stat-num {
    color: #10b981;
  }
  .alert.error {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
    padding: 10px 14px;
    border-radius: 6px;
    border: 1px solid rgba(239, 68, 68, 0.3);
    font-size: 0.85rem;
  }
  .fuzzer-results-view {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: #64748b;
    padding: 40px;
    text-align: center;
  }
  .empty-icon {
    font-size: 2.5rem;
  }
  .table-wrap {
    overflow-x: auto;
    border: 1px solid #1e293b;
    border-radius: 8px;
  }
  .fuzz-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  .fuzz-table th {
    background: #1e293b;
    color: #94a3b8;
    text-align: left;
    padding: 10px 12px;
    font-weight: 600;
    border-bottom: 1px solid #334155;
  }
  .fuzz-table td {
    padding: 8px 12px;
    border-bottom: 1px solid #1e293b;
    color: #e2e8f0;
  }
  .row-anomaly {
    background: rgba(239, 68, 68, 0.08);
  }
  .col-center {
    text-align: center;
  }
  .col-payload code {
    font-family: var(--font-mono);
    color: #a5f3fc;
    word-break: break-all;
  }
  .status-pill {
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 600;
    font-size: 0.75rem;
    background: #334155;
  }
  .anomaly-tag {
    color: #f87171;
    font-weight: 600;
    font-size: 0.8rem;
  }
  .pass-tag {
    color: #34d399;
    font-size: 0.8rem;
  }
</style>
