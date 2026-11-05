<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invokeIpc } from '../lib/ipc';

  export let isOpen: boolean = false;
  export let appName: string = 'Application';

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
        dispatch('statusChange', res);
      }
    } catch (e: any) {
      console.warn('License status check:', e);
    } finally {
      isLoading = false;
    }
  }

  async function handleActivate() {
    if (!licenseJsonInput.trim()) {
      errorMessage = 'Please paste a valid offline license certificate (JSON).';
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
      successMessage = `License verified & activated! Tier: ${res.tier.toUpperCase()} (${res.licensee_name}).`;
      dispatch('statusChange', res);
      licenseJsonInput = '';
    } catch (err: any) {
      errorMessage = err?.message || String(err) || 'Cryptographic verification failed. Invalid or tampered certificate.';
    } finally {
      isActivating = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      close();
    }
  }

  function close() {
    dispatch('close');
  }

  function formatExpiry(timestamp: number | null): string {
    if (!timestamp) return 'Perpetual Offline Lifetime (Never Expires)';
    const d = new Date(timestamp * 1000);
    return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  }

  function getTierLabel(tier: string): string {
    switch (tier.toLowerCase()) {
      case 'premium':
        return 'Pro Edition';
      case 'business':
        return 'Enterprise Edition';
      default:
        return 'Community Edition';
    }
  }

  function getTierDescription(tier: string): string {
    switch (tier.toLowerCase()) {
      case 'premium':
        return 'Advanced professional features unlocked with offline cryptographic certificate.';
      case 'business':
        return 'Full enterprise multi-seat capabilities, compliance controls, and unlimited throughput.';
      default:
        return 'Core air-gapped features enabled with zero telemetry and standard SLA quotas.';
    }
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
  <div
    class="license-modal-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="license-modal-title"
    tabindex="-1"
    on:click|self={close}
    on:keydown={handleKeydown}
  >
    <div class="license-modal-card glass-panel">
      <!-- Modal Header -->
      <div class="modal-header">
        <div class="header-left">
          <div class="header-icon-box">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="header-icon">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
            </svg>
          </div>
          <div class="header-titles">
            <h3 id="license-modal-title">Offline Licensing &amp; Tiers</h3>
            <p class="header-subtitle">{appName} • Ed25519 Cryptographic Verification</p>
          </div>
        </div>
        <button type="button" class="close-btn" on:click={close} aria-label="Close modal">&times;</button>
      </div>

      <!-- Modal Body -->
      <div class="modal-body">
        <!-- Current Status Banner -->
        <div class="tier-card tier-{licenseStatus.tier.toLowerCase()}">
          <div class="tier-header-row">
            <div class="tier-badge-wrap">
              <span class="tier-pill tier-{licenseStatus.tier.toLowerCase()}">
                [{licenseStatus.tier.toUpperCase()}]
              </span>
              <span class="tier-name">{getTierLabel(licenseStatus.tier)}</span>
            </div>
            <div class="status-indicator">
              {#if licenseStatus.is_active}
                <span class="active-dot"></span>
                <span class="active-text">Cryptographically Verified</span>
              {:else}
                <span class="community-dot"></span>
                <span class="community-text">Community Mode</span>
              {/if}
            </div>
          </div>
          <p class="tier-desc">{getTierDescription(licenseStatus.tier)}</p>

          <div class="status-grid">
            <div class="status-field">
              <span class="field-label">Licensee</span>
              <span class="field-value">{licenseStatus.licensee_name || 'Community / Unregistered'}</span>
            </div>
            <div class="status-field">
              <span class="field-label">Associated Email</span>
              <span class="field-value">{licenseStatus.licensee_email || 'offline@localhost'}</span>
            </div>
            <div class="status-field">
              <span class="field-label">Seats Allowed</span>
              <span class="field-value">{licenseStatus.max_seats} seat{licenseStatus.max_seats > 1 ? 's' : ''}</span>
            </div>
            <div class="status-field">
              <span class="field-label">Expiration</span>
              <span class="field-value">{formatExpiry(licenseStatus.expires_at)}</span>
            </div>
          </div>

          {#if licenseStatus.features && licenseStatus.features.length > 0}
            <div class="unlocked-features">
              <span class="features-label">Active Entitlements:</span>
              <div class="feature-pills">
                {#each licenseStatus.features as feature}
                  <span class="feature-pill">{feature}</span>
                {/each}
              </div>
            </div>
          {/if}
        </div>

        <!-- Feedback Alerts -->
        {#if errorMessage}
          <div class="alert alert-error">
            <span class="alert-icon">⚠️</span>
            <div class="alert-content">
              <strong>Activation Error:</strong>
              <span>{errorMessage}</span>
            </div>
          </div>
        {/if}

        {#if successMessage}
          <div class="alert alert-success">
            <span class="alert-icon">✓</span>
            <div class="alert-content">
              <strong>Success:</strong>
              <span>{successMessage}</span>
            </div>
          </div>
        {/if}

        <!-- Activation Input Form -->
        <div class="activation-section">
          <label for="license-certificate-input" class="input-label">
            Paste Signed Offline License Certificate (JSON)
          </label>
          <p class="input-hint">
            Offline licenses are cryptographically signed with the official Ed25519 authority key.
            No internet connection or server validation is required.
          </p>
          <textarea
            id="license-certificate-input"
            class="certificate-textarea"
            rows="5"
            placeholder='&#123;"signature": "...", "parameters": &#123;"license_id": "...", "tier": "Premium", ...&#125;&#125;'
            bind:value={licenseJsonInput}
          ></textarea>

          <div class="action-row">
            <button
              type="button"
              class="btn btn-secondary"
              on:click={() => { licenseJsonInput = ''; errorMessage = ''; }}
              disabled={!licenseJsonInput || isActivating}
            >
              Clear
            </button>
            <button
              type="button"
              class="btn btn-primary"
              on:click={handleActivate}
              disabled={!licenseJsonInput.trim() || isActivating}
            >
              {#if isActivating}
                Verifying Signature...
              {:else}
                Verify &amp; Activate License
              {/if}
            </button>
          </div>
        </div>

        <!-- Air-gap Security Notice -->
        <div class="security-notice">
          <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16">
            <path fill-rule="evenodd" d="M2.166 4.999A11.954 11.954 0 0010 1.944 11.954 11.954 0 0017.834 5c.11.65.166 1.32.166 2.001 0 5.225-3.34 9.67-8 11.317C5.34 16.67 2 12.225 2 7c0-.682.057-1.35-.166-2.001zm11.541 3.708a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
          </svg>
          <span>
            <strong>100% Offline &amp; Zero Egress:</strong> Certificate validation is executed purely on-device using SHA-512 + Ed25519 curve mathematics. Your system emits zero network packets.
          </span>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .license-modal-overlay {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
  }

  .license-modal-card {
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 12px;
    width: 100%;
    max-width: 620px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
    overflow: hidden;
    color: #f1f5f9;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(30, 41, 59, 0.5);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .header-icon-box {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    background: rgba(59, 130, 246, 0.15);
    border: 1px solid rgba(59, 130, 246, 0.3);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #60a5fa;
  }

  .header-icon {
    width: 20px;
    height: 20px;
  }

  .header-titles h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 700;
    color: #f8fafc;
  }

  .header-subtitle {
    margin: 2px 0 0 0;
    font-size: 12px;
    color: #94a3b8;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 24px;
    line-height: 1;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 6px;
    transition: all 0.15s ease;
  }

  .close-btn:hover {
    color: #ffffff;
    background: rgba(255, 255, 255, 0.1);
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-height: 80vh;
    overflow-y: auto;
  }

  .tier-card {
    background: rgba(30, 41, 59, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 10px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .tier-card.tier-premium {
    background: rgba(59, 130, 246, 0.08);
    border-color: rgba(59, 130, 246, 0.35);
  }

  .tier-card.tier-business {
    background: rgba(168, 85, 247, 0.08);
    border-color: rgba(168, 85, 247, 0.35);
  }

  .tier-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .tier-badge-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .tier-pill {
    font-size: 11px;
    font-weight: 800;
    padding: 3px 8px;
    border-radius: 4px;
    letter-spacing: 0.05em;
  }

  .tier-pill.tier-free {
    background: rgba(148, 163, 184, 0.2);
    color: #cbd5e1;
    border: 1px solid rgba(148, 163, 184, 0.3);
  }

  .tier-pill.tier-premium {
    background: rgba(59, 130, 246, 0.25);
    color: #93c5fd;
    border: 1px solid rgba(59, 130, 246, 0.5);
  }

  .tier-pill.tier-business {
    background: rgba(168, 85, 247, 0.25);
    color: #e9d5ff;
    border: 1px solid rgba(168, 85, 247, 0.5);
  }

  .tier-name {
    font-size: 15px;
    font-weight: 700;
    color: #f8fafc;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }

  .active-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #10b981;
    box-shadow: 0 0 8px #10b981;
  }

  .active-text {
    color: #34d399;
    font-weight: 600;
  }

  .community-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #94a3b8;
  }

  .community-text {
    color: #94a3b8;
  }

  .tier-desc {
    margin: 0;
    font-size: 12px;
    color: #cbd5e1;
    line-height: 1.4;
  }

  .status-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
    background: rgba(15, 23, 42, 0.6);
    padding: 10px 12px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .status-field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .field-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #64748b;
  }

  .field-value {
    font-size: 12px;
    font-weight: 500;
    color: #e2e8f0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .unlocked-features {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .features-label {
    font-size: 11px;
    font-weight: 600;
    color: #94a3b8;
  }

  .feature-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .feature-pill {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid rgba(16, 185, 129, 0.3);
    color: #6ee7b7;
    font-size: 10px;
    padding: 2px 7px;
    border-radius: 4px;
    font-family: monospace;
  }

  .alert {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 14px;
    border-radius: 8px;
    font-size: 12px;
    line-height: 1.4;
  }

  .alert-error {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.35);
    color: #fca5a5;
  }

  .alert-success {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid rgba(16, 185, 129, 0.35);
    color: #6ee7b7;
  }

  .alert-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .activation-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .input-label {
    font-size: 13px;
    font-weight: 600;
    color: #f1f5f9;
  }

  .input-hint {
    margin: 0;
    font-size: 11px;
    color: #94a3b8;
    line-height: 1.4;
  }

  .certificate-textarea {
    width: 100%;
    box-sizing: border-box;
    background: rgba(15, 23, 42, 0.9);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 8px;
    color: #e2e8f0;
    font-family: monospace;
    font-size: 11px;
    padding: 10px;
    resize: vertical;
    outline: none;
    transition: border-color 0.15s ease;
  }

  .certificate-textarea:focus {
    border-color: #3b82f6;
  }

  .action-row {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 4px;
  }

  .btn {
    padding: 8px 16px;
    font-size: 12px;
    font-weight: 600;
    border-radius: 6px;
    cursor: pointer;
    border: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    transition: all 0.15s ease;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: rgba(255, 255, 255, 0.08);
    color: #cbd5e1;
    border: 1px solid rgba(255, 255, 255, 0.12);
  }

  .btn-secondary:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.14);
  }

  .btn-primary {
    background: #2563eb;
    color: #ffffff;
  }

  .btn-primary:hover:not(:disabled) {
    background: #1d4ed8;
  }

  .security-notice {
    display: flex;
    align-items: center;
    gap: 10px;
    background: rgba(15, 23, 42, 0.5);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
    padding: 10px 12px;
    font-size: 11px;
    color: #94a3b8;
    line-height: 1.4;
  }

  .security-notice svg {
    flex-shrink: 0;
    color: #10b981;
  }
</style>
