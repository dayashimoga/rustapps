<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { saveNativeDialog } from '../lib/ipc';

  export let isOpen = false;
  export let totalLines = 0;
  export let filePath = '';
  export let onExport: (destPath: string) => Promise<{ dest_path: string; records_redacted: number }>;

  const dispatch = createEventDispatcher<{
    close: void;
  }>();

  const todayStr = new Date().toISOString().slice(0, 10);
  let destPath = `redacted_export_${todayStr}.log`;
  let maskIps = true;
  let maskCards = true;
  let maskEmails = true;
  let maskSecrets = true;

  let loading = false;
  let errorMsg = '';
  let exportSummary: { dest_path: string; records_redacted: number } | null = null;

  function setDefaultName() {
    destPath = `redacted_export_${new Date().toISOString().slice(0, 10)}.log`;
  }

  async function handleBrowseSavePath() {
    const selected = await saveNativeDialog({
      title: 'Save Sanitized Redacted Log File',
      defaultPath: destPath,
      filters: [{ name: 'Log Files', extensions: ['log', 'txt', 'jsonl', 'csv'] }],
    });
    if (selected && typeof selected === 'string') {
      destPath = selected;
    }
  }

  async function handleExport() {
    errorMsg = '';
    exportSummary = null;

    if (!destPath.trim()) {
      errorMsg = 'Destination file path cannot be empty';
      return;
    }

    loading = true;
    try {
      exportSummary = await onExport(destPath.trim());
    } catch (err: unknown) {
      const e = err as { message?: string };
      errorMsg = e?.message || String(err);
    } finally {
      loading = false;
    }
  }

  function handleClose() {
    errorMsg = '';
    exportSummary = null;
    dispatch('close');
  }
</script>

