<script lang="ts">
  import type { ComplianceScanSummary, SensitiveFinding } from '../types';

  export let currentPath: string;
  export let onScanDirectory: (path: string) => Promise<ComplianceScanSummary>;
  export let onScanFile: (path: string) => Promise<SensitiveFinding[]>;

  let scanPath = currentPath;
  $: scanPath = currentPath;

  let isScanning = false;
  let summary: ComplianceScanSummary | null = null;
  let errorMsg: string | null = null;
  let selectedSeverityFilter: 'all' | 'critical' | 'high' | 'medium' = 'all';

  async function handleStartScan() {
    if (!scanPath.trim()) return;
    isScanning = true;
    errorMsg = null;
    try {
      const trimmed = scanPath.trim();
      if (trimmed.includes('.') && !trimmed.endsWith('/') && !trimmed.endsWith('\\')) {
        // Likely a single file
        const findings = await onScanFile(trimmed);
        const critical = findings.filter(f => f.severity === 'critical').length;
        const high = findings.filter(f => f.severity === 'high').length;
        const medium = findings.filter(f => f.severity === 'medium').length;
        summary = {
          scanned_files_count: 1,
          findings_count: findings.length,
          critical_count: critical,
          high_count: high,
          medium_count: medium,
          findings,
          scan_duration_ms: 1.0,
        };
      } else {
        summary = await onScanDirectory(trimmed);
      }
    } catch (e: any) {
      errorMsg = e?.message || String(e);
    } finally {
      isScanning = false;
    }
  }

  $: filteredFindings = summary?.findings.filter(f => {
    if (selectedSeverityFilter === 'all') return true;
    return f.severity === selectedSeverityFilter;
  }) ?? [];

  function formatType(t: string): string {
    switch (t) {
      case 'private_key': return 'RSA/SSH Key';
      case 'aws_access_key': return 'AWS Access Key';
      case 'ssn': return 'SSN (PII)';
      case 'credit_card': return 'Credit Card';
      case 'connection_string_password': return 'DB Password';
      default: return t;
    }
  }
</script>

