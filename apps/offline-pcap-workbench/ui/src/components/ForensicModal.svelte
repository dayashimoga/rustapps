<script lang="ts">
  import type { ForensicAuditReport } from '../lib/types';

  export let isOpen = false;
  export let report: ForensicAuditReport | null = null;
  export let onClose: () => void = () => {};

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
</script>

{#if isOpen && report}
  <div class="modal-backdrop" on:click={onClose} on:keydown={handleKeyDown} role="presentation">
    <div class="modal-card" on:click|stopPropagation role="dialog" aria-modal="true" aria-label="Forensic Report" tabindex="-1">
      <div class="modal-header">
        <div class="modal-title">
          <span class="icon">🛡️</span>
          <span>Forensic Security & Integrity Certificate</span>
        </div>
        <button type="button" class="close-btn" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body">
        <div class="status-banner" class:has-alert={report.cleartext_credentials_found > 0}>
          <div class="status-icon">
            {report.cleartext_credentials_found > 0 ? '⚠️' : '✅'}
          </div>
          <div class="status-text">
            <strong>{report.status}</strong>
            <p>100% Offline Air-Gapped Verification with Ed25519 Hardware Signature</p>
          </div>
        </div>

        <div class="stats-grid">
          <div class="stat-cell">
            <span class="lbl">Report ID</span>
            <span class="val mono">{report.report_id}</span>
          </div>
          <div class="stat-cell">
            <span class="lbl">Capture File</span>
            <span class="val mono">{report.capture_filename}</span>
          </div>
          <div class="stat-cell">
            <span class="lbl">Total Packets</span>
            <span class="val">{report.total_packets.toLocaleString()}</span>
          </div>
          <div class="stat-cell">
            <span class="lbl">Total Data</span>
            <span class="val">{formatBytes(report.total_bytes)}</span>
          </div>
          <div class="stat-cell">
            <span class="lbl">TCP Conversations</span>
            <span class="val">{report.tcp_streams} streams</span>
          </div>
          <div class="stat-cell">
            <span class="lbl">Duration</span>
            <span class="val">{report.duration_seconds.toFixed(2)}s</span>
          </div>
        </div>

        <!-- Security Findings -->
        <div class="findings-section">
          <h4>Security Findings & Anomalies</h4>
          <div class="finding-row">
            <span>Cleartext Basic Auth / Credentials:</span>
            <strong class={report.cleartext_credentials_found > 0 ? 'alert-danger' : 'alert-clean'}>
              {report.cleartext_credentials_found} detected
            </strong>
          </div>
          <div class="finding-row">
            <span>Suspicious Ports:</span>
            <span>{report.suspicious_ports.length > 0 ? report.suspicious_ports.join(', ') : 'None'}</span>
          </div>
          <div class="finding-row">
            <span>TLS Client Hello JA3 Fingerprints:</span>
            <span>{report.tls_ja3_hashes.length} unique</span>
          </div>
        </div>

        {#if report.tls_ja3_hashes.length > 0}
          <div class="ja3-list">
            <span class="lbl">Identified JA3 Hashes:</span>
            {#each report.tls_ja3_hashes as ja3}
              <code class="ja3-code">{ja3}</code>
            {/each}
          </div>
        {/if}

        <!-- Cryptographic Proof -->
        <div class="crypto-proof">
          <div class="crypto-row">
            <span class="lbl">Ed25519 Public Key:</span>
            <code class="sig-code">{report.ed25519_public_key}</code>
          </div>
          <div class="crypto-row">
            <span class="lbl">Cryptographic Signature:</span>
            <code class="sig-code">{report.signature_hex}</code>
          </div>
        </div>
      </div>

      <div class="modal-footer">
        <button type="button" class="btn-close" on:click={onClose}>Done</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
  }
  .modal-card {
    background: #181825;
    border: 1px solid #313244;
    border-radius: 12px;
    width: 640px;
    max-width: 90vw;
    box-shadow: 0 25px 50px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    outline: none;
  }
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    background: #11111b;
    border-bottom: 1px solid #313244;
  }
  .modal-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    font-weight: 600;
    color: #cdd6f4;
  }
  .close-btn {
    background: none;
    border: none;
    color: #6c7086;
    font-size: 16px;
    cursor: pointer;
  }
  .close-btn:hover {
    color: #cdd6f4;
  }
  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-height: 70vh;
    overflow-y: auto;
  }
  .status-banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-radius: 8px;
    background: rgba(166, 227, 161, 0.15);
    border: 1px solid #a6e3a1;
  }
  .status-banner.has-alert {
    background: rgba(243, 139, 168, 0.15);
    border-color: #f38ba8;
  }
  .status-icon {
    font-size: 24px;
  }
  .status-text strong {
    color: #cdd6f4;
    font-size: 14px;
  }
  .status-text p {
    margin: 2px 0 0;
    font-size: 12px;
    color: #a6adc8;
  }
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
  }
  .stat-cell {
    background: #1e1e2e;
    padding: 8px 12px;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
  }
  .stat-cell .lbl {
    font-size: 10px;
    color: #6c7086;
    text-transform: uppercase;
  }
  .stat-cell .val {
    font-size: 13px;
    font-weight: 600;
    color: #cdd6f4;
  }
  .mono {
    font-family: monospace;
    font-size: 11px !important;
  }
  .findings-section {
    background: #1e1e2e;
    border-radius: 8px;
    padding: 12px 14px;
  }
  .findings-section h4 {
    margin: 0 0 8px;
    font-size: 12px;
    color: #89b4fa;
    text-transform: uppercase;
  }
  .finding-row {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    padding: 4px 0;
    color: #a6adc8;
  }
  .alert-danger {
    color: #f38ba8;
  }
  .alert-clean {
    color: #a6e3a1;
  }
  .ja3-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .ja3-list .lbl {
    font-size: 11px;
    color: #6c7086;
  }
  .ja3-code {
    background: #11111b;
    color: #cba6f7;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-family: monospace;
  }
  .crypto-proof {
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 8px;
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .crypto-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .crypto-row .lbl {
    font-size: 10px;
    color: #6c7086;
  }
  .sig-code {
    font-size: 10px;
    font-family: monospace;
    color: #94e2d5;
    word-break: break-all;
  }
  .modal-footer {
    padding: 12px 20px;
    background: #11111b;
    border-top: 1px solid #313244;
    display: flex;
    justify-content: flex-end;
  }
  .btn-close {
    background: #89b4fa;
    color: #11111b;
    border: none;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
</style>
