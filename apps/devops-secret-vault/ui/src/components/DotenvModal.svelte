<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invokeIpc } from '../lib/ipc';

  export let isOpen = false;

  const dispatch = createEventDispatcher<{
    close: void;
    imported: void;
  }>();

  let activeTab: 'import' | 'export' = 'import';
  let importFormat: 'dotenv' | 'k8s' = 'dotenv';
  let targetEnvironment = 'production';
  let pasteContent = '';
  let exportContent = '';
  let isExecuting = false;
  let statusMessage = '';
  let isError = false;

  $: if (isOpen && activeTab === 'export') {
    loadExportContent();
  }

  async function loadExportContent() {
    isExecuting = true;
    try {
      const res = await invokeIpc<any>('export_dotenv');
      if (res && res.dotenv_content) {
        exportContent = res.dotenv_content;
      } else if (typeof res === 'string') {
        exportContent = res;
      } else {
        exportContent = '# No secrets found in vault to export.';
      }
    } catch (err: any) {
      exportContent = '# Error loading export: ' + (err?.message || String(err));
    } finally {
      isExecuting = false;
    }
  }

  async function handleImport() {
    if (!pasteContent.trim()) {
      statusMessage = 'Please paste .env or Kubernetes Secret YAML content first.';
      isError = true;
      return;
    }

    isExecuting = true;
    statusMessage = '';
    isError = false;

    try {
      if (importFormat === 'dotenv') {
        const res = await invokeIpc<any>('import_dotenv', {
          content: pasteContent,
          environment: targetEnvironment,
        });
        statusMessage = `Import complete: ${res.imported_count} imported, ${res.updated_count} updated.`;
        if (res.failed_keys && res.failed_keys.length > 0) {
          statusMessage += ` (${res.failed_keys.length} errors: ${res.failed_keys.join(', ')})`;
          isError = true;
        }
      } else {
        const res = await invokeIpc<any>('import_k8s_secret', {
          yaml_content: pasteContent,
          environment: targetEnvironment,
        });
        statusMessage = `K8s Secret [${res.secret_name}] imported: ${res.imported_count} imported, ${res.updated_count} updated.`;
        if (res.failed_keys && res.failed_keys.length > 0) {
          statusMessage += ` (${res.failed_keys.length} errors)`;
          isError = true;
        }
      }
      pasteContent = '';
      dispatch('imported');
    } catch (err: any) {
      statusMessage = 'Import failed: ' + (err?.message || String(err));
      isError = true;
    } finally {
      isExecuting = false;
    }
  }

  function handleDownloadExport() {
    if (!exportContent || exportContent.startsWith('# Error')) return;
    const blob = new Blob([exportContent], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = importFormat === 'k8s' ? 'k8s-secret.yaml' : '.env.vault';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }

  async function handleCopyExport() {
    if (!exportContent) return;
    await navigator.clipboard.writeText(exportContent);
    statusMessage = 'Copied to clipboard!';
    isError = false;
    setTimeout(() => {
      if (statusMessage === 'Copied to clipboard!') statusMessage = '';
    }, 3000);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      dispatch('close');
    }
  }
</script>

<svelte:window on:keydown={handleKeyDown} />

