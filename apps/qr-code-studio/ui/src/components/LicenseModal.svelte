<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from '../lib/ipc';

  export let isOpen = false;
  export let onClose: () => void = () => {};

  let licenseKey = '';
  let statusReport: {
    app_id: string;
    tier: string;
    is_licensed: boolean;
    licensee: string | null;
    expires_at: number | null;
    hardware_fingerprint: string;
    entitlements: string[];
  } | null = null;

  let isActivating = false;
  let activationError = '';
  let activationSuccess = '';

  onMount(() => {
    fetchLicenseStatus();
  });

  async function fetchLicenseStatus() {
    try {
      statusReport = await invokeIpc<any>('get_license_status', {});
    } catch (e: any) {
      console.warn('Failed to get license status:', e);
    }
  }

  async function activate() {
    if (!licenseKey.trim()) return;
    isActivating = true;
    activationError = '';
    activationSuccess = '';

    try {
      const res = await invokeIpc<any>('activate_license', { key: licenseKey.trim() });
      statusReport = res;
      activationSuccess = `Successfully activated ${res.tier} license!`;
      licenseKey = '';
    } catch (e: any) {
      activationError = e?.message || String(e);
    } finally {
      isActivating = false;
    }
  }
</script>

{#if isOpen}
  <div class="modal-backdrop" role="presentation" on:click={onClose} on:keydown={(e) => e.key === 'Escape' && onClose()}>
    <div class="modal-dialog" role="dialog" aria-modal="true" on:click|stopPropagation on:keydown|stopPropagation>
      <div class="modal-header">
        <h3>Offline Licensing & Entitlements</h3>
        <button class="btn-close" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body">
        {#if statusReport}
          <div class="status-card">
            <div class="tier-row">
              <span class="tier-name">{statusReport.tier} Edition</span>
              <span class="licensed-pill" class:active={statusReport.is_licensed}>
                {statusReport.is_licensed ? '✓ Activated' : 'Community / Unregistered'}
              </span>
            </div>

            <div class="meta-row">
              <span class="meta-label">Licensee:</span>
              <span class="meta-val">{statusReport.licensee || 'Local Workstation'}</span>
            </div>

            <div class="meta-row">
              <span class="meta-label">Hardware Hash:</span>
              <span class="meta-val font-mono">{statusReport.hardware_fingerprint.slice(0, 16)}...</span>
            </div>

            <div class="entitlements-box">
              <span class="entitlements-label">Active Capabilities:</span>
              <div class="entitlements-tags">
                {#each statusReport.entitlements as ent}
                  <span class="ent-tag">✓ {ent}</span>
                {/each}
              </div>
            </div>
          </div>
        {/if}

        <div class="activation-section">
          <label for="lic-key">Activate Offline License Key</label>
          <div class="key-row">
            <input
              id="lic-key"
              type="text"
              placeholder="e.g. AGQR-PRO-XXXX-XXXX-XXXX"
              bind:value={licenseKey}
              class="lic-input"
            />
            <button class="btn-activate" on:click={activate} disabled={isActivating}>
              {isActivating ? 'Verifying...' : 'Activate'}
            </button>
          </div>

          {#if activationError}
            <div class="msg-error">⚠️ {activationError}</div>
          {/if}
          {#if activationSuccess}
            <div class="msg-success">✓ {activationSuccess}</div>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-dialog {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    width: 480px;
    max-width: 90vw;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
  }

  .modal-header {
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .modal-header h3 {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .btn-close {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 1.1rem;
    cursor: pointer;
  }

  .modal-body {
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .status-card {
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .tier-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .tier-name {
    font-size: 1.1rem;
    font-weight: 800;
    color: var(--accent-cyan);
  }

  .licensed-pill {
    padding: 0.25rem 0.6rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 700;
    background: rgba(148, 163, 184, 0.2);
    color: var(--text-muted);
  }

  .licensed-pill.active {
    background: rgba(16, 185, 129, 0.15);
    color: var(--accent-emerald);
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .meta-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.8rem;
  }

  .meta-label {
    color: var(--text-muted);
  }

  .meta-val {
    color: var(--text-primary);
  }

  .font-mono {
    font-family: var(--font-mono);
  }

  .entitlements-box {
    margin-top: 0.5rem;
    border-top: 1px solid var(--border-color);
    padding-top: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .entitlements-label {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .entitlements-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  .ent-tag {
    background: rgba(99, 102, 241, 0.15);
    color: var(--accent-indigo);
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
  }

  .activation-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .activation-section label {
    font-size: 0.8rem;
    font-weight: 700;
    color: var(--text-secondary);
    text-transform: uppercase;
  }

  .key-row {
    display: flex;
    gap: 0.5rem;
  }

  .lic-input {
    flex: 1;
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: 6px;
    padding: 0.55rem 0.75rem;
    font-size: 0.85rem;
    font-family: var(--font-mono);
  }

  .lic-input:focus {
    outline: none;
    border-color: var(--accent-cyan);
  }

  .btn-activate {
    background: var(--accent-cyan);
    color: #0b0f19;
    font-weight: 600;
    font-size: 0.85rem;
    padding: 0 1.25rem;
    border: none;
    border-radius: 6px;
    cursor: pointer;
  }

  .msg-error {
    color: var(--accent-rose);
    font-size: 0.8rem;
  }

  .msg-success {
    color: var(--accent-emerald);
    font-size: 0.8rem;
  }
</style>
