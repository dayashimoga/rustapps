<script lang="ts">
  import type { BackupExportSummary, BackupImportSummary } from '../types';

  export let onExport: (passphrase: string, outputPath: string) => Promise<BackupExportSummary>;
  export let onImport: (passphrase: string, inputPath: string, overwrite: boolean) => Promise<BackupImportSummary>;
  export let onClose: () => void;
  export let onRefreshVault: () => void;

  let activeTab: 'export' | 'import' = 'export';

  // Export state
  let exportPassphrase = '';
  let exportConfirm = '';
  let exportPath = 'vault_backup.vaultbackup';
  let exportLoading = false;
  let exportSummary: BackupExportSummary | null = null;
  let exportError = '';

  // Import state
  let importPath = '';
  let importPassphrase = '';
  let importOverwrite = false;
  let importLoading = false;
  let importSummary: BackupImportSummary | null = null;
  let importError = '';

  async function handleExport() {
    exportError = '';
    exportSummary = null;

    if (!exportPassphrase.trim()) {
      exportError = 'Backup passphrase cannot be empty';
      return;
    }
    if (exportPassphrase !== exportConfirm) {
      exportError = 'Passphrase and confirmation do not match';
      return;
    }
    if (!exportPath.trim()) {
      exportError = 'Output file path cannot be empty';
      return;
    }

    exportLoading = true;
    try {
      exportSummary = await onExport(exportPassphrase, exportPath);
      exportPassphrase = '';
      exportConfirm = '';
    } catch (err: any) {
      exportError = err?.message || String(err);
    } finally {
      exportLoading = false;
    }
  }

  async function handleImport() {
    importError = '';
    importSummary = null;

    if (!importPath.trim()) {
      importError = 'Backup file path cannot be empty';
      return;
    }
    if (!importPassphrase.trim()) {
      importError = 'Backup passphrase is required to decrypt archive';
      return;
    }

    importLoading = true;
    try {
      importSummary = await onImport(importPassphrase, importPath, importOverwrite);
      importPassphrase = '';
      onRefreshVault();
    } catch (err: any) {
      importError = err?.message || String(err);
    } finally {
      importLoading = false;
    }
  }
</script>

