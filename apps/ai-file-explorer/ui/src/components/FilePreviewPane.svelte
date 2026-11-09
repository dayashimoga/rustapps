<script lang="ts">
  import { onMount } from 'svelte';
  import type { FileItem, FilePreviewData } from '../types';
  import { invokeIpc, getLocalFileUrl } from '../lib/ipc';

  export let item: FileItem;
  export let onClose: () => void = () => {};
  export let onOpenInOs: (path: string) => void = () => {};
  export let onInspectDoc: (path: string) => void = () => {};

  let preview: FilePreviewData | null = null;
  let isLoading = false;
  let loadError = '';
  let copiedHash = false;
  let copiedPath = false;
  let copiedContent = false;
  let isZoomed = false;
  let isHashing = false;

  $: if (item && item.path) {
    loadPreview(item.path);
  }

  async function loadPreview(path: string) {
    isLoading = true;
    loadError = '';
    preview = null;
    isHashing = false;
    try {
      preview = await invokeIpc<FilePreviewData>('get_file_preview_cmd', { path });
    } catch (e: any) {
      loadError = e?.message || String(e);
    } finally {
      isLoading = false;
    }
  }

  async function computeHash() {
    if (!preview || isHashing) return;
    isHashing = true;
    try {
      const hash = await invokeIpc<string>('compute_file_hash_cmd', { path: preview.path });
      if (preview && hash) {
        preview.blake3_hash = hash;
      }
    } catch (err) {
      console.error('Failed to compute hash:', err);
    } finally {
      isHashing = false;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function formatDate(secs: number): string {
    if (!secs) return '—';
    return new Date(secs * 1000).toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  }

  function getMediaUrl(path: string): string {
    return getLocalFileUrl(path);
  }

  async function copyToClipboard(text: string, type: 'path' | 'hash' | 'content') {
    try {
      await navigator.clipboard.writeText(text);
      if (type === 'path') {
        copiedPath = true;
        setTimeout(() => (copiedPath = false), 2000);
      } else if (type === 'hash') {
        copiedHash = true;
        setTimeout(() => (copiedHash = false), 2000);
      } else if (type === 'content') {
        copiedContent = true;
        setTimeout(() => (copiedContent = false), 2000);
      }
    } catch (e) {
      console.warn('Clipboard copy failed:', e);
    }
  }
</script>

<aside class="preview-pane glass-panel">
  <!-- Pane Header -->
  <div class="pane-header">
    <div class="header-title-box">
      <span class="preview-badge">FILE PREVIEW</span>
      <h3 class="file-title" title={item.file_name}>{item.file_name}</h3>
    </div>
    <div class="header-actions">
      <button class="icon-btn-sm" on:click={() => copyToClipboard(item.path, 'path')} title="Copy Full Path">
        {copiedPath ? '✓ Copied' : '📋 Path'}
      </button>
      <button class="icon-btn-sm" on:click={() => onOpenInOs(item.path)} title="Open in Default Application">
        ↗ Open
      </button>
      <button class="close-btn" on:click={onClose} title="Close Preview Pane">✕</button>
    </div>
  </div>

  <!-- Content Viewer -->
  <div class="pane-body">
    {#if isLoading}
      <div class="preview-loading">
        <div class="spinner"></div>
        <span>Loading preview...</span>
      </div>
    {:else if loadError}
      <div class="preview-error">
        <span>⚠️ Unable to generate preview: {loadError}</span>
        <button class="btn btn-secondary" on:click={() => onOpenInOs(item.path)}>Open with OS</button>
      </div>
    {:else if preview}
      <!-- Image Viewer -->
      {#if preview.is_image}
        <div class="image-viewer-box">
          <div class="image-controls">
            <button class="chip-sm" on:click={() => (isZoomed = !isZoomed)}>
              {isZoomed ? 'Fit to Screen' : 'Zoom 100%'}
            </button>
            <span class="meta-pill">{preview.extension.toUpperCase()}</span>
          </div>
          <div class="image-viewport {isZoomed ? 'zoomed' : ''}">
            <img src={getMediaUrl(preview.path)} alt={preview.file_name} loading="lazy" />
          </div>
        </div>

      <!-- PDF Viewer -->
      {:else if preview.is_pdf}
        <div class="pdf-viewer-box">
          <div class="pdf-banner">
            <div class="pdf-meta-left">
              <span class="pdf-badge">PDF DOCUMENT</span>
              {#if preview.pdf_pages}
                <span class="meta-pill">{preview.pdf_pages} pages</span>
              {/if}
            </div>
            <button class="inspect-btn" on:click={() => onInspectDoc(item.path)}>
              🔍 Full Document Inspector
            </button>
          </div>
          {#if preview.pdf_title || preview.pdf_author}
            <div class="pdf-info-row">
              {#if preview.pdf_title}<strong>{preview.pdf_title}</strong>{/if}
              {#if preview.pdf_author}<span>by {preview.pdf_author}</span>{/if}
            </div>
          {/if}
          <div class="text-viewport">
            <pre class="extracted-text">{preview.text_content || 'No readable text layer extracted from this PDF.'}</pre>
          </div>
        </div>

      <!-- Video / Audio Media Player -->
      {:else if preview.is_media}
        <div class="media-viewer-box">
          {#if ['mp4', 'webm', 'mov', 'mkv', 'avi'].includes(preview.extension)}
            <video controls src={getMediaUrl(preview.path)} class="video-player">
              <track kind="captions" />
            </video>
          {:else}
            <div class="audio-player-box">
              <div class="audio-icon">🎵</div>
              <audio controls src={getMediaUrl(preview.path)} class="audio-player"></audio>
            </div>
          {/if}
        </div>

      <!-- Text / Code Viewer -->
      {:else if preview.is_text && preview.text_content}
        <div class="text-viewer-box">
          <div class="code-header">
            <div class="code-meta">
              <span class="meta-pill">{preview.extension ? preview.extension.toUpperCase() : 'TEXT'}</span>
              {#if preview.total_lines}
                <span class="meta-text">{preview.total_lines} lines</span>
              {/if}
            </div>
            <button class="chip-sm" on:click={() => copyToClipboard(preview?.text_content || '', 'content')}>
              {copiedContent ? '✓ Copied' : 'Copy All'}
            </button>
          </div>
          <div class="code-viewport">
            <pre class="code-content"><code>{preview.text_content}</code></pre>
          </div>
        </div>

      <!-- Archive Viewer (.zip, .tar, .gz) -->
      {:else if preview.is_archive}
        <div class="archive-viewer-box">
          <div class="archive-header">
            <span class="archive-icon">🗜️</span>
            <div>
              <h4>Archive Package Contents</h4>
              <p class="archive-sub">
                {preview.archive_entries ? preview.archive_entries.length : 0} items listed
              </p>
            </div>
          </div>
          {#if preview.archive_entries && preview.archive_entries.length > 0}
            <div class="archive-table-container">
              <table class="archive-table">
                <thead>
                  <tr>
                    <th>Entry Name</th>
                    <th style="text-align: right;">Size</th>
                  </tr>
                </thead>
                <tbody>
                  {#each preview.archive_entries as entry}
                    <tr>
                      <td class="entry-name-cell">
                        <span class="entry-icon">{entry.is_dir ? '📁' : '📄'}</span>
                        <span class="entry-name" title={entry.name}>{entry.name}</span>
                      </td>
                      <td class="entry-size-cell">{entry.is_dir ? '—' : formatBytes(entry.size_bytes)}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {:else}
            <div class="archive-empty">No entries or unreadable archive header.</div>
          {/if}
        </div>

      <!-- Table Viewer (.csv, .tsv, .sqlite, .db) -->
      {:else if preview.is_table && preview.table_preview}
        <div class="archive-viewer-box">
          <div class="archive-header">
            <span class="archive-icon">📊</span>
            <div>
              <h4>Table & Schema Preview</h4>
              <p class="archive-sub">
                {preview.table_preview.total_rows} rows &bull; {preview.table_preview.total_columns} columns
              </p>
            </div>
          </div>
          <div class="archive-table-container">
            <table class="archive-table">
              <thead>
                <tr>
                  {#each preview.table_preview.headers as header}
                    <th>{header}</th>
                  {/each}
                </tr>
              </thead>
              <tbody>
                {#each preview.table_preview.rows as row}
                  <tr>
                    {#each row as cell}
                      <td style="max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{cell}</td>
                    {/each}
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>

      <!-- Binary / Fallback File -->
      {:else}
        <div class="binary-viewer-box">
          <div class="binary-icon">📦</div>
          <h4>{preview.file_name}</h4>
          <span class="meta-pill">{preview.extension ? preview.extension.toUpperCase() : 'BINARY'} FILE</span>
          <p class="binary-desc">Binary or non-text document ready for external execution or workflow dispatch.</p>
          <button class="btn btn-primary" on:click={() => onOpenInOs(item.path)}>
            🚀 Launch in Default Application
          </button>
        </div>
      {/if}

      <!-- Metadata & Hash Card -->
      <div class="metadata-card">
        <h4 class="card-heading">File Properties</h4>
        <div class="meta-grid">
          <div class="meta-item">
            <span class="meta-label">File Size:</span>
            <span class="meta-value">{formatBytes(preview.size_bytes)} ({preview.size_bytes.toLocaleString()} bytes)</span>
          </div>
          <div class="meta-item">
            <span class="meta-label">Modified:</span>
            <span class="meta-value">{formatDate(preview.modified_timestamp_secs)}</span>
          </div>
          <div class="meta-item full-width">
            <span class="meta-label">Location:</span>
            <span class="meta-value path-val" title={preview.path}>{preview.path}</span>
          </div>
          <div class="meta-item full-width">
            <span class="meta-label">BLAKE3 Hash:</span>
            <div class="hash-row">
              {#if preview.blake3_hash}
                <code class="hash-code">{preview.blake3_hash}</code>
                <button class="hash-copy-btn" on:click={() => copyToClipboard(preview?.blake3_hash || '', 'hash')}>
                  {copiedHash ? '✓' : 'Copy'}
                </button>
              {:else if isHashing}
                <span class="hash-computing">Computing 256-bit hash...</span>
              {:else}
                <button class="compute-hash-btn" on:click={computeHash}>Compute BLAKE3 Hash</button>
              {/if}
            </div>
          </div>
        </div>
      </div>
    {/if}
  </div>
</aside>

<style>
  .preview-pane {
    width: 380px;
    min-width: 320px;
    max-width: 460px;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border-color);
    background: rgba(13, 17, 23, 0.95);
    height: 100%;
    overflow: hidden;
    animation: slideIn 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateX(20px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  .pane-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border-color);
    background: rgba(22, 27, 34, 0.85);
  }

  .header-title-box {
    overflow: hidden;
    flex: 1;
  }

  .preview-badge {
    font-size: 0.62rem;
    font-weight: 700;
    color: #38bdf8;
    letter-spacing: 0.05em;
  }

  .file-title {
    margin: 0;
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-main);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .icon-btn-sm {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border-color);
    color: #cbd5e1;
    font-size: 0.72rem;
    padding: 3px 7px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .icon-btn-sm:hover {
    background: rgba(255, 255, 255, 0.12);
    color: white;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    cursor: pointer;
    font-size: 0.95rem;
    padding: 4px;
  }

  .close-btn:hover {
    color: white;
  }

  .pane-body {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: 0.85rem;
    gap: 0.85rem;
  }

  .preview-loading,
  .preview-error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    gap: 0.8rem;
    color: #94a3b8;
    text-align: center;
    font-size: 0.82rem;
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 3px solid rgba(56, 189, 248, 0.2);
    border-top-color: #38bdf8;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .image-viewer-box {
    display: flex;
    flex-direction: column;
    background: #090d16;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    overflow: hidden;
  }

  .image-controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    background: rgba(255, 255, 255, 0.03);
    border-bottom: 1px solid var(--border-color);
  }

  .image-viewport {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 10px;
    max-height: 280px;
    overflow: auto;
  }

  .image-viewport img {
    max-width: 100%;
    max-height: 260px;
    object-fit: contain;
    border-radius: 4px;
  }

  .image-viewport.zoomed img {
    max-width: none;
    max-height: none;
  }

  .pdf-viewer-box,
  .text-viewer-box {
    display: flex;
    flex-direction: column;
    background: #090d16;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    overflow: hidden;
    max-height: 320px;
  }

  .pdf-banner,
  .code-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    background: rgba(255, 255, 255, 0.03);
    border-bottom: 1px solid var(--border-color);
  }

  .pdf-badge {
    background: rgba(239, 68, 68, 0.2);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: #f87171;
    font-size: 0.65rem;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .meta-pill {
    background: rgba(56, 189, 248, 0.15);
    border: 1px solid rgba(56, 189, 248, 0.35);
    color: #38bdf8;
    font-size: 0.65rem;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .inspect-btn {
    background: #2563eb;
    border: none;
    color: white;
    font-size: 0.7rem;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
  }

  .inspect-btn:hover {
    background: #1d4ed8;
  }

  .pdf-info-row {
    padding: 6px 10px;
    font-size: 0.75rem;
    color: #cbd5e1;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    gap: 6px;
  }

  .text-viewport,
  .code-viewport {
    flex: 1;
    overflow: auto;
    padding: 8px 10px;
    background: #05080f;
  }

  .extracted-text,
  .code-content {
    margin: 0;
    font-family: var(--font-mono, monospace);
    font-size: 0.75rem;
    line-height: 1.45;
    color: #e2e8f0;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .media-viewer-box {
    background: #090d16;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .video-player {
    width: 100%;
    max-height: 240px;
    border-radius: 4px;
  }

  .audio-player-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
    gap: 8px;
  }

  .audio-icon {
    font-size: 2.2rem;
  }

  .audio-player {
    width: 100%;
  }

  .binary-viewer-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 1.5rem 1rem;
    background: #090d16;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    gap: 0.5rem;
  }

  .binary-icon {
    font-size: 2.5rem;
  }

  .binary-desc {
    font-size: 0.74rem;
    color: #94a3b8;
    margin: 0;
  }

  .metadata-card {
    background: rgba(22, 27, 34, 0.8);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .card-heading {
    margin: 0;
    font-size: 0.78rem;
    font-weight: 700;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .meta-grid {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .meta-item {
    display: flex;
    justify-content: space-between;
    font-size: 0.74rem;
    gap: 6px;
  }

  .meta-item.full-width {
    flex-direction: column;
    align-items: flex-start;
  }

  .meta-label {
    color: #64748b;
  }

  .meta-value {
    color: #cbd5e1;
    font-weight: 500;
  }

  .path-val {
    word-break: break-all;
    font-family: var(--font-mono, monospace);
    font-size: 0.7rem;
    color: #94a3b8;
  }

  .hash-row {
    display: flex;
    align-items: center;
    width: 100%;
    gap: 6px;
    margin-top: 2px;
  }

  .hash-code {
    flex: 1;
    font-size: 0.68rem;
    background: #05080f;
    padding: 3px 6px;
    border-radius: 4px;
    border: 1px solid #1e293b;
    color: #38bdf8;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .hash-copy-btn,
  .chip-sm {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid var(--border-color);
    color: #cbd5e1;
    font-size: 0.68rem;
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
  }

  .hash-copy-btn:hover,
  .chip-sm:hover,
  .compute-hash-btn:hover {
    background: rgba(255, 255, 255, 0.16);
    color: white;
  }

  .compute-hash-btn {
    background: rgba(59, 130, 246, 0.15);
    border: 1px solid rgba(59, 130, 246, 0.35);
    color: #93c5fd;
    font-size: 0.72rem;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .hash-computing {
    font-size: 0.72rem;
    color: #94a3b8;
    font-style: italic;
  }

  .archive-viewer-box {
    display: flex;
    flex-direction: column;
    gap: 12px;
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 12px;
  }

  .archive-header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .archive-icon {
    font-size: 24px;
  }

  .archive-header h4 {
    margin: 0;
    font-size: 0.88rem;
    color: #f1f5f9;
  }

  .archive-sub {
    margin: 2px 0 0 0;
    font-size: 0.72rem;
    color: #94a3b8;
  }

  .archive-table-container {
    max-height: 220px;
    overflow-y: auto;
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
  }

  .archive-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.75rem;
  }

  .archive-table th {
    position: sticky;
    top: 0;
    background: #0d1322;
    padding: 6px 10px;
    color: #94a3b8;
    font-weight: 500;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    text-align: left;
  }

  .archive-table td {
    padding: 6px 10px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }

  .entry-name-cell {
    display: flex;
    align-items: center;
    gap: 6px;
    color: #e2e8f0;
    max-width: 200px;
  }

  .entry-icon {
    font-size: 13px;
  }

  .entry-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono, monospace);
    font-size: 0.72rem;
  }

  .entry-size-cell {
    color: #94a3b8;
    font-family: var(--font-mono, monospace);
    font-size: 0.72rem;
    text-align: right;
  }

  .archive-empty {
    padding: 16px;
    text-align: center;
    font-size: 0.75rem;
    color: #64748b;
  }
</style>
