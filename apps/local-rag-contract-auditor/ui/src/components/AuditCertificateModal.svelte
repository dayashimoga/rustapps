<script lang="ts">
  import type { LegalAuditCertificate } from '../lib/types';

  export let isOpen = false;
  export let certificate: LegalAuditCertificate | null = null;
  export let onClose: () => void = () => {};

  let copyStatus = '';

  function copyManifest() {
    if (!certificate) return;
    const text = JSON.stringify(certificate, null, 2);
    navigator.clipboard.writeText(text);
    copyStatus = 'Copied to clipboard!';
    setTimeout(() => {
      copyStatus = '';
    }, 2500);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

{#if isOpen && certificate}
  <div class="modal-backdrop" on:click={onClose} on:keydown={handleKeyDown} role="presentation">
    <div class="modal-card" on:click|stopPropagation role="dialog" aria-modal="true" aria-label="Legal Audit Certificate" tabindex="-1">
      <div class="modal-header">
        <div class="modal-title">
          <span class="icon">🛡️</span>
          <span>Ed25519 Legal Audit Certificate</span>
        </div>
        <button type="button" class="close-btn" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body">
        <div class="cert-seal">
          <div class="seal-icon">🏛️</div>
          <div class="seal-title">AIR-GAPPED COMPLIANCE AUDIT CERTIFICATE</div>
          <div class="seal-sub">Cryptographically Anchored via BLAKE3 + Ed25519 Digital Signature</div>
        </div>

        <div class="details-table">
          <div class="row">
            <span class="lbl">Certificate ID</span>
            <span class="val mono highlight">{certificate.certificate_id}</span>
          </div>
          <div class="row">
            <span class="lbl">Contract Title</span>
            <span class="val font-semibold">{certificate.document_title}</span>
          </div>
          <div class="row">
            <span class="lbl">Document ID</span>
            <span class="val mono">{certificate.document_id}</span>
          </div>
          <div class="row">
            <span class="lbl">BLAKE3 Hash</span>
            <span class="val mono break">{certificate.document_blake3_hash}</span>
          </div>
          <div class="row">
            <span class="lbl">Audit Risk Grade</span>
            <span class="val grade-pill">{certificate.overall_risk_grade}</span>
          </div>
          <div class="row">
            <span class="lbl">Clause Breakdown</span>
            <span class="val">
              {certificate.total_clauses} Clauses ({certificate.critical_findings_count} Critical, {certificate.high_findings_count} High Risk)
            </span>
          </div>
          <div class="row">
            <span class="lbl">Issued Timestamp</span>
            <span class="val">{new Date(certificate.issued_at_epoch_ms).toUTCString()}</span>
          </div>
          <div class="row">
            <span class="lbl">Ed25519 Signature</span>
            <span class="val mono break sig">{certificate.ed25519_signature_hex}</span>
          </div>
          <div class="row">
            <span class="lbl">Auditor Public Key</span>
            <span class="val mono break">{certificate.auditor_public_key_hex}</span>
          </div>
        </div>

        {#if copyStatus}
          <div class="copy-feedback">{copyStatus}</div>
        {/if}
      </div>

      <div class="modal-footer">
        <button type="button" class="btn secondary" on:click={onClose}>Close</button>
        <button type="button" class="btn primary" on:click={copyManifest}>
          Copy Full JSON Manifest
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(4, 7, 14, 0.85);
    backdrop-filter: blur(8px);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
  }

  .modal-card {
    background: #0d121f;
    border: 1px solid #1f293d;
    border-radius: 12px;
    width: 680px;
    max-width: 90vw;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.7);
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid #1a2336;
    background: #090e18;
  }

  .modal-title {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 1.1rem;
    font-weight: 600;
    color: #f1f5f9;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #64748b;
    font-size: 1.2rem;
    cursor: pointer;
  }

  .close-btn:hover {
    color: #f1f5f9;
  }

  .modal-body {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .cert-seal {
    text-align: center;
    padding: 16px;
    background: rgba(30, 41, 59, 0.4);
    border: 1px dashed #334155;
    border-radius: 8px;
  }

  .seal-icon {
    font-size: 2rem;
    margin-bottom: 6px;
  }

  .seal-title {
    font-size: 0.95rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: #38bdf8;
  }

  .seal-sub {
    font-size: 0.76rem;
    color: #94a3b8;
    margin-top: 4px;
  }

  .details-table {
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: #060a12;
    padding: 14px;
    border-radius: 8px;
    border: 1px solid #1a2336;
  }

  .row {
    display: flex;
    justify-content: space-between;
    font-size: 0.84rem;
    padding-bottom: 6px;
    border-bottom: 1px solid #111827;
  }

  .row:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }

  .lbl {
    color: #94a3b8;
    font-weight: 500;
    min-width: 140px;
  }

  .val {
    color: #f1f5f9;
    text-align: right;
  }

  .val.mono {
    font-family: monospace;
    font-size: 0.8rem;
  }

  .val.break {
    word-break: break-all;
    max-width: 440px;
  }

  .val.highlight {
    color: #38bdf8;
    font-weight: 600;
  }

  .val.sig {
    color: #a78bfa;
  }

  .grade-pill {
    background: #1e293b;
    padding: 2px 8px;
    border-radius: 4px;
    color: #f59e0b;
    font-weight: 600;
  }

  .copy-feedback {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid rgba(16, 185, 129, 0.4);
    color: #34d399;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.84rem;
    text-align: center;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 20px;
    border-top: 1px solid #1a2336;
    background: #090e18;
  }

  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.88rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
  }

  .btn.secondary {
    background: #1e293b;
    color: #cbd5e1;
  }

  .btn.secondary:hover {
    background: #334155;
  }

  .btn.primary {
    background: #2563eb;
    color: #fff;
  }

  .btn.primary:hover {
    background: #1d4ed8;
  }
</style>
