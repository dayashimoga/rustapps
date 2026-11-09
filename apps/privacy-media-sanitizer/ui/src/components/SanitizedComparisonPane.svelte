<script lang="ts">
  import type { MediaInspectionReport, SanitizedResult } from '../lib/types';

  export let report: MediaInspectionReport | null = null;
  export let result: SanitizedResult | null = null;
  export let onSignCertificate: () => void = () => {};
</script>

{#if result}
  <div class="comparison-pane-shell">
    <div class="comp-header">
      <div class="ch-left">
        <span class="shield-icon">🛡️</span>
        <div class="ch-titles">
          <h2 class="title font-semibold">Zero-Trace Sanitization Verified</h2>
          <span class="sub">100% of EXIF, GPS, camera serials, and XMP streams eradicated</span>
        </div>
      </div>

      <div class="ch-actions">
        <button type="button" class="btn cert-btn" on:click={onSignCertificate}>
          🔏 Issue Cryptographic Privacy Certificate
        </button>
      </div>
    </div>

    <div class="comp-body">
      <!-- Delta Summary Banner -->
      <div class="delta-banner">
        <div class="delta-stat">
          <span class="d-val text-green">{result.tags_stripped_count}</span>
          <span class="d-lbl">Metadata Tags Destroyed</span>
        </div>
        <div class="delta-divider"></div>
        <div class="delta-stat">
          <span class="d-val text-blue">{(result.bytes_removed / 1024).toFixed(2)} KB</span>
          <span class="d-lbl">Overhead Stripped</span>
        </div>
        <div class="delta-divider"></div>
        <div class="delta-stat">
          <span class="d-val text-green">ZERO</span>
          <span class="d-lbl">GPS Geolocation Traces</span>
        </div>
        <div class="delta-divider"></div>
        <div class="delta-stat">
          <span class="d-val text-green">100% CLEAN</span>
          <span class="d-lbl">Air-Gapped Privacy Status</span>
        </div>
      </div>

      <!-- Before vs After Comparison Grid -->
      <div class="before-after-grid">
        <!-- Original File Pane -->
        <div class="state-card before">
          <div class="sc-hdr">
            <span class="sc-icon">⚠️</span>
            <span class="sc-title">ORIGINAL (DIRTY WITH METADATA LEAKS)</span>
          </div>

          <div class="sc-details">
            <div class="sc-row">
              <span class="k">Filename:</span>
              <span class="v font-semibold">{result.filename}</span>
            </div>
            <div class="sc-row">
              <span class="k">Original Size:</span>
              <span class="v">{(result.original_size_bytes / 1024).toFixed(2)} KB ({result.original_size_bytes} bytes)</span>
            </div>
            <div class="sc-row">
              <span class="k">Original BLAKE3:</span>
              <span class="v mono break">{result.original_blake3}</span>
            </div>
            {#if report}
              <div class="sc-row">
                <span class="k">Identified Leaks:</span>
                <span class="v tag-leak-cnt">{report.metadata_tags.length} surveillance properties</span>
              </div>
              {#if report.has_gps}
                <div class="sc-row">
                  <span class="k">GPS Location:</span>
                  <span class="v text-red mono">{report.gps_coordinates || 'Precise Latitude & Longitude'}</span>
                </div>
              {/if}
              {#if report.device_fingerprint}
                <div class="sc-row">
                  <span class="k">Device ID:</span>
                  <span class="v text-orange">{report.device_fingerprint}</span>
                </div>
              {/if}
            {/if}
          </div>
        </div>

        <!-- Sanitized Clean File Pane -->
        <div class="state-card after">
          <div class="sc-hdr">
            <span class="sc-icon">🛡️</span>
            <span class="sc-title">SANITIZED (ZERO-TRACE AIR-GAPPED CLEAN)</span>
          </div>

          <div class="sc-details">
            <div class="sc-row">
              <span class="k">Clean Filename:</span>
              <span class="v font-semibold text-green">{result.filename}</span>
            </div>
            <div class="sc-row">
              <span class="k">Sanitized Size:</span>
              <span class="v text-green">{(result.sanitized_size_bytes / 1024).toFixed(2)} KB ({result.sanitized_size_bytes} bytes)</span>
            </div>
            <div class="sc-row">
              <span class="k">Sanitized BLAKE3:</span>
              <span class="v mono break text-blue">{result.sanitized_blake3}</span>
            </div>
            <div class="sc-row">
              <span class="k">EXIF / XMP / IPTC:</span>
              <span class="v text-green font-semibold">COMPLETELY ERADICATED</span>
            </div>
            <div class="sc-row">
              <span class="k">GPS Geolocation:</span>
              <span class="v text-green font-semibold">PURGED & ZEROED</span>
            </div>
            <div class="sc-row">
              <span class="k">Camera Serial Numbers:</span>
              <span class="v text-green font-semibold">DESTROYED</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
{:else}
  <div class="empty-state">
    <div class="empty-icon">🔏</div>
    <h3>No Sanitization Completed</h3>
    <p>Select a file and execute "Purge All Metadata" to inspect before/after forensic comparison.</p>
  </div>
{/if}

<style>
  .comparison-pane-shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #030712;
  }

  .comp-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    background: #090e18;
    border-bottom: 1px solid #1a2336;
  }

  .ch-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .shield-icon {
    font-size: 1.6rem;
  }

  .ch-titles {
    display: flex;
    flex-direction: column;
  }

  .title {
    margin: 0;
    font-size: 1.15rem;
    color: #34d399;
  }

  .sub {
    font-size: 0.74rem;
    color: #94a3b8;
  }

  .cert-btn {
    background: rgba(16, 185, 129, 0.2);
    border: 1px solid #10b981;
    color: #a7f3d0;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 700;
    cursor: pointer;
  }

  .cert-btn:hover {
    background: rgba(16, 185, 129, 0.35);
  }

  .comp-body {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .delta-banner {
    display: flex;
    align-items: center;
    justify-content: space-around;
    background: #070b14;
    border: 1px solid #151f33;
    border-radius: 8px;
    padding: 14px 20px;
  }

  .delta-stat {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .d-val {
    font-size: 1.3rem;
    font-weight: 800;
    line-height: 1.1;
  }

  .d-lbl {
    font-size: 0.68rem;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-top: 4px;
  }

  .delta-divider {
    width: 1px;
    height: 30px;
    background: #1e293b;
  }

  .text-green { color: #34d399; }
  .text-blue { color: #38bdf8; }
  .text-red { color: #f87171; }
  .text-orange { color: #fb923c; }

  .before-after-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .state-card {
    background: #070b14;
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid #1e293b;
    display: flex;
    flex-direction: column;
  }

  .state-card.before {
    border-top: 4px solid #ef4444;
  }

  .state-card.after {
    border-top: 4px solid #10b981;
  }

  .sc-hdr {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: #090e18;
    border-bottom: 1px solid #151f33;
  }

  .sc-icon {
    font-size: 1rem;
  }

  .sc-title {
    font-size: 0.74rem;
    font-weight: 700;
    color: #94a3b8;
    letter-spacing: 0.05em;
  }

  .sc-details {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    font-size: 0.84rem;
  }

  .sc-row {
    display: flex;
    justify-content: space-between;
    padding-bottom: 6px;
    border-bottom: 1px solid #111827;
  }

  .sc-row:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }

  .k {
    color: #94a3b8;
    min-width: 130px;
  }

  .v {
    color: #f1f5f9;
    text-align: right;
  }

  .mono { font-family: monospace; font-size: 0.78rem; }
  .font-semibold { font-weight: 600; }
  .break { word-break: break-all; max-width: 240px; }

  .tag-leak-cnt {
    color: #f87171;
    font-weight: 600;
  }

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
