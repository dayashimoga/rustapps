<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { SecretMetadata, SecretType } from '../types';

  export let secrets: SecretMetadata[] = [];
  export let selectedSecretId: string | null = null;
  export let filterType: SecretType | 'all' = 'all';

  const dispatch = createEventDispatcher();

  $: filteredSecrets = secrets.filter(s => {
    if (filterType === 'all') return true;
    return s.secret_type === filterType;
  });

  function getTypeBadgeClass(type: SecretType): string {
    switch (type) {
      case 'password': return 'badge-password';
      case 'ssh_key': return 'badge-ssh';
      case 'certificate': return 'badge-cert';
      case 'api_key': return 'badge-api';
      case 'database_credential': return 'badge-db';
      default: return '';
    }
  }

  function getTypeLabel(type: SecretType): string {
    switch (type) {
      case 'password': return 'Password';
      case 'ssh_key': return 'SSH Key';
      case 'certificate': return 'Certificate';
      case 'api_key': return 'API Key';
      case 'database_credential': return 'Database';
      default: return type;
    }
  }

  function formatDate(timestampSecs: number): string {
    if (!timestampSecs) return '';
    const date = new Date(timestampSecs * 1000);
    return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  }
</script>

<div class="secret-list-panel">
  <div class="list-header">
    <span class="count-label">{filteredSecrets.length} Secrets</span>
    <span class="sort-label">Sorted by Recently Updated</span>
  </div>

  <div class="list-container">
    {#if filteredSecrets.length === 0}
      <div class="empty-state">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" class="empty-icon">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
        </svg>
        <p class="empty-title">No secrets found</p>
        <p class="empty-desc">Create your first encrypted secret to get started.</p>
        <button class="btn btn-primary" on:click={() => dispatch('openNewSecret')}>
          + Add Secret
        </button>
      </div>
    {:else}
      {#each filteredSecrets as secret (secret.id)}
        <div
          class="secret-card {selectedSecretId === secret.id ? 'selected' : ''}"
          role="button"
          tabindex="0"
          on:click={() => dispatch('selectSecret', secret.id)}
          on:keydown={e => { if (e.key === 'Enter' || e.key === ' ') dispatch('selectSecret', secret.id); }}
        >
          <div class="card-top">
            <span class="badge {getTypeBadgeClass(secret.secret_type)}">
              {getTypeLabel(secret.secret_type)}
            </span>
            <span class="card-date">{formatDate(secret.updated_at)}</span>
          </div>

          <div class="card-title-row">
            <h3 class="card-title">{secret.title}</h3>
          </div>

          {#if secret.username || secret.url}
            <div class="card-subtitle">
              {#if secret.username}
                <span class="sub-user">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="12" height="12">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                  </svg>
                  {secret.username}
                </span>
              {/if}
              {#if secret.url}
                <span class="sub-url">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="12" height="12">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
                  </svg>
                  {secret.url}
                </span>
              {/if}
            </div>
          {/if}

          {#if secret.tags && secret.tags.length > 0}
            <div class="card-tags">
              {#each secret.tags as tag}
                <span class="tag-chip">#{tag}</span>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .secret-list-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    border-right: 1px solid var(--border-subtle);
    background: rgba(10, 14, 23, 0.4);
    overflow: hidden;
  }

  .list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 18px;
    border-bottom: 1px solid var(--border-subtle);
    font-size: 11px;
    color: var(--text-faint);
    text-transform: uppercase;
    font-weight: 600;
    letter-spacing: 0.05em;
  }

  .list-container {
    flex: 1;
    overflow-y: auto;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .secret-card {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 12px 14px;
    cursor: pointer;
    transition: all 0.18s ease;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .secret-card:hover {
    background: var(--bg-card-hover);
    border-color: rgba(255, 255, 255, 0.16);
    transform: translateY(-1px);
  }

  .secret-card.selected {
    border-color: var(--accent-emerald);
    background: rgba(16, 185, 129, 0.06);
    box-shadow: 0 0 16px rgba(16, 185, 129, 0.15);
  }

  .card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .card-date {
    font-size: 10px;
    color: var(--text-faint);
    font-family: var(--font-mono);
  }

  .card-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .card-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-main);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-subtitle {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 11px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub-user, .sub-url {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .card-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 2px;
  }

  .tag-chip {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--accent-cyan);
    background: rgba(6, 182, 212, 0.08);
    padding: 1px 6px;
    border-radius: 4px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    height: 100%;
    padding: 30px;
    gap: 10px;
  }

  .empty-icon {
    width: 48px;
    height: 48px;
    color: var(--text-faint);
    opacity: 0.5;
  }

  .empty-title {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .empty-desc {
    font-size: 12px;
    color: var(--text-faint);
    max-width: 200px;
    margin-bottom: 8px;
  }
</style>
