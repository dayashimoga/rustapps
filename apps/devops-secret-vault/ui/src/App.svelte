<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import Header from './components/Header.svelte';
  import SecretList from './components/SecretList.svelte';
  import SecretDetail from './components/SecretDetail.svelte';
  import UnlockModal from './components/UnlockModal.svelte';
  import NewSecretModal from './components/NewSecretModal.svelte';
  import HealthDashboard from './components/HealthDashboard.svelte';
  import BackupModal from './components/BackupModal.svelte';
  import ShamirModal from './components/ShamirModal.svelte';
  import LeakScannerModal from './components/LeakScannerModal.svelte';
  import PasswordGeneratorModal from './components/PasswordGeneratorModal.svelte';
  import CommandPalette, { type CommandAction } from './components/CommandPalette.svelte';
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
  let vaultPath = '';

  let secrets: SecretMetadata[] = [];
  let selectedSecretId: string | null = null;
  let selectedSecret: DecryptedSecret | null = null;
  let activeAuditLogs: SecretAuditEntry[] = [];

  let searchQuery = '';
  let filterType: SecretType | 'all' = 'all';

  let isNewSecretModalOpen = false;
  let editingSecret: DecryptedSecret | null = null;
  let isHealthDashboardOpen = false;
  let isBackupModalOpen = false;
  let isShamirModalOpen = false;
  let isLeakScannerOpen = false;
  let isGeneratorOpen = false;
  let isCommandPaletteOpen = false;
  let shamirModalMode: 'split' | 'unlock' = 'split';
  let biometricAvailable = false;
  let clipboardSeconds: number | null = null;
  let clipboardTimer: number | null = null;

  $: commandActions = [
    {
      id: 'act-new-secret',
      title: 'Create New Secret',
      subtitle: 'Add password, API key, certificate, or SSH key',
      category: 'Actions',
      icon: '➕',
      shortcut: 'Ctrl+N',
      run: () => { editingSecret = null; isNewSecretModalOpen = true; },
    },
    {
      id: 'act-lock',
      title: 'Lock Vault Immediately',
      subtitle: 'Zeroize master keys and wipe decrypted state',
      category: 'Security',
      icon: '🔒',
      shortcut: 'Ctrl+L',
      run: handleLock,
    },
    {
      id: 'act-health',
      title: 'Security Health & Audit Chain',
      subtitle: 'Evaluate entropy, reuse, age, and HMAC verification',
      category: 'Security',
      icon: '🛡️',
      shortcut: 'Ctrl+H',
      run: () => { isHealthDashboardOpen = true; },
    },
    {
      id: 'act-leak-scanner',
      title: 'Offline Secret Leak Scanner',
      subtitle: 'Scan repositories and folders for hardcoded credentials',
      category: 'DevOps',
      icon: '🔍',
      shortcut: 'Ctrl+Shift+L',
      run: () => { isLeakScannerOpen = true; },
    },
    {
      id: 'act-generator',
      title: 'Password & Passphrase Generator',
      subtitle: 'NIST 800-63B compliant high-entropy offline generator',
      category: 'Tools',
      icon: '⚡',
      shortcut: 'Ctrl+G',
      run: () => { isGeneratorOpen = true; },
    },
    {
      id: 'act-backup',
      title: 'Encrypted Backup & Portability',
      subtitle: 'Export or restore Argon2id + AES-256-GCM encrypted vault backup',
      category: 'DevOps',
      icon: '💾',
      shortcut: 'Ctrl+B',
      run: () => { isBackupModalOpen = true; },
    },
    {
      id: 'act-shamir',
      title: 'Shamir Secret Sharing (M-of-N)',
      subtitle: 'Split master key into threshold shards or recover',
      category: 'Security',
      icon: '🧩',
      shortcut: 'Ctrl+Shift+S',
      run: () => { shamirModalMode = 'split'; isShamirModalOpen = true; },
    },
    {
      id: 'act-export-env',
      title: 'Export .env / KubeConfig',
      subtitle: 'Format active secrets for local dev environments',
      category: 'DevOps',
      icon: '📦',
      shortcut: 'Ctrl+E',
      run: handleExportDotenv,
    },
    {
      id: 'act-clear-clipboard',
      title: 'Clear Clipboard Memory',
      subtitle: 'Immediately wipe active clipboard buffer',
      category: 'Security',
      icon: '🧹',
      shortcut: 'Ctrl+Shift+X',
      run: handleClearClipboard,
    },
    {
      id: 'filter-all',
      title: 'Show All Secrets',
      subtitle: 'Display complete vault contents',
      category: 'Filter',
      icon: '📑',
      shortcut: '',
      run: () => { filterType = 'all'; },
    },
    {
      id: 'filter-password',
      title: 'Filter: Passwords & Credentials',
      subtitle: 'Show web and service logins',
      category: 'Filter',
      icon: '🔑',
      shortcut: '',
      run: () => { filterType = 'password'; },
    },
    {
      id: 'filter-api-key',
      title: 'Filter: API Keys & Bearer Tokens',
      subtitle: 'Show cloud and third-party tokens',
      category: 'Filter',
      icon: '🏷️',
      shortcut: '',
      run: () => { filterType = 'api_key'; },
    },
    {
      id: 'filter-ssh',
      title: 'Filter: SSH Keypairs',
      subtitle: 'Show Ed25519 & RSA private keys',
      category: 'Filter',
      icon: '🖥️',
      shortcut: '',
      run: () => { filterType = 'ssh_key'; },
    },
    {
      id: 'filter-cert',
      title: 'Filter: X.509 Certificates',
      subtitle: 'Show TLS certificates and root CAs',
      category: 'Filter',
      icon: '📜',
      shortcut: '',
      run: () => { filterType = 'certificate'; },
    },
    {
      id: 'filter-db',
      title: 'Filter: Database Connection URIs',
      subtitle: 'Show PostgreSQL, MySQL, Redis, MongoDB URIs',
      category: 'Filter',
      icon: '🗄️',
      shortcut: '',
      run: () => { filterType = 'database'; },
    },
  ];

  function handleGlobalKeyDown(e: KeyboardEvent) {
    const isCtrl = e.ctrlKey || e.metaKey;

    if (isCtrl && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      isCommandPaletteOpen = !isCommandPaletteOpen;
      return;
    }

    if (e.key === 'Escape') {
      if (isCommandPaletteOpen) {
        e.preventDefault();
        isCommandPaletteOpen = false;
        return;
      }
      if (isNewSecretModalOpen) {
        e.preventDefault();
        isNewSecretModalOpen = false;
        editingSecret = null;
        return;
      }
      if (isHealthDashboardOpen) {
        e.preventDefault();
        isHealthDashboardOpen = false;
        return;
      }
      if (isBackupModalOpen) {
        e.preventDefault();
        isBackupModalOpen = false;
        return;
      }
      if (isShamirModalOpen) {
        e.preventDefault();
        isShamirModalOpen = false;
        return;
      }
      if (isLeakScannerOpen) {
        e.preventDefault();
        isLeakScannerOpen = false;
        return;
      }
      if (isGeneratorOpen) {
        e.preventDefault();
        isGeneratorOpen = false;
        return;
      }
      if (searchQuery) {
        e.preventDefault();
        searchQuery = '';
        return;
      }
      return;
    }

    // Ignore other shortcuts when typing in inputs/textareas
    const activeTagName = (document.activeElement?.tagName || '').toLowerCase();
    const isInputActive = activeTagName === 'input' || activeTagName === 'textarea';
    if (isInputActive) return;

    if (isCtrl && e.key.toLowerCase() === 'n') {
      e.preventDefault();
      editingSecret = null;
      isNewSecretModalOpen = true;
    } else if (isCtrl && e.key.toLowerCase() === 'l') {
      e.preventDefault();
      handleLock();
    } else if (isCtrl && e.key.toLowerCase() === 'h') {
      e.preventDefault();
      isHealthDashboardOpen = true;
    } else if (isCtrl && e.key.toLowerCase() === 'g') {
      e.preventDefault();
      isGeneratorOpen = true;
    } else if (isCtrl && e.key.toLowerCase() === 'b') {
      e.preventDefault();
      isBackupModalOpen = true;
    } else if (isCtrl && e.shiftKey && e.key.toLowerCase() === 'l') {
      e.preventDefault();
      isLeakScannerOpen = true;
    } else if (isCtrl && e.shiftKey && e.key.toLowerCase() === 's') {
      e.preventDefault();
      shamirModalMode = 'split';
      isShamirModalOpen = true;
    } else if (isCtrl && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      (document.querySelector('.search-input') as HTMLInputElement)?.focus();
    }
  }

  // Notification toast
  let toastMessage = '';
  let toastTimeout: number | null = null;

  function showToast(msg: string) {
    toastMessage = msg;
    if (toastTimeout) clearTimeout(toastTimeout);
    toastTimeout = window.setTimeout(() => { toastMessage = ''; }, 3500);
  }

  // --- Standardized Desktop IPC Bridge ---
  async function invokeTauri<T>(cmd: string, args: Record<string, any> = {}): Promise<T> {
    return await invokeIpc<T>(cmd, args);
  }

  onMount(async () => {
    await checkStatus();
    startClipboardPoll();
  });

  onDestroy(() => {
    if (clipboardTimer) clearInterval(clipboardTimer);
  });

  async function checkStatus() {
    try {
      const res: any = await invokeTauri('get_vault_status');
      isInitialized = Boolean(res.is_initialized);
      isLocked = res.is_locked ?? true;
      if (res.vault_path) {
        vaultPath = res.vault_path;
      }
      try {
        biometricAvailable = await invokeTauri('is_biometric_available');
      } catch (_) {
        biometricAvailable = false;
      }
      if (!isLocked) {
        await loadSecrets();
      }
    } catch (err: any) {
      console.warn('Status check fallback to local uninitialized state:', err);
      isInitialized = false;
      isLocked = true;
      biometricAvailable = false;
    }
  }

  async function handleUnlock(event: CustomEvent<{ password: string }>) {
    isLoading = true;
    errorMessage = '';
    const { password } = event.detail;

    try {
      await invokeTauri('unlock_vault', { masterPassword: password });
      isLocked = false;
      await loadSecrets();
      showToast('Vault unlocked. Ephemeral Tantivy index loaded into RAM.');
    } catch (err: any) {
      errorMessage = err?.message || err?.toString() || 'Authentication failed';
    } finally {
      isLoading = false;
    }
  }

  async function handleInitialize(event: CustomEvent<{ password: string }>) {
    isLoading = true;
    errorMessage = '';
    const { password } = event.detail;

    try {
      await invokeTauri('initialize_vault', { masterPassword: password });
      isInitialized = true;
      isLocked = false;
      await loadSecrets();
      showToast('Vault initialized with Argon2id master key.');
    } catch (err: any) {
      errorMessage = err?.message || err?.toString() || 'Initialization failed';
    } finally {
      isLoading = false;
    }
  }

  async function handleBiometricUnlock() {
    isLoading = true;
    errorMessage = '';
    try {
      await invokeTauri('unlock_with_biometrics');
      isLocked = false;
      await loadSecrets();
      showToast('Unlocked via Windows Hello / Hardware Biometrics.');
    } catch (err: any) {
      errorMessage = err?.message || err?.toString() || 'Biometric verification failed.';
    } finally {
      isLoading = false;
    }
  }

  async function handleToggleBiometricEnrollment() {
    try {
      if (biometricAvailable) {
        await invokeTauri('disable_biometric_unlock');
        biometricAvailable = false;
        showToast('Biometric unlock disabled and key purged from OS Keyring.');
      } else {
        await invokeTauri('enable_biometric_unlock');
        biometricAvailable = true;
        showToast('Windows Hello / OS Biometric unlock enabled for this vault.');
      }
    } catch (err: any) {
      showToast('Biometric enrollment failed: ' + (err?.message || err?.toString() || 'Unknown error'));
    }
  }

  async function handleResetVault() {
    isLoading = true;
    errorMessage = '';
    try {
      const status: any = await invokeTauri('reset_vault', {});
      isInitialized = Boolean(status?.is_initialized);
      isLocked = status?.is_locked ?? true;
      if (status?.vault_path) vaultPath = status.vault_path;
      secrets = [];
      selectedSecretId = null;
      selectedSecret = null;
      showToast('Vault reset to uninitialized state. Set your new Master Passphrase.');
    } catch (err: any) {
      errorMessage = err?.message || err?.toString() || 'Failed to reset vault';
    } finally {
      isLoading = false;
    }
  }

  async function handleSwitchVault(event: CustomEvent<{ path: string }>) {
    isLoading = true;
    errorMessage = '';
    try {
      const targetPath = event.detail.path;
      const status: any = await invokeTauri('switch_vault_db', { path: targetPath });
      isInitialized = Boolean(status?.is_initialized);
      isLocked = status?.is_locked ?? true;
      if (status?.vault_path) vaultPath = status.vault_path;
      secrets = [];
      selectedSecretId = null;
      selectedSecret = null;
      showToast(`Switched active vault database to: ${targetPath}`);
    } catch (err: any) {
      errorMessage = err?.message || err?.toString() || 'Failed to switch vault file';
    } finally {
      isLoading = false;
    }
  }

  async function handleLock() {
    try {
      await invokeTauri('lock_vault');
    } catch (err) {
      console.warn('Lock vault error:', err);
    }
    isLocked = true;
    selectedSecretId = null;
    selectedSecret = null;
    secrets = [];
    clipboardSeconds = null;
    showToast('Vault locked. In-memory master key zeroized.');
  }

  async function loadSecrets() {
    try {
      secrets = await invokeTauri<SecretMetadata[]>('list_secrets');
    } catch (err) {
      console.warn('Failed to list secrets:', err);
      secrets = [];
    }
    if (secrets.length > 0 && !selectedSecretId) {
      selectSecret(secrets[0].id);
    }
  }

  async function selectSecret(id: string) {
    selectedSecretId = id;
    try {
      selectedSecret = await invokeTauri<DecryptedSecret>('get_secret', { id });
      activeAuditLogs = await invokeTauri<SecretAuditEntry[]>('get_audit_logs', { secretId: id, limit: 20 });
    } catch (err) {
      console.error('Failed to get secret details:', err);
      selectedSecret = null;
      activeAuditLogs = [];
    }
  }

  async function handleSearch(event: CustomEvent<string>) {
    const query = event.detail.trim();
    if (!query) {
      await loadSecrets();
      return;
    }

    try {
      const hits: any[] = await invokeTauri('search_secrets', { query, limit: 50 });
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
    } catch (err) {
      console.warn('Search secrets failed:', err);
    }
  }

  function getPrimarySecretValue(payload: SecretPayload): string {
    switch (payload.type) {
      case 'password': return payload.data.password;
      case 'ssh_key': return payload.data.private_key;
      case 'certificate': return payload.data.private_key;
      case 'api_key': return payload.data.api_key;
      case 'database_credential': return payload.data.password;
      case 'aws_credential': return payload.data.secret_access_key;
      case 'azure_credential': return payload.data.client_secret;
      case 'terraform_secret': return payload.data.provider_token || payload.data.state_key || payload.data.tf_vars;
      case 'totp': return payload.data.secret_base32;
      default: return '';
    }
  }

  function getSecretFieldValue(sec: DecryptedSecret | null, field: string, directVal?: string): string {
    if (directVal) return directVal;
    if (!sec) return '';
    if (field === 'username') return sec.metadata.username || '';
    if (field === 'url') return sec.metadata.url || '';
    if (field === 'notes') return sec.metadata.notes || '';
    if (field === 'primary') return getPrimarySecretValue(sec.payload);

    const data: Record<string, any> = (sec.payload as any).data || {};
    if (field in data && data[field] !== undefined && data[field] !== null) {
      return String(data[field]);
    }
    return getPrimarySecretValue(sec.payload);
  }

  async function handleCopySecret(event: CustomEvent<{ id: string; field?: string; value?: string; directValue?: boolean }>) {
    const { id, field = 'primary', value } = event.detail;
    clipboardSeconds = 30;

    const textToCopy = getSecretFieldValue(selectedSecret, field, value);
    if (textToCopy && navigator.clipboard && navigator.clipboard.writeText) {
      try {
        await navigator.clipboard.writeText(textToCopy);
      } catch (e) {
        console.warn('Browser clipboard write warning:', e);
      }
    }

    if (id) {
      try {
        await invokeTauri('copy_to_clipboard', { id, ttlSecs: 30 });
      } catch (e) {
        console.warn('Backend clipboard audit warning:', e);
      }
    }

    showToast(`Copied ${field === 'primary' ? 'secret' : field.replace('_', ' ')} to clipboard. Auto-wiping in 30s.`);
  }

  async function handleClearClipboard() {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      try {
        await navigator.clipboard.writeText('');
      } catch {
        // ignore
      }
    }
    try {
      await invokeTauri('clear_clipboard');
    } catch (err) {
      console.warn('Clear clipboard error:', err);
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
          if (navigator.clipboard && navigator.clipboard.writeText) {
            navigator.clipboard.writeText('').catch(() => {});
          }
          showToast('Clipboard timer expired. Secret zeroized from memory.');
        }
      }
    }, 1000);
  }

  async function handleCreateSecret(event: CustomEvent<any>) {
    isLoading = true;
    try {
      const input = event.detail;
      const meta = await invokeTauri<SecretMetadata>('create_secret', { input });
      secrets = [meta, ...secrets];
      selectSecret(meta.id);

      isNewSecretModalOpen = false;
      editingSecret = null;
      showToast('Encrypted secret stored with AES-256-GCM.');
    } catch (err: any) {
      showToast('Error saving secret: ' + (err?.message || err?.toString()));
    } finally {
      isLoading = false;
    }
  }

  async function handleUpdateSecret(event: CustomEvent<{ id: string; input: any }>) {
    isLoading = true;
    try {
      const { id, input } = event.detail;
      const meta = await invokeTauri<SecretMetadata>('update_secret', { id, input });
      secrets = secrets.map(s => (s.id === id ? meta : s));
      await selectSecret(id);
      showToast(`Encrypted secret updated (v${meta.version}).`);
      isNewSecretModalOpen = false;
      editingSecret = null;
    } catch (err: any) {
      showToast('Error updating secret: ' + (err?.message || err?.toString()));
    } finally {
      isLoading = false;
    }
  }

  async function handleDeleteSecret(event: CustomEvent<string>) {
    const id = event.detail;
    if (!confirm('Are you sure you want to permanently delete this secret?')) return;

    try {
      await invokeTauri('delete_secret', { id });
      showToast('Secret permanently removed from database.');
      selectedSecret = null;
      selectedSecretId = null;
      await loadSecrets();
    } catch (err: any) {
      showToast('Failed to delete secret: ' + (err?.message || err?.toString()));
    }
  }

  async function handleFetchHealthReport(): Promise<VaultHealthReport> {
    return await invokeTauri<VaultHealthReport>('get_health_report');
  }

  async function handleVerifyAuditChain(): Promise<boolean> {
    return await invokeTauri<boolean>('verify_audit_chain');
  }

  async function handleGenerateTotp(secretId: string): Promise<TotpCodeResponse> {
    return await invokeTauri<TotpCodeResponse>('generate_totp_code', { secretId });
  }

  async function handleInspectCert(secretId: string): Promise<CertInspectionReport> {
    return await invokeTauri<CertInspectionReport>('inspect_certificate', { secretId });
  }

  async function handleExportBackup(passphrase: string, outputPath: string): Promise<BackupExportSummary> {
    return await invokeTauri<BackupExportSummary>('export_backup', { passphrase, outputPath });
  }

  async function handleImportBackup(passphrase: string, inputPath: string, overwrite: boolean): Promise<BackupImportSummary> {
    return await invokeTauri<BackupImportSummary>('import_backup', { passphrase, inputPath, overwrite });
  }

  // --- DevOps Special Features ---
  async function handleScanDirectory(dirPath: string): Promise<any[]> {
    return await invokeTauri<any[]>('scan_directory_for_leaks', { dirPath });
  }

  async function handleGeneratePassword(params: {
    length: number;
    includeUppercase: boolean;
    includeLowercase: boolean;
    includeNumbers: boolean;
    includeSymbols: boolean;
    mode: 'password' | 'passphrase';
    wordCount: number;
  }): Promise<{ password: string; entropy_bits: number; strength_label: string }> {
    return await invokeTauri('generate_secure_password', params);
  }

  async function handleExportDotenv() {
    try {
      const dotenvContent = await invokeTauri<string>('export_dotenv');
      if (!dotenvContent || !dotenvContent.trim()) {
        showToast('No secrets available to export.');
        return;
      }
      const blob = new Blob([dotenvContent], { type: 'text/plain;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = '.env.vault';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      showToast('Exported decrypted keys as .env.vault');
    } catch (err: any) {
      showToast('Failed to export .env: ' + (err?.message || err?.toString()));
    }
  }

  function handleSaveLeakedSecret(event: CustomEvent<{ title: string; type: string; value: string }>) {
    const { title, type, value } = event.detail;
    isLeakScannerOpen = false;
    editingSecret = {
      metadata: {
        id: '',
        title,
        secret_type: type as SecretType,
        tags: ['scanned-leak'],
        username: '',
        url: '',
        notes: `Imported from Secret Leak Scanner`,
        version: 1,
        created_at: Math.floor(Date.now() / 1000),
        updated_at: Math.floor(Date.now() / 1000),
      },
      payload: {
        type: (type === 'api_key' ? 'api_key' : 'password') as any,
        data: type === 'api_key' ? { api_key: value } : { password: value },
      },
    };
    isNewSecretModalOpen = true;
  }
</script>

<div class="app-layout">
  <!-- Top Navigation Header -->
  <Header
    {searchQuery}
    selectedType={filterType}
    {clipboardSeconds}
    {isLocked}
    biometricEnrolled={biometricAvailable}
    on:search={handleSearch}
    on:filterChange={e => { filterType = e.detail; }}
    on:openNewSecretModal={() => { editingSecret = null; isNewSecretModalOpen = true; }}
    on:openHealthDashboard={() => { isHealthDashboardOpen = true; }}
    on:openBackupModal={() => { isBackupModalOpen = true; }}
    on:openShamirModal={() => { shamirModalMode = 'split'; isShamirModalOpen = true; }}
    on:openLeakScanner={() => { isLeakScannerOpen = true; }}
    on:openGenerator={() => { isGeneratorOpen = true; }}
    on:openDotenv={handleExportDotenv}
    on:toggleBiometricEnrollment={handleToggleBiometricEnrollment}
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
          on:openNewSecret={() => { editingSecret = null; isNewSecretModalOpen = true; }}
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
          on:editSecret={e => { editingSecret = e.detail; isNewSecretModalOpen = true; }}
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
      {vaultPath}
      on:unlock={handleUnlock}
      on:initialize={handleInitialize}
      on:biometricUnlock={handleBiometricUnlock}
      on:resetVault={handleResetVault}
      on:switchVault={handleSwitchVault}
      on:openShamirRecovery={() => { shamirModalMode = 'unlock'; isShamirModalOpen = true; }}
    />
  {/if}

  <!-- New / Edit Secret Modal -->
  <NewSecretModal
    isOpen={isNewSecretModalOpen}
    {isLoading}
    {editingSecret}
    on:close={() => { isNewSecretModalOpen = false; editingSecret = null; }}
    on:createSecret={handleCreateSecret}
    on:updateSecret={handleUpdateSecret}
  />

  <!-- Health Dashboard Modal -->
  {#if isHealthDashboardOpen}
    <HealthDashboard
      onFetchHealth={handleFetchHealthReport}
      onVerifyAuditChain={handleVerifyAuditChain}
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

  <!-- DevOps Offline Secret Leak Scanner Modal -->
  <LeakScannerModal
    isOpen={isLeakScannerOpen}
    onScanDirectory={handleScanDirectory}
    on:close={() => { isLeakScannerOpen = false; }}
    on:saveSecret={handleSaveLeakedSecret}
  />

  <!-- DevOps Cryptographic Password & Passphrase Generator Modal -->
  <PasswordGeneratorModal
    isOpen={isGeneratorOpen}
    onGenerate={handleGeneratePassword}
    on:close={() => { isGeneratorOpen = false; }}
    on:usePassword={(e) => {
      navigator.clipboard?.writeText(e.detail.password);
      clipboardSeconds = 30;
      showToast('Copied generated password to clipboard (auto-wiping in 30s).');
      isGeneratorOpen = false;
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

  <!-- Global Command Palette -->
  <CommandPalette
    isOpen={isCommandPaletteOpen}
    actions={commandActions}
    on:close={() => { isCommandPaletteOpen = false; }}
  />
</div>

<svelte:window on:keydown={handleGlobalKeyDown} />

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
