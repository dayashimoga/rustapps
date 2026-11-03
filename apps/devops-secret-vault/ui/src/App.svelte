<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Header from './components/Header.svelte';
  import SecretList from './components/SecretList.svelte';
  import SecretDetail from './components/SecretDetail.svelte';
  import UnlockModal from './components/UnlockModal.svelte';
  import NewSecretModal from './components/NewSecretModal.svelte';
  import HealthDashboard from './components/HealthDashboard.svelte';
  import BackupModal from './components/BackupModal.svelte';
  import ShamirModal from './components/ShamirModal.svelte';
  import type {
    BackupExportSummary,
    BackupImportSummary,
    CertInspectionReport,
    DecryptedSecret,
    SecretAuditEntry,
    SecretMetadata,
    SecretPayload,
    SecretType,
    TotpCodeResponse,
    VaultHealthReport,
  } from './types';

  let isInitialized = false;
  let isLocked = true;
  let isLoading = false;
  let errorMessage = '';

  let secrets: SecretMetadata[] = [];
  let selectedSecretId: string | null = null;
  let selectedSecret: DecryptedSecret | null = null;
  let activeAuditLogs: SecretAuditEntry[] = [];

  let searchQuery = '';
  let filterType: SecretType | 'all' = 'all';

  let isNewSecretModalOpen = false;
  let isHealthDashboardOpen = false;
  let isBackupModalOpen = false;
  let isShamirModalOpen = false;
  let shamirModalMode: 'split' | 'unlock' = 'split';
  let biometricAvailable = true;
  let clipboardSeconds: number | null = null;
  let clipboardTimer: number | null = null;

  // Notification toast
  let toastMessage = '';
  let toastTimeout: number | null = null;

  function showToast(msg: string) {
    toastMessage = msg;
    if (toastTimeout) clearTimeout(toastTimeout);
    toastTimeout = window.setTimeout(() => { toastMessage = ''; }, 3500);
  }

  // --- Tauri IPC or Browser Mock Bridge ---
  const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

  // Mock in-memory state for development / preview
  let mockSecrets: DecryptedSecret[] = [
    {
      metadata: {
        id: 'mock-1',
        title: 'Production AWS Root Account',
        secret_type: 'api_key',
        tags: ['aws', 'prod', 'infra'],
        username: 'devops-root',
        url: 'https://aws.amazon.com',
        notes: 'Master credentials for cloud infrastructure provisioning.',
        version: 1,
        created_at: Math.floor(Date.now() / 1000) - 86400,
        updated_at: Math.floor(Date.now() / 1000) - 3600,
      },
      payload: {
        type: 'api_key',
        data: {
          api_key: 'AKIAIOSFODNN7EXAMPLE',
          api_secret: 'wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY',
          endpoint_url: 'https://iam.amazonaws.com',
        },
      },
    },
    {
      metadata: {
        id: 'mock-2',
        title: 'Bastion SSH Private Key',
        secret_type: 'ssh_key',
        tags: ['ssh', 'k8s', 'bastion'],
        username: 'ubuntu',
        url: 'ssh://bastion.cloud.internal:2222',
        notes: 'Access key for Kubernetes cluster nodes.',
        version: 2,
        created_at: Math.floor(Date.now() / 1000) - 172800,
        updated_at: Math.floor(Date.now() / 1000) - 7200,
      },
      payload: {
        type: 'ssh_key',
        data: {
          private_key: '-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAACFwAAAAdzc2gtcn\nNhAAAAAwEAAQAAAgEA3q1...\n-----END OPENSSH PRIVATE KEY-----',
          public_key: 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICl+...',
          passphrase: 'secure-passphrase-2026',
        },
      },
    },
    {
      metadata: {
        id: 'mock-3',
        title: 'PostgreSQL Analytics Replica',
        secret_type: 'database_credential',
        tags: ['db', 'postgres', 'analytics'],
        username: 'analytics_ro',
        url: 'postgres://analytics.db.internal:5432/dw',
        notes: 'Read-only connection for Data Warehouse ingestion.',
        version: 1,
        created_at: Math.floor(Date.now() / 1000) - 300000,
        updated_at: Math.floor(Date.now() / 1000) - 10000,
      },
      payload: {
        type: 'database_credential',
        data: {
          engine: 'postgresql',
          host: 'analytics.db.internal',
          port: 5432,
          database: 'dw',
          username: 'analytics_ro',
          password: 'Analytics-P@ssword-2026!',
          connection_string: 'postgresql://analytics_ro:Analytics-P@ssword-2026!@analytics.db.internal:5432/dw',
        },
      },
    },
  ];

  onMount(async () => {
    await checkStatus();
    startClipboardPoll();
  });

  onDestroy(() => {
    if (clipboardTimer) clearInterval(clipboardTimer);
  });

  async function checkStatus() {
    if (isTauri) {
      try {
        const res: any = await (window as any).__TAURI__.invoke('get_vault_status');
        isInitialized = res.is_initialized;
        isLocked = res.is_locked;
        try {
          biometricAvailable = await (window as any).__TAURI__.invoke('is_biometric_available');
        } catch (_) {
          biometricAvailable = false;
        }
        if (!isLocked) {
          await loadSecrets();
        }
      } catch (err: any) {
        console.error('Status check error:', err);
      }
    } else {
      // Mock mode default: initialized but locked
      isInitialized = true;
      isLocked = true;
      biometricAvailable = true;
    }
  }

  async function handleUnlock(event: CustomEvent<{ password: string }>) {
    isLoading = true;
    errorMessage = '';
    const { password } = event.detail;

    try {
      if (isTauri) {
        await (window as any).__TAURI__.invoke('unlock_vault', { masterPassword: password });
      } else {
        // Mock unlock check
        await new Promise(r => setTimeout(r, 400));
        if (password.length < 4) {
          throw new Error('Invalid master passphrase or token');
        }
      }
      isLocked = false;
      await loadSecrets();
      showToast('Vault unlocked. Ephemeral Tantivy index loaded into RAM.');
    } catch (err: any) {
      errorMessage = err.toString() || 'Authentication failed';
    } finally {
      isLoading = false;
    }
  }

  async function handleInitialize(event: CustomEvent<{ password: string }>) {
    isLoading = true;
    errorMessage = '';
    const { password } = event.detail;

    try {
      if (isTauri) {
        await (window as any).__TAURI__.invoke('initialize_vault', { masterPassword: password });
      } else {
        await new Promise(r => setTimeout(r, 500));
      }
      isInitialized = true;
      isLocked = false;
      await loadSecrets();
      showToast('Vault initialized with Argon2id master key.');
    } catch (err: any) {
      errorMessage = err.toString() || 'Initialization failed';
    } finally {
      isLoading = false;
    }
  }

  async function handleBiometricUnlock() {
    isLoading = true;
    errorMessage = '';
    try {
      if (isTauri) {
        await (window as any).__TAURI__.invoke('unlock_with_biometrics');
      } else {
        await new Promise(r => setTimeout(r, 600));
      }
      isLocked = false;
      await loadSecrets();
      showToast('Unlocked via Windows Hello / Hardware Biometrics.');
    } catch (err: any) {
      errorMessage = err?.toString() || 'Biometric verification failed.';
    } finally {
      isLoading = false;
    }
  }

  async function handleLock() {
    if (isTauri) {
      await (window as any).__TAURI__.invoke('lock_vault');
    }
    isLocked = true;
    selectedSecretId = null;
    selectedSecret = null;
    secrets = [];
    clipboardSeconds = null;
    showToast('Vault locked. In-memory master key zeroized.');
  }

  async function loadSecrets() {
    if (isTauri) {
      secrets = await (window as any).__TAURI__.invoke('list_secrets');
    } else {
      secrets = mockSecrets.map(s => s.metadata);
    }
    if (secrets.length > 0 && !selectedSecretId) {
      selectSecret(secrets[0].id);
    }
  }

  async function selectSecret(id: string) {
    selectedSecretId = id;
    if (isTauri) {
      selectedSecret = await (window as any).__TAURI__.invoke('get_secret', { id });
      activeAuditLogs = await (window as any).__TAURI__.invoke('get_audit_logs', { secretId: id, limit: 20 });
    } else {
      selectedSecret = mockSecrets.find(s => s.metadata.id === id) || null;
      activeAuditLogs = [
        { id: '1', secret_id: id, action: 'created', timestamp: Math.floor(Date.now() / 1000) - 3600, client: 'desktop-tauri' },
        { id: '2', secret_id: id, action: 'read', timestamp: Math.floor(Date.now() / 1000) - 60, client: 'desktop-tauri' },
      ];
    }
  }

  async function handleSearch(event: CustomEvent<string>) {
    const query = event.detail.trim();
    if (!query) {
      await loadSecrets();
      return;
    }

    if (isTauri) {
      const hits: any[] = await (window as any).__TAURI__.invoke('search_secrets', { query, limit: 50 });
      secrets = hits.map(h => ({
        id: h.id,
        title: h.title,
        secret_type: h.secret_type,
        tags: h.tags,
        username: h.username,
        version: 1,
        created_at: 0,
        updated_at: 0,
      }));
    } else {
      const q = query.toLowerCase();
      secrets = mockSecrets
        .filter(s =>
          s.metadata.title.toLowerCase().includes(q) ||
          s.metadata.tags.some(t => t.includes(q)) ||
          (s.metadata.username && s.metadata.username.toLowerCase().includes(q))
        )
        .map(s => s.metadata);
    }
  }

  function getPrimarySecretValue(payload: SecretPayload): string {
    switch (payload.type) {
      case 'password': return payload.data.password;
      case 'ssh_key': return payload.data.private_key;
      case 'certificate': return payload.data.private_key;
      case 'api_key': return payload.data.api_key;
      case 'database_credential': return payload.data.password;
      default: return '';
    }
  }

  async function handleCopySecret(event: CustomEvent<{ id: string; field: string }>) {
    const { id } = event.detail;
    clipboardSeconds = 30;

    if (isTauri) {
      await (window as any).__TAURI__.invoke('copy_to_clipboard', { id, ttlSecs: 30 });
    } else {
      if (selectedSecret) {
        await navigator.clipboard.writeText(getPrimarySecretValue(selectedSecret.payload));
      }
    }

    showToast('Copied to secure clipboard. Auto-wiping memory in 30 seconds.');
  }

  async function handleClearClipboard() {
    if (isTauri) {
      await (window as any).__TAURI__.invoke('clear_clipboard');
    }
    clipboardSeconds = null;
    showToast('Clipboard zeroized and cleared.');
  }

  function startClipboardPoll() {
    clipboardTimer = window.setInterval(() => {
      if (clipboardSeconds !== null && clipboardSeconds > 0) {
        clipboardSeconds -= 1;
        if (clipboardSeconds <= 0) {
          clipboardSeconds = null;
          showToast('Clipboard timer expired. Secret zeroized from memory.');
        }
      }
    }, 1000);
  }

  async function handleCreateSecret(event: CustomEvent<any>) {
    isLoading = true;
    try {
      const input = event.detail;
      if (isTauri) {
        const meta = await (window as any).__TAURI__.invoke('create_secret', { input });
        secrets = [meta, ...secrets];
        selectSecret(meta.id);
      } else {
        const newSec: DecryptedSecret = {
          metadata: {
            id: `mock-${Date.now()}`,
            title: input.title,
            secret_type: input.payload.type,
            tags: input.tags,
            username: input.username,
            url: input.url,
            notes: input.notes,
            version: 1,
            created_at: Math.floor(Date.now() / 1000),
            updated_at: Math.floor(Date.now() / 1000),
          },
          payload: input.payload,
        };
        mockSecrets.unshift(newSec);
        await loadSecrets();
        selectSecret(newSec.metadata.id);
      }

      isNewSecretModalOpen = false;
      showToast('Encrypted secret stored with AES-256-GCM.');
    } catch (err: any) {
      showToast('Error saving secret: ' + err.toString());
    } finally {
      isLoading = false;
    }
  }

  async function handleDeleteSecret(event: CustomEvent<string>) {
    const id = event.detail;
    if (!confirm('Are you sure you want to permanently delete this secret?')) return;

    if (isTauri) {
      await (window as any).__TAURI__.invoke('delete_secret', { id });
    } else {
      mockSecrets = mockSecrets.filter(s => s.metadata.id !== id);
    }

    showToast('Secret permanently removed from database.');
    selectedSecret = null;
    selectedSecretId = null;
    await loadSecrets();
  }

  async function handleFetchHealthReport(): Promise<VaultHealthReport> {
    if (isTauri) {
      return await (window as any).__TAURI__.invoke('get_health_report');
    }
    return {
      overall_score: 92,
      grade: 'A',
      total_secrets: mockSecrets.length,
      compromised_count: 0,
      reused_count: 0,
      weak_count: 0,
      stale_count: 0,
      cert_issues_count: 0,
      recommendations: [],
    };
  }

  async function handleGenerateTotp(secretId: string): Promise<TotpCodeResponse> {
    if (isTauri) {
      return await (window as any).__TAURI__.invoke('generate_totp_code', { secretId });
    }
    const elapsed = Math.floor(Date.now() / 1000) % 30;
    return {
      code: '849201',
      remaining_seconds: 30 - elapsed,
      total_step_seconds: 30,
      progress_ratio: elapsed / 30,
    };
  }

  async function handleInspectCert(secretId: string): Promise<CertInspectionReport> {
    if (isTauri) {
      return await (window as any).__TAURI__.invoke('inspect_certificate', { secretId });
    }
    return {
      subject_cn: 'api.vault.internal',
      issuer_cn: 'DigiCert Global Root CA',
      not_before_epoch: Math.floor(Date.now() / 1000) - 86400 * 30,
      not_after_epoch: Math.floor(Date.now() / 1000) + 86400 * 335,
      days_until_expiration: 335,
      status: 'valid',
      is_self_signed: false,
      signature_algorithm: 'SHA256withRSA',
      is_weak_signature: false,
      public_key_algorithm: 'RSA',
      key_size_bits: 2048,
      is_weak_key: false,
      subject_alt_names: ['api.vault.internal', 'gateway.vault.internal'],
      serial_number: '12:34:56:78:9A:BC:DE:F0',
    };
  }

  async function handleExportBackup(passphrase: string, outputPath: string): Promise<BackupExportSummary> {
    if (isTauri) {
      return await (window as any).__TAURI__.invoke('export_backup', { passphrase, outputPath });
    }
    return {
      output_path: outputPath,
      total_secrets: mockSecrets.length,
      file_size_bytes: 4096,
      exported_at: Math.floor(Date.now() / 1000),
    };
  }

  async function handleImportBackup(passphrase: string, inputPath: string, overwrite: boolean): Promise<BackupImportSummary> {
    if (isTauri) {
      return await (window as any).__TAURI__.invoke('import_backup', { passphrase, inputPath, overwrite });
    }
    return {
      total_imported: 3,
      total_skipped: 0,
      total_overwritten: 0,
      original_exported_at: Math.floor(Date.now() / 1000) - 86400,
    };
  }
