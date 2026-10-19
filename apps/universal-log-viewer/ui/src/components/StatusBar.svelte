<script lang="ts">
  export let filePath: string = '';
  export let totalLines: number = 0;
  export let visibleStart: number = 0;
  export let visibleCount: number = 0;
  export let fileSizeBytes: number = 0;
  export let tailModeActive: boolean = false;

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }
</script>

<footer class="status-bar glass-panel">
  <!-- Left info -->
  <div class="status-group">
    <div class="status-indicator">
      <span class="status-dot"></span>
      <span class="status-label">Air-Gapped & Offline</span>
    </div>

    {#if filePath}
      <span class="divider">|</span>
      <span class="file-info">{filePath}</span>
    {/if}
  </div>

  <!-- Right stats -->
  <div class="status-group">
    {#if totalLines > 0}
      <span class="stat-item">
        Lines: <strong>{totalLines.toLocaleString()}</strong>
      </span>
      <span class="divider">|</span>
      <span class="stat-item">
        Viewing: <strong>{visibleStart + 1}</strong> – <strong>{Math.min(totalLines, visibleStart + visibleCount)}</strong>
      </span>
      <span class="divider">|</span>
      <span class="stat-item">
        Size: <strong>{formatBytes(fileSizeBytes)}</strong>
      </span>
    {/if}

    {#if tailModeActive}
      <span class="divider">|</span>
      <span class="tail-badge">
        <span class="live-indicator"></span>
        STREAMING
      </span>
    {/if}
  </div>
</footer>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 28px;
    padding: 0 16px;
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-muted);
    border-bottom: none;
    border-left: none;
    border-right: none;
    z-index: 50;
  }

  .status-group {
    display: flex;
    align-items: center;
    gap: 10px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--accent-emerald);
    font-weight: 500;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-emerald);
    box-shadow: 0 0 6px var(--accent-emerald);
  }

  .divider {
    opacity: 0.3;
  }

  .file-info {
    max-width: 380px;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-secondary);
  }

  .stat-item strong {
    color: var(--text-primary);
    font-weight: 600;
  }

  .tail-badge {
    display: flex;
    align-items: center;
    gap: 5px;
    font-weight: 700;
    color: var(--accent-emerald);
    letter-spacing: 0.5px;
  }
</style>
