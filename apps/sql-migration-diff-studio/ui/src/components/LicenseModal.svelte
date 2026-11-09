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
        tier: string;
        is_valid: boolean;
      }>('apply_license_key', {
        license_key: licenseKey.trim(),
      });

      isSuccess = true;
      statusMessage = `License activated successfully! Tier: ${res.tier}`;
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
          Enter an offline cryptographic license key to unlock Enterprise zero-downtime online migration plans, multi-tenant DB schema synchronization, and Ed25519 change governance manifests.
        </p>

        <div class="input-group">
          <label for="lic-key">Cryptographic License Key</label>
          <textarea
            id="lic-key"
            bind:value={licenseKey}
            placeholder="Paste your offline Ed25519 signed license key..."
            rows="4"
          ></textarea>
        </div>

        {#if statusMessage}
          <div class="status-box" class:success={isSuccess} class:error={!isSuccess}>
            {statusMessage}
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button type="button" class="btn secondary" on:click={onClose}>Close</button>
        <button type="button" class="btn primary" on:click={applyLicense} disabled={isLoading}>
          {isLoading ? 'Verifying...' : 'Activate License'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(4, 7, 14, 0.82);
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
    width: 520px;
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
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .desc {
    color: #94a3b8;
    font-size: 0.88rem;
    line-height: 1.5;
    margin: 0;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label {
    color: #cbd5e1;
    font-size: 0.82rem;
    font-weight: 500;
  }

  textarea {
    background: #060a12;
    border: 1px solid #1e293b;
    border-radius: 6px;
    color: #e2e8f0;
    padding: 10px;
    font-family: monospace;
    font-size: 0.85rem;
    resize: vertical;
    outline: none;
  }

  textarea:focus {
    border-color: #3b82f6;
  }

  .status-box {
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .status-box.success {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid rgba(16, 185, 129, 0.4);
    color: #34d399;
  }

  .status-box.error {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: #f87171;
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

  .btn.primary:hover:not(:disabled) {
    background: #1d4ed8;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
