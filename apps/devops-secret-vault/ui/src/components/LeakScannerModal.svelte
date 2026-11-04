<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let isOpen = false;
  export let onScanDirectory: (dirPath: string) => Promise<any[]>;

  const dispatch = createEventDispatcher<{
    close: void;
    saveSecret: { title: string; type: string; value: string };
  }>();

  let scanPath = '.';
  let isScanning = false;
  let findings: Array<{
    file_path: string;
    line_number: number;
    leak_type: string;
    severity: string;
    snippet: string;
  }> = [];
  let filterSeverity: string = 'ALL';
  let scanError = '';
  let scannedAt = '';

  $: filteredFindings = findings.filter((f) => {
    if (filterSeverity === 'ALL') return true;
    return f.severity.toUpperCase() === filterSeverity.toUpperCase();
  });

  async function browseFolder() {
    try {
      const tauri = (window as any).__TAURI__;
      if (tauri?.dialog?.open) {
        const picked = await tauri.dialog.open({
          directory: true,
          multiple: false,
          title: 'Select Folder / Repo to Scan for Leaks',
        });
        if (typeof picked === 'string' && picked.trim()) {
          scanPath = picked.trim();
          return;
        }
      }
      if (typeof fetch !== 'undefined' && typeof window !== 'undefined' && window.location?.protocol.startsWith('http')) {
        const res = await fetch('/__dialog/open', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            directory: true,
            title: 'Select Folder / Repo to Scan for Leaks',
          }),
        });
        if (res.ok) {
          const data = await res.json();
          if (data && data.path) {
            scanPath = data.path;
            return;
          }
        }
      }
    } catch (e) {
      console.warn('Dialog error:', e);
    }
  }

  async function handleStartScan() {
    isScanning = true;
    scanError = '';
    try {
      findings = await onScanDirectory(scanPath);
      scannedAt = new Date().toLocaleTimeString();
    } catch (e: any) {
      scanError = e?.message || String(e);
    } finally {
      isScanning = false;
    }
  }

  function getSeverityClass(sev: string): string {
    switch (sev.toUpperCase()) {
      case 'CRITICAL':
        return 'badge-critical';
      case 'HIGH':
        return 'badge-high';
      case 'MEDIUM':
        return 'badge-medium';
      default:
        return 'badge-low';
    }
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="modal-backdrop" on:click={() => dispatch('close')}>
    <div class="scanner-modal glass-panel" on:click|stopPropagation>
      <div class="modal-header">
        <div class="header-title-box">
          <span class="shield-icon">🛡️</span>
          <div>
            <h3>DevOps Secret Leak Scanner</h3>
            <p class="subtitle">Scan repositories &amp; folders offline for leaked API keys, tokens, private keys, and connection strings</p>
          </div>
        </div>
        <button class="btn-close" on:click={() => dispatch('close')}>✕</button>
      </div>

      <div class="scanner-controls">
        <div class="path-input-group">
          <input
            type="text"
            class="input-path"
            placeholder="Repository root (e.g. . or H:\rustapps)..."
            bind:value={scanPath}
          />
          <button class="btn btn-secondary btn-browse" on:click={browseFolder} title="Browse Directory">
            📁 Browse
          </button>
          <button class="btn btn-primary btn-scan" on:click={handleStartScan} disabled={isScanning}>
            {isScanning ? 'Scanning Directory...' : '🔍 Scan for Leaks'}
          </button>
        </div>

        {#if findings.length > 0}
          <div class="filter-row">
            <span class="filter-label">Filter:</span>
            {#each ['ALL', 'CRITICAL', 'HIGH', 'MEDIUM'] as sev}
              <button
                class="pill-btn {filterSeverity === sev ? 'active' : ''} {sev.toLowerCase()}"
                on:click={() => (filterSeverity = sev)}
              >
                {sev}
              </button>
            {/each}
            <span class="findings-count">{filteredFindings.length} finding(s)</span>
          </div>
        {/if}
      </div>

      {#if scanError}
        <div class="error-banner">⚠️ {scanError}</div>
      {/if}

      <div class="findings-container">
        {#if isScanning}
          <div class="loading-state">
            <div class="spinner"></div>
            <p>Scanning directory files and checking regex pattern signatures...</p>
          </div>
        {:else if findings.length === 0}
          <div class="empty-state">
            {#if scannedAt}
              <div class="clean-badge">✅ Clean Repository</div>
              <p>No leaked secrets, private keys, or exposed credentials detected at {scannedAt}!</p>
            {:else}
              <p>Enter a folder path or repository root above and click <strong>Scan for Leaks</strong>.</p>
            {/if}
          </div>
        {:else}
          <div class="findings-table-wrapper">
            <table class="findings-table">
              <thead>
                <tr>
                  <th style="width: 100px;">Severity</th>
                  <th style="width: 180px;">Type</th>
                  <th>Location</th>
                  <th>Snippet</th>
                  <th style="width: 90px; text-align: right;">Action</th>
                </tr>
              </thead>
              <tbody>
                {#each filteredFindings as f}
                  <tr>
                    <td>
                      <span class="badge {getSeverityClass(f.severity)}">{f.severity}</span>
                    </td>
                    <td class="type-cell">{f.leak_type}</td>
                    <td class="loc-cell" title={f.file_path}>
                      <span class="file-name">{f.file_path.split(/[/\\]/).pop()}</span>
                      <span class="line-num">:{f.line_number}</span>
                    </td>
                    <td class="snippet-cell">
                      <code>{f.snippet}</code>
                    </td>
                    <td class="action-cell">
                      <button
                        class="btn-sm btn-vault"
                        title="Import into Vault"
                        on:click={() => dispatch('saveSecret', {
                          title: `Imported ${f.leak_type} from ${f.file_path.split(/[/\\]/).pop()}`,
                          type: 'api_key',
                          value: f.snippet,
                        })}
                      >
                        + Vault
                      </button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <span class="footer-note">100% Offline • Zero external network egress or API telemetry</span>
        <button class="btn btn-secondary" on:click={() => dispatch('close')}>Close</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .scanner-modal {
    width: 820px;
    max-height: 85vh;
    border-radius: 14px;
    display: flex;
    flex-direction: column;
    padding: 24px;
    background: hsla(222, 24%, 7%, 0.98);
    border: 1px solid var(--border-color);
  }

  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 18px;
  }

  .header-title-box {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .shield-icon {
    font-size: 28px;
  }

  .header-title-box h3 {
    font-size: 18px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .subtitle {
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 2px;
  }

  .btn-close {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 18px;
    cursor: pointer;
  }

  .scanner-controls {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 16px;
  }

  .path-input-group {
    display: flex;
    gap: 10px;
  }

  .input-path {
    flex: 1;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 8px 12px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
    outline: none;
  }

  .btn-scan {
    padding: 8px 16px;
    font-weight: 600;
    font-size: 13px;
    white-space: nowrap;
  }

  .filter-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
  }

  .filter-label {
    color: var(--text-muted);
    font-weight: 600;
  }

  .pill-btn {
    padding: 3px 8px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 11px;
    font-weight: 600;
  }

  .pill-btn.active {
    background: hsla(217, 91%, 60%, 0.25);
    border-color: var(--accent-blue);
    color: #fff;
  }

  .pill-btn.critical.active {
    background: rgba(239, 68, 68, 0.3);
    border-color: #ef4444;
    color: #fca5a5;
  }

  .findings-count {
    margin-left: auto;
    color: var(--text-muted);
  }

  .error-banner {
    padding: 8px 12px;
    background: rgba(239, 68, 68, 0.2);
    border: 1px solid rgba(239, 68, 68, 0.4);
    border-radius: 6px;
    color: #fca5a5;
    font-size: 12px;
    margin-bottom: 12px;
  }

  .findings-container {
    flex: 1;
    overflow-y: auto;
    min-height: 240px;
    max-height: 400px;
    background: rgba(0, 0, 0, 0.25);
    border-radius: 8px;
    border: 1px solid var(--border-color);
  }

  .loading-state, .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 220px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .clean-badge {
    padding: 6px 14px;
    border-radius: 9999px;
    background: rgba(16, 185, 129, 0.16);
    border: 1px solid rgba(16, 185, 129, 0.4);
    color: #34d399;
    font-weight: 600;
    margin-bottom: 8px;
  }

  .findings-table-wrapper {
    overflow-x: auto;
  }

  .findings-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
    text-align: left;
  }

  .findings-table th {
    position: sticky;
    top: 0;
    background: hsla(222, 24%, 10%, 0.98);
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-color);
    color: var(--text-muted);
    font-size: 11px;
    text-transform: uppercase;
  }

  .findings-table td {
    padding: 8px 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }

  .badge {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
  }

  .badge-critical {
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
    border: 1px solid rgba(239, 68, 68, 0.4);
  }

  .badge-high {
    background: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
    border: 1px solid rgba(245, 158, 11, 0.4);
  }

  .badge-medium {
    background: rgba(59, 130, 246, 0.2);
    color: #60a5fa;
    border: 1px solid rgba(59, 130, 246, 0.4);
  }

  .loc-cell {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-primary);
  }

  .line-num {
    color: var(--accent-cyan);
  }

  .snippet-cell code {
    font-family: var(--font-mono);
    color: #fca5a5;
    background: rgba(239, 68, 68, 0.08);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
  }

  .btn-vault {
    background: rgba(16, 185, 129, 0.16);
    border: 1px solid rgba(16, 185, 129, 0.4);
    color: #34d399;
    border-radius: 4px;
    padding: 2px 8px;
    font-size: 11px;
    cursor: pointer;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--border-color);
  }

  .footer-note {
    font-size: 11px;
    color: var(--text-muted);
  }
</style>
