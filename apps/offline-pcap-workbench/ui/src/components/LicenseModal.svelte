<script lang="ts">
  import { invokeIpc } from '../lib/ipc';

  export let isOpen = false;
  export let onClose: () => void = () => {};
  export let onLicenseUpdated: () => void = () => {};

  let licenseKey = '';
  let statusMessage = '';
  let isSuccess = false;
  let isLoading = false;

  async function applyLicense() {
    if (!licenseKey.trim()) {
      statusMessage = 'Please enter a valid cryptographic license key';
      isSuccess = false;
      return;
    }

    isLoading = true;
    statusMessage = '';

    try {
      const res = await invokeIpc<{
        success: boolean;
        tier: string;
        licensee: string;
        expiry_timestamp: number;
      }>('apply_license_key', {
        license_key: licenseKey.trim(),
      });

      isSuccess = true;
      statusMessage = `License activated! Tier: ${res.tier} (${res.licensee})`;
      licenseKey = '';
      onLicenseUpdated();
    } catch (e: any) {
      isSuccess = false;
      statusMessage = `Activation failed: ${e.message || e}`;
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
    <div class="modal-card" on:click|stopPropagation role="dialog" aria-modal="true" aria-label="License Activation" tabindex="-1">
      <div class="modal-header">
        <div class="modal-title">
          <span class="icon">🔑</span>
          <span>Air-Gapped License Manager</span>
        </div>
        <button type="button" class="close-btn" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body">
        <p class="desc">
          Offline PCAP Workbench operates 100% locally. Enter an Ed25519-signed enterprise license key
          to unlock unlimited high-speed packet ingestion, deep payload carving, and forensic exports.
        </p>

        <div class="input-group">
          <label for="pcap-lic-key">Enterprise License Key</label>
          <textarea
            id="pcap-lic-key"
            bind:value={licenseKey}
            placeholder="PAW-LIC-..."
            rows="3"
            spellcheck="false"
          ></textarea>
        </div>

        {#if statusMessage}
          <div class="status-box" class:success={isSuccess} class:error={!isSuccess}>
            {statusMessage}
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button type="button" class="btn-cancel" on:click={onClose}>Close</button>
        <button type="button" class="btn-activate" on:click={applyLicense} disabled={isLoading}>
          {isLoading ? 'Verifying Ed25519 Signature...' : 'Activate License'}
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
    width: 520px;
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
    border-bottom: 1px solid #313244;
    background: #11111b;
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
  }
  .desc {
    font-size: 13px;
    color: #a6adc8;
    line-height: 1.5;
    margin: 0;
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
  .input-group textarea {
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 6px;
    padding: 10px;
    color: #cdd6f4;
    font-family: monospace;
    font-size: 12px;
    resize: vertical;
    outline: none;
  }
  .input-group textarea:focus {
    border-color: #89b4fa;
  }
  .status-box {
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 12px;
    line-height: 1.4;
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
  .btn-activate {
    background: #89b4fa;
    color: #11111b;
    border: none;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .btn-activate:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
