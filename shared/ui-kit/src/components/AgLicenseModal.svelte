<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { LicenseStatusInfo } from '../ipc/types';

  export let open: boolean = false;
  export let license: LicenseStatusInfo | null = null;
  export let appTitle: string = 'Application';

  const dispatch = createEventDispatcher<{
    close: void;
    activate: { licenseKey: string };
  }>();

  let licenseKeyInput: string = '';
  let copiedHardwareId: boolean = false;
  let isActivating: boolean = false;
  let activationError: string = '';

  function copyHardwareId() {
    if (!license?.hardware_id) return;
    navigator.clipboard.writeText(license.hardware_id);
    copiedHardwareId = true;
    setTimeout(() => {
      copiedHardwareId = false;
    }, 2000);
  }

  function handleFileDrop(e: DragEvent) {
    e.preventDefault();
    const files = e.dataTransfer?.files;
    if (files && files[0]) {
      const reader = new FileReader();
      reader.onload = (event) => {
        const text = event.target?.result as string;
        if (text) {
          licenseKeyInput = text.trim();
        }
      };
      reader.readAsText(files[0]);
    }
  }

  function handleActivate() {
    if (!licenseKeyInput.trim()) return;
    activationError = '';
    isActivating = true;
    dispatch('activate', { licenseKey: licenseKeyInput.trim() });
    setTimeout(() => {
      isActivating = false;
    }, 500);
  }

  function close() {
    open = false;
    dispatch('close');
  }
</script>

{#if open}
  <div class="ag-modal-overlay" on:click|self={close} role="presentation">
    <div class="ag-modal-container ag-glass-panel">
      <div class="ag-modal-header">
        <div class="ag-modal-title-group">
          <h2 class="ag-modal-title">{appTitle} License & Entitlements</h2>
          <span class="ag-badge ag-badge-{license?.tier.toLowerCase() || 'free'}">
            {license?.tier || 'Free'} Tier
          </span>
        </div>
        <button class="ag-close-btn" on:click={close} aria-label="Close">✕</button>
      </div>

      <div class="ag-modal-body">
        <!-- Current Status Banner -->
        <div class="ag-status-box">
          <div class="ag-status-row">
            <span class="ag-status-label">Licensed To:</span>
            <span class="ag-status-val">{license?.licensed_to || 'Community User'}</span>
          </div>
          <div class="ag-status-row">
            <span class="ag-status-label">Status:</span>
            <span class="ag-status-val">
              {#if license?.tier === 'ProLifetime'}
                <span class="ag-badge ag-badge-success">Perpetual (Never Expires)</span>
              {:else if license?.expires_at}
                <span class="ag-badge ag-badge-info">Active ({license.days_remaining ?? 365} days left)</span>
              {:else}
                <span class="ag-badge">Free Offline Edition</span>
              {/if}
            </span>
          </div>
          <div class="ag-status-row">
            <span class="ag-status-label">Machine Fingerprint:</span>
            <div class="ag-hw-group">
              <code class="ag-hw-code">{license?.hardware_id || 'LOCAL-OFFLINE-HOST'}</code>
              <button type="button" class="ag-btn" on:click={copyHardwareId}>
                {copiedHardwareId ? '✓ Copied' : 'Copy'}
              </button>
            </div>
          </div>
        </div>

        <!-- License Key or .offlic Dropzone -->
        <div class="ag-license-input-section">
          <label for="ag-license-key-area" class="ag-field-label">Activate Pro or Enterprise License</label>
          <div
            class="ag-dropzone"
            on:dragover|preventDefault
            on:drop={handleFileDrop}
            role="region"
            aria-label="License file dropzone"
          >
            <textarea
              id="ag-license-key-area"
              bind:value={licenseKeyInput}
              class="ag-license-textarea"
              placeholder="Paste license certificate JSON or drag & drop an offline .offlic license file here..."
              rows="4"
              spellcheck="false"
            ></textarea>
          </div>
          {#if activationError}
            <div class="ag-activation-err">{activationError}</div>
          {/if}
        </div>
      </div>

      <div class="ag-modal-footer">
        <button type="button" class="ag-btn" on:click={close}>Cancel</button>
        <button
          type="button"
          class="ag-btn ag-btn-primary"
          disabled={!licenseKeyInput.trim() || isActivating}
          on:click={handleActivate}
        >
          {isActivating ? 'Verifying...' : 'Activate License'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .ag-modal-overlay {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: oklch(0 0 0 / 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 20px;
  }

  .ag-modal-container {
    width: 100%;
    max-width: 520px;
    border-radius: var(--ag-radius-lg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .ag-modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
    border-bottom: 1px solid var(--ag-border);
  }

  .ag-modal-title-group {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .ag-modal-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  .ag-close-btn {
    background: transparent;
    border: none;
    color: var(--ag-text-muted);
    font-size: 14px;
    cursor: pointer;
  }

  .ag-close-btn:hover {
    color: var(--ag-text-primary);
  }

  .ag-modal-body {
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .ag-status-box {
    background: var(--ag-bg-panel);
    border: 1px solid var(--ag-border);
    border-radius: var(--ag-radius-md);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ag-status-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
  }

  .ag-status-label {
    color: var(--ag-text-muted);
  }

  .ag-status-val {
    font-weight: 500;
  }

  .ag-hw-group {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .ag-hw-code {
    font-size: 11px;
    background: var(--ag-bg-input);
    padding: 2px 6px;
    border-radius: var(--ag-radius-sm);
    border: 1px solid var(--ag-border);
  }

  .ag-field-label {
    display: block;
    font-size: 12px;
    font-weight: 500;
    margin-bottom: 6px;
    color: var(--ag-text-secondary);
  }

  .ag-dropzone {
    border: 1px dashed var(--ag-border-hover);
    border-radius: var(--ag-radius-md);
    padding: 4px;
    background: var(--ag-bg-input);
  }

  .ag-license-textarea {
    width: 100%;
    background: transparent;
    border: none;
    outline: none;
    color: var(--ag-text-primary);
    font-family: var(--ag-font-mono);
    font-size: 11px;
    resize: none;
    padding: 6px;
  }

  .ag-activation-err {
    margin-top: 6px;
    font-size: 11px;
    color: var(--ag-status-error);
  }

  .ag-modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    padding: 12px 18px;
    background: var(--ag-bg-panel);
    border-top: 1px solid var(--ag-border);
  }
</style>

