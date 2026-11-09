<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invokeIpc } from '../lib/ipc';

  export let isOpen: boolean = false;
  export let appName: string = 'Local Synthetic Data & Anonymization Studio';

  const dispatch = createEventDispatcher();

  let licenseKeyInput: string = '';
  let statusMessage: string = '';
  let isError: boolean = false;

  async function handleApplyKey() {
    if (!licenseKeyInput.trim()) return;
    try {
      const res = await invokeIpc<any>('apply_license_key', {
        license_key: licenseKeyInput.trim(),
      });
      if (res.tier) {
        statusMessage = `License activated: ${res.tier.toUpperCase()}`;
        isError = false;
        dispatch('statusChange', { tier: res.tier });
      } else {
        statusMessage = 'Invalid or expired offline Ed25519 license key.';
        isError = true;
      }
    } catch (err: any) {
      statusMessage = `Activation error: ${err?.message || err}`;
      isError = true;
    }
  }

  function handleClose() {
    statusMessage = '';
    dispatch('close');
  }
</script>

{#if isOpen}
  <div
    class="modal-backdrop"
    on:click={handleClose}
    on:keydown={(e) => e.key === 'Escape' && handleClose()}
    role="presentation"
  >
    <div
      class="modal-card"
      on:click|stopPropagation
      role="dialog"
      aria-modal="true"
      aria-label="Offline License Activation"
    >
      <div class="modal-header">
        <h3>OFFLINE LICENSE ACTIVATION</h3>
        <button class="close-x" on:click={handleClose}>✕</button>
      </div>

      <div class="modal-body">
        <p class="app-title">{appName}</p>
        <p class="desc">
          Paste your cryptographically signed Ed25519 offline license key (<code>.offlic</code> content).
          Validation executes 100% locally with zero network egress.
        </p>

        <div class="input-wrap">
          <label for="lic-input">Offline License Certificate Key:</label>
          <textarea
            id="lic-input"
            bind:value={licenseKeyInput}
            placeholder="AGLIC-V1:eyJhbGciOiJFZDI1NTE5Iiw..."
            rows="4"
          ></textarea>
        </div>

        {#if statusMessage}
          <div class="status-box" class:error={isError} class:success={!isError}>
            {statusMessage}
          </div>
        {/if}

        <div class="tier-features">
          <span class="feat-title">Commercial Tiers:</span>
          <ul>
            <li><strong>Community:</strong> Single-table schema synthesis & basic HMAC pseudonymization</li>
            <li><strong>Pro ($129/yr):</strong> Multi-table referential integrity & Gaussian distribution cloning</li>
            <li><strong>Enterprise ($45/mo):</strong> Signed differential privacy audit certificates & unlimited row exports</li>
          </ul>
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" on:click={handleClose}>Cancel</button>
        <button class="btn-activate" on:click={handleApplyKey}>Activate License</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(4, 7, 12, 0.75);
    backdrop-filter: blur(8px);
    z-index: 10001;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal-card {
    width: 480px;
    max-width: 90vw;
    background: #0f141c;
    border: 1px solid #232c3d;
    border-radius: 10px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.7);
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 18px;
    background: #141b26;
    border-bottom: 1px solid #1e2838;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 0.85rem;
    font-family: monospace;
    color: #38bdf8;
    letter-spacing: 0.5px;
  }

  .close-x {
    background: none;
    border: none;
    color: #64748b;
    font-size: 1rem;
    cursor: pointer;
  }

  .modal-body {
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .app-title {
    font-size: 0.95rem;
    font-weight: 600;
    color: #f1f5f9;
    margin: 0;
  }

  .desc {
    font-size: 0.8rem;
    color: #8b949e;
    line-height: 1.4;
    margin: 0;
  }

  .input-wrap {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-wrap label {
    font-size: 0.75rem;
    font-family: monospace;
    color: #cbd5e1;
  }

  textarea {
    background: #090d14;
    border: 1px solid #28374d;
    border-radius: 6px;
    color: #f1f5f9;
    padding: 8px;
    font-family: monospace;
    font-size: 0.75rem;
    resize: none;
    outline: none;
  }

  textarea:focus {
    border-color: #38bdf8;
  }

  .status-box {
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.75rem;
    font-family: monospace;
  }

  .status-box.success {
    background: rgba(56, 189, 248, 0.15);
    border: 1px solid #38bdf8;
    color: #38bdf8;
  }

  .status-box.error {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid #ef4444;
    color: #f87171;
  }

  .tier-features {
    background: #141b26;
    border: 1px solid #1e2838;
    border-radius: 6px;
    padding: 10px 14px;
    font-size: 0.75rem;
  }

  .feat-title {
    color: #94a3b8;
    font-weight: 600;
  }

  ul {
    margin: 6px 0 0 16px;
    padding: 0;
    color: #cbd5e1;
  }

  li {
    margin-bottom: 4px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 12px 18px;
    background: #090d14;
    border-top: 1px solid #161e2b;
  }

  .btn-cancel {
    background: #1e2838;
    border: 1px solid #2e3e56;
    color: #94a3b8;
    padding: 6px 14px;
    border-radius: 4px;
    font-size: 0.8rem;
    cursor: pointer;
  }

  .btn-activate {
    background: #0284c7;
    border: 1px solid #38bdf8;
    color: #ffffff;
    font-weight: 700;
    padding: 6px 16px;
    border-radius: 4px;
    font-size: 0.8rem;
    cursor: pointer;
  }

  .btn-activate:hover {
    background: #0369a1;
  }
</style>
