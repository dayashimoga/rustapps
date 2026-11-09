<script lang="ts">
  import type { MediaInspectionReport } from '../lib/types';

  export let report: MediaInspectionReport | null = null;
  export let onSanitize: () => void = () => {};
  export let isSanitizing = false;
</script>

{#if report}
  <div class="inspection-pane-shell">
    <div class="pane-header">
      <div class="ph-meta">
        <div class="file-title-row">
          <span class="file-icon">📷</span>
          <h2 class="filename font-semibold">{report.filename}</h2>
          <span class="type-pill">{report.media_type.toUpperCase()}</span>
          <span class="risk-badge {report.overall_privacy_risk}">
            {report.overall_privacy_risk.replace(/_/g, ' ').toUpperCase()}
          </span>
        </div>
        <div class="file-specs">
          <span class="spec-item">Size: {(report.original_size_bytes / 1024).toFixed(1)} KB</span>
          <span class="spec-item mono">BLAKE3: {report.original_blake3.substring(0, 16)}...</span>
        </div>
      </div>

      <div class="ph-actions">
        <button
          type="button"
          class="btn sanitize-btn"
          on:click={onSanitize}
          disabled={isSanitizing}
        >
          {isSanitizing ? 'Sanitizing Bitstream...' : '⚡ Purge All Metadata (Zero-Trace Scrub)'}
        </button>
      </div>
    </div>

    <div class="pane-body">
      <!-- High Severity Surveillance Alerts -->
      <div class="exposure-alerts-grid">
        {#if report.has_gps && report.gps_coordinates}
          <div class="alert-card critical">
            <div class="ac-icon">📍</div>
            <div class="ac-content">
              <div class="ac-title">EXIF GPS GEOLOCATION LEAK DETECTED</div>
              <div class="ac-val mono">{report.gps_coordinates}</div>
              <div class="ac-desc">
                High-precision GPS coordinates expose the exact latitude, longitude, and elevation of the photo capture location.
              </div>
            </div>
          </div>
        {/if}

        {#if report.device_fingerprint}
          <div class="alert-card high">
            <div class="ac-icon">📱</div>
            <div class="ac-content">
              <div class="ac-title">HARDWARE / OS DEVICE FINGERPRINT</div>
              <div class="ac-val">{report.device_fingerprint}</div>
              <div class="ac-desc">
                Physical camera model, lens specifications, and operating system build parameters identify individual user hardware.
              </div>
            </div>
          </div>
        {/if}
      </div>

      <!-- Itemized Tags Table -->
      <div class="tags-section">
        <div class="ts-hdr">
          <span class="ts-title">DETECTED METADATA ATTRIBUTES ({report.metadata_tags.length})</span>
          <span class="ts-sub">Surveillance parameters marked for cryptographic destruction</span>
        </div>

        <div class="tags-table-wrapper">
          <table class="tags-table">
            <thead>
              <tr>
                <th>Tag Identifier</th>
                <th>Category</th>
                <th>Raw Extracted Value</th>
                <th>Privacy Risk</th>
                <th>Forensic Impact</th>
              </tr>
            </thead>
            <tbody>
              {#each report.metadata_tags as tag}
                <tr>
                  <td class="mono font-semibold tag-name">{tag.tag_name}</td>
                  <td>
                    <span class="category-pill">{tag.category}</span>
                  </td>
                  <td class="mono raw-val">{tag.raw_value}</td>
                  <td>
                    <span class="risk-pill {tag.risk_grade}">
                      {tag.risk_grade.replace(/_/g, ' ').toUpperCase()}
                    </span>
                  </td>
                  <td class="explanation">{tag.explanation}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
{:else}
  <div class="empty-state">
    <div class="empty-icon">🛡️</div>
    <h3>No Media Selected</h3>
    <p>Select a file from the left sidebar or import an image or PDF document to inspect hidden metadata.</p>
  </div>
{/if}

<style>
  .inspection-pane-shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #030712;
  }

  .pane-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    background: #090e18;
    border-bottom: 1px solid #1a2336;
  }

  .ph-meta {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .file-title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .file-icon {
    font-size: 1.3rem;
  }

  .filename {
    margin: 0;
    font-size: 1.2rem;
    color: #f8fafc;
  }

  .type-pill {
    background: #1e3a8a;
    color: #93c5fd;
    font-size: 0.72rem;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 4px;
  }

  .risk-badge {
    padding: 3px 10px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 700;
  }

  .risk-badge.critical {
    background: rgba(239, 68, 68, 0.2);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.4);
  }

  .risk-badge.high {
    background: rgba(249, 115, 22, 0.2);
    color: #fb923c;
    border: 1px solid rgba(249, 115, 22, 0.4);
  }

  .risk-badge.medium {
    background: rgba(245, 158, 11, 0.2);
    color: #fcd34d;
    border: 1px solid rgba(245, 158, 11, 0.4);
  }

  .file-specs {
    display: flex;
    gap: 12px;
    font-size: 0.78rem;
    color: #94a3b8;
  }

  .sanitize-btn {
    background: #059669;
    color: #fff;
    padding: 9px 18px;
    border-radius: 6px;
    font-size: 0.88rem;
    font-weight: 700;
    border: 1px solid #10b981;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .sanitize-btn:hover:not(:disabled) {
    background: #047857;
  }

  .sanitize-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .pane-body {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .exposure-alerts-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
    gap: 14px;
  }

  .alert-card {
    display: flex;
    gap: 14px;
    padding: 14px 16px;
    border-radius: 8px;
    background: #090e18;
    border: 1px solid #1e293b;
  }

  .alert-card.critical {
    border-left: 4px solid #ef4444;
    background: rgba(239, 68, 68, 0.05);
  }

  .alert-card.high {
    border-left: 4px solid #f97316;
    background: rgba(249, 115, 22, 0.05);
  }

  .ac-icon {
    font-size: 1.5rem;
  }

  .ac-content {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .ac-title {
    font-size: 0.74rem;
    font-weight: 700;
    color: #94a3b8;
    letter-spacing: 0.05em;
  }

  .ac-val {
    font-size: 0.92rem;
    font-weight: 700;
    color: #f1f5f9;
  }

  .ac-desc {
    font-size: 0.8rem;
    color: #cbd5e1;
    line-height: 1.4;
  }

  .tags-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .ts-hdr {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .ts-title {
    font-size: 0.76rem;
    font-weight: 700;
    color: #64748b;
    letter-spacing: 0.05em;
  }

  .ts-sub {
    font-size: 0.74rem;
    color: #94a3b8;
  }

  .tags-table-wrapper {
    background: #060a12;
    border: 1px solid #1a2336;
    border-radius: 8px;
    overflow: hidden;
  }

  .tags-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.84rem;
  }

  .tags-table th {
    text-align: left;
    padding: 10px 14px;
    background: #090e18;
    color: #94a3b8;
    font-weight: 600;
    font-size: 0.74rem;
    text-transform: uppercase;
    border-bottom: 1px solid #1a2336;
  }

  .tags-table td {
    padding: 10px 14px;
    border-bottom: 1px solid #111827;
    color: #cbd5e1;
  }

  .tags-table tr:last-child td {
    border-bottom: none;
  }

  .tag-name {
    color: #38bdf8;
  }

  .category-pill {
    background: #1e293b;
    color: #cbd5e1;
    font-size: 0.7rem;
    padding: 2px 7px;
    border-radius: 4px;
  }

  .raw-val {
    color: #f1f5f9;
    max-width: 280px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .risk-pill {
    font-size: 0.68rem;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 4px;
  }

  .risk-pill.critical { background: rgba(239, 68, 68, 0.2); color: #f87171; }
  .risk-pill.high { background: rgba(249, 115, 22, 0.2); color: #fb923c; }
  .risk-pill.medium { background: rgba(245, 158, 11, 0.2); color: #fcd34d; }
  .risk-pill.clean { background: rgba(16, 185, 129, 0.2); color: #34d399; }

  .explanation {
    font-size: 0.8rem;
    color: #94a3b8;
  }

  .mono { font-family: monospace; }
  .font-semibold { font-weight: 600; }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    color: #64748b;
  }

  .empty-icon {
    font-size: 3rem;
    opacity: 0.4;
    margin-bottom: 12px;
  }
</style>
