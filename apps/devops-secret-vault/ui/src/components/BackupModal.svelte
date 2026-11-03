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
  const todayStr = new Date().toISOString().slice(0, 10);
  let exportPath = `vault_backup_${todayStr}.vaultbackup`;
  let exportLoading = false;
  let exportSummary: BackupExportSummary | null = null;
  let exportError = '';

  // Import state
  let importPath = '';
  let importPassphrase = '';
  let conflictMode: 'skip' | 'overwrite' = 'skip';
  let importLoading = false;
  let importSummary: BackupImportSummary | null = null;
  let importError = '';

  function setDefaultExportName() {
    exportPath = `vault_backup_${new Date().toISOString().slice(0, 10)}.vaultbackup`;
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }

  function formatEpoch(epoch: number): string {
    if (!epoch || epoch <= 0) return 'Just now';
    return new Date(epoch * 1000).toLocaleString();
  }

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
    } catch (err: unknown) {
      const e = err as { message?: string };
      exportError = e?.message || String(err);
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
      const overwrite = conflictMode === 'overwrite';
      importSummary = await onImport(importPassphrase, importPath, overwrite);
      importPassphrase = '';
      onRefreshVault();
    } catch (err: unknown) {
      const e = err as { message?: string };
      importError = e?.message || String(err);
    } finally {
      importLoading = false;
    }
  }
</script>