{#if isOpen}
  <!-- svelte-ignore a11y-no-noninteractive-element-interactions a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="modal-backdrop" on:click={() => dispatch('close')}>
    <!-- svelte-ignore a11y-no-noninteractive-element-interactions a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <div
      class="modal-dialog glass-panel"
      on:click|stopPropagation
      role="dialog"
      aria-modal="true"
      aria-label=".env and Kubernetes Secrets Manager"
    >
      <div class="modal-header">
        <div class="title-with-badge">
          <h2>.env &amp; Kubernetes Secret Sync</h2>
          <span class="badge badge-cyan">100% Offline AES-256</span>
        </div>
        <button class="close-btn" on:click={() => dispatch('close')} aria-label="Close modal">✕</button>
      </div>

      <div class="tabs-bar">
        <button
          class="tab-btn {activeTab === 'import' ? 'active' : ''}"
          on:click={() => (activeTab = 'import')}
        >
          📥 Import into Vault
        </button>
        <button
          class="tab-btn {activeTab === 'export' ? 'active' : ''}"
          on:click={() => {
            activeTab = 'export';
            loadExportContent();
          }}
        >
          📤 Export Decrypted .env
        </button>
      </div>

      <div class="modal-body">
        {#if activeTab === 'import'}
          <div class="form-row">
            <label for="format-select">Format</label>
            <div class="radio-group">
              <label class="radio-label">
                <input type="radio" bind:group={importFormat} value="dotenv" />
                <span>Standard .env (KEY=VALUE)</span>
              </label>
              <label class="radio-label">
                <input type="radio" bind:group={importFormat} value="k8s" />
                <span>Kubernetes Secret (YAML)</span>
              </label>
            </div>
          </div>

          <div class="form-row">
            <label for="env-select">Target Environment Tag</label>
            <select id="env-select" class="form-select" bind:value={targetEnvironment}>
              <option value="production">production</option>
              <option value="staging">staging</option>
              <option value="development">development</option>
              <option value="testing">testing</option>
            </select>
          </div>

          <div class="form-row">
            <label for="paste-area">Paste Content</label>
            <textarea
              id="paste-area"
              class="code-textarea"
              bind:value={pasteContent}
              placeholder={importFormat === 'dotenv'
                ? "# Paste your .env file here:\nDB_HOST=127.0.0.1\nDB_PASS=super_secret_password\nAPI_KEY=ak_live_12345"
                : "apiVersion: v1\nkind: Secret\nmetadata:\n  name: app-secrets\ntype: Opaque\ndata:\n  DB_PASS: c3VwZXJfc2VjcmV0\nstringData:\n  API_KEY: ak_live_12345"}
            ></textarea>
          </div>

          {#if statusMessage}
            <div class="status-banner {isError ? 'error' : 'success'}">
              {statusMessage}
            </div>
          {/if}

          <div class="action-footer">
            <button class="btn btn-secondary" on:click={() => dispatch('close')}>Cancel</button>
            <button
              class="btn btn-primary"
              disabled={isExecuting || !pasteContent.trim()}
              on:click={handleImport}
            >
              {isExecuting ? 'Encrypting & Saving...' : 'Encrypt & Import to Vault'}
            </button>
          </div>
        {:else}
          <div class="form-row">
            <label for="export-preview">Decrypted .env Output</label>
            <textarea
              id="export-preview"
              class="code-textarea"
              readonly
              bind:value={exportContent}
            ></textarea>
          </div>

          {#if statusMessage}
            <div class="status-banner {isError ? 'error' : 'success'}">
              {statusMessage}
            </div>
          {/if}

          <div class="action-footer">
            <button class="btn btn-secondary" on:click={handleCopyExport}>Copy to Clipboard</button>
            <button class="btn btn-primary" on:click={handleDownloadExport}>Download .env.vault</button>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(10, 15, 29, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-dialog {
    width: 650px;
    max-width: 95vw;
    background: rgba(15, 23, 42, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: var(--radius-lg, 12px);
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.7);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .title-with-badge {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .title-with-badge h2 {
    margin: 0;
    font-size: 16px;
    color: var(--text-main, #f1f5f9);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted, #94a3b8);
    font-size: 18px;
    cursor: pointer;
    line-height: 1;
  }

  .tabs-bar {
    display: flex;
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .tab-btn {
    flex: 1;
    padding: 10px;
    background: transparent;
    border: none;
    color: var(--text-muted, #94a3b8);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tab-btn.active {
    color: var(--accent-cyan, #06b6d4);
    background: rgba(6, 182, 212, 0.1);
    border-bottom: 2px solid var(--accent-cyan, #06b6d4);
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .form-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-row label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted, #94a3b8);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .radio-group {
    display: flex;
    gap: 16px;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text-main, #f1f5f9);
    cursor: pointer;
  }

  .form-select {
    background: rgba(11, 17, 30, 0.85);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    padding: 8px 12px;
    color: var(--text-main, #f1f5f9);
    font-size: 13px;
    outline: none;
  }

  .code-textarea {
    height: 180px;
    background: rgba(11, 17, 30, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    padding: 10px 12px;
    color: #38bdf8;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    line-height: 1.5;
    resize: vertical;
    outline: none;
  }

  .status-banner {
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
  }

  .status-banner.success {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid rgba(16, 185, 129, 0.4);
    color: #10b981;
  }

  .status-banner.error {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: #ef4444;
  }

  .action-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 8px;
  }
</style>
