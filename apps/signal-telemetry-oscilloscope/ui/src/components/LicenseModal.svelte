<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invokeIpc } from '../lib/ipc';

  export let isOpen: boolean = false;
  export let appName: string = 'Scientific Signal & Telemetry Oscilloscope';

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
    licensee_name: 'Laboratory Free Tier',
    licensee_email: 'lab@localhost',
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
          <span class="icon">🔬</span>
          <h2>Offline Laboratory Licensing</h2>
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
                Perpetual Air-Gapped Instrument License
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
              <span class="meta-label">Instrument</span>
              <span class="meta-val">{appName}</span>
            </div>
          </div>
        </div>

        <div class="input-section">
          <label for="license-json">Paste Ed25519-Signed License JSON</label>
          <textarea
            id="license-json"
            bind:value={licenseJsonInput}
            placeholder="&#123;&quot;payload&quot;: &#123;&quot;app_id&quot;: &quot;signal-telemetry-oscilloscope&quot;, ...&#125;, &quot;signature&quot;: &quot;...&quot;&#125;"
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
          {isActivating ? 'Verifying Ed25519 Signature...' : 'Activate Instrument License'}
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
    background: #141c26;
    border: 1px solid #1e2c3d;
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
    border-bottom: 1px solid #1e2c3d;
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
    font-size: 16px;
    font-weight: 600;
    color: #e6edf3;
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
    color: #e6edf3;
    background: #1e2c3d;
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .status-card {
    background: #0d131a;
    border: 1px solid #1e2c3d;
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
    background: #2a3d54;
    color: #8b949e;
  }

  .badge.active {
    background: #238636;
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
    color: #e6edf3;
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
    background: #0d131a;
    border: 1px solid #1e2c3d;
    border-radius: 6px;
    color: #e6edf3;
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 10px;
    resize: vertical;
  }

  textarea:focus {
    outline: none;
    border-color: #58a6ff;
  }

  .alert {
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
  }

  .alert.error {
    background: #f8514922;
    border: 1px solid #f8514944;
    color: #f85149;
  }

  .alert.success {
    background: #2ea04322;
    border: 1px solid #2ea04344;
    color: #3fb950;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 20px;
    background: #0d131a;
    border-top: 1px solid #1e2c3d;
  }

  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    border: none;
  }

  .btn.secondary {
    background: #1e2c3d;
    color: #c9d1d9;
  }

  .btn.secondary:hover {
    background: #2a3d54;
  }

  .btn.primary {
    background: #238636;
    color: #fff;
  }

  .btn.primary:hover:not(:disabled) {
    background: #2ea043;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>

