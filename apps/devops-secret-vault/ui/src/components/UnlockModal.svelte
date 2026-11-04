<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let isInitialized: boolean = false;
  export let errorMessage: string = '';
  export let isLoading: boolean = false;
  export let biometricAvailable: boolean = true;
  export let vaultPath: string = '';

  let masterPassword = '';
  let confirmPassword = '';
  let showPassword = false;
  let shakeError = false;
  let confirmResetMode = false;
  let showSwitchPathInput = false;
  let customVaultPath = '';

  const dispatch = createEventDispatcher();

  $: if (errorMessage) {
    shakeError = true;
    setTimeout(() => { shakeError = false; }, 600);
  }

  function handleSubmit() {
    if (isInitialized) {
      if (!masterPassword) return;
      dispatch('unlock', { password: masterPassword });
    } else {
      if (!masterPassword || masterPassword !== confirmPassword) {
        errorMessage = "Passwords do not match or are empty";
        shakeError = true;
        setTimeout(() => { shakeError = false; }, 600);
        return;
      }
      dispatch('initialize', { password: masterPassword });
    }
  }

  function handleBiometric() {
    dispatch('biometricUnlock');
  }

  async function handleBrowseVaultFile() {
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const tauri = (window as any).__TAURI__;
      if (tauri?.dialog?.open) {
        const picked = await tauri.dialog.open({
          title: 'Open or Select DevOps Secret Vault Database (.db)',
          directory: false,
          multiple: false,
          filters: 'Vault Database (*.db;*.sqlite)|*.db;*.sqlite|All Files (*.*)|*.*',
        });
        if (typeof picked === 'string' && picked.trim()) {
          dispatch('switchVault', { path: picked.trim() });
          return;
        }
      }
      if (typeof fetch !== 'undefined' && typeof window !== 'undefined' && window.location?.protocol.startsWith('http')) {
        const res = await fetch('/__dialog/open', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            title: 'Open or Select DevOps Secret Vault Database (.db)',
            filters: [{ name: 'Vault Database', extensions: ['db', 'sqlite'] }],
          }),
        });
        if (res.ok) {
          const data = await res.json();
          if (data && data.path) {
            dispatch('switchVault', { path: data.path });
            return;
          }
        }
      }
    } catch (e) {
      console.error('Vault file dialog error:', e);
    }
    customVaultPath = vaultPath;
    showSwitchPathInput = !showSwitchPathInput;
  }

  function applyCustomVaultPath() {
    if (!customVaultPath.trim()) return;
    showSwitchPathInput = false;
    dispatch('switchVault', { path: customVaultPath.trim() });
  }
</script>

