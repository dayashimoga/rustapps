<script lang="ts">
  import type { DeduplicationSummary, DuplicateGroup } from '../types';

  export let searchRoot: string;
  export let summary: DeduplicationSummary | null;
  export let isScanning: boolean;
  export let onStartScan: (root: string, minSize: number) => void;

  let minSizeFilter = 1024; // 1KB default
  let selectedDuplicateFiles: string[] = [];

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function handleStartScan() {
    if (searchRoot.trim()) {
      onStartScan(searchRoot.trim(), minSizeFilter);
    }
  }

  function toggleFileSelection(path: string) {
    if (selectedDuplicateFiles.includes(path)) {
      selectedDuplicateFiles = selectedDuplicateFiles.filter(p => p !== path);
    } else {
      selectedDuplicateFiles = [...selectedDuplicateFiles, path];
    }
  }

  function selectAllRedundant() {
    if (!summary) return;
    const toSelect: string[] = [];
    for (const group of summary.duplicate_groups) {
      // Keep the first file, select the rest
      if (group.file_paths.length > 1) {
        toSelect.push(...group.file_paths.slice(1));
      }
    }
    selectedDuplicateFiles = toSelect;
  }

  function clearDuplicateSelection() {
    selectedDuplicateFiles = [];
  }
</script>

