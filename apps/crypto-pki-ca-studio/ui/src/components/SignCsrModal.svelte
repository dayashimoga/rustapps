<script lang="ts">
  import { invokeIpc } from '../lib/ipc';
  import type { X509Certificate } from '../lib/types';

  export let isOpen = false;
  export let cas: X509Certificate[] = [];
  export let onClose: () => void = () => {};
  export let onCertIssued: () => void = () => {};

  let selectedCaId = '';
  let commonName = '';
  let sansInput = '';
  let certType = 'tls_server';
  let validityDays = 365;
  let statusMessage = '';
  let isSuccess = false;
  let isLoading = false;

  $: if (cas.length > 0 && !selectedCaId) {
    selectedCaId = cas[0].certificate_id;
  }

  async function handleSign() {
    if (!commonName.trim()) {
      statusMessage = 'Common Name (CN) is required';
      isSuccess = false;
      return;
    }

    isLoading = true;
    statusMessage = '';

    const sans = sansInput
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);

    try {
      const cert = await invokeIpc<X509Certificate>('sign_csr', {
        ca_id: selectedCaId,
        common_name: commonName.trim(),
        cert_type: certType,
        sans,
        validity_days: validityDays,
      });

      isSuccess = true;
      statusMessage = `Certificate issued! Serial: ${cert.serial_number_hex}`;
      commonName = '';
      sansInput = '';
      onCertIssued();
      setTimeout(() => {
        onClose();
        statusMessage = '';
      }, 1500);
    } catch (e: any) {
      isSuccess = false;
      statusMessage = `Issuance failed: ${e.message || e}`;
    } finally {
      isLoading = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

{#if isOpen}
  <div class="modal-backdrop" on:click={onClose} on:keydown={handleKeyDown} role="presentation">
    <div class="modal-card" on:click|stopPropagation role="dialog" aria-modal="true" aria-label="Sign Certificate" tabindex="-1">
      <div class="modal-header">
        <div class="modal-title">
          <span class="icon">✍️</span>
          <span>Issue & Sign New X.509 Certificate</span>
        </div>
        <button type="button" class="close-btn" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body">
        <!-- Issuer CA Selection -->
        <div class="input-group">
          <label for="pki-issuer-ca">Signing Certificate Authority (Issuer)</label>
          <select id="pki-issuer-ca" bind:value={selectedCaId}>
            {#each cas as ca}
              <option value={ca.certificate_id}>
                {ca.subject.common_name} ({ca.cert_type === 'root_ca' ? 'Root' : 'Subordinate'})
              </option>
            {/each}
          </select>
        </div>

        <!-- Subject Common Name -->
        <div class="input-group">
          <label for="pki-common-name">Subject Common Name (CN)</label>
          <input
            id="pki-common-name"
            type="text"
            placeholder="e.g. cluster-api.corp.internal"
            bind:value={commonName}
          />
        </div>

        <!-- SANs -->
        <div class="input-group">
          <label for="pki-sans">Subject Alternative Names (Comma-separated)</label>
          <input
            id="pki-sans"
            type="text"
            placeholder="e.g. DNS:cluster-api.corp.internal, IP:10.0.0.5"
            bind:value={sansInput}
          />
        </div>

        <!-- Certificate Purpose -->
        <div class="grid-inputs">
          <div class="input-group">
            <label for="pki-cert-type">Certificate Type</label>
            <select id="pki-cert-type" bind:value={certType}>
              <option value="tls_server">TLS Server Certificate</option>
              <option value="tls_client">Mutual TLS Client (mTLS)</option>
              <option value="code_signing">Code Signing Certificate</option>
            </select>
          </div>

          <div class="input-group">
            <label for="pki-validity">Validity (Days)</label>
            <input
              id="pki-validity"
              type="number"
              bind:value={validityDays}
              min="1"
              max="7300"
            />
          </div>
        </div>

        {#if statusMessage}
          <div class="status-box" class:success={isSuccess} class:error={!isSuccess}>
            {statusMessage}
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button type="button" class="btn-cancel" on:click={onClose}>Cancel</button>
        <button type="button" class="btn-sign" on:click={handleSign} disabled={isLoading}>
          {isLoading ? 'Signing with CA Private Key...' : 'Issue Certificate'}
        </button>
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
    width: 560px;
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
    gap: 14px;
  }
  .input-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .input-group label {
    font-size: 12px;
    font-weight: 500;
    color: #cdd6f4;
  }
  .input-group input,
  .input-group select {
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 6px;
    padding: 9px 12px;
    color: #cdd6f4;
    font-size: 13px;
    outline: none;
  }
  .input-group input:focus,
  .input-group select:focus {
    border-color: #89b4fa;
  }
  .grid-inputs {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }
  .status-box {
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 12px;
  }
  .status-box.success {
    background: rgba(166, 227, 161, 0.15);
    border: 1px solid #a6e3a1;
    color: #a6e3a1;
  }
  .status-box.error {
    background: rgba(243, 139, 168, 0.15);
    border: 1px solid #f38ba8;
    color: #f38ba8;
  }
  .modal-footer {
    padding: 14px 20px;
    background: #11111b;
    border-top: 1px solid #313244;
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
  .btn-cancel {
    background: #313244;
    color: #cdd6f4;
    border: none;
    padding: 8px 14px;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
  }
  .btn-sign {
    background: #89b4fa;
    color: #11111b;
    border: none;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .btn-sign:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
