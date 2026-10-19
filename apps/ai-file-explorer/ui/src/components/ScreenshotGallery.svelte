<script lang="ts">
  import type { ScreenshotFingerprint, ScreenshotSearchResult } from '../types';

  let ocrQuery = '';
  let isScanning = false;
  let activeSimilarTarget: ScreenshotFingerprint | null = null;
  let similarMatches: ScreenshotSearchResult[] = [];

  // Mock initial screenshots for demonstration
  let screenshots: ScreenshotFingerprint[] = [
    {
      path: 'C:/Users/Developer/Screenshots/vscode_terminal_error.png',
      dhash: 0xa5a5_5a5a_3333_cccc,
      ocr_text: 'cargo test --workspace --all-targets\nerror: connection timeout to PostgreSQL on port 5432\nfatal: worker process aborted unexpectedly',
      dimensions: [1920, 1080],
    },
    {
      path: 'C:/Users/Developer/Screenshots/aws_s3_bucket_policy.png',
      dhash: 0xa5a5_5a5a_3333_ccee, // slight variation (2 bits diff)
      ocr_text: 'Amazon S3 > Buckets > prod-data-vault\nPermissions: Block Public Access (Bucket settings) ON\nKMS Customer Managed Key encryption enabled',
      dimensions: [1920, 1080],
    },
    {
      path: 'C:/Users/Developer/Screenshots/kubernetes_pod_logs.png',
      dhash: 0x1234_5678_9abc_def0,
      ocr_text: 'kubectl logs -n ingress-nginx ingress-controller-7d9f\n[WARN] upstream response buffering disabled for streaming endpoint\n[INFO] SSL certificate renewed for vault.local',
      dimensions: [2560, 1440],
    },
    {
      path: 'C:/Users/Developer/Screenshots/github_actions_workflow.png',
      dhash: 0x9876_5432_10fe_dcba,
      ocr_text: 'GitHub Actions > Run #481\nJob: security-audit (cargo deny check)\nOutcome: SUCCESS (0 advisories, 0 copyleft violations)',
      dimensions: [1920, 1200],
    },
  ];

  function getFilteredScreenshots(): ScreenshotFingerprint[] {
    if (!ocrQuery.trim()) return screenshots;
    const lower = ocrQuery.toLowerCase();
    return screenshots.filter((s) => s.ocr_text.toLowerCase().includes(lower));
  }

  function handleFindSimilar(target: ScreenshotFingerprint) {
    activeSimilarTarget = target;
    // Compute Hamming distance against all other screenshots
    similarMatches = screenshots
      .filter((s) => s.path !== target.path)
      .map((s) => {
        // Approximate popcount on 64-bit difference
        const xor = BigInt(s.dhash) ^ BigInt(target.dhash);
        let dist = 0;
        let temp = xor;
        while (temp > 0n) {
          if (temp & 1n) dist++;
          temp >>= 1n;
        }
        const similarity = Math.max(0, Math.min(100, Math.round(((64 - dist) / 64) * 100)));
        return {
          path: s.path,
          dhash: s.dhash,
          hamming_distance: dist,
          similarity_pct: similarity,
          ocr_snippet: s.ocr_text.slice(0, 120),
        };
      })
      .sort((a, b) => a.hamming_distance - b.hamming_distance);
  }

  function formatDhash(hash: number): string {
    return '0x' + (BigInt(hash) & 0xffffffffffffffffn).toString(16).toUpperCase().padStart(16, '0');
  }
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
      <button class="index-btn" on:click={() => (isScanning = true)} disabled={isScanning}>
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
        placeholder="Filter by OCR text (e.g. 'PostgreSQL port', 'Kubectl logs', 'GitHub Actions')..."
        bind:value={ocrQuery}
      />
    </div>
  </div>

  <!-- Similar Match Drawer / Alert if active -->
  {#if activeSimilarTarget}
    <div class="similar-banner">
      <div class="banner-content">
        <span class="banner-title">
          Visual Similarity Matches for: <strong>{activeSimilarTarget.path.split('/').pop()}</strong>
        </span>
        <button class="close-banner-btn" on:click={() => (activeSimilarTarget = null)}>Clear Target</button>
      </div>
      <div class="similar-chips">
        {#each similarMatches as match}
          <div class="match-chip {match.hamming_distance <= 10 ? 'high-match' : ''}">
            <span class="pct">{match.similarity_pct}% Visual Match</span>
            <span class="dist">(Hamming Dist: {match.hamming_distance})</span>
            <span class="name">{match.path.split('/').pop()}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Gallery Grid -->
  <div class="gallery-grid">
    {#each getFilteredScreenshots() as item}
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
              {#each item.ocr_text.split('\n').slice(0, 3) as line}
                <div class="mock-line">{line}</div>
              {/each}
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

          <h3 class="file-name" title={item.path}>{item.path.split('/').pop()}</h3>

          <div class="ocr-preview-box">
            <span class="ocr-label">OCR Text Recognized:</span>
            <p>{item.ocr_text}</p>
          </div>

          <div class="actions-row">
            <button class="action-btn" on:click={() => handleFindSimilar(item)}>
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="11" cy="11" r="8" />
                <path d="m21 21-4.35-4.35" />
              </svg>
              Find Visually Similar
            </button>
          </div>
        </div>
      </div>
    {/each}
  </div>
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
    padding: 8px 12px;
    gap: 10px;
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

  .close-banner-btn {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    color: #94a3b8;
    padding: 4px 8px;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .close-banner-btn:hover {
    color: #ffffff;
    background: rgba(255, 255, 255, 0.1);
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
    height: 140px;
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

  .file-name {
    font-size: 0.92rem;
    font-weight: 600;
    color: #f1f5f9;
    margin: 0;
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
</style>