</script>

<div class="app-layout">
  <!-- Top Navigation Header -->
  <Header
    {searchQuery}
    selectedType={filterType}
    {clipboardSeconds}
    {isLocked}
    on:search={handleSearch}
    on:filterChange={e => { filterType = e.detail; }}
    on:openNewSecretModal={() => { isNewSecretModalOpen = true; }}
    on:openHealthDashboard={() => { isHealthDashboardOpen = true; }}
    on:openBackupModal={() => { isBackupModalOpen = true; }}
    on:openShamirModal={() => { shamirModalMode = 'split'; isShamirModalOpen = true; }}
    on:lockVault={handleLock}
    on:clearClipboard={handleClearClipboard}
  />

  <!-- Main View Split -->
  {#if !isLocked}
    <main class="main-split">
      <div class="sidebar">
        <SecretList
          {secrets}
          {selectedSecretId}
          {filterType}
          on:selectSecret={e => selectSecret(e.detail)}
          on:openNewSecret={() => { isNewSecretModalOpen = true; }}
        />
      </div>

      <div class="content-pane">
        <SecretDetail
          secret={selectedSecret}
          auditLogs={activeAuditLogs}
          {isLoading}
          {clipboardSeconds}
          onGenerateTotp={handleGenerateTotp}
          onInspectCert={handleInspectCert}
          on:copySecret={handleCopySecret}
          on:deleteSecret={handleDeleteSecret}
        />
      </div>
    </main>
  {/if}

  <!-- Unlock / Init Modal -->
  {#if isLocked}
    <UnlockModal
      {isInitialized}
      {errorMessage}
      {isLoading}
      {biometricAvailable}
      on:unlock={handleUnlock}
      on:initialize={handleInitialize}
      on:biometricUnlock={handleBiometricUnlock}
      on:openShamirRecovery={() => { shamirModalMode = 'unlock'; isShamirModalOpen = true; }}
    />
  {/if}

  <!-- New Secret Modal -->
  <NewSecretModal
    isOpen={isNewSecretModalOpen}
    {isLoading}
    on:close={() => { isNewSecretModalOpen = false; }}
    on:createSecret={handleCreateSecret}
  />

  <!-- Health Dashboard Modal -->
  {#if isHealthDashboardOpen}
    <HealthDashboard
      onFetchHealth={handleFetchHealthReport}
      onSelectSecret={(id) => {
        selectSecret(id);
        isHealthDashboardOpen = false;
      }}
      onClose={() => { isHealthDashboardOpen = false; }}
    />
  {/if}

  <!-- Backup & Portability Modal -->
  {#if isBackupModalOpen}
    <BackupModal
      onExport={handleExportBackup}
      onImport={handleImportBackup}
      onClose={() => { isBackupModalOpen = false; }}
      onRefreshVault={loadSecrets}
    />
  {/if}

  <!-- Shamir Secret Sharing Modal -->
  <ShamirModal
    isOpen={isShamirModalOpen}
    mode={shamirModalMode}
    on:close={() => { isShamirModalOpen = false; }}
    on:unlocked={async () => {
      isLocked = false;
      isShamirModalOpen = false;
      await loadSecrets();
      showToast('Vault master key reconstructed via Shamir threshold shares.');
    }}
  />

  <!-- Toast Notification -->
  {#if toastMessage}
    <div class="toast">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="16" height="16" class="toast-icon">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span>{toastMessage}</span>
    </div>
  {/if}
</div>

<style>
  .app-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .main-split {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .sidebar {
    width: 360px;
    height: 100%;
    flex-shrink: 0;
  }

  .content-pane {
    flex: 1;
    height: 100%;
    overflow: hidden;
  }

  .toast {
    position: fixed;
    bottom: 24px;
    right: 24px;
    background: rgba(14, 20, 32, 0.95);
    border: 1px solid var(--accent-emerald);
    color: #ffffff;
    padding: 12px 18px;
    border-radius: var(--radius-sm);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5), 0 0 15px rgba(16, 185, 129, 0.25);
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    font-weight: 500;
    z-index: 9999;
    animation: slideUp 0.25s ease-out;
  }

  .toast-icon {
    color: var(--accent-emerald);
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