<div class="compliance-container glass-panel">
  <!-- Header Bar -->
  <div class="compliance-header">
    <div class="header-left">
      <div class="shield-badge">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path>
        </svg>
      </div>
      <div>
        <h2>PII & Sensitive Data Compliance Scanner</h2>
        <p class="subtitle">Air-gapped offline detection for private keys, AWS keys, SSNs, credit cards & passwords</p>
      </div>
    </div>
    <div class="tier-pill">Business Tier Active</div>
  </div>

  <!-- Search / Input Controls -->
  <div class="scan-bar">
    <input
      type="text"
      class="scan-input"
      bind:value={scanPath}
      placeholder="Directory or file path to scan..."
      disabled={isScanning}
    />
    <button class="btn btn-primary" on:click={handleStartScan} disabled={isScanning}>
      {#if isScanning}
        <span class="spinner"></span> Scanning...
      {:else}
        Scan Directory
      {/if}
    </button>
  </div>

  {#if errorMsg}
    <div class="error-banner">
      <strong>Error:</strong> {errorMsg}
    </div>
  {/if}

  {#if summary}
    <!-- Summary Stats -->
    <div class="stats-cards">
      <div class="stat-card">
        <span class="stat-num">{summary.scanned_files_count}</span>
        <span class="stat-label">Files Scanned</span>
      </div>
      <div class="stat-card {summary.findings_count > 0 ? 'alert' : 'clean'}">
        <span class="stat-num">{summary.findings_count}</span>
        <span class="stat-label">Total Violations</span>
      </div>
      <div class="stat-card crit">
        <span class="stat-num">{summary.critical_count}</span>
        <span class="stat-label">Critical (Keys)</span>
      </div>
      <div class="stat-card high">
        <span class="stat-num">{summary.high_count}</span>
        <span class="stat-label">High (SSN/CC)</span>
      </div>
      <div class="stat-card duration">
        <span class="stat-num">{summary.scan_duration_ms.toFixed(1)} ms</span>
        <span class="stat-label">Scan Latency</span>
      </div>
    </div>

    <!-- Filter Buttons -->
    <div class="filter-bar">
      <span class="filter-label">Filter by Severity:</span>
      <button
        class="filter-btn {selectedSeverityFilter === 'all' ? 'active' : ''}"
        on:click={() => (selectedSeverityFilter = 'all')}
      >
        All ({summary.findings_count})
      </button>
      <button
        class="filter-btn crit {selectedSeverityFilter === 'critical' ? 'active' : ''}"
        on:click={() => (selectedSeverityFilter = 'critical')}
      >
        Critical ({summary.critical_count})
      </button>
      <button
        class="filter-btn high {selectedSeverityFilter === 'high' ? 'active' : ''}"
        on:click={() => (selectedSeverityFilter = 'high')}
      >
        High ({summary.high_count})
      </button>
      <button
        class="filter-btn med {selectedSeverityFilter === 'medium' ? 'active' : ''}"
        on:click={() => (selectedSeverityFilter = 'medium')}
      >
        Medium ({summary.medium_count})
      </button>
    </div>

    <!-- Findings Table -->
    {#if filteredFindings.length === 0}
      <div class="empty-state">
        <div class="clean-check">✓</div>
        <h3>No Compliance Violations Detected</h3>
        <p>No unencrypted secrets or sensitive PII records were found matching the filter.</p>
      </div>
    {:else}
      <div class="findings-list">
        {#each filteredFindings as finding (finding.id)}
          <div class="finding-item sev-{finding.severity}">
            <div class="finding-header">
              <span class="badge sev-badge sev-{finding.severity}">{finding.severity.toUpperCase()}</span>
              <span class="type-badge">{formatType(finding.data_type)}</span>
              <span class="file-path" title={finding.file_path}>
                {finding.file_path} <span class="line-tag">Line {finding.line_number}</span>
              </span>
            </div>
            <div class="preview-box">
              <code>{finding.redacted_preview}</code>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .compliance-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 24px;
    margin: 0 16px 16px 16px;
    height: calc(100vh - 120px);
    overflow-y: auto;
  }

  .compliance-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-color, rgba(255, 255, 255, 0.1));
    padding-bottom: 16px;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .shield-badge {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 48px;
    height: 48px;
    border-radius: 12px;
    background: rgba(0, 229, 255, 0.15);
    color: #00e5ff;
  }

  h2 {
    margin: 0 0 4px 0;
    font-size: 1.25rem;
    color: #f1f5f9;
  }

  .subtitle {
    margin: 0;
    font-size: 0.85rem;
    color: #94a3b8;
  }

  .tier-pill {
    padding: 6px 14px;
    border-radius: 20px;
    background: linear-gradient(135deg, #10b981 0%, #059669 100%);
    color: white;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .scan-bar {
    display: flex;
    gap: 12px;
  }

  .scan-input {
    flex: 1;
    padding: 10px 16px;
    border-radius: 8px;
    border: 1px solid var(--border-color, rgba(255, 255, 255, 0.15));
    background: rgba(15, 23, 42, 0.6);
    color: #f8fafc;
    font-family: monospace;
    font-size: 0.9rem;
  }

  .error-banner {
    padding: 12px 16px;
    border-radius: 8px;
    background: rgba(239, 68, 68, 0.2);
    border: 1px solid #ef4444;
    color: #fca5a5;
    font-size: 0.9rem;
  }

  .stats-cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 12px;
  }

  .stat-card {
    padding: 14px;
    border-radius: 8px;
    background: rgba(30, 41, 59, 0.5);
    border: 1px solid rgba(255, 255, 255, 0.05);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .stat-card.alert {
    border-color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
  }

  .stat-card.clean {
    border-color: #10b981;
    background: rgba(16, 185, 129, 0.1);
  }

  .stat-card.crit .stat-num {
    color: #ef4444;
  }

  .stat-card.high .stat-num {
    color: #f59e0b;
  }

  .stat-card.duration .stat-num {
    color: #00e5ff;
  }

  .stat-num {
    font-size: 1.5rem;
    font-weight: 700;
    color: #f8fafc;
  }

  .stat-label {
    font-size: 0.75rem;
    color: #94a3b8;
    text-transform: uppercase;
  }

  .filter-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
  }

  .filter-label {
    font-size: 0.8rem;
    color: #94a3b8;
    margin-right: 4px;
  }

  .filter-btn {
    padding: 4px 12px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: transparent;
    color: #cbd5e1;
    font-size: 0.78rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .filter-btn:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .filter-btn.active {
    background: #00e5ff;
    color: #0b1120;
    font-weight: 600;
    border-color: #00e5ff;
  }

  .findings-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow-y: auto;
  }

  .finding-item {
    padding: 12px 16px;
    border-radius: 8px;
    background: rgba(15, 23, 42, 0.7);
    border-left: 4px solid #94a3b8;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .finding-item.sev-critical {
    border-left: 4px solid #ef4444;
  }

  .finding-item.sev-high {
    border-left: 4px solid #f59e0b;
  }

  .finding-item.sev-medium {
    border-left: 4px solid #3b82f6;
  }

  .finding-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
  }

  .badge {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.7rem;
    font-weight: 700;
  }

  .sev-badge.sev-critical {
    background: #ef4444;
    color: white;
  }

  .sev-badge.sev-high {
    background: #f59e0b;
    color: black;
  }

  .sev-badge.sev-medium {
    background: #3b82f6;
    color: white;
  }

  .type-badge {
    background: rgba(255, 255, 255, 0.08);
    color: #e2e8f0;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
  }

  .file-path {
    font-family: monospace;
    font-size: 0.85rem;
    color: #94a3b8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .line-tag {
    color: #64748b;
    font-size: 0.75rem;
    margin-left: 6px;
  }

  .preview-box {
    background: rgba(0, 0, 0, 0.4);
    padding: 8px 12px;
    border-radius: 6px;
    font-family: monospace;
    font-size: 0.8rem;
    color: #f1f5f9;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px;
    text-align: center;
  }

  .clean-check {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: rgba(16, 185, 129, 0.2);
    color: #10b981;
    font-size: 2rem;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 16px;
  }
</style>
