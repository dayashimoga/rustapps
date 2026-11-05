<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invokeIpc } from '../lib/ipc';

  export let isOpen: boolean = false;
  export let appName: string = 'AI Audio Mastering & Stem Separation Studio';

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
    licensee_name: 'Studio Community License',
    licensee_email: 'producer@localhost',
    expires_at: null,
    max_seats: 1,
    features: [],
  };

  let licenseJsonInput: string = '';
  let isActivating: boolean = false;
  let errorMessage: string = '';
  let successMessage: string = '';

  $: if (isOpen) {
    loadStatus();
    errorMessage = '';
    successMessage = '';
  }

  export async function loadStatus() {
    try {
      const res = await invokeIpc<LicenseStatusReport>('get_license_status');
      if (res) {
        licenseStatus = res;
        dispatch('statusChange', licenseStatus);
      }
    } catch (e: any) {
      console.error('Failed to load license status:', e);
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
        license_key: licenseJsonInput.trim(),
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
      role="document"
    >
      <div class="modal-header">
        <div class="title-wrap">
          <span class="icon">🎹</span>
          <h2>Offline DAW & Mastering Studio Licensing</h2>
        </div>
        <button class="close-btn" on:click={handleClose} aria-label="Close modal">✕</button>
      </div>

      <div class="modal-body">
        <div class="status-card">
          <div class="status-header">
            <span class="badge" class:active={licenseStatus.is_active}>
              {licenseStatus.tier.toUpperCase()} TIER
            </span>
            <span class="validity">
              {#if licenseStatus.expires_at}
                Expires: {new Date(licenseStatus.expires_at * 1000).toLocaleDateString()}
              {:else}
                Perpetual Air-Gapped Commercial License
              {/if}
            </span>
          </div>

          <div class="meta-grid">
            <div class="meta-item">
              <span class="meta-label">Licensee</span>
              <span class="meta-val">{licenseStatus.licensee_name}</span>
            </div>
            <div class="meta-item">
              <span class="meta-label">Account Email</span>
              <span class="meta-val">{licenseStatus.licensee_email}</span>
            </div>
            <div class="meta-item">
              <span class="meta-label">Seats</span>
              <span class="meta-val">{licenseStatus.max_seats}</span>
            </div>
            <div class="meta-item">
              <span class="meta-label">Application</span>
              <span class="meta-val">{appName}</span>
            </div>
          </div>
        </div>

        <div class="input-section">
          <label for="license-json">Paste Ed25519-Signed License JSON</label>
          <textarea
            id="license-json"
            bind:value={licenseJsonInput}
            placeholder="&#123;&quot;payload&quot;: &#123;&quot;app_id&quot;: &quot;audio-mastering-studio&quot;, ...&#125;, &quot;signature&quot;: &quot;...&quot;&#125;"
            rows="5"
          ></textarea>
        </div>

        {#if errorMessage}
          <div class="alert error">{errorMessage}</div>
        {/if}
        {#if successMessage}
          <div class="alert success">{successMessage}</div>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn secondary" on:click={handleClose}>Cancel</button>
        <button class="btn primary" on:click={handleActivate} disabled={isActivating}>
          {isActivating ? 'Verifying Ed25519 Signature...' : 'Activate Studio License'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-card {
    background: #161b24;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    width: 580px;
    max-width: 90vw;
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.6);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color);
  }

  .title-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .icon {
    font-size: 20px;
  }

  h2 {
    font-size: 15px;
    font-weight: 600;
    color: #f1f5f9;
  }

  .close-btn {
    background: none;
    border: none;
    color: #8b949e;
    font-size: 16px;
    padding: 4px 8px;
    border-radius: 4px;
  }

  .close-btn:hover {
    color: #f1f5f9;
    background: #243042;
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .status-card {
    background: #0d1016;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .status-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .badge {
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.5px;
    background: #243042;
    color: #8b949e;
  }

  .badge.active {
    background: #10b981;
    color: #ffffff;
  }

  .validity {
    font-size: 12px;
    color: #8b949e;
  }

  .meta-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }

  .meta-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .meta-label {
    font-size: 10px;
    color: #8b949e;
    text-transform: uppercase;
    font-family: var(--font-mono);
  }

  .meta-val {
    font-size: 12px;
    color: #f1f5f9;
    font-weight: 500;
  }

  .input-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label {
    font-size: 11px;
    font-weight: 600;
    color: #8b949e;
    font-family: var(--font-mono);
  }

  textarea {
    background: #0d1016;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: #f1f5f9;
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 10px;
    resize: vertical;
  }

  textarea:focus {
    outline: none;
    border-color: var(--accent-gold);
  }

  .alert {
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
  }

  .alert.error {
    background: #ef444422;
    border: 1px solid #ef444444;
    color: #ef4444;
  }

  .alert.success {
    background: #10b98122;
    border: 1px solid #10b98144;
    color: #10b981;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 20px;
    background: #0d1016;
    border-top: 1px solid var(--border-color);
  }

  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    border: none;
  }

  .btn.secondary {
    background: #1e2838;
    color: #cbd5e1;
  }

  .btn.secondary:hover {
    background: #2e3e56;
  }

  .btn.primary {
    background: var(--accent-gold);
    color: #0e1117;
  }

  .btn.primary:hover:not(:disabled) {
    background: #f59e0b;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>

