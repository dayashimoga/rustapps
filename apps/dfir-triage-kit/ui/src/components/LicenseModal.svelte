<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invokeIpc } from '../lib/ipc';

  export let isOpen: boolean = false;
  export let appName: string = 'Offline DFIR Triage Kit';

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
    licensee_name: 'Community Responder',
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
          <span class="icon">🛡️</span>
          <h2>Offline Licensing & Entitlements</h2>
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
                Perpetual Air-Gapped License
              {/if}
            </span>
          </div>

          <div class="meta-grid">
            <div class="meta-item">
              <span class="meta-label">Licensee</span>
              <span class="meta-val">{licenseStatus.licensee_name}</span>
            </div>
            <div class="meta-item">
              <span class="meta-label">Designated Email</span>
              <span class="meta-val">{licenseStatus.licensee_email}</span>
            </div>
            <div class="meta-item">
              <span class="meta-label">Max Offline Seats</span>
              <span class="meta-val">{licenseStatus.max_seats}</span>
            </div>
            <div class="meta-item">
              <span class="meta-label">Target Application</span>
              <span class="meta-val">{appName}</span>
            </div>
          </div>
        </div>

        <div class="input-section">
          <label for="license-json">Paste Ed25519-Signed License JSON</label>
          <textarea
            id="license-json"
            bind:value={licenseJsonInput}
            placeholder="&#123;&quot;payload&quot;: &#123;&quot;app_id&quot;: &quot;dfir-triage-kit&quot;, ...&#125;, &quot;signature&quot;: &quot;...&quot;&#125;"
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
          {isActivating ? 'Verifying Ed25519 Signature...' : 'Activate Offline License'}
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
    background: var(--bg-card);
    border: 1px solid var(--border);
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
    border-bottom: 1px solid var(--border);
  }

  .title-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .title-wrap h2 {
    font-size: 1.1rem;
    color: var(--text-main);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 1.2rem;
  }

  .close-btn:hover {
    color: var(--text-main);
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .status-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 14px;
  }

  .status-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .badge {
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 700;
    background: var(--border);
    color: var(--text-muted);
  }

  .badge.active {
    background: rgba(16, 185, 129, 0.2);
    color: var(--accent-green);
    border: 1px solid var(--accent-green);
  }

  .validity {
    font-size: 0.8rem;
    color: var(--text-sub);
  }

  .meta-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .meta-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .meta-label {
    font-size: 0.72rem;
    color: var(--text-sub);
    text-transform: uppercase;
  }

  .meta-val {
    font-size: 0.85rem;
    color: var(--text-main);
    font-family: var(--font-mono);
  }

  .input-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-section label {
    font-size: 0.85rem;
    color: var(--text-muted);
  }

  textarea {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px;
    color: var(--text-main);
    font-family: var(--font-mono);
    font-size: 0.8rem;
    resize: vertical;
  }

  textarea:focus {
    outline: none;
    border-color: var(--border-focus);
  }

  .alert {
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .alert.error {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid var(--accent-red);
    color: #fca5a5;
  }

  .alert.success {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid var(--accent-green);
    color: #6ee7b7;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 20px;
    border-top: 1px solid var(--border);
  }

  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 600;
  }

  .btn.primary {
    background: var(--accent-blue);
    border: 1px solid var(--accent-blue);
    color: #fff;
  }

  .btn.secondary {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-muted);
  }
</style>