<div class="modal-overlay" role="dialog" aria-modal="true" tabindex="-1">
  <div class="backup-modal">
    <div class="modal-header">
      <div class="header-left">
        <div class="header-icon-box">
          <svg viewBox="0 0 20 20" fill="currentColor" class="header-icon">
            <path d="M7 3a1 1 0 000 2h6a1 1 0 100-2H7zM4 7a1 1 0 011-1h10a1 1 0 110 2H5a1 1 0 01-1-1zM2 11a2 2 0 012-2h12a2 2 0 012 2v4a2 2 0 01-2 2H4a2 2 0 01-2-2v-4z" />
          </svg>
        </div>
        <div class="header-titles">
          <h3>Encrypted Backup & Migration</h3>
          <p class="header-subtitle">Argon2id + AES-256-GCM Authenticated Offline Archives</p>
        </div>
      </div>
      <button type="button" class="close-btn" on:click={onClose} aria-label="Close modal">&times;</button>
    </div>

    <div class="tab-nav">
      <button
        type="button"
        class="tab-btn"
        class:active={activeTab === 'export'}
        on:click={() => (activeTab = 'export')}
      >
        <span class="tab-num">1</span> Export Archive (.vaultbackup)
      </button>
      <button
        type="button"
        class="tab-btn"
        class:active={activeTab === 'import'}
        on:click={() => (activeTab = 'import')}
      >
        <span class="tab-num">2</span> Restore & Conflict Resolution
      </button>
    </div>

    <div class="modal-body">
      {#if activeTab === 'export'}
        <div class="tab-content">
          <p class="tab-description">
            Generates an authenticated standalone binary archive encrypted with <strong>Argon2id (M:64MB, T:3, P:4) + AES-256-GCM</strong>.
            All metadata, payloads, TOTP keys, and digital integrity signatures are preserved 100% offline.
          </p>

          {#if exportError}
            <div class="banner error-banner">
              <span class="banner-icon">⚠️</span>
              <span>{exportError}</span>
            </div>
          {/if}

          {#if exportSummary}
            <div class="banner success-banner">
              <div class="success-header">
                <span class="banner-icon">✓</span>
                <strong>Backup Export Completed Successfully!</strong>
              </div>
              <div class="summary-details">
                <div class="stat-row">
                  <span class="stat-lbl">Output Archive:</span>
                  <code class="stat-val path-val">{exportSummary.output_path}</code>
                </div>
                <div class="stat-grid">
                  <div class="stat-col">
                    <span class="stat-lbl">Secrets Packaged</span>
                    <span class="stat-metric">{exportSummary.total_secrets}</span>
                  </div>
                  <div class="stat-col">
                    <span class="stat-lbl">Archive Size</span>
                    <span class="stat-metric">{formatBytes(exportSummary.file_size_bytes)}</span>
                  </div>
                  <div class="stat-col">
                    <span class="stat-lbl">Exported At</span>
                    <span class="stat-metric">{formatEpoch(exportSummary.exported_at)}</span>
                  </div>
                </div>
              </div>
            </div>
          {/if}

          <div class="form-group">
            <div class="label-row">
              <label for="export-path">Destination File Path</label>
              <button type="button" class="btn-link" on:click={setDefaultExportName}>Default Name</button>
            </div>
            <input
              id="export-path"
              type="text"
              class="input-field"
              placeholder="e.g. vault_backup_2026-11-03.vaultbackup"
              bind:value={exportPath}
            />
          </div>

          <div class="form-group">
            <label for="export-pass">Archive Passphrase (Key Derivation)</label>
            <input
              id="export-pass"
              type="password"
              class="input-field"
              placeholder="Choose a strong backup passphrase"
              bind:value={exportPassphrase}
            />
          </div>

          <div class="form-group">
            <label for="export-confirm">Confirm Passphrase</label>
            <input
              id="export-confirm"
              type="password"
              class="input-field"
              placeholder="Re-enter backup passphrase"
              bind:value={exportConfirm}
            />
            {#if exportPassphrase && exportConfirm && exportPassphrase !== exportConfirm}
              <span class="validation-warn">⚠️ Passphrase confirmation does not match</span>
            {/if}
          </div>

          <div class="modal-footer">
            <button type="button" class="cancel-btn" on:click={onClose}>Cancel</button>
            <button
              type="button"
              class="action-btn"
              disabled={exportLoading}
              on:click={handleExport}
            >
              {#if exportLoading}
                <span class="spinner-sm"></span> Encrypting Archive...
              {:else}
                Export Encrypted Backup
              {/if}
            </button>
          </div>
        </div>
      {:else}
        <div class="tab-content">
          <p class="tab-description">
            Restores secrets from an authenticated <code>.vaultbackup</code> archive. The archive SHA-256
            checksum is cryptographically verified before any records are committed into local storage.
          </p>

          {#if importError}
            <div class="banner error-banner">
              <span class="banner-icon">⚠️</span>
              <span>{importError}</span>
            </div>
          {/if}

          {#if importSummary}
            <div class="banner success-banner">
              <div class="success-header">
                <span class="banner-icon">✓</span>
                <strong>Restore & Merge Completed!</strong>
              </div>
              <div class="summary-details">
                <div class="stat-grid">
                  <div class="stat-col">
                    <span class="stat-lbl">Newly Imported</span>
                    <span class="stat-metric text-emerald">{importSummary.total_imported}</span>
                  </div>
                  <div class="stat-col">
                    <span class="stat-lbl">Overwritten</span>
                    <span class="stat-metric text-amber">{importSummary.total_overwritten}</span>
                  </div>
                  <div class="stat-col">
                    <span class="stat-lbl">Skipped / Preserved</span>
                    <span class="stat-metric text-slate">{importSummary.total_skipped}</span>
                  </div>
                  <div class="stat-col">
                    <span class="stat-lbl">Original Archive Date</span>
                    <span class="stat-metric">{formatEpoch(importSummary.original_exported_at)}</span>
                  </div>
                </div>
              </div>
            </div>
          {/if}

          <div class="form-group">
            <label for="import-path">Backup Archive File (.vaultbackup)</label>
            <input
              id="import-path"
              type="text"
              class="input-field"
              placeholder="e.g. vault_backup_2026-11-03.vaultbackup"
              bind:value={importPath}
            />
          </div>

          <div class="form-group">
            <label for="import-pass">Archive Passphrase</label>
            <input
              id="import-pass"
              type="password"
              class="input-field"
              placeholder="Passphrase used when creating this archive"
              bind:value={importPassphrase}
            />
          </div>

          <!-- Conflict Resolution Strategy -->
          <div class="form-group conflict-section">
            <span class="conflict-title">Conflict Resolution Strategy (Matching Secret IDs)</span>
            <div class="conflict-options">
              <label class="conflict-card" class:selected={conflictMode === 'skip'}>
                <input
                  type="radio"
                  name="conflictMode"
                  value="skip"
                  checked={conflictMode === 'skip'}
                  on:change={() => (conflictMode = 'skip')}
                />
                <div class="conflict-info">
                  <div class="conflict-header-row">
                    <strong>Skip Existing Secrets (Safe)</strong>
                    <span class="mode-pill safe">Preserve Local</span>
                  </div>
                  <p class="conflict-desc">
                    Retains current local secrets unchanged. Only new secrets with IDs not found in the current vault will be imported.
                  </p>
                </div>
              </label>

              <label class="conflict-card" class:selected={conflictMode === 'overwrite'}>
                <input
                  type="radio"
                  name="conflictMode"
                  value="overwrite"
                  checked={conflictMode === 'overwrite'}
                  on:change={() => (conflictMode = 'overwrite')}
                />
                <div class="conflict-info">
                  <div class="conflict-header-row">
                    <strong>Overwrite Existing Secrets</strong>
                    <span class="mode-pill warn">Replace Local</span>
                  </div>
                  <p class="conflict-desc">
                    Replaces local records with the archive versions whenever an ID matches. Useful for restoring canonical vault backups.
                  </p>
                </div>
              </label>
            </div>
          </div>

          <div class="modal-footer">
            <button type="button" class="cancel-btn" on:click={onClose}>Cancel</button>
            <button
              type="button"
              class="action-btn restore"
              disabled={importLoading}
              on:click={handleImport}
            >
              {#if importLoading}
                <span class="spinner-sm"></span> Verifying & Restoring...
              {:else}
                Restore from Archive
              {/if}
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
    width: 620px;
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
    gap: 12px;
  }

  .header-icon-box {
    width: 38px;
    height: 38px;
    border-radius: 10px;
    background: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .header-icon {
    width: 22px;
    height: 22px;
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
    background: none;
    border: none;
    color: #94a3b8;
    font-size: 24px;
    cursor: pointer;
    line-height: 1;
    padding: 4px;
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
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px 16px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: #94a3b8;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .tab-num {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.1);
    font-size: 11px;
  }

  .tab-btn:hover {
    color: #e2e8f0;
  }

  .tab-btn.active {
    color: #38bdf8;
    border-bottom-color: #38bdf8;
    background: rgba(56, 189, 248, 0.05);
  }

  .tab-btn.active .tab-num {
    background: #38bdf8;
    color: #0f172a;
    font-weight: 700;
  }

  .modal-body {
    padding: 22px 24px;
    max-height: 75vh;
    overflow-y: auto;
  }

  .tab-description {
    font-size: 12px;
    color: #94a3b8;
    line-height: 1.5;
    margin: 0 0 16px 0;
  }

  .tab-description strong {
    color: #e2e8f0;
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
    color: #cbd5e1;
    margin-bottom: 6px;
  }

  .label-row label {
    margin-bottom: 0;
  }

  .btn-link {
    background: none;
    border: none;
    color: #38bdf8;
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
    padding: 10px 14px;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;
    color: #f8fafc;
    font-size: 13px;
    box-sizing: border-box;
    transition: all 0.15s ease;
  }

  .input-field:focus {
    outline: none;
    border-color: #38bdf8;
    box-shadow: 0 0 0 2px rgba(56, 189, 248, 0.2);
    background: rgba(0, 0, 0, 0.6);
  }

  .validation-warn {
    display: block;
    font-size: 11px;
    color: #fbbf24;
    margin-top: 4px;
  }

  /* Conflict Section */
  .conflict-section {
    margin-top: 18px;
    padding: 14px;
    background: rgba(30, 41, 59, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
  }

  .conflict-title {
    display: block;
    font-size: 12px;
    font-weight: 600;
    margin-bottom: 10px;
    color: #e2e8f0;
  }

  .conflict-options {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .conflict-card {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 8px;
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.08);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .conflict-card:hover {
    border-color: rgba(255, 255, 255, 0.18);
  }

  .conflict-card.selected {
    border-color: rgba(56, 189, 248, 0.4);
    background: rgba(56, 189, 248, 0.06);
  }

  .conflict-card input[type="radio"] {
    margin-top: 3px;
    accent-color: #38bdf8;
  }

  .conflict-info {
    flex: 1;
  }

  .conflict-header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 3px;
  }

  .conflict-header-row strong {
    font-size: 13px;
    color: #f1f5f9;
  }

  .mode-pill {
    font-size: 10px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
  }

  .mode-pill.safe {
    background: rgba(16, 185, 129, 0.15);
    color: #10b981;
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .mode-pill.warn {
    background: rgba(245, 158, 11, 0.15);
    color: #f59e0b;
    border: 1px solid rgba(245, 158, 11, 0.3);
  }

  .conflict-desc {
    margin: 0;
    font-size: 11px;
    color: #94a3b8;
    line-height: 1.4;
  }

  /* Banners */
  .banner {
    padding: 12px 14px;
    border-radius: 8px;
    margin-bottom: 16px;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 12px;
  }

  .error-banner {
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #f87171;
  }

  .success-banner {
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.3);
    color: #34d399;
    flex-direction: column;
    gap: 10px;
  }

  .success-header {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }

  .summary-details {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .stat-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .path-val {
    background: rgba(0, 0, 0, 0.35);
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 11px;
    color: #a7f3d0;
    word-break: break-all;
  }

  .stat-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
    gap: 8px;
  }

  .stat-col {
    background: rgba(0, 0, 0, 0.25);
    padding: 8px 10px;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
  }

  .stat-lbl {
    font-size: 10px;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-bottom: 2px;
  }

  .stat-metric {
    font-size: 13px;
    font-weight: 700;
    color: #f1f5f9;
  }

  .text-emerald { color: #10b981; }
  .text-amber { color: #f59e0b; }
  .text-slate { color: #94a3b8; }

  /* Modal Footer */
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 22px;
  }

  .cancel-btn {
    padding: 9px 16px;
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
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 9px 18px;
    background: #0284c7;
    border: none;
    color: #ffffff;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover:not(:disabled) {
    background: #0369a1;
  }

  .action-btn.restore {
    background: #059669;
  }

  .action-btn.restore:hover:not(:disabled) {
    background: #047857;
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
