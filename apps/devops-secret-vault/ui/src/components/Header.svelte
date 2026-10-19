<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { SecretType } from '../types';

  export let searchQuery: string = '';
  export let selectedType: SecretType | 'all' = 'all';
  export let clipboardSeconds: number | null = null;
  export let isLocked: boolean = false;

  const dispatch = createEventDispatcher();

  const filterTabs: Array<{ id: SecretType | 'all'; label: string }> = [
    { id: 'all', label: 'All Items' },
    { id: 'password', label: 'Passwords' },
    { id: 'ssh_key', label: 'SSH Keys' },
    { id: 'certificate', label: 'Certificates' },
    { id: 'api_key', label: 'API Keys' },
    { id: 'database_credential', label: 'Databases' },
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

  @keyframes pulseGlow {
    0%, 100% {
      box-shadow: 0 0 0 rgba(245, 158, 11, 0);
    }
    50% {
      box-shadow: 0 0 10px rgba(245, 158, 11, 0.3);
    }
  }
</style>
