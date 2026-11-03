<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let isOpen: boolean = false;
  export let mode: 'split' | 'unlock' = 'split';

  const dispatch = createEventDispatcher();

  // Split mode state
  let threshold: number = 3;
  let totalShares: number = 5;
  let isSplitting: boolean = false;
  let splitError: string = '';
  let generatedShares: Array<{
    index: number;
    threshold: number;
    total_shares: number;
    data: number[];
    token?: string;
  }> = [];

  // Unlock mode state
  let shareTokens: string[] = ['', '', ''];
  let isUnlocking: boolean = false;
  let unlockError: string = '';

  async function invokeIpc<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
    const tauri = (window as unknown as { __TAURI__?: { invoke?: (c: string, a: Record<string, unknown>) => Promise<T> } }).__TAURI__;
    if (tauri?.invoke) {
      return await tauri.invoke(cmd, args);
    }
    if (typeof fetch !== 'undefined' && typeof window !== 'undefined' && window.location && window.location.protocol.startsWith('http')) {
      const res = await fetch('/__ipc/' + encodeURIComponent(cmd), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(args || {}),
      });
      if (res.ok) {
        const payload = await res.json();
        if (payload && '__ipc_data' in payload) return payload.__ipc_data as T;
        return payload as T;
      }
    }
    throw new Error('IPC bridge not available');
  }

  function formatShareToken(share: { index: number; threshold: number; total_shares: number; data: number[] }): string {
    const hexData = share.data.map(b => b.toString(16).padStart(2, '0')).join('');
    return `vault-share-k${share.threshold}n${share.total_shares}-${share.index.toString(16).padStart(2, '0')}-${hexData}`;
  }

  async function handleSplitKey() {
    splitError = '';
    if (threshold > totalShares) {
      splitError = 'Threshold (k) cannot exceed total shares (n).';
      return;
    }
    if (threshold < 2) {
      splitError = 'Threshold must be at least 2.';
      return;
    }

    isSplitting = true;
    try {
      const res = await invokeIpc<Array<{ index: number; threshold: number; total_shares: number; data: number[] }>>(
        'split_master_key',
        { threshold: Number(threshold), totalShares: Number(totalShares) }
      );
      generatedShares = res.map(s => ({
        ...s,
        token: formatShareToken(s),
      }));
    } catch (e: unknown) {
      const err = e as { message?: string };
      splitError = err.message || String(e);
    } finally {
      isSplitting = false;
    }
  }

  async function handleUnlockWithShares() {
    unlockError = '';
    const activeTokens = shareTokens.map(t => t.trim()).filter(Boolean);
    if (activeTokens.length < 2) {
      unlockError = 'Please provide at least 2 custodian share tokens.';
      return;
    }

    isUnlocking = true;
    try {
      await invokeIpc('unlock_with_shamir_shares', { shares: activeTokens });
      dispatch('unlocked');
      closeModal();
    } catch (e: unknown) {
      const err = e as { message?: string };
      unlockError = err.message || String(e);
    } finally {
      isUnlocking = false;
    }
  }

  function copyToken(token: string) {
    if (navigator?.clipboard) {
      navigator.clipboard.writeText(token);
    }
  }

  function downloadAllShares() {
    if (generatedShares.length === 0) return;
    const lines = [
      `# DevOps Secret Vault - Shamir Master Key Shares`,
      `# Threshold: ${threshold} of ${totalShares} required for recovery`,
      `# Timestamp: ${new Date().toISOString()}`,
      `# WARNING: Distribute each share to a separate trusted custodian!`,
      '',
    ];
    generatedShares.forEach((s, idx) => {
      lines.push(`Share #${idx + 1} (Custodian ${idx + 1}):`);
      lines.push(s.token || formatShareToken(s));
      lines.push('');
    });

    const blob = new Blob([lines.join('\n')], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `shamir-shares-k${threshold}n${totalShares}-${Date.now()}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function addShareInput() {
    if (shareTokens.length < 10) {
      shareTokens = [...shareTokens, ''];
    }
  }

  function removeShareInput(idx: number) {
    if (shareTokens.length > 2) {
      shareTokens = shareTokens.filter((_, i) => i !== idx);
    }
  }

  function closeModal() {
    dispatch('close');
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions a11y-no-noninteractive-element-interactions -->
  <div class="modal-backdrop" role="dialog" aria-modal="true" tabindex="-1" on:click|self={closeModal} on:keydown={e => { if (e.key === 'Escape') closeModal(); }}>
    <div class="modal-card glass-panel">
      <div class="modal-header">
        <div class="header-left">
          <div class="shamir-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="20" height="20">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
            </svg>
          </div>
          <div>
            <h2 class="modal-title">
              {mode === 'split' ? "Shamir's Secret Sharing (GF-256)" : 'Emergency Shamir Key Recovery'}
            </h2>
            <p class="modal-subtitle">
              {mode === 'split'
                ? 'Split vault master key into k-of-n threshold shares for multi-custodian governance.'
                : 'Reconstruct 256-bit AES master key from custodian threshold shares.'}
            </p>
          </div>
        </div>
        <button class="close-btn" on:click={closeModal}>✕</button>
      </div>

      <div class="modal-body">
        {#if mode === 'split'}
          <div class="split-controls">
            <div class="grid-2">
              <div class="input-field">
                <label for="shamir-k">Threshold (k) — Required to Unlock</label>
                <input
                  id="shamir-k"
                  type="number"
                  min="2"
                  max="10"
                  bind:value={threshold}
                  disabled={isSplitting}
                />
                <span class="field-hint">e.g. 3 custodians must agree</span>
              </div>
              <div class="input-field">
                <label for="shamir-n">Total Shares (n) — Generated</label>
                <input
                  id="shamir-n"
                  type="number"
                  min="2"
                  max="10"
                  bind:value={totalShares}
                  disabled={isSplitting}
                />
                <span class="field-hint">e.g. distributed across 5 custodians</span>
              </div>
            </div>

            <div class="action-row">
              <button class="btn btn-primary" on:click={handleSplitKey} disabled={isSplitting}>
                {#if isSplitting}
                  <span class="spinner"></span> Generating Polynomial...
                {:else}
                  Generate {totalShares} Shamir Shares ({threshold}-of-{totalShares})
                {/if}
              </button>
            </div>

            {#if splitError}
              <div class="error-banner">
                <strong>Error:</strong> {splitError}
              </div>
            {/if}

            {#if generatedShares.length > 0}
              <div class="shares-output-section">
                <div class="shares-header">
                  <h3>Generated Custodian Shares ({generatedShares.length})</h3>
                  <button class="btn-download" on:click={downloadAllShares}>
                    📥 Download All (.txt)
                  </button>
                </div>
                <div class="warning-callout">
                  ⚠️ <strong>Security Note:</strong> Any {threshold} of these {totalShares} shares will immediately
                  reconstruct the master encryption key. Do NOT store all shares in one place!
                </div>

                <div class="shares-list">
                  {#each generatedShares as share, idx}
                    <div class="share-card">
                      <div class="share-header">
                        <span class="share-badge">Share #{idx + 1} of {share.total_shares}</span>
                        <button class="btn-copy" on:click={() => copyToken(share.token || '')}>
                          Copy Token
                        </button>
                      </div>
                      <div class="share-token-box">
                        <code>{share.token}</code>
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {:else}
          <!-- Unlock Mode -->
          <div class="unlock-controls">
            <p class="unlock-desc">
              Paste the required threshold custodian tokens below. The shares will be mathematically evaluated
              over Galois Field GF(2⁸) to reconstruct the master key and decrypt your vault.
            </p>

            <div class="tokens-list">
              {#each shareTokens as token, idx}
                <div class="token-input-row">
                  <div class="token-label">Custodian Share #{idx + 1}</div>
                  <div class="input-with-del">
                    <textarea
                      class="share-textarea mono"
                      placeholder="vault-share-k3n5-01-..."
                      bind:value={shareTokens[idx]}
                      rows="2"
                      disabled={isUnlocking}
                    ></textarea>
                    {#if shareTokens.length > 2}
                      <button
                        type="button"
                        class="btn-del"
                        on:click={() => removeShareInput(idx)}
                        title="Remove share"
                      >
                        ✕
                      </button>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>

            <div class="add-share-row">
              <button type="button" class="btn-add" on:click={addShareInput} disabled={shareTokens.length >= 10}>
                + Add Another Custodian Share
              </button>
            </div>

            {#if unlockError}
              <div class="error-banner">
                <strong>Error:</strong> {unlockError}
              </div>
            {/if}

            <div class="unlock-action-row">
              <button
                class="btn btn-primary submit-btn"
                on:click={handleUnlockWithShares}
                disabled={isUnlocking}
              >
                {#if isUnlocking}
                  <span class="spinner"></span> Reconstructing GF(256) Key...
                {:else}
                  Reconstruct Key & Unlock Vault
                {/if}
              </button>
            </div>
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn btn-secondary" on:click={closeModal}>Close</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(4, 7, 12, 0.85);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 250;
  }

  .modal-card {
    width: 680px;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: rgba(15, 23, 42, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 12px;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .shamir-icon {
    width: 38px;
    height: 38px;
    border-radius: 8px;
    background: rgba(168, 85, 247, 0.15);
    color: #a855f7;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal-title {
    font-size: 1.05rem;
    font-weight: 600;
    color: #f8fafc;
    margin: 0;
  }

  .modal-subtitle {
    font-size: 0.8rem;
    color: #94a3b8;
    margin: 2px 0 0 0;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #64748b;
    font-size: 18px;
    cursor: pointer;
  }

  .close-btn:hover {
    color: white;
  }

  .modal-body {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }

  .input-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-field label {
    font-size: 0.8rem;
    font-weight: 600;
    color: #cbd5e1;
  }

  .input-field input {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    padding: 8px 12px;
    color: #f8fafc;
    font-size: 0.95rem;
  }

  .field-hint {
    font-size: 0.75rem;
    color: #64748b;
  }

  .action-row {
    margin-top: 14px;
  }

  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-primary {
    background: #a855f7;
    color: white;
    border: none;
  }

  .btn-primary:hover:not(:disabled) {
    background: #9333ea;
  }

  .btn-secondary {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: #94a3b8;
  }

  .btn-secondary:hover {
    color: white;
    background: rgba(255, 255, 255, 0.05);
  }

  .error-banner {
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 6px;
    padding: 8px 12px;
    color: #f87171;
    font-size: 0.82rem;
  }

  .warning-callout {
    background: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.3);
    border-radius: 6px;
    padding: 8px 12px;
    color: #fcd34d;
    font-size: 0.82rem;
    margin-bottom: 12px;
  }

  .shares-output-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 16px;
  }

  .shares-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .shares-header h3 {
    margin: 0;
    font-size: 0.95rem;
    color: #e2e8f0;
  }

  .btn-download {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    color: #cbd5e1;
    font-size: 0.78rem;
    padding: 4px 10px;
    cursor: pointer;
  }

  .btn-download:hover {
    color: white;
    background: rgba(255, 255, 255, 0.15);
  }

  .shares-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 280px;
    overflow-y: auto;
  }

  .share-card {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .share-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .share-badge {
    font-size: 0.75rem;
    font-weight: 700;
    color: #c084fc;
    text-transform: uppercase;
  }

  .btn-copy {
    background: rgba(168, 85, 247, 0.15);
    border: 1px solid rgba(168, 85, 247, 0.3);
    color: #e9d5ff;
    border-radius: 4px;
    font-size: 0.72rem;
    padding: 2px 8px;
    cursor: pointer;
  }

  .btn-copy:hover {
    background: rgba(168, 85, 247, 0.3);
  }

  .share-token-box {
    background: rgba(0, 0, 0, 0.4);
    border-radius: 4px;
    padding: 6px 8px;
  }

  .share-token-box code {
    font-family: monospace;
    font-size: 0.75rem;
    color: #38bdf8;
    word-break: break-all;
  }

  .unlock-desc {
    font-size: 0.85rem;
    color: #94a3b8;
    margin: 0 0 12px 0;
  }

  .tokens-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .token-input-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .token-label {
    font-size: 0.78rem;
    font-weight: 600;
    color: #cbd5e1;
  }

  .input-with-del {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }

  .share-textarea {
    flex: 1;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    padding: 6px 10px;
    color: #38bdf8;
    font-size: 0.78rem;
    resize: vertical;
  }

  .btn-del {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    color: #94a3b8;
    padding: 6px 10px;
    cursor: pointer;
  }

  .btn-del:hover {
    color: #ef4444;
    border-color: #ef4444;
  }

  .add-share-row {
    margin-top: 8px;
  }

  .btn-add {
    background: transparent;
    border: 1px dashed rgba(255, 255, 255, 0.2);
    border-radius: 6px;
    color: #94a3b8;
    padding: 6px 12px;
    font-size: 0.8rem;
    cursor: pointer;
    width: 100%;
  }

  .btn-add:hover {
    color: white;
    border-color: #a855f7;
  }

  .unlock-action-row {
    margin-top: 14px;
  }

  .submit-btn {
    width: 100%;
    padding: 10px;
    font-size: 0.95rem;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    padding: 12px 20px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
  }

  .mono {
    font-family: monospace;
  }
</style>

