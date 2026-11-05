<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export interface TriageSummary {
    triage_id: string;
    host_name: string;
    os_type: string;
    collected_at: number;
    total_processes: number;
    total_sockets: number;
    total_files_scanned: number;
    total_logs_analyzed: number;
    threats_detected: number;
    threat_score: number;
    severity: 'Informational' | 'Low' | 'Medium' | 'High' | 'Critical';
  }

  export let summary: TriageSummary | null = null;
  export let isScanning: boolean = false;

  const dispatch = createEventDispatcher<{
    runTriage: void;
  }>();

  function getSeverityColor(sev: string): string {
    switch (sev) {
      case 'Critical':
        return 'var(--accent-red)';
      case 'High':
        return '#f97316';
      case 'Medium':
        return 'var(--accent-yellow)';
      case 'Low':
        return 'var(--accent-cyan)';
      default:
        return 'var(--accent-green)';
    }
  }

  function getScoreColor(score: number): string {
    if (score >= 80) return 'var(--accent-red)';
    if (score >= 50) return '#f97316';
    if (score >= 20) return 'var(--accent-yellow)';
    return 'var(--accent-green)';
  }
</script>

<div class="summary-card">
  <div class="card-top">
    <div class="header-left">
      <div class="host-pill">
        <span class="icon">💻</span>
        <span class="host-name">{summary ? summary.host_name : 'LOCAL-SYSTEM'}</span>
        <span class="os-tag">{summary ? summary.os_type : 'Offline'}</span>
      </div>
      <div class="triage-id">
        {summary ? summary.triage_id : 'No active triage record'}
      </div>
    </div>

    <div class="actions">
      <button class="scan-btn" on:click={() => dispatch('runTriage')} disabled={isScanning}>
        {#if isScanning}
          <span class="spinner"></span> Scanning System...
        {:else}
          ⚡ Run Live Triage Sweep
        {/if}
      </button>
    </div>
  </div>

  {#if summary}
    <div class="metrics-row">
      <!-- Threat Score Gauge -->
      <div class="gauge-box">
        <div class="gauge-header">
          <span class="gauge-label">Threat Severity Score</span>
          <span
            class="severity-badge"
            style="background: {getSeverityColor(summary.severity)}22; color: {getSeverityColor(summary.severity)}; border-color: {getSeverityColor(summary.severity)}"
          >
            {summary.severity.toUpperCase()}
          </span>
        </div>
        <div class="score-display">
          <span class="score-number" style="color: {getScoreColor(summary.threat_score)}">
            {summary.threat_score}
          </span>
          <span class="score-max">/100</span>
        </div>
        <div class="meter-bar">
          <div
            class="meter-fill"
            style="width: {summary.threat_score}%; background: {getScoreColor(summary.threat_score)}"
          ></div>
        </div>
      </div>

      <!-- Quick Metrics Grid -->
      <div class="stats-grid">
        <div class="stat-box">
          <span class="stat-label">Processes</span>
          <span class="stat-value">{summary.total_processes}</span>
        </div>
        <div class="stat-box">
          <span class="stat-label">Active Sockets</span>
          <span class="stat-value">{summary.total_sockets}</span>
        </div>
        <div class="stat-box">
          <span class="stat-label">Files Scanned</span>
          <span class="stat-value">{summary.total_files_scanned}</span>
        </div>
        <div class="stat-box">
          <span class="stat-label">Logs Analyzed</span>
          <span class="stat-value">{summary.total_logs_analyzed}</span>
        </div>
        <div class="stat-box threat">
          <span class="stat-label">Flagged Anomalies</span>
          <span class="stat-value" class:danger={summary.threats_detected > 0}>
            {summary.threats_detected}
          </span>
        </div>
      </div>
    </div>
  {:else}
    <div class="empty-state">
      <p>No forensic triage run loaded. Click <strong>Run Live Triage Sweep</strong> to acquire live host telemetry.</p>
    </div>
  {/if}
</div>

<style>
  .summary-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .card-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .host-pill {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-secondary);
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid var(--border);
  }

  .host-name {
    font-weight: 700;
    font-size: 0.95rem;
    color: var(--text-main);
  }

  .os-tag {
    font-size: 0.72rem;
    background: var(--border);
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .triage-id {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--text-sub);
  }

  .scan-btn {
    background: linear-gradient(135deg, #2563eb, #1d4ed8);
    color: #fff;
    border: 1px solid #3b82f6;
    padding: 8px 18px;
    border-radius: 6px;
    font-weight: 600;
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: background 0.15s ease;
  }

  .scan-btn:hover:not(:disabled) {
    background: #1d4ed8;
  }

  .scan-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .metrics-row {
    display: grid;
    grid-template-columns: 260px 1fr;
    gap: 16px;
  }

  @media (max-width: 860px) {
    .metrics-row {
      grid-template-columns: 1fr;
    }
  }

  .gauge-box {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .gauge-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .gauge-label {
    font-size: 0.75rem;
    color: var(--text-sub);
    text-transform: uppercase;
    font-weight: 600;
  }

  .severity-badge {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 700;
    border: 1px solid transparent;
  }

  .score-display {
    display: flex;
    align-items: baseline;
    gap: 4px;
  }

  .score-number {
    font-size: 2.2rem;
    font-weight: 800;
    font-family: var(--font-mono);
    line-height: 1;
  }

  .score-max {
    font-size: 0.9rem;
    color: var(--text-sub);
  }

  .meter-bar {
    width: 100%;
    height: 8px;
    background: var(--border);
    border-radius: 4px;
    overflow: hidden;
  }

  .meter-fill {
    height: 100%;
    transition: width 0.3s ease;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
    gap: 10px;
  }

  .stat-box {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 4px;
  }

  .stat-label {
    font-size: 0.72rem;
    color: var(--text-sub);
    text-transform: uppercase;
  }

  .stat-value {
    font-size: 1.35rem;
    font-weight: 700;
    font-family: var(--font-mono);
    color: var(--text-main);
  }

  .stat-value.danger {
    color: var(--accent-red);
  }

  .empty-state {
    padding: 20px;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.9rem;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
