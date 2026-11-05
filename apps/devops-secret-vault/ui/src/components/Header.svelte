<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { SecretType } from '../types';

  export let searchQuery: string = '';
  export let selectedType: SecretType | 'all' = 'all';
  export let clipboardSeconds: number | null = null;
  export let isLocked: boolean = false;
  export let biometricEnrolled: boolean = false;
  export let tier: string = 'Free';

  const dispatch = createEventDispatcher();

  const filterTabs: Array<{ id: SecretType | 'all'; label: string }> = [
    { id: 'all', label: 'All Items' },
    { id: 'password', label: 'Passwords' },
    { id: 'ssh_key', label: 'SSH Keys' },
    { id: 'certificate', label: 'Certificates' },
    { id: 'api_key', label: 'API Keys' },
    { id: 'database_credential', label: 'Databases' },
    { id: 'aws_credential', label: 'AWS' },
    { id: 'azure_credential', label: 'Azure' },
    { id: 'terraform_secret', label: 'Terraform' },
    { id: 'totp', label: '2FA / TOTP' },
  ];
</script>

<header class="vault-header glass-panel">
  <div class="header-left">
    <div class="brand">
      <svg class="brand-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
      </svg>
      <div class="brand-info">
        <span class="brand-title">DevOps Secret Vault</span>
        <span class="brand-tag">AES-256-GCM • Offline Zero-Knowledge</span>
        <button type="button" class="tier-badge-btn" on:click={() => dispatch('openLicenseModal')} title="Manage Offline License">
          <span class="tier-badge tier-{tier.toLowerCase()}">[{tier.toUpperCase()}]</span>
        </button>
      </div>
    </div>

    {#if !isLocked}
      <div class="search-box">
        <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <input
          type="text"
          placeholder="Search 100k+ secrets (<50ms)..."
          bind:value={searchQuery}
          on:input={() => dispatch('search', searchQuery)}
        />
        {#if searchQuery}
          <button class="clear-search" on:click={() => { searchQuery = ''; dispatch('search', ''); }}>✕</button>
        {/if}
      </div>
    {/if}
  </div>

  {#if !isLocked}
    <div class="header-center">
      <div class="filter-pills">
        {#each filterTabs as tab}
          <button
            class="pill {selectedType === tab.id ? 'active' : ''}"
            on:click={() => {
              selectedType = tab.id;
              dispatch('filterChange', tab.id);
            }}
          >
            {tab.label}
          </button>
        {/each}
      </div>
    </div>

    <div class="header-right">
      {#if clipboardSeconds !== null}
        <div class="clipboard-badge" title="Auto-clears clipboard memory">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" class="clip-icon">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
          </svg>
          <span>Clipboard: {clipboardSeconds}s</span>
          <button class="clip-clear-btn" on:click={() => dispatch('clearClipboard')}>Clear</button>
        </div>
      {/if}

      <button class="btn btn-secondary" on:click={() => dispatch('openLeakScanner')} title="Scan local repository for secret leaks">
        <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16">
          <path fill-rule="evenodd" d="M8 4a4 4 0 100 8 4 4 0 000-8zM2 8a6 6 0 1110.89 3.476l4.817 4.817a1 1 0 01-1.414 1.414l-4.816-4.816A6 6 0 012 8z" clip-rule="evenodd" />
        </svg>
        Leaks
      </button>

      <button class="btn btn-secondary" on:click={() => dispatch('openGenerator')} title="Cryptographic Password & Diceware Passphrase Generator">
        <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16">
          <path fill-rule="evenodd" d="M11.3 1.046A1 1 0 0112 2v5h4a1 1 0 01.82 1.573l-7 10A1 1 0 018 18v-5H4a1 1 0 01-.82-1.573l7-10a1 1 0 011.12-.38z" clip-rule="evenodd" />
        </svg>
        Generator
      </button>

      <button class="btn btn-secondary" on:click={() => dispatch('openDotenv')} title="Export vault secrets as .env or Kubernetes Secret YAML">
        <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16">
          <path fill-rule="evenodd" d="M3 17a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm3.293-7.707a1 1 0 011.414 0L9 10.586V3a1 1 0 112 0v7.586l1.293-1.293a1 1 0 111.414 1.414l-3 3a1 1 0 01-1.414 0l-3-3a1 1 0 010-1.414z" clip-rule="evenodd" />
        </svg>
        .env
      </button>

      <button class="btn btn-secondary" on:click={() => dispatch('openHealthDashboard')} title="View security risk score & posture">
        <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16">
          <path fill-rule="evenodd" d="M2.166 4.999A11.954 11.954 0 0010 1.944 11.954 11.954 0 0017.834 5c.11.65.166 1.32.166 2.001 0 5.225-3.34 9.67-8 11.317C5.34 16.67 2 12.225 2 7c0-.682.057-1.35-.166-2.001zm11.541 3.708a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
        </svg>
        Health
      </button>

      <button class="btn btn-secondary" on:click={() => dispatch('openBackupModal')} title="Export or restore encrypted .vaultbackup archive">
        <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16">
          <path d="M7 3a1 1 0 000 2h6a1 1 0 100-2H7zM4 7a1 1 0 011-1h10a1 1 0 110 2H5a1 1 0 01-1-1zM2 11a2 2 0 012-2h12a2 2 0 012 2v4a2 2 0 01-2 2H4a2 2 0 01-2-2v-4z" />
        </svg>
        Backup
      </button>

      <button class="btn btn-secondary" on:click={() => dispatch('openShamirModal')} title="Split or manage k-of-n Shamir key shares">
        <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16">
          <path fill-rule="evenodd" d="M10 2a1 1 0 011 1v1.323l3.954 1.582 1.599-.8a1 1 0 01.894 1.79l-1.233.616 1.738 5.42a1 1 0 01-.285 1.05A3.989 3.989 0 0115 15a3.989 3.989 0 01-2.667-1.019 1 1 0 01-.285-1.05l1.715-5.349L11 6.477V16h2a1 1 0 110 2H7a1 1 0 110-2h2V6.477L6.237 7.582l1.715 5.349a1 1 0 01-.285 1.05A3.989 3.989 0 015 15a3.989 3.989 0 01-2.667-1.019 1 1 0 01-.285-1.05l1.738-5.42-1.233-.617a1 1 0 01.894-1.788l1.599.799L9 4.323V3a1 1 0 011-1z" clip-rule="evenodd" />
        </svg>
        Shamir
      </button>

      <button
        class="btn btn-secondary {biometricEnrolled ? 'bio-enrolled' : ''}"
        on:click={() => dispatch('toggleBiometricEnrollment')}
        title={biometricEnrolled ? 'Biometric Unlock Enabled — Click to disable' : 'Enroll Windows Hello / Touch ID / OS Biometric Unlock'}
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="16" height="16">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 11c0 3.517-1.009 6.799-2.753 9.571m-3.44-2.04l.054-.09A13.916 13.916 0 008 11a4 4 0 118 0c0 1.017-.07 2.019-.203 3m-2.118 6.844A21.88 21.88 0 0015.171 17m3.839 1.132c.645-2.266.99-4.659.99-7.132A8 8 0 008 4.07M3 15.364c.64-1.319 1-2.8 1-4.364 0-1.457.39-2.823 1.07-4" />
        </svg>
        {biometricEnrolled ? 'Biometrics ON' : 'Biometrics'}
      </button>

      <button class="btn btn-primary" on:click={() => dispatch('openNewSecretModal')}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="16" height="16">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        New Secret
      </button>

      <button class="btn btn-danger" on:click={() => dispatch('lockVault')} title="Immediately lock vault and zeroize keys">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="16" height="16">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
        </svg>
        Lock
      </button>
    </div>
  {/if}
</header>

<style>
  .vault-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 18px;
    border-radius: 0;
    border-top: none;
    border-left: none;
    border-right: none;
    gap: 16px;
    z-index: 50;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 20px;
    flex-shrink: 0;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .brand-icon {
    width: 28px;
    height: 28px;
    color: var(--accent-emerald);
    filter: drop-shadow(0 0 8px rgba(16, 185, 129, 0.4));
  }

  .brand-info {
    display: flex;
    flex-direction: column;
  }

  .brand-title {
    font-size: 15px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: #ffffff;
  }

  .brand-tag {
    font-size: 10px;
    color: var(--accent-emerald);
    font-family: var(--font-mono);
  }

  .search-box {
    position: relative;
    display: flex;
    align-items: center;
    width: 280px;
  }

  .search-icon {
    position: absolute;
    left: 10px;
    width: 15px;
    height: 15px;
    color: var(--text-faint);
  }

  .search-box input {
    width: 100%;
    padding: 7px 28px 7px 32px;
    border-radius: 999px;
    font-size: 12px;
  }

  .clear-search {
    position: absolute;
    right: 8px;
    background: transparent;
    border: none;
    color: var(--text-faint);
    cursor: pointer;
    font-size: 12px;
  }

  .header-center {
    display: flex;
    align-items: center;
    overflow-x: auto;
  }

  .filter-pills {
    display: flex;
    gap: 6px;
    background: rgba(10, 14, 23, 0.6);
    padding: 4px;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
  }

  .pill {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    padding: 5px 12px;
    border-radius: 999px;
    cursor: pointer;
    transition: all 0.18s ease;
    white-space: nowrap;
  }

  .pill:hover {
    color: var(--text-main);
  }

  .pill.active {
    background: rgba(255, 255, 255, 0.12);
    color: #ffffff;
    font-weight: 600;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .clipboard-badge {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(245, 158, 11, 0.12);
    border: 1px solid rgba(245, 158, 11, 0.35);
    color: #fcd34d;
    padding: 4px 10px;
    border-radius: 999px;
    font-size: 11px;
    font-family: var(--font-mono);
    animation: pulseGlow 2s infinite ease-in-out;
  }

  .clip-icon {
    width: 13px;
    height: 13px;
  }

  .clip-clear-btn {
    background: transparent;
    border: none;
    color: #fcd34d;
    font-weight: 700;
    text-decoration: underline;
    cursor: pointer;
    font-size: 11px;
    margin-left: 2px;
  }

  .bio-enrolled {
    border-color: rgba(16, 185, 129, 0.45) !important;
    color: var(--accent-emerald, #10b981) !important;
    background: rgba(16, 185, 129, 0.1) !important;
  }

  @keyframes pulseGlow {
    0%, 100% {
      box-shadow: 0 0 0 rgba(245, 158, 11, 0);
    }
    50% {
      box-shadow: 0 0 10px rgba(245, 158, 11, 0.3);
    }
  }

  .tier-badge-btn {
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    margin-top: 2px;
  }
  .tier-badge {
    cursor: pointer;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 10px;
    text-transform: uppercase;
    transition: transform 0.15s ease, opacity 0.15s ease;
  }
  .tier-badge:hover {
    transform: scale(1.05);
    opacity: 0.9;
  }
  .tier-badge.tier-free {
    background: rgba(148, 163, 184, 0.2);
    color: #cbd5e1;
    border: 1px solid rgba(148, 163, 184, 0.3);
  }
  .tier-badge.tier-premium {
    background: rgba(59, 130, 246, 0.25);
    color: #93c5fd;
    border: 1px solid rgba(59, 130, 246, 0.45);
  }
  .tier-badge.tier-business {
    background: rgba(168, 85, 247, 0.25);
    color: #e9d5ff;
    border: 1px solid rgba(168, 85, 247, 0.45);
  }
</style>
