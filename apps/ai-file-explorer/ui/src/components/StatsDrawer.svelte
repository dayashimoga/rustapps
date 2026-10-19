<script lang="ts">
  import type { FolderStats } from '../types';

  export let stats: FolderStats | null;
  export let isOpen: boolean;
  export let onClose: () => void;

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function getSortedExtensions(stats: FolderStats | null): [string, number, number][] {
    if (!stats || !stats.extension_counts) return [];
    const entries = Object.entries(stats.extension_counts).map(([ext, count]) => {
      const size = stats.extension_sizes[ext] || 0;
      return [ext, count, size] as [string, number, number];
    });
    // Sort by total size descending
    entries.sort((a, b) => b[2] - a[2]);
    return entries.slice(0, 10);
  }

  $: sortedExts = getSortedExtensions(stats);
</script>

{#if isOpen}
  <div class="drawer-overlay" role="button" tabindex="0" on:click={onClose} on:keydown={(e) => e.key === 'Escape' && onClose()}>
    <div class="drawer glass-panel" role="presentation" on:click|stopPropagation on:keydown|stopPropagation>
      <div class="drawer-header">
        <div class="header-title">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 20V10"></path>
            <path d="M12 20V4"></path>
            <path d="M6 20v-6"></path>
          </svg>
          <h3>Storage Analytics</h3>
        </div>
        <button class="close-btn" on:click={onClose}>&times;</button>
      </div>

      {#if stats}
        <div class="drawer-body">
          <!-- Overview Cards -->
          <div class="stats-overview">
            <div class="overview-box">
              <span class="box-label">Total Size</span>
              <span class="box-val text-cyan">{formatBytes(stats.total_size_bytes)}</span>
            </div>
            <div class="overview-box">
              <span class="box-label">Total Files</span>
              <span class="box-val">{stats.total_files}</span>
            </div>
            <div class="overview-box">
              <span class="box-label">Directories</span>
              <span class="box-val">{stats.total_dirs}</span>
            </div>
          </div>

          <div class="path-label">
            Analyzed: <span class="font-mono">{stats.root_path}</span>
          </div>

          <!-- Extension Distribution -->
          <div class="section-box">
            <h4>File Type Distribution</h4>
            <div class="ext-list">
              {#each sortedExts as [ext, count, size]}
                {@const pct = stats.total_size_bytes > 0 ? (size / stats.total_size_bytes) * 100 : 0}
                <div class="ext-row">
                  <div class="ext-header">
                    <span class="badge badge-cyan">{ext || 'none'}</span>
                    <span class="ext-meta">{count} files ({formatBytes(size)})</span>
                  </div>
                  <div class="progress-track">
                    <div class="progress-fill" style="width: {pct.toFixed(1)}%;"></div>
                  </div>
                </div>
              {/each}
            </div>
          </div>

          <!-- Top Largest Files -->
          {#if stats.largest_files && stats.largest_files.length > 0}
            <div class="section-box">
              <h4>Largest Space Consumers</h4>
              <div class="largest-files-list">
                {#each stats.largest_files as file}
                  <div class="large-file-row">
                    <div class="large-file-name" title={file.file_name}>{file.file_name}</div>
                    <div class="large-file-size font-mono">{formatBytes(file.size_bytes)}</div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {:else}
        <div class="loading-state">
          Analyzing directory storage...
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .drawer-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(4px);
    z-index: 1000;
    display: flex;
    justify-content: flex-end;
  }

  .drawer {
    width: 420px;
    height: 100%;
    background: var(--bg-card);
    border-left: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    box-shadow: -12px 0 36px rgba(0, 0, 0, 0.7);
    animation: slideIn 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes slideIn {
    from { transform: translateX(100%); }
    to { transform: translateX(0); }
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 18px 20px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--accent-cyan);
  }
  .header-title h3 {
    font-size: 15px;
    font-weight: 700;
    color: var(--text-main);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 20px;
    cursor: pointer;
  }
  .close-btn:hover {
    color: var(--text-main);
  }

  .drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .stats-overview {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }

  .overview-box {
    background: rgba(11, 17, 30, 0.7);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .box-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-faint);
    text-transform: uppercase;
  }

  .box-val {
    font-family: var(--font-mono);
    font-size: 14px;
    font-weight: 700;
  }
  .text-cyan {
    color: var(--accent-cyan);
  }

  .path-label {
    font-size: 11px;
    color: var(--text-faint);
    word-break: break-all;
  }

  .section-box {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .section-box h4 {
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    color: var(--text-muted);
    letter-spacing: 0.05em;
  }

  .ext-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ext-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .ext-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .ext-meta {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-muted);
  }

  .progress-track {
    width: 100%;
    height: 4px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent-cyan), var(--accent-blue));
    border-radius: 2px;
  }

  .largest-files-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .large-file-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    background: rgba(11, 17, 30, 0.5);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    gap: 12px;
  }

  .large-file-name {
    font-size: 12px;
    color: var(--text-main);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .large-file-size {
    font-size: 11px;
    color: var(--accent-amber);
    white-space: nowrap;
  }

  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 200px;
    color: var(--text-faint);
  }

  .font-mono {
    font-family: var(--font-mono);
  }
</style>
