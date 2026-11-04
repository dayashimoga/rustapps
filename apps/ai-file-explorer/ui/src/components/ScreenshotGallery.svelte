<script lang="ts">
  import { onMount } from 'svelte';
  import type { ScreenshotFingerprint, ScreenshotSearchResult } from '../types';
  import { invokeIpc } from '../lib/ipc';

  export let currentPath: string = '';
  export let onInspectDoc: (path: string) => void = () => {};
  export let onScanFolder: (path: string) => Promise<ScreenshotFingerprint[]> = async (path) => {
    return await invokeIpc<ScreenshotFingerprint[]>('scan_screenshots_dir_cmd', { path });
  };
  export let onSearchSimilar: (
    hash: number | bigint,
    maxDist: number,
    limit: number
  ) => Promise<ScreenshotSearchResult[]> = async (hash, maxDist, limit) => {
    return await invokeIpc<ScreenshotSearchResult[]>('search_screenshots_by_hash_cmd', {
      target_hash: hash,
      max_distance: maxDist,
      limit,
    });
  };
  export let onSearchOcr: (
    query: string,
    limit: number
  ) => Promise<ScreenshotSearchResult[]> = async (query, limit) => {
    return await invokeIpc<ScreenshotSearchResult[]>('search_screenshots_by_text_cmd', {
      query,
      limit,
    });
  };

  let screenshots: ScreenshotFingerprint[] = [];
  let isScanning = false;
  let ocrQuery = '';
  let statusMessage = '';
  let activeSimilarTarget: ScreenshotFingerprint | null = null;
  let similarMatches: ScreenshotSearchResult[] = [];
  let isSearchingSimilar = false;
  let ocrSearchResults: ScreenshotSearchResult[] | null = null;
  let isSearchingOcr = false;

  async function handleScanCurrentFolder() {
    if (!currentPath) return;
    isScanning = true;
    statusMessage = 'Scanning folder for images & extracting OCR...';
    try {
      screenshots = await onScanFolder(currentPath);
      activeSimilarTarget = null;
      similarMatches = [];
      ocrSearchResults = null;
      if (screenshots.length === 0) {
        statusMessage = `No image files (.png, .jpg, .bmp, .webp) found in: ${currentPath}`;
      } else {
        statusMessage = `Indexed ${screenshots.length} screenshot(s) with 64-bit dHash & OCR tokens.`;
      }
    } catch (e: unknown) {
      const err = e as { message?: string };
      statusMessage = `Scan failed: ${err.message || String(e)}`;
    } finally {
      isScanning = false;
    }
  }

  async function handleSearchOcrText() {
    const trimmed = ocrQuery.trim();
    if (!trimmed) {
      ocrSearchResults = null;
      return;
    }

    isSearchingOcr = true;
    try {
      ocrSearchResults = await onSearchOcr(trimmed, 50);
      statusMessage = `Found ${ocrSearchResults.length} screenshot match(es) for text "${trimmed}".`;
    } catch (e: unknown) {
      const err = e as { message?: string };
      statusMessage = `OCR search failed: ${err.message || String(e)}`;
    } finally {
      isSearchingOcr = false;
    }
  }

  function handleClearOcrSearch() {
    ocrQuery = '';
    ocrSearchResults = null;
  }

  async function handleFindSimilar(target: ScreenshotFingerprint) {
    activeSimilarTarget = target;
    isSearchingSimilar = true;
    try {
      const hits = await onSearchSimilar(target.dhash, 16, 20);
      // Filter out exact match itself
      similarMatches = hits.filter((h) => h.path !== target.path);
      statusMessage = `Found ${similarMatches.length} visually similar image(s) within Hamming distance ≤ 16.`;
    } catch (e: unknown) {
      const err = e as { message?: string };
      statusMessage = `Similarity search failed: ${err.message || String(e)}`;
    } finally {
      isSearchingSimilar = false;
    }
  }

  function handleClearSimilarTarget() {
    activeSimilarTarget = null;
    similarMatches = [];
  }

  function formatDhash(hash: number | bigint): string {
    const big = BigInt(hash) & 0xffffffffffffffffn;
    return '0x' + big.toString(16).toUpperCase().padStart(16, '0');
  }

  onMount(() => {
    if (currentPath && screenshots.length === 0) {
      handleScanCurrentFolder();
    }
  });
</script>

