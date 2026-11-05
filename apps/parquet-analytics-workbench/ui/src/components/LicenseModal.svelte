<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invokeIpc } from '../lib/ipc';

  export let isOpen: boolean = false;
  export let appName: string = 'Parquet Analytics Workbench';

  export interface LicenseStatusReport {
    tier: 'Free' | 'Premium' | 'Business';
    is_active: boolean;
    licensee_name: string;
    licensee_email: string;
    expires_at: number | null;
    max_seats: number;
    features: string[];
  }

  const dispatch = createEventDispatcher<{
    close: void;
    statusChange: LicenseStatusReport;
  }>();

  let licenseStatus: LicenseStatusReport = {
    tier: 'Free',
    is_active: false,
    licensee_name: 'Community User',
    licensee_email: 'offline@localhost',
    expires_at: null,
    max_seats: 1,
    features: [],
  };

  let licenseJsonInput: string = '';
  let isLoading: boolean = false;
  let isActivating: boolean = false;
  let errorMessage: string = '';
  let successMessage: string = '';

  $: if (isOpen) {
    loadStatus();
    errorMessage = '';
    successMessage = '';
  }

  export async function loadStatus() {
    isLoading = true;
    try {
      const res = await invokeIpc<LicenseStatusReport>('get_license_status');
      if (res) {
        licenseStatus = res;
        dispatch('statusChange', licenseStatus);
      }
    } catch (e: any) {
      console.error('Failed to load license status:', e);
    } finally {
      isLoading = false;
    }
  }

  async function handleActivate() {
    if (!licenseJsonInput.trim()) {
      errorMessage = 'Please paste your Ed25519-signed offline license certificate JSON.';
      return;
    }
    isActivating = true;
    errorMessage = '';
    successMessage = '';
    try {
      const res = await invokeIpc<LicenseStatusReport>('activate_license', {
        license_json: licenseJsonInput.trim(),
      });
      licenseStatus = res;
      successMessage = `Successfully activated ${res.tier} tier for ${res.licensee_name}!`;
      licenseJsonInput = '';
      dispatch('statusChange', licenseStatus);
    } catch (e: any) {
      errorMessage = e?.message || 'Activation failed: Invalid signature or mismatched App ID.';
    } finally {
      isActivating = false;
    }
  }

  function handleClose() {
    dispatch('close');
  }
</script>

{#if isOpen}
  <!-- Backdrop -->
  <div
    class="modal-backdrop"
    on:click={handleClose}
    role="button"
    tabindex="0"
    on:keydown={(e) => e.key === 'Escape' && handleClose()}
  >
    <div
      class="modal-card"
      on:click|stopPropagation
      on:keydown|stopPropagation
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
      tabindex="-1"
    >
      <div class="modal-header">
        <div class="title-wrap">
          <span class="icon">📜</span>
          <h2 id="modal-title">{appName} Offline Licensing</h2>
        </div>
        <button class="close-btn" on:click={handleClose} aria-label="Close modal">✕</button>
      </div>

      <div class="modal-body">
        <!-- Current Status Banner -->
        <div class="status-banner tier-{licenseStatus.tier.toLowerCase()}">
          <div class="tier-badge-large">{licenseStatus.tier.toUpperCase()} TIER</div>
          <div class="licensee-info">
            <span class="name">{licenseStatus.licensee_name}</span>
            <span class="email">{licenseStatus.licensee_email}</span>
          </div>
          <div class="seat-info">
            {#if licenseStatus.is_active}
              <span class="status-tag active">● 100% Offline Active</span>
            {:else}
              <span class="status-tag free">● Free Community Edition</span>
            {/if}
            {#if licenseStatus.expires_at}
              <span class="expiry">Expires: {new Date(licenseStatus.expires_at * 1000).toLocaleDateString()}</span>
            {:else}
              <span class="expiry">Perpetual Offline License</span>
            {/if}
          </div>
        </div>

        {#if errorMessage}
          <div class="alert error">{errorMessage}</div>
        {/if}
        {#if successMessage}
          <div class="alert success">{successMessage}</div>
        {/if}

        <!-- Paste License Section -->
        <div class="input-section">
          <label for="lic-json">Activate Offline License Certificate</label>
          <textarea
            id="lic-json"
            bind:value={licenseJsonInput}
            placeholder="Paste your Ed25519-signed .offlic JSON certificate here..."
            rows="5"
          ></textarea>
          <div class="action-row">
            <button class="btn btn-secondary" on:click={handleClose}>Cancel</button>
            <button class="btn btn-primary" on:click={handleActivate} disabled={isActivating || !licenseJsonInput.trim()}>
              {#if isActivating}Verifying...{:else}Activate Offline License{/if}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .modal-card {
    background: #111827;
    border: 1px solid rgba(75, 85, 99, 0.4);
    border-radius: 12px;
    width: 90%;
    max-width: 600px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
    overflow: hidden;
  }
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid rgba(75, 85, 99, 0.3);
  }
  .title-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .title-wrap h2 {
    font-size: 1.15rem;
    font-weight: 600;
    color: #f9fafb;
    margin: 0;
  }
  .close-btn {
    background: none;
    border: none;
    color: #9ca3af;
    font-size: 1.25rem;
    cursor: pointer;
  }
  .close-btn:hover {
    color: #f3f4f6;
  }
  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .status-banner {
    padding: 16px;
    border-radius: 8px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }
  .status-banner.tier-free {
    background: rgba(55, 65, 81, 0.5);
  }
  .status-banner.tier-premium {
    background: rgba(14, 165, 233, 0.15);
    border-color: rgba(14, 165, 233, 0.4);
  }
  .status-banner.tier-business {
    background: rgba(139, 92, 246, 0.2);
    border-color: rgba(139, 92, 246, 0.5);
  }
  .tier-badge-large {
    font-size: 0.85rem;
    font-weight: 700;
    padding: 4px 10px;
    border-radius: 6px;
    background: #374151;
    color: #e5e7eb;
  }
  .tier-premium .tier-badge-large {
    background: #0284c7;
    color: #ffffff;
  }
  .tier-business .tier-badge-large {
    background: #7c3aed;
    color: #ffffff;
  }
  .licensee-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .licensee-info .name {
    font-weight: 600;
    color: #f3f4f6;
  }
  .licensee-info .email {
    font-size: 0.8rem;
    color: #9ca3af;
  }
  .seat-info {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
  }
  .status-tag.active {
    color: #34d399;
    font-size: 0.8rem;
    font-weight: 600;
  }
  .status-tag.free {
    color: #9ca3af;
    font-size: 0.8rem;
  }
  .expiry {
    font-size: 0.75rem;
    color: #6b7280;
  }
  .alert {
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 0.85rem;
  }
  .alert.error {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.3);
  }
  .alert.success {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.3);
  }
  .input-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .input-section label {
    font-size: 0.85rem;
    font-weight: 500;
    color: #d1d5db;
  }
  textarea {
    background: #1f2937;
    border: 1px solid #374151;
    border-radius: 6px;
    color: #f3f4f6;
    padding: 10px;
    font-family: monospace;
    font-size: 0.85rem;
    resize: vertical;
  }
  textarea:focus {
    outline: none;
    border-color: #6366f1;
  }
  .action-row {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 6px;
  }
  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
  }
  .btn-primary {
    background: #6366f1;
    color: #ffffff;
  }
  .btn-primary:hover:not(:disabled) {
    background: #4f46e5;
  }
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .btn-secondary {
    background: #374151;
    color: #d1d5db;
  }
  .btn-secondary:hover {
    background: #4b5563;
  }
</style>