<div class="modal-overlay">
  <div class="backup-modal">
    <div class="modal-header">
      <div class="header-left">
        <svg viewBox="0 0 20 20" fill="currentColor" class="header-icon">
          <path d="M7 3a1 1 0 000 2h6a1 1 0 100-2H7zM4 7a1 1 0 011-1h10a1 1 0 110 2H5a1 1 0 01-1-1zM2 11a2 2 0 012-2h12a2 2 0 012 2v4a2 2 0 01-2 2H4a2 2 0 01-2-2v-4z" />
        </svg>
        <h3>Encrypted Backup & Portability</h3>
      </div>
      <button type="button" class="close-btn" on:click={onClose}>&times;</button>
    </div>

    <div class="tab-nav">
      <button
        type="button"
        class="tab-btn"
        class:active={activeTab === 'export'}
        on:click={() => (activeTab = 'export')}
      >
        Export Archive (.vaultbackup)
      </button>
      <button
        type="button"
        class="tab-btn"
        class:active={activeTab === 'import'}
        on:click={() => (activeTab = 'import')}
      >
        Restore from Archive
      </button>
    </div>

    <div class="modal-body">
      {#if activeTab === 'export'}
        <div class="tab-content">
          <p class="tab-description">
            Exports a self-contained, authenticated binary archive encrypted with <strong>Argon2id + AES-256-GCM</strong>.
            All secret metadata, payloads, and SHA-256 digital integrity tags are preserved.
          </p>

          {#if exportError}
            <div class="error-banner">{exportError}</div>
          {/if}

          {#if exportSummary}
            <div class="success-banner">
              <strong>Export Succeeded!</strong>
              <span>
                Backed up {exportSummary.total_secrets} secrets ({exportSummary.file_size_bytes} bytes) to
                <code>{exportSummary.output_path}</code>
              </span>
            </div>
          {/if}

          <div class="form-group">
            <label for="export-path">Destination File Path (.vaultbackup)</label>
            <input
              id="export-path"
              type="text"
              class="input-field"
              placeholder="e.g. backup_2026.vaultbackup"
              bind:value={exportPath}
            />
          </div>

          <div class="form-group">
            <label for="export-pass">Backup Passphrase (Used to Encrypt)</label>
            <input
              id="export-pass"
              type="password"
              class="input-field"
              placeholder="Choose a strong backup passphrase"
              bind:value={exportPassphrase}
            />
          </div>

          <div class="form-group">
            <label for="export-confirm">Confirm Backup Passphrase</label>
            <input
              id="export-confirm"
              type="password"
              class="input-field"
              placeholder="Re-enter backup passphrase"
              bind:value={exportConfirm}
            />
          </div>

          <div class="modal-footer">
            <button type="button" class="cancel-btn" on:click={onClose}>Cancel</button>
            <button
              type="button"
              class="action-btn"
              disabled={exportLoading}
              on:click={handleExport}
            >
              {exportLoading ? 'Deriving Key & Encrypting...' : 'Export Backup'}
            </button>
          </div>
        </div>
      {:else}
        <div class="tab-content">
          <p class="tab-description">
            Restores secrets from an authenticated <code>.vaultbackup</code> archive. The archive checksum will be
            verified against corruption before records are merged into local storage.
          </p>

          {#if importError}
            <div class="error-banner">{importError}</div>
          {/if}

          {#if importSummary}
            <div class="success-banner">
              <strong>Restore Completed!</strong>
              <span>
                {importSummary.total_imported} imported, {importSummary.total_overwritten} overwritten, {importSummary.total_skipped} skipped.
              </span>
            </div>
          {/if}

          <div class="form-group">
            <label for="import-path">Backup Archive Path (.vaultbackup)</label>
            <input
              id="import-path"
              type="text"
              class="input-field"
              placeholder="e.g. backup_2026.vaultbackup"
              bind:value={importPath}
            />
          </div>

          <div class="form-group">
            <label for="import-pass">Backup Passphrase</label>
            <input
              id="import-pass"
              type="password"
              class="input-field"
              placeholder="Passphrase used when creating this backup"
              bind:value={importPassphrase}
            />
          </div>

          <div class="checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" bind:checked={importOverwrite} />
              <span>Overwrite existing secrets with matching IDs</span>
            </label>
          </div>

          <div class="modal-footer">
            <button type="button" class="cancel-btn" on:click={onClose}>Cancel</button>
            <button
              type="button"
              class="action-btn restore"
              disabled={importLoading}
              on:click={handleImport}
            >
              {importLoading ? 'Decrypting & Verifying Integrity...' : 'Restore Archive'}
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

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

  .backup-modal {
    width: 560px;
    max-width: 95vw;
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 16px;
    overflow: hidden;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6);
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 18px 24px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .header-icon {
    width: 22px;
    height: 22px;
    color: #38bdf8;
  }

  h3 {
    margin: 0;
    font-size: 16px;
    color: #f8fafc;
  }

  .close-btn {
    background: none;
    border: none;
    color: #94a3b8;
    font-size: 24px;
    cursor: pointer;
  }

  .close-btn:hover {
    color: #f1f5f9;
  }

  .tab-nav {
    display: flex;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(30, 41, 59, 0.4);
  }

  .tab-btn {
    flex: 1;
    padding: 12px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: #94a3b8;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .tab-btn:hover {
    color: #e2e8f0;
  }

  .tab-btn.active {
    color: #38bdf8;
    border-bottom-color: #38bdf8;
    background: rgba(56, 189, 248, 0.05);
  }

  .modal-body {
    padding: 20px 24px;
  }

  .tab-description {
    font-size: 12px;
    color: #94a3b8;
    line-height: 1.5;
    margin: 0 0 16px 0;
  }

  .form-group {
    margin-bottom: 14px;
  }

  label {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: #cbd5e1;
    margin-bottom: 6px;
  }

  .input-field {
    width: 100%;
    padding: 10px 12px;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #f8fafc;
    font-size: 13px;
    box-sizing: border-box;
  }

  .input-field:focus {
    outline: none;
    border-color: #38bdf8;
    box-shadow: 0 0 0 2px rgba(56, 189, 248, 0.2);
  }

  .checkbox-group {
    margin: 16px 0;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: #cbd5e1;
    cursor: pointer;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 20px;
  }

  .cancel-btn {
    padding: 10px 16px;
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: #cbd5e1;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }

  .cancel-btn:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .action-btn {
    padding: 10px 20px;
    background: #0284c7;
    border: none;
    color: #ffffff;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .action-btn:hover:not(:disabled) {
    background: #0369a1;
  }

  .action-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .action-btn.restore {
    background: #059669;
  }

  .action-btn.restore:hover:not(:disabled) {
    background: #047857;
  }

  .error-banner {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #f87171;
    font-size: 12px;
    padding: 10px 12px;
    border-radius: 8px;
    margin-bottom: 14px;
  }

  .success-banner {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid rgba(16, 185, 129, 0.3);
    color: #34d399;
    font-size: 12px;
    padding: 10px 12px;
    border-radius: 8px;
    margin-bottom: 14px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .success-banner code {
    background: rgba(0, 0, 0, 0.3);
    padding: 2px 6px;
    border-radius: 4px;
    color: #a7f3d0;
  }
</style>