<div class="screenshot-gallery-container">
  <div class="gallery-header-panel">
    <div class="header-info-row">
      <div>
        <h2>Screenshot Search & Visual Perceptual Hashing</h2>
        <p class="subtitle">
          Search screenshots by extracted OCR text or match visually similar images using 64-bit difference hashing (&lt;1µs).
        </p>
      </div>
      <button class="index-btn" on:click={handleScanCurrentFolder} disabled={isScanning}>
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          <circle cx="8.5" cy="8.5" r="1.5" />
          <polyline points="21 15 16 10 5 21" />
        </svg>
        {isScanning ? 'Scanning Screenshots...' : 'Scan Screenshot Folder'}
      </button>
    </div>

    <!-- Search Bar -->
    <div class="search-input-box">
      <svg class="search-icon" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8" />
        <line x1="21" y1="21" x2="16.65" y2="16.65" />
      </svg>
      <input
        type="text"
        placeholder="Filter by OCR text (e.g. 'PostgreSQL port', 'Kubectl logs', 'GitHub Actions')... Press Enter to search"
        bind:value={ocrQuery}
        on:keydown={(e) => e.key === 'Enter' && handleSearchOcrText()}
      />
      {#if ocrQuery}
        <button class="clear-search-btn" on:click={handleClearOcrSearch} title="Clear text search">✕</button>
      {/if}
      <button class="search-btn" on:click={handleSearchOcrText} disabled={isSearchingOcr}>
        {isSearchingOcr ? 'Searching...' : 'Search Text'}
      </button>
    </div>

    {#if statusMessage}
      <div class="status-banner">
        <span>{statusMessage}</span>
      </div>
    {/if}
  </div>

  <!-- Similar Match Drawer / Alert if active -->
  {#if activeSimilarTarget}
    <div class="similar-banner">
      <div class="banner-content">
        <span class="banner-title">
          Visual Similarity Matches for: <strong>{activeSimilarTarget.path.split('/').pop()}</strong>
          <span class="target-hash">({formatDhash(activeSimilarTarget.dhash)})</span>
        </span>
        <button class="close-banner-btn" on:click={handleClearSimilarTarget}>Clear Target</button>
      </div>

      {#if isSearchingSimilar}
        <div class="searching-indicator">Computing Hamming distances across catalog...</div>
      {:else if similarMatches.length === 0}
        <div class="no-matches-msg">No visually similar screenshots found within Hamming distance threshold.</div>
      {:else}
        <div class="similar-chips">
          {#each similarMatches as match}
            <div
              class="match-chip {match.hamming_distance <= 10 ? 'high-match' : ''}"
              on:click={() => onInspectDoc(match.path)}
              role="button"
              tabindex="0"
              on:keydown={(e) => e.key === 'Enter' && onInspectDoc(match.path)}
              title="Click to inspect document details"
            >
              <span class="pct">{match.similarity_pct.toFixed(1)}% Match</span>
              <span class="dist">(Hamming: {match.hamming_distance})</span>
              <span class="name">{match.path.split('/').pop()}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <!-- OCR Search Results Mode -->
  {#if ocrSearchResults !== null}
    <div class="search-results-section">
      <div class="section-header">
        <h3>OCR Keyword Search Hits ({ocrSearchResults.length})</h3>
        <button class="clear-banner-btn" on:click={handleClearOcrSearch}>Show All Screenshots</button>
      </div>

      {#if ocrSearchResults.length === 0}
        <div class="empty-state">
          <p>No screenshots containing "{ocrQuery}" in OCR extracted text.</p>
        </div>
      {:else}
        <div class="gallery-grid">
          {#each ocrSearchResults as hit}
            <div class="screenshot-card">
              <div class="card-body">
                <div class="meta-row">
                  <span class="hash-badge" title="64-bit dHash perceptual fingerprint">
                    {formatDhash(hit.dhash)}
                  </span>
                  <span class="match-score-badge">Text Match</span>
                </div>

                <button
                  type="button"
                  class="file-name clickable"
                  title={hit.path}
                  on:click={() => onInspectDoc(hit.path)}
                >
                  {hit.path.split('/').pop()}
                </button>

                {#if hit.ocr_snippet}
                  <div class="ocr-preview-box">
                    <span class="ocr-label">OCR Snippet:</span>
                    <p>{hit.ocr_snippet}</p>
                  </div>
                {/if}

                <div class="actions-row">
                  <button class="action-btn" on:click={() => onInspectDoc(hit.path)}>
                    <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                      <circle cx="12" cy="12" r="3" />
                    </svg>
                    Inspect File
                  </button>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <!-- Gallery Grid for All Indexed Screenshots -->
    {#if screenshots.length === 0}
      <div class="empty-gallery-state">
        <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          <circle cx="8.5" cy="8.5" r="1.5" />
          <polyline points="21 15 16 10 5 21" />
        </svg>
        <h3>No Screenshots Cataloged Yet</h3>
        <p>
          Click <strong>Scan Screenshot Folder</strong> above to catalog image files (.png, .jpg, .bmp, .webp)
          in <code>{currentPath || 'the active workspace'}</code> with offline dHash and OCR text indexing.
        </p>
      </div>
    {:else}
      <div class="gallery-grid">
        {#each screenshots as item}
          <div class="screenshot-card">
            <div class="preview-thumbnail">
              <div class="mock-screenshot-display">
                <div class="mock-window-bar">
                  <span class="dot red"></span>
                  <span class="dot yellow"></span>
                  <span class="dot green"></span>
                  <span class="window-title">{item.path.split('/').pop()}</span>
                </div>
                <div class="mock-code-lines">
                  {#if item.ocr_text}
                    {#each item.ocr_text.split('\n').filter(Boolean).slice(0, 3) as line}
                      <div class="mock-line">{line}</div>
                    {/each}
                  {:else}
                    <div class="mock-line muted">(No visible text recognized)</div>
                  {/if}
                </div>
              </div>
            </div>

            <div class="card-body">
              <div class="meta-row">
                <span class="dim-badge">{item.dimensions[0]} × {item.dimensions[1]}</span>
                <span class="hash-badge" title="64-bit dHash perceptual fingerprint">
                  {formatDhash(item.dhash)}
                </span>
              </div>

              <button
                type="button"
                class="file-name clickable"
                title={item.path}
                on:click={() => onInspectDoc(item.path)}
              >
                {item.path.split('/').pop()}
              </button>

              {#if item.ocr_text}
                <div class="ocr-preview-box">
                  <span class="ocr-label">OCR Text Recognized:</span>
                  <p>{item.ocr_text}</p>
                </div>
              {/if}

              <div class="actions-row">
                <button class="action-btn" on:click={() => handleFindSimilar(item)}>
                  <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="11" cy="11" r="8" />
                    <path d="m21 21-4.35-4.35" />
                  </svg>
                  Find Similar
                </button>
                <button class="action-btn secondary" on:click={() => onInspectDoc(item.path)}>
                  <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                    <circle cx="12" cy="12" r="3" />
                  </svg>
                  Inspect
                </button>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .screenshot-gallery-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
    height: 100%;
    overflow-y: auto;
  }

  .gallery-header-panel {
    background: rgba(15, 23, 42, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .header-info-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
  }

  .header-info-row h2 {
    font-size: 1.25rem;
    font-weight: 600;
    color: #f8fafc;
    margin: 0;
  }

  .subtitle {
    font-size: 0.85rem;
    color: #94a3b8;
    margin: 4px 0 0 0;
  }

  .index-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-radius: 6px;
    background: #0ea5e9;
    color: #ffffff;
    border: none;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .index-btn:hover:not(:disabled) {
    background: #0284c7;
  }

  .index-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .search-input-box {
    display: flex;
    align-items: center;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;
    padding: 6px 10px;
    gap: 8px;
  }

  .search-icon {
    color: #64748b;
  }

  .search-input-box input {
    flex: 1;
    background: transparent;
    border: none;
    color: #f8fafc;
    font-size: 0.92rem;
    outline: none;
  }

  .clear-search-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 2px 6px;
  }

  .clear-search-btn:hover {
    color: #f8fafc;
  }

  .search-btn {
    background: #3b82f6;
    color: white;
    border: none;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 0.82rem;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .search-btn:hover:not(:disabled) {
    background: #2563eb;
  }

  .search-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .status-banner {
    font-size: 0.8rem;
    color: #38bdf8;
    background: rgba(14, 165, 233, 0.1);
    border: 1px solid rgba(14, 165, 233, 0.2);
    padding: 6px 12px;
    border-radius: 6px;
  }

  .similar-banner {
    background: rgba(14, 165, 233, 0.12);
    border: 1px solid rgba(14, 165, 233, 0.3);
    border-radius: 10px;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .banner-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .banner-title {
    font-size: 0.88rem;
    color: #e0f2fe;
  }

  .target-hash {
    font-family: monospace;
    font-size: 0.78rem;
    color: #a855f7;
    margin-left: 6px;
  }

  .close-banner-btn,
  .clear-banner-btn {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    color: #94a3b8;
    padding: 4px 8px;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .close-banner-btn:hover,
  .clear-banner-btn:hover {
    color: #ffffff;
    background: rgba(255, 255, 255, 0.1);
  }

  .searching-indicator,
  .no-matches-msg {
    font-size: 0.82rem;
    color: #94a3b8;
    font-style: italic;
  }

  .similar-chips {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .match-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 4px 10px;
    font-size: 0.78rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .match-chip:hover {
    border-color: #38bdf8;
    background: rgba(14, 165, 233, 0.2);
  }

  .match-chip.high-match {
    border-color: rgba(34, 197, 94, 0.4);
    background: rgba(34, 197, 94, 0.15);
  }

  .match-chip .pct {
    font-weight: 600;
    color: #38bdf8;
  }

  .match-chip.high-match .pct {
    color: #4ade80;
  }

  .match-chip .dist {
    color: #64748b;
    font-size: 0.72rem;
  }

  .match-chip .name {
    color: #e2e8f0;
  }

  .search-results-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .section-header h3 {
    margin: 0;
    font-size: 1.05rem;
    color: #f8fafc;
  }

  .empty-state,
  .empty-gallery-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    background: rgba(15, 23, 42, 0.4);
    border: 1px dashed rgba(255, 255, 255, 0.1);
    border-radius: 12px;
    text-align: center;
    gap: 10px;
  }

  .empty-gallery-state svg {
    color: #475569;
  }

  .empty-gallery-state h3 {
    margin: 0;
    color: #e2e8f0;
    font-size: 1.1rem;
  }

  .empty-gallery-state p {
    margin: 0;
    color: #94a3b8;
    font-size: 0.88rem;
    max-width: 500px;
  }

  .gallery-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 16px;
  }

  .screenshot-card {
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    transition: all 0.15s ease;
  }

  .screenshot-card:hover {
    border-color: rgba(14, 165, 233, 0.3);
    transform: translateY(-2px);
  }

  .preview-thumbnail {
    height: 120px;
    background: #020617;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    padding: 10px;
  }

  .mock-screenshot-display {
    height: 100%;
    background: rgba(15, 23, 42, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .mock-window-bar {
    height: 20px;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    padding: 0 8px;
    gap: 5px;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .dot.red { background: #ef4444; }
  .dot.yellow { background: #f59e0b; }
  .dot.green { background: #10b981; }

  .window-title {
    font-size: 0.65rem;
    color: #64748b;
    margin-left: 6px;
    font-family: monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mock-code-lines {
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow: hidden;
  }

  .mock-line {
    font-size: 0.68rem;
    color: #38bdf8;
    font-family: monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mock-line.muted {
    color: #64748b;
    font-style: italic;
  }

  .card-body {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .meta-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .dim-badge {
    font-size: 0.72rem;
    color: #94a3b8;
    background: rgba(255, 255, 255, 0.04);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: monospace;
  }

  .hash-badge {
    font-size: 0.7rem;
    font-family: monospace;
    color: #a855f7;
    background: rgba(168, 85, 247, 0.12);
    border: 1px solid rgba(168, 85, 247, 0.25);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .match-score-badge {
    font-size: 0.7rem;
    font-weight: 600;
    color: #4ade80;
    background: rgba(74, 222, 128, 0.12);
    border: 1px solid rgba(74, 222, 128, 0.25);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .file-name {
    font-size: 0.92rem;
    font-weight: 600;
    color: #f1f5f9;
    margin: 0;
    background: none;
    border: none;
    padding: 0;
    text-align: left;
    display: block;
    width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: inherit;
  }

  .file-name.clickable {
    cursor: pointer;
  }

  .file-name.clickable:hover {
    color: #38bdf8;
    text-decoration: underline;
  }

  .ocr-preview-box {
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .ocr-label {
    font-size: 0.7rem;
    color: #64748b;
    text-transform: uppercase;
    font-weight: 600;
  }

  .ocr-preview-box p {
    font-size: 0.78rem;
    color: #cbd5e1;
    margin: 0;
    line-height: 1.35;
    max-height: 52px;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: monospace;
  }

  .actions-row {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #cbd5e1;
    font-size: 0.78rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover {
    background: rgba(14, 165, 233, 0.15);
    border-color: rgba(14, 165, 233, 0.4);
    color: #38bdf8;
  }

  .action-btn.secondary:hover {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.2);
    color: #ffffff;
  }
</style>
