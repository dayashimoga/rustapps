<script lang="ts">
  import { invokeIpc } from '../lib/ipc';

  export interface FileIntegrityRecord {
    path: string;
    file_size_bytes: number;
    blake3_hash: string;
    sha256_hash: string;
    entropy: number;
    is_suspicious: boolean;
    anomaly_notes: string | null;
  }

  export let files: FileIntegrityRecord[] = [];

  let customPath = '';
  let maxDepth = 2;
  let isScanningCustom = false;
  let scanError = '';
  let showOnlySuspicious = false;

  function getEntropyColor(entropy: number): string {
    if (entropy > 7.3) return 'var(--accent-red)';
    if (entropy > 6.0) return 'var(--accent-yellow)';
    return 'var(--accent-green)';
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
  }

  async function handleScanDirectory() {
    if (!customPath.trim()) {
      scanError = 'Please enter a valid directory path to scan.';
      return;
    }
    isScanningCustom = true;
    scanError = '';
    try {
      const records = await invokeIpc<FileIntegrityRecord[]>('scan_directory', {
        path: customPath.trim(),
        max_depth: maxDepth,
      });
      files = records;
    } catch (e: any) {
      scanError = e?.message || 'Directory integrity scan failed.';
    } finally {
      isScanningCustom = false;
    }
  }

  $: displayFiles = files.filter((f) => {
    if (showOnlySuspicious && !f.is_suspicious) return false;
    return true;
  });
</script>

<div class="files-container">
  <div class="scan-bar">
    <div class="path-input-group">
      <span class="icon">📁</span>
      <input
        type="text"
        bind:value={customPath}
        placeholder="Enter absolute directory path to scan (e.g. C:\Windows\Temp or /tmp)..."
      />
      <div class="depth-select">
        <label for="depth-select">Depth:</label>
        <select id="depth-select" bind:value={maxDepth}>
          <option value={1}>1</option>
          <option value={2}>2</option>
          <option value={3}>3</option>
          <option value={5}>5</option>
        </select>
      </div>
      <button class="scan-btn" on:click={handleScanDirectory} disabled={isScanningCustom}>
        {isScanningCustom ? 'Scanning...' : 'Scan Directory'}
      </button>
    </div>

    <div class="filter-actions">
      <label class="toggle-label">
        <input type="checkbox" bind:checked={showOnlySuspicious} />
        <span>Flagged Anomalies Only ({files.filter((f) => f.is_suspicious).length})</span>
      </label>
      <span class="count-tag">Total Scanned: {displayFiles.length} files</span>
    </div>
  </div>

  {#if scanError}
    <div class="alert error">{scanError}</div>
  {/if}

  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th>File Path</th>
          <th style="width: 100px;">Size</th>
          <th style="width: 150px;">Entropy (0-8)</th>
          <th style="width: 150px;">BLAKE3 Hash</th>
          <th style="width: 110px;">Status</th>
          <th>Integrity & Anomaly Notes</th>
        </tr>
      </thead>
      <tbody>
        {#if displayFiles.length === 0}
          <tr>
            <td colspan="6" class="empty-cell">No file integrity records loaded.</td>
          </tr>
        {:else}
          {#each displayFiles as file}
            <tr class:suspicious-row={file.is_suspicious}>
              <td>
                <span class="file-path">{file.path}</span>
              </td>
              <td class="mono">{formatBytes(file.file_size_bytes)}</td>
              <td>
                <div class="entropy-cell">
                  <span class="entropy-val mono" style="color: {getEntropyColor(file.entropy)}">
                    {file.entropy.toFixed(2)}
                  </span>
                  <div class="entropy-meter">
                    <div
                      class="entropy-fill"
                      style="width: {(file.entropy / 8.0) * 100}%; background: {getEntropyColor(file.entropy)}"
                    ></div>
                  </div>
                </div>
              </td>
              <td class="mono hash-cell" title={file.blake3_hash}>
                {file.blake3_hash.slice(0, 14)}...
              </td>
              <td>
                {#if file.is_suspicious}
                  <span class="badge danger">⚠️ ANOMALY</span>
                {:else}
                  <span class="badge normal">CLEAN</span>
                {/if}
              </td>
              <td class="notes-cell">
                {#if file.anomaly_notes}
                  <span class="note-text">{file.anomaly_notes}</span>
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
  .files-container {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .scan-bar {
    padding: 12px 16px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .path-input-group {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .path-input-group input {
    flex: 1;
    min-width: 280px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 7px 12px;
    color: var(--text-main);
    font-size: 0.85rem;
    outline: none;
  }

  .path-input-group input:focus {
    border-color: var(--border-focus);
  }

  .depth-select {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .depth-select select {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 6px 8px;
    color: var(--text-main);
    font-size: 0.8rem;
  }

  .scan-btn {
    background: var(--accent-blue);
    color: #fff;
    border: 1px solid var(--accent-blue);
    padding: 7px 14px;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 600;
  }

  .scan-btn:hover:not(:disabled) {
    background: #2563eb;
  }

  .filter-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
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

  .alert.error {
    margin: 12px 16px 0;
    padding: 10px;
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid var(--accent-red);
    color: #fca5a5;
    border-radius: 6px;
    font-size: 0.82rem;
  }

  .table-wrap {
    overflow-x: auto;
    max-height: calc(100vh - 400px);
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

  .file-path {
    font-family: var(--font-mono);
    color: var(--text-main);
    font-size: 0.8rem;
    word-break: break-all;
  }

  .entropy-cell {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .entropy-val {
    font-size: 0.8rem;
    font-weight: 700;
  }

  .entropy-meter {
    width: 100%;
    height: 6px;
    background: var(--border);
    border-radius: 3px;
    overflow: hidden;
  }

  .entropy-fill {
    height: 100%;
  }

  .hash-cell {
    font-size: 0.78rem;
    color: var(--text-sub);
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

  .note-text {
    font-size: 0.78rem;
    color: #fca5a5;
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
