<script lang="ts">
  import type { X509Certificate } from '../lib/types';

  export let certificate: X509Certificate | null = null;
  export let onRevoke: (certId: string) => void = () => {};

  let copyStatus = '';

  async function copyPem() {
    if (!certificate) return;
    try {
      await navigator.clipboard.writeText(certificate.certificate_pem);
      copyStatus = 'Copied to clipboard!';
      setTimeout(() => (copyStatus = ''), 2000);
    } catch {
      copyStatus = 'Failed to copy';
    }
  }

  function formatDate(epochMs: number): string {
    return new Date(epochMs).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }
</script>

<div class="detail-container">
  {#if certificate}
    <div class="detail-header">
      <div class="title-group">
        <h2>{certificate.subject.common_name}</h2>
        <div class="meta-row">
          <span class="type-pill">{certificate.cert_type.replace('_', ' ').toUpperCase()}</span>
          <span class="serial-pill">Serial: {certificate.serial_number_hex}</span>
          {#if certificate.is_revoked}
            <span class="revoked-pill">REVOKED ({certificate.revocation_reason})</span>
          {:else}
            <span class="valid-pill">VALID</span>
          {/if}
        </div>
      </div>

      <div class="header-actions">
        {#if !certificate.is_revoked && !certificate.is_ca}
          <button
            type="button"
            class="btn-revoke"
            on:click={() => onRevoke(certificate.certificate_id)}
          >Revoke</button>
        {/if}
        <button type="button" class="btn-copy" on:click={copyPem}>
          {copyStatus || 'Copy PEM'}
        </button>
      </div>
    </div>

    <div class="detail-body">
      <!-- Section: Subject & Issuer -->
      <div class="info-section">
        <h3>Subject & Issuer Distinguished Name</h3>
        <div class="grid-props">
          <div class="prop-item">
            <span class="lbl">Common Name (CN)</span>
            <span class="val bold">{certificate.subject.common_name}</span>
          </div>
          <div class="prop-item">
            <span class="lbl">Organization (O)</span>
            <span class="val">{certificate.subject.organization || '-'}</span>
          </div>
          <div class="prop-item">
            <span class="lbl">Organizational Unit (OU)</span>
            <span class="val">{certificate.subject.organizational_unit || '-'}</span>
          </div>
          <div class="prop-item">
            <span class="lbl">Location</span>
            <span class="val">
              {[certificate.subject.locality, certificate.subject.state_or_province, certificate.subject.country]
                .filter(Boolean)
                .join(', ') || '-'}
            </span>
          </div>
          <div class="prop-item full-width">
            <span class="lbl">Issuer DN</span>
            <span class="val mono">{certificate.issuer_dn}</span>
          </div>
        </div>
      </div>

      <!-- Section: Validity & Key Algorithm -->
      <div class="info-section">
        <h3>Cryptographic Properties & Validity</h3>
        <div class="grid-props">
          <div class="prop-item">
            <span class="lbl">Key Algorithm</span>
            <span class="val bold highlight">{certificate.key_algorithm}</span>
          </div>
          <div class="prop-item">
            <span class="lbl">Basic Constraints</span>
            <span class="val">
              {certificate.is_ca ? `Is CA: TRUE (pathlen: ${certificate.path_len_constraint ?? 'none'})` : 'Is CA: FALSE (End-Entity)'}
            </span>
          </div>
          <div class="prop-item">
            <span class="lbl">Valid From (Not Before)</span>
            <span class="val mono">{formatDate(certificate.validity_not_before_epoch_ms)}</span>
          </div>
          <div class="prop-item">
            <span class="lbl">Valid Until (Not After)</span>
            <span class="val mono">{formatDate(certificate.validity_not_after_epoch_ms)}</span>
          </div>
        </div>
      </div>

      <!-- Section: Subject Alternative Names -->
      {#if certificate.subject_alt_names.length > 0}
        <div class="info-section">
          <h3>Subject Alternative Names (SANs)</h3>
          <div class="san-tags">
            {#each certificate.subject_alt_names as san}
              <span class="san-tag">{san}</span>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Section: Key Usages -->
      <div class="info-section">
        <h3>Key Usages & Extended Key Usages</h3>
        <div class="usage-pills">
          {#if certificate.key_usage.key_cert_sign}<span class="usage-pill">Certificate Sign</span>{/if}
          {#if certificate.key_usage.crl_sign}<span class="usage-pill">CRL Sign</span>{/if}
          {#if certificate.key_usage.digital_signature}<span class="usage-pill">Digital Signature</span>{/if}
          {#if certificate.key_usage.key_encipherment}<span class="usage-pill">Key Encipherment</span>{/if}
          {#if certificate.ext_key_usage.server_auth}<span class="usage-pill ext">Server Auth (TLS)</span>{/if}
          {#if certificate.ext_key_usage.client_auth}<span class="usage-pill ext">Client Auth (mTLS)</span>{/if}
          {#if certificate.ext_key_usage.code_signing}<span class="usage-pill ext">Code Signing</span>{/if}
        </div>
      </div>

      <!-- Section: Fingerprint & PEM Block -->
      <div class="info-section">
        <h3>SHA-256 Fingerprint</h3>
        <code class="fp-code">{certificate.fingerprint_sha256}</code>

        <h3 class="pem-title">X.509 Certificate PEM</h3>
        <pre class="pem-block"><code>{certificate.certificate_pem}</code></pre>
      </div>
    </div>
  {:else}
    <div class="empty-detail">Select a certificate from the hierarchy to view details</div>
  {/if}
</div>

<style>
  .detail-container {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: #11111b;
    overflow-y: auto;
  }
  .detail-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 16px 20px;
    background: #181825;
    border-bottom: 1px solid #313244;
  }
  .title-group h2 {
    margin: 0 0 6px;
    font-size: 16px;
    color: #cdd6f4;
  }
  .meta-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .type-pill {
    background: #11111b;
    color: #89b4fa;
    border: 1px solid #313244;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
  }
  .serial-pill {
    color: #6c7086;
    font-size: 11px;
    font-family: monospace;
  }
  .valid-pill {
    background: rgba(166, 227, 161, 0.2);
    color: #a6e3a1;
    border: 1px solid rgba(166, 227, 161, 0.4);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
  }
  .revoked-pill {
    background: rgba(243, 139, 168, 0.2);
    color: #f38ba8;
    border: 1px solid rgba(243, 139, 168, 0.4);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
  }
  .header-actions {
    display: flex;
    gap: 8px;
  }
  .btn-revoke {
    background: rgba(243, 139, 168, 0.2);
    border: 1px solid #f38ba8;
    color: #f38ba8;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .btn-copy {
    background: #313244;
    border: 1px solid #45475a;
    color: #cdd6f4;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
  }
  .btn-copy:hover {
    background: #45475a;
  }
  .detail-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .info-section {
    background: #181825;
    border: 1px solid #313244;
    border-radius: 8px;
    padding: 14px 16px;
  }
  .info-section h3 {
    margin: 0 0 12px;
    font-size: 11px;
    text-transform: uppercase;
    color: #89b4fa;
    letter-spacing: 0.5px;
  }
  .grid-props {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }
  .prop-item {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .prop-item.full-width {
    grid-column: 1 / -1;
  }
  .prop-item .lbl {
    font-size: 10px;
    color: #6c7086;
  }
  .prop-item .val {
    font-size: 12px;
    color: #cdd6f4;
  }
  .bold {
    font-weight: 600;
  }
  .highlight {
    color: #94e2d5;
  }
  .mono {
    font-family: monospace;
    font-size: 11px !important;
  }
  .san-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .san-tag {
    background: #1e1e2e;
    color: #89dceb;
    border: 1px solid #313244;
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-family: monospace;
  }
  .usage-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .usage-pill {
    background: #1e1e2e;
    color: #cdd6f4;
    border: 1px solid #313244;
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 11px;
  }
  .usage-pill.ext {
    color: #cba6f7;
    border-color: rgba(203, 166, 247, 0.4);
  }
  .fp-code {
    display: block;
    background: #11111b;
    color: #fab387;
    padding: 8px;
    border-radius: 4px;
    font-size: 11px;
    font-family: monospace;
    word-break: break-all;
    margin-bottom: 12px;
  }
  .pem-title {
    margin-top: 14px !important;
  }
  .pem-block {
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 6px;
    padding: 10px;
    font-size: 10px;
    color: #a6adc8;
    overflow-x: auto;
    font-family: monospace;
    margin: 0;
  }
  .empty-detail {
    padding: 50px;
    text-align: center;
    color: #6c7086;
  }
</style>
