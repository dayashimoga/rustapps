<script lang="ts">
  import { invokeIpc } from '../lib/ipc';

  export let onBatchComplete: () => void = () => {};

  let csvRaw = `https://product1.corp.internal,Product 1 Tag,M,#06b6d4
https://product2.corp.internal,Product 2 Tag,M,#8b5cf6
WIFI:S:GuestNet;T:WPA;P:GuestPass99;;,Lobby Wi-Fi,H,#10b981
mailto:support@antigravity.dev,Customer Support,M,#f59e0b`;

  let isProcessing = false;
  let progressText = '';
  let errorMessage = '';

  let batchReport: {
    total_requested: number;
    successful: number;
    failed: number;
    items: Array<{
      index: number;
      label: string | null;
      payload_type: string;
      svg_markup: string;
      png_base64: string;
      success: boolean;
      error: string | null;
    }>;
  } | null = null;

  function parseCsvRows(): Array<{ raw_text: string; label: string | null; ecc: string | null; color: string | null }> {
    const lines = csvRaw.trim().split('\n');
    const items = [];
    for (const line of lines) {
      const parts = line.split(',');
      if (parts[0] && parts[0].trim()) {
        items.push({
          payload: parts[0].trim(),
          label: parts[1] ? parts[1].trim() : 'Item ' + (items.length + 1),
        });
      }
    }
    return items;
  }

  async function runBatch() {
    isProcessing = true;
    errorMessage = '';
    progressText = 'Parsing CSV records...';
    try {
      const parsed = parseCsvRows();
      if (parsed.length === 0) {
        throw new Error('No valid records found in CSV input');
      }

      progressText = `Synthesizing ${parsed.length} QR matrices...`;
      const report = await invokeIpc<any>('batch_generate_qr', {
        items: parsed,
        style: null,
      });

      batchReport = report;
      onBatchComplete();
    } catch (e: any) {
      errorMessage = e?.message || String(e);
    } finally {
      isProcessing = false;
      progressText = '';
    }
  }

  function downloadItemSvg(item: any) {
    const blob = new Blob([item.svg_markup], { type: 'image/svg+xml' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `qr_batch_${item.index + 1}_${item.label || 'code'}.svg`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function downloadItemPng(item: any) {
    const a = document.createElement('a');
    a.href = item.png_base64;
    a.download = `qr_batch_${item.index + 1}_${item.label || 'code'}.png`;
    a.click();
  }
</script>

<div class="batch-container">
  <div class="batch-header">
    <h2>Batch QR Generator</h2>
    <p class="subtitle">Bulk Synthesis via CSV Schema: text, label, ecc, color</p>
  </div>

  <div class="input-card">
    <div class="card-title-row">
      <label for="csv-input">CSV Input Editor</label>
      <span class="hint">Columns: raw_text, label, ecc (L/M/Q/H), hex_color</span>
    </div>

    <textarea
      id="csv-input"
      rows="7"
      bind:value={csvRaw}
      class="csv-textarea"
      placeholder="https://example.com,Example Label,M,#06b6d4"
    ></textarea>

    <div class="action-row">
      <button class="btn-run" on:click={runBatch} disabled={isProcessing}>
        {isProcessing ? progressText || 'Processing...' : '▶ Execute Batch Job'}
      </button>
    </div>
  </div>

  {#if errorMessage}
    <div class="error-banner">⚠️ {errorMessage}</div>
  {/if}

  {#if batchReport}
    <div class="report-section">
      <div class="report-stats">
        <div class="stat-pill">Total: {batchReport.total_requested}</div>
        <div class="stat-pill success">Success: {batchReport.successful}</div>
        {#if batchReport.failed > 0}
          <div class="stat-pill fail">Failed: {batchReport.failed}</div>
        {/if}
      </div>

      <div class="results-table-wrap">
        <table class="results-table">
          <thead>
            <tr>
              <th>#</th>
              <th>Preview</th>
              <th>Label</th>
              <th>Type</th>
              <th>Status</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each batchReport.items as item}
              <tr>
                <td>{item.index + 1}</td>
                <td class="preview-cell">
                  <div class="mini-svg">
                    {@html item.svg_markup}
                  </div>
                </td>
                <td class="label-cell">{item.label || '—'}</td>
                <td><span class="type-pill">{item.payload_type}</span></td>
                <td>
                  {#if item.success}
                    <span class="status-ok">✓ Generated</span>
                  {:else}
                    <span class="status-fail">✗ {item.error}</span>
                  {/if}
                </td>
                <td class="actions-cell">
                  <button class="btn-download" on:click={() => downloadItemSvg(item)}>SVG</button>
                  <button class="btn-download" on:click={() => downloadItemPng(item)}>PNG</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}
</div>

<style>
  .batch-container {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    height: 100%;
    overflow-y: auto;
  }

  .batch-header h2 {
    font-size: 1.25rem;
    font-weight: 700;
  }

  .subtitle {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-top: 0.25rem;
  }

  .input-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .card-title-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .card-title-row label {
    font-size: 0.8rem;
    font-weight: 700;
    color: var(--text-secondary);
    text-transform: uppercase;
  }

  .hint {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .csv-textarea {
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: 6px;
    padding: 0.75rem;
    font-family: var(--font-mono);
    font-size: 0.85rem;
    resize: vertical;
    line-height: 1.4;
  }

  .csv-textarea:focus {
    outline: none;
    border-color: var(--accent-cyan);
  }

  .action-row {
    display: flex;
    justify-content: flex-end;
  }

  .btn-run {
    background: var(--accent-cyan);
    color: #0b0f19;
    font-weight: 600;
    font-size: 0.9rem;
    padding: 0.6rem 1.5rem;
    border: none;
    border-radius: 6px;
    cursor: pointer;
  }

  .btn-run:hover {
    background: #22d3ee;
  }

  .error-banner {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: #fca5a5;
    padding: 0.75rem 1rem;
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .report-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .report-stats {
    display: flex;
    gap: 0.75rem;
  }

  .stat-pill {
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    padding: 0.35rem 0.75rem;
    border-radius: 9999px;
    font-size: 0.8rem;
    font-weight: 600;
  }

  .stat-pill.success {
    color: var(--accent-emerald);
    border-color: rgba(16, 185, 129, 0.3);
  }

  .stat-pill.fail {
    color: var(--accent-rose);
    border-color: rgba(239, 68, 68, 0.3);
  }

  .results-table-wrap {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    overflow: hidden;
  }

  .results-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .results-table th {
    background: var(--bg-elevated);
    padding: 0.75rem 1rem;
    text-align: left;
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    border-bottom: 1px solid var(--border-color);
  }

  .results-table td {
    padding: 0.6rem 1rem;
    border-bottom: 1px solid rgba(71, 85, 105, 0.2);
    vertical-align: middle;
  }

  .preview-cell {
    width: 60px;
  }

  .mini-svg {
    width: 44px;
    height: 44px;
    background: #000;
    padding: 4px;
    border-radius: 4px;
  }

  .mini-svg :global(svg) {
    width: 100%;
    height: 100%;
  }

  .type-pill {
    background: rgba(6, 182, 212, 0.15);
    color: var(--accent-cyan);
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
  }

  .status-ok {
    color: var(--accent-emerald);
    font-weight: 600;
  }

  .status-fail {
    color: var(--accent-rose);
  }

  .actions-cell {
    display: flex;
    gap: 0.4rem;
  }

  .btn-download {
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 0.3rem 0.6rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.75rem;
  }

  .btn-download:hover {
    background: rgba(255, 255, 255, 0.1);
  }
</style>