<div class="modal-backdrop">
  <div class="unlock-card glass-panel {shakeError ? 'shake' : ''}">
    <div class="card-icon-wrapper">
      <div class="vault-shield">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" class="shield-svg">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
        </svg>
      </div>
    </div>

    <h2 class="card-title">
      {isInitialized ? 'Unlock DevOps Secret Vault' : 'Initialize New Master Vault'}
    </h2>
    <p class="card-subtitle">
      {isInitialized
        ? 'Enter your master passphrase or use hardware biometrics to decrypt in-memory keys.'
        : 'Set a strong master passphrase. It will derive your 256-bit AES master key via Argon2id.'}
    </p>

    {#if vaultPath}
      <div class="vault-path-pill" title={vaultPath}>
        <span>🗄️ Vault DB:</span>
        <code>{vaultPath}</code>
      </div>
    {/if}

    {#if errorMessage}
      <div class="error-banner">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="16" height="16">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span>{errorMessage}</span>
      </div>
    {/if}

    <form on:submit|preventDefault={handleSubmit} class="unlock-form">
      <div class="input-group">
        <label for="master-password">
          {isInitialized ? 'Master Passphrase' : 'New Master Passphrase'}
        </label>
        <div class="password-wrapper">
          {#if showPassword}
            <input
              id="master-password"
              type="text"
              placeholder="••••••••••••••••••••"
              bind:value={masterPassword}
              disabled={isLoading}
              autocomplete="current-password"
            />
          {:else}
            <input
              id="master-password"
              type="password"
              placeholder="••••••••••••••••••••"
              bind:value={masterPassword}
              disabled={isLoading}
              autocomplete="current-password"
            />
          {/if}
          <button
            type="button"
            class="toggle-vis-btn"
            on:click={() => { showPassword = !showPassword; }}
            title={showPassword ? 'Hide password' : 'Show password'}
          >
            {#if showPassword}
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="16" height="16">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l18 18" />
              </svg>
            {:else}
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="16" height="16">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
              </svg>
            {/if}
          </button>
        </div>
      </div>

      {#if !isInitialized}
        <div class="input-group">
          <label for="confirm-password">Confirm Master Passphrase</label>
          {#if showPassword}
            <input
              id="confirm-password"
              type="text"
              placeholder="••••••••••••••••••••"
              bind:value={confirmPassword}
              disabled={isLoading}
            />
          {:else}
            <input
              id="confirm-password"
              type="password"
              placeholder="••••••••••••••••••••"
              bind:value={confirmPassword}
              disabled={isLoading}
            />
          {/if}
        </div>
      {/if}

      <div class="actions">
        <button type="submit" class="btn btn-primary submit-btn" disabled={isLoading}>
          {#if isLoading}
            <span class="spinner"></span>
            Deriving Argon2id Key...
          {:else}
            {isInitialized ? 'Unlock Vault' : 'Initialize & Encrypt'}
          {/if}
        </button>

        {#if isInitialized && biometricAvailable}
          <button
            type="button"
            class="btn btn-biometric"
            on:click={handleBiometric}
            disabled={isLoading}
            title="Authenticate with Windows Hello / Touch ID"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="18" height="18">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 11c0 3.517-1.009 6.799-2.753 9.571m-3.44-2.04l.054-.09A13.916 13.916 0 008 11a4 4 0 118 0c0 1.017-.07 2.019-.203 3m-2.118 6.844A21.88 21.88 0 0015.171 17m3.839 1.132c.645-2.266.99-4.659.99-7.132A8 8 0 004.07 9m5.936 8.79A14.04 14.04 0 0112 11" />
            </svg>
            Unlock with Biometrics
          </button>
        {/if}

        {#if isInitialized}
          <button
            type="button"
            class="btn btn-shamir"
            on:click={() => dispatch('openShamirRecovery')}
            disabled={isLoading}
            title="Recover master vault using k-of-n Shamir shares"
          >
            <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16">
              <path fill-rule="evenodd" d="M10 2a1 1 0 011 1v1.323l3.954 1.582 1.599-.8a1 1 0 01.894 1.79l-1.233.616 1.738 5.42a1 1 0 01-.285 1.05A3.989 3.989 0 0115 15a3.989 3.989 0 01-2.667-1.019 1 1 0 01-.285-1.05l1.715-5.349L11 6.477V16h2a1 1 0 110 2H7a1 1 0 110-2h2V6.477L6.237 7.582l1.715 5.349a1 1 0 01-.285 1.05A3.989 3.989 0 015 15a3.989 3.989 0 01-2.667-1.019 1 1 0 01-.285-1.05l1.738-5.42-1.233-.617a1 1 0 01.894-1.788l1.599.799L9 4.323V3a1 1 0 011-1z" clip-rule="evenodd" />
            </svg>
            Emergency Shamir Recovery
          </button>
        {/if}

        <div class="vault-recovery-row">
          <button
            type="button"
            class="btn btn-secondary-action"
            on:click={handleBrowseVaultFile}
            disabled={isLoading}
            title="Switch to a different .db vault file on disk"
          >
            📂 Switch Vault File...
          </button>

          {#if isInitialized}
            {#if !confirmResetMode}
              <button
                type="button"
                class="btn btn-reset-action"
                on:click={() => { confirmResetMode = true; }}
                disabled={isLoading}
                title="Reset vault database and create a new master passphrase"
              >
                🔄 Reset / Create New Vault
              </button>
            {:else}
              <button
                type="button"
                class="btn btn-danger-confirm"
                on:click={() => {
                  confirmResetMode = false;
                  masterPassword = '';
                  confirmPassword = '';
                  dispatch('resetVault');
                }}
                disabled={isLoading}
              >
                ⚠️ Confirm Wipe &amp; Create New
              </button>
            {/if}
          {/if}
        </div>

        {#if showSwitchPathInput}
          <div class="switch-path-row">
            <input
              type="text"
              bind:value={customVaultPath}
              placeholder="Enter .db file path (e.g. C:\vaults\team_vault.db)..."
              on:keydown={(e) => e.key === 'Enter' && applyCustomVaultPath()}
            />
            <button type="button" class="btn btn-primary" on:click={applyCustomVaultPath}>Open</button>
          </div>
        {/if}
      </div>
    </form>

    <div class="security-footer">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="13" height="13">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
      </svg>
      <span>Argon2id (64MB RAM, 3 iterations) • AES-256-GCM • Zero-RAM Retention</span>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(4, 7, 12, 0.85);
    backdrop-filter: blur(20px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .unlock-card {
    width: 440px;
    padding: 36px 32px;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.7), 0 0 0 1px rgba(255, 255, 255, 0.08);
  }

  .card-icon-wrapper {
    margin-bottom: 18px;
  }

  .vault-shield {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 0 24px rgba(16, 185, 129, 0.25);
  }

  .shield-svg {
    width: 28px;
    height: 28px;
    color: var(--accent-emerald);
  }

  .card-title {
    font-size: 20px;
    font-weight: 700;
    color: #ffffff;
    margin-bottom: 8px;
  }

  .card-subtitle {
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.5;
    margin-bottom: 24px;
  }

  .error-banner {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.35);
    color: #fca5a5;
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    margin-bottom: 18px;
    text-align: left;
  }

  .unlock-form {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    text-align: left;
  }

  .input-group label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .password-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .password-wrapper input {
    width: 100%;
    padding-right: 40px;
  }

  .toggle-vis-btn {
    position: absolute;
    right: 10px;
    background: transparent;
    border: none;
    color: var(--text-faint);
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  .toggle-vis-btn:hover {
    color: var(--text-main);
  }

  .actions {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 10px;
  }

  .submit-btn {
    width: 100%;
    padding: 12px;
    font-size: 14px;
  }

  .btn-biometric {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    padding: 10px;
    font-size: 13px;
  }

  .btn-biometric:hover {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .btn-shamir {
    background: rgba(168, 85, 247, 0.1);
    border: 1px solid rgba(168, 85, 247, 0.25);
    color: #c084fc;
    padding: 10px;
    font-size: 13px;
  }

  .btn-shamir:hover {
    background: rgba(168, 85, 247, 0.2);
    border-color: rgba(168, 85, 247, 0.4);
    color: #e9d5ff;
  }

  .vault-path-pill {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(15, 23, 42, 0.65);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 6px 10px;
    font-size: 11px;
    color: var(--text-muted);
    margin-bottom: 14px;
    overflow: hidden;
  }

  .vault-path-pill code {
    font-family: var(--font-mono);
    color: var(--accent-cyan);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .vault-recovery-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-top: 4px;
  }

  .btn-secondary-action {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    padding: 8px 10px;
    font-size: 12px;
  }

  .btn-secondary-action:hover {
    background: rgba(255, 255, 255, 0.09);
    color: #ffffff;
  }

  .btn-reset-action {
    background: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.3);
    color: #fcd34d;
    padding: 8px 10px;
    font-size: 12px;
  }

  .btn-reset-action:hover {
    background: rgba(245, 158, 11, 0.2);
    border-color: rgba(245, 158, 11, 0.5);
  }

  .btn-danger-confirm {
    background: rgba(239, 68, 68, 0.25);
    border: 1px solid #ef4444;
    color: #fecaca;
    padding: 8px 10px;
    font-size: 12px;
    font-weight: 700;
  }

  .switch-path-row {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }

  .switch-path-row input {
    flex: 1;
    font-size: 12px;
    padding: 6px 10px;
  }

  .security-footer {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    color: var(--text-faint);
    font-family: var(--font-mono);
    margin-top: 24px;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-radius: 50%;
    border-top-color: #ffffff;
    animation: spin 0.8s linear infinite;
    display: inline-block;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .shake {
    animation: shakeAnim 0.5s cubic-bezier(.36,.07,.19,.97) both;
  }

  @keyframes shakeAnim {
    10%, 90% { transform: translate3d(-2px, 0, 0); }
    20%, 80% { transform: translate3d(3px, 0, 0); }
    30%, 50%, 70% { transform: translate3d(-5px, 0, 0); }
    40%, 60% { transform: translate3d(5px, 0, 0); }
  }
</style>