{#if isOpen}
  <div class="modal-overlay" role="dialog" aria-modal="true" tabindex="-1">
    <div class="redact-modal glass-panel">
      <div class="modal-header">
        <div class="header-left">
          <div class="shield-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="20" height="20" stroke-width="2">
              <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path>
            </svg>
          </div>
          <div class="header-titles">
            <h3>PII & CREDENTIAL REDACTION EXPORT</h3>
            <p class="header-subtitle">Air-Gapped Business-Tier Enterprise Compliance</p>
          </div>
        </div>
        <button type="button" class="close-btn" on:click={handleClose} aria-label="Close modal">&times;</button>
      </div>

      <div class="modal-body">
        <p class="modal-desc">
          Streams log records through the offline regex and Luhn-validation pipeline, masking sensitive
          data before writing clean sanitized records to the destination file.
        </p>

        {#if errorMsg}
          <div class="error-banner">
            <span class="banner-icon">⚠️</span>
            <span>{errorMsg}</span>
          </div>
        {/if}

        {#if exportSummary}
          <div class="success-banner">
            <div class="success-title">
              <span class="banner-icon">✓</span>
              <strong>Redacted Export Completed!</strong>
            </div>
            <div class="success-stats">
              <div class="stat-pill">
                <span class="stat-lbl">Records Sanitized</span>
                <span class="stat-val">{exportSummary.records_redacted.toLocaleString()}</span>
              </div>
              <div class="stat-pill">
                <span class="stat-lbl">Source File</span>
                <span class="stat-val">{filePath || 'default_cluster.log'}</span>
              </div>
            </div>
            <div class="path-box">
              <span class="path-lbl">Saved to:</span>
              <code class="path-code">{exportSummary.dest_path}</code>
            </div>
          </div>
        {/if}

        <div class="form-group">
          <div class="label-row">
            <label for="dest-path">Destination File Path</label>
            <div style="display: flex; gap: 10px;">
              <button type="button" class="btn-link" on:click={handleBrowseSavePath}>Browse...</button>
              <button type="button" class="btn-link" on:click={setDefaultName}>Reset Name</button>
            </div>
          </div>
          <input
            id="dest-path"
            type="text"
            class="input-field"
            placeholder="e.g. sanitized_compliance_2026.log"
            bind:value={destPath}
          />
        </div>

        <div class="rules-section">
          <span class="rules-title">Active Redaction Filters</span>
          <div class="rules-grid">
            <label class="rule-card" class:active={maskCards}>
              <input type="checkbox" bind:checked={maskCards} />
              <div class="rule-info">
                <strong>Payment Cards (PCI-DSS)</strong>
                <span class="rule-tag">[CARD_REDACTED]</span>
                <p>16-digit PANs with Luhn Mod-10 checksum validation</p>
              </div>
            </label>

            <label class="rule-card" class:active={maskIps}>
              <input type="checkbox" bind:checked={maskIps} />
              <div class="rule-info">
                <strong>Network Addresses (IP)</strong>
                <span class="rule-tag">[IP_REDACTED]</span>
                <p>IPv4 octets and IPv6 colon-hexadecimal addresses</p>
              </div>
            </label>

            <label class="rule-card" class:active={maskEmails}>
              <input type="checkbox" bind:checked={maskEmails} />
              <div class="rule-info">
                <strong>Email Addresses</strong>
                <span class="rule-tag">[EMAIL_REDACTED]</span>
                <p>RFC 5322 compliant user and system mail addresses</p>
              </div>
            </label>

            <label class="rule-card" class:active={maskSecrets}>
              <input type="checkbox" bind:checked={maskSecrets} />
              <div class="rule-info">
                <strong>Bearer Tokens & Keys</strong>
                <span class="rule-tag">[SECRET_REDACTED]</span>
                <p>JWT auth tokens, AWS secret keys, and passwords</p>
              </div>
            </label>
          </div>
        </div>

        <div class="modal-footer">
          <button type="button" class="cancel-btn" on:click={handleClose}>
            {exportSummary ? 'Done' : 'Cancel'}
          </button>
          <button
            type="button"
            class="action-btn"
            disabled={loading}
            on:click={handleExport}
          >
            {#if loading}
              <span class="spinner-sm"></span> Redacting Stream...
            {:else}
              Export Redacted Logs ({totalLines.toLocaleString()} records)
            {/if}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .redact-modal {
    width: 600px;
    max-width: 95vw;
    background: hsla(220, 24%, 10%, 0.98);
    border: 1px solid var(--border-subtle);
    border-radius: 14px;
    overflow: hidden;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.7);
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .shield-icon {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    background: hsla(217, 91%, 60%, 0.15);
    color: var(--accent-blue);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .header-titles h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.5px;
    color: var(--text-primary);
  }

  .header-subtitle {
    margin: 2px 0 0 0;
    font-size: 11px;
    color: var(--text-muted);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 20px;
    cursor: pointer;
    line-height: 1;
    padding: 4px;
  }

  .close-btn:hover {
    color: var(--text-primary);
  }

  .modal-body {
    padding: 20px;
    max-height: 75vh;
    overflow-y: auto;
  }

  .modal-desc {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 0 0 16px 0;
  }

  .form-group {
    margin-bottom: 16px;
  }

  .label-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }

  label {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .btn-link {
    background: none;
    border: none;
    color: var(--accent-blue);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    padding: 0;
  }

  .btn-link:hover {
    text-decoration: underline;
  }

  .input-field {
    width: 100%;
    padding: 9px 12px;
    background: hsla(220, 20%, 8%, 0.8);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-mono);
    box-sizing: border-box;
  }

  .input-field:focus {
    outline: none;
    border-color: var(--accent-blue);
    box-shadow: 0 0 0 2px hsla(217, 91%, 60%, 0.25);
  }

  .rules-section {
    margin: 16px 0;
    padding: 14px;
    background: hsla(220, 20%, 8%, 0.5);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
  }

  .rules-title {
    display: block;
    font-size: 11px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 10px;
  }

  .rules-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .rule-card {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 6px;
    background: hsla(220, 20%, 14%, 0.6);
    border: 1px solid var(--border-subtle);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .rule-card.active {
    border-color: hsla(217, 91%, 60%, 0.4);
    background: hsla(217, 91%, 60%, 0.08);
  }

  .rule-card input[type="checkbox"] {
    margin-top: 3px;
    accent-color: var(--accent-blue);
  }

  .rule-info {
    flex: 1;
  }

  .rule-info strong {
    display: block;
    font-size: 11.5px;
    color: var(--text-primary);
  }

  .rule-tag {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--accent-blue);
  }

  .rule-info p {
    margin: 3px 0 0 0;
    font-size: 10.5px;
    color: var(--text-muted);
    line-height: 1.3;
  }

  /* Banners */
  .error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 6px;
    background: hsla(0, 85%, 60%, 0.15);
    border: 1px solid hsla(0, 85%, 60%, 0.35);
    color: hsl(0, 85%, 65%);
    font-size: 12px;
    margin-bottom: 14px;
  }

  .success-banner {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border-radius: 8px;
    background: hsla(145, 80%, 45%, 0.12);
    border: 1px solid hsla(145, 80%, 45%, 0.35);
    color: hsl(145, 80%, 60%);
    margin-bottom: 16px;
  }

  .success-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }

  .success-stats {
    display: flex;
    gap: 12px;
  }

  .stat-pill {
    background: rgba(0, 0, 0, 0.35);
    padding: 6px 10px;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
  }

  .stat-lbl {
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .stat-val {
    font-size: 13px;
    font-weight: 700;
    color: #fff;
  }

  .path-box {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
  }

  .path-code {
    background: rgba(0, 0, 0, 0.4);
    padding: 3px 6px;
    border-radius: 4px;
    color: #a7f3d0;
    font-family: var(--font-mono);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 20px;
  }

  .cancel-btn {
    padding: 8px 16px;
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .cancel-btn:hover {
    background: hsla(220, 20%, 20%, 0.4);
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 18px;
    background: var(--accent-blue);
    border: none;
    color: #fff;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .action-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .spinner-sm {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #ffffff;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