<div class="dedup-container glass-panel">
  <!-- Scan Setup Header -->
  <div class="scan-header">
    <div class="setup-form">
      <div class="input-group">
        <label for="dedup-target-path">Target Search Directory</label>
        <input
          id="dedup-target-path"
          type="text"
          class="input-field"
          bind:value={searchRoot}
          placeholder="Folder to scan for duplicates..."
        />
      </div>

      <div class="input-group" style="width: 180px;">
        <label for="min-size-select">Minimum File Size</label>
        <select id="min-size-select" class="input-field select-field" bind:value={minSizeFilter}>
          <option value={0}>Any Size (> 0 B)</option>
          <option value={1024}>1 KB</option>
          <option value={1048576}>1 MB</option>
          <option value={10485760}>10 MB</option>
          <option value={104857600}>100 MB</option>
        </select>
      </div>

      <button
        class="btn btn-primary scan-btn"
        on:click={handleStartScan}
        disabled={isScanning}
      >
        {#if isScanning}
          <span class="spinner"></span> Scanning...
        {:else}
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline>
          </svg>
          Scan (3-Tier BLAKE3)
        {/if}
      </button>
    </div>
  </div>

  {#if summary}
    <!-- Summary Metric Cards -->
    <div class="summary-cards">
      <div class="metric-card highlight-cyan">
        <span class="metric-label">Reclaimable Space</span>
        <span class="metric-val">{formatBytes(summary.potential_reclaimed_bytes)}</span>
      </div>

      <div class="metric-card">
        <span class="metric-label">Duplicate Groups</span>
        <span class="metric-val">{summary.duplicate_groups.length}</span>
      </div>

      <div class="metric-card">
        <span class="metric-label">Duplicate Files</span>
        <span class="metric-val">{summary.total_duplicate_files}</span>
      </div>

      <div class="metric-card">
        <span class="metric-label">Scanned Files</span>
        <span class="metric-val">{summary.total_scanned_files} ({formatBytes(summary.total_scanned_bytes)})</span>
      </div>

      <div class="metric-card">
        <span class="metric-label">Scan Duration</span>
        <span class="metric-val">{summary.elapsed_millis} ms</span>
      </div>
    </div>

    <!-- Actions Bar -->
    {#if summary.duplicate_groups.length > 0}
      <div class="group-actions-bar">
        <div class="actions-left">
          <button class="btn btn-secondary" on:click={selectAllRedundant}>
            Select All Redundant (Keep 1st)
          </button>
          {#if selectedDuplicateFiles.length > 0}
            <button class="btn btn-secondary" on:click={clearDuplicateSelection}>
              Clear Selection ({selectedDuplicateFiles.length})
            </button>
          {/if}
        </div>

        {#if selectedDuplicateFiles.length > 0}
          <span class="reclaim-hint">
            Ready to reclaim: {formatBytes(selectedDuplicateFiles.length * (summary.duplicate_groups[0]?.file_size || 0))}
          </span>
        {/if}
      </div>

      <!-- Duplicate Groups List -->
      <div class="groups-list">
        {#each summary.duplicate_groups as group, idx}
          <div class="group-card">
            <div class="group-header">
              <div class="group-meta">
                <span class="group-index">Group #{idx + 1}</span>
                <span class="badge badge-cyan">{formatBytes(group.file_size)} each</span>
                <span class="badge badge-amber">{group.file_paths.length} copies</span>
                <span class="badge badge-rose">Wasted: {formatBytes(group.wasted_bytes)}</span>
              </div>
              <span class="hash-tag" title={group.blake3_hash}>
                BLAKE3: {group.blake3_hash.slice(0, 16)}...
              </span>
            </div>

            <div class="paths-list">
              {#each group.file_paths as path, pathIdx}
                {@const isSelected = selectedDuplicateFiles.includes(path)}
                <div class="path-row {isSelected ? 'selected' : ''}">
                  <input
                    type="checkbox"
                    checked={isSelected}
                    on:change={() => toggleFileSelection(path)}
                  />
                  {#if pathIdx === 0}
                    <span class="badge badge-emerald keep-badge">Primary</span>
                  {:else}
                    <span class="badge badge-amber keep-badge">Copy</span>
                  {/if}
                  <span class="path-text" title={path}>{path}</span>
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="empty-state">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="12" cy="12" r="10"></circle>
          <path d="m9 12 2 2 4-4"></path>
        </svg>
        <h3>No Duplicate Files Found</h3>
        <p>The scanned directory tree contains completely unique files.</p>
      </div>
    {/if}
  {:else if !isScanning}
    <div class="empty-state initial">
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <rect width="18" height="18" x="3" y="3" rx="2"></rect>
        <path d="M7 7h10"></path>
        <path d="M7 12h10"></path>
        <path d="M7 17h10"></path>
      </svg>
      <h3>Ready to Detect Duplicates</h3>
      <p>Uses a deterministic 3-tier filtration pipeline (Size &rarr; 4KB Header/Footer Hash &rarr; Full BLAKE3) to eliminate 95%+ of unnecessary disk I/O.</p>
    </div>
  {/if}
</div>

<style>
  .dedup-container {
    display: flex;
    flex-direction: column;
    flex: 1;
    margin: 0 16px 16px 16px;
    padding: 16px;
    overflow-y: auto;
    gap: 16px;
  }

  .scan-header {
    background: rgba(11, 17, 30, 0.4);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 16px;
  }

  .setup-form {
    display: flex;
    align-items: flex-end;
    gap: 12px;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }

  .input-group label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .select-field {
    cursor: pointer;
    background: rgba(11, 17, 30, 0.95);
  }

  .scan-btn {
    height: 42px;
    padding: 0 20px;
    white-space: nowrap;
  }

  .summary-cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 12px;
  }

  .metric-card {
    background: rgba(11, 17, 30, 0.6);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 14px 18px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .highlight-cyan {
    border-color: rgba(6, 182, 212, 0.4);
    background: rgba(6, 182, 212, 0.05);
  }

  .metric-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .metric-val {
    font-family: var(--font-mono);
    font-size: 18px;
    font-weight: 700;
    color: var(--text-main);
  }

  .group-actions-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: rgba(11, 17, 30, 0.4);
    border-radius: var(--radius-md);
  }

  .actions-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .reclaim-hint {
    font-size: 12px;
    color: var(--accent-cyan);
    font-weight: 500;
  }

  .groups-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .group-card {
    background: rgba(11, 17, 30, 0.7);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .group-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: rgba(255, 255, 255, 0.02);
    border-bottom: 1px solid var(--border-subtle);
  }

  .group-meta {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .group-index {
    font-size: 12px;
    font-weight: 700;
    color: var(--text-main);
  }

  .hash-tag {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-faint);
  }

  .paths-list {
    padding: 8px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .path-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    transition: background 0.15s ease;
  }
  .path-row:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .path-row.selected {
    background: rgba(244, 63, 94, 0.1);
  }

  .path-row input {
    cursor: pointer;
    accent-color: var(--accent-rose);
  }

  .keep-badge {
    font-size: 10px;
  }

  .path-text {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    text-align: center;
    color: var(--text-faint);
    gap: 12px;
  }
  .empty-state h3 {
    color: var(--text-main);
    font-size: 16px;
  }
  .empty-state p {
    font-size: 13px;
    max-width: 480px;
  }

  .spinner {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-radius: 50%;
    border-top-color: white;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
