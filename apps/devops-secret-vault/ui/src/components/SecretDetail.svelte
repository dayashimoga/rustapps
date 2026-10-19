<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { DecryptedSecret, SecretAuditEntry } from '../types';

  export let secret: DecryptedSecret | null = null;
  export let auditLogs: SecretAuditEntry[] = [];
  export let isLoading: boolean = false;
  export let clipboardSeconds: number | null = null;

  let showSensitive = false;
  let activeTab: 'secret' | 'audit' = 'secret';

  const dispatch = createEventDispatcher();

  function copyField(fieldName: string) {
    if (!secret) return;
    dispatch('copySecret', { id: secret.metadata.id, field: fieldName });
  }

  function formatDate(timestampSecs: number): string {
    if (!timestampSecs) return 'N/A';
    return new Date(timestampSecs * 1000).toLocaleString();
  }
</script>

<div class="secret-detail-panel">
  {#if !secret}
    <div class="no-selection">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" class="select-icon">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 15l-2 5L9 9l11 4-5 2zm0 0l5 5M7.188 2.239l.777 2.897M5.136 7.965l-2.898-.777M13.95 4.05l-2.122 2.122m-5.657 5.656l-2.12 2.122" />
      </svg>
      <p class="select-title">No Secret Selected</p>
      <p class="select-desc">Select an item from the sidebar to inspect or decrypt its contents.</p>
    </div>
  {:else}
    <!-- Detail Header -->
    <div class="detail-header">
      <div class="header-main">
        <div class="title-row">
          <span class="type-pill">{secret.metadata.secret_type.replace('_', ' ').toUpperCase()}</span>
          <span class="version-tag">v{secret.metadata.version}</span>
          <h2 class="detail-title">{secret.metadata.title}</h2>
        </div>

        <div class="detail-meta">
          <span>Created: {formatDate(secret.metadata.created_at)}</span>
          <span>•</span>
          <span>Updated: {formatDate(secret.metadata.updated_at)}</span>
          {#if secret.metadata.url}
            <span>•</span>
            <a href={secret.metadata.url} target="_blank" rel="noopener noreferrer" class="link-url">
              {secret.metadata.url}
            </a>
          {/if}
        </div>
      </div>

      <div class="header-actions">
        <button
          class="btn"
          on:click={() => { showSensitive = !showSensitive; }}
          title={showSensitive ? 'Hide sensitive content' : 'Reveal sensitive content'}
        >
          {#if showSensitive}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="16" height="16">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l18 18" />
            </svg>
            Mask
          {:else}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="16" height="16">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
            </svg>
            Reveal
          {/if}
        </button>

        <button class="btn btn-primary" on:click={() => copyField('primary')}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="16" height="16">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
          </svg>
          Copy Secret {clipboardSeconds ? `(${clipboardSeconds}s)` : ''}
        </button>

        <button class="btn btn-danger btn-icon" on:click={() => dispatch('deleteSecret', secret?.metadata.id)} disabled={isLoading} title="Delete Secret">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="16" height="16">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Navigation Tabs -->
    <div class="detail-tabs">
      <button class="tab-btn {activeTab === 'secret' ? 'active' : ''}" on:click={() => { activeTab = 'secret'; }}>
        Secret Details
      </button>
      <button class="tab-btn {activeTab === 'audit' ? 'active' : ''}" on:click={() => { activeTab = 'audit'; }}>
        Audit Trail ({auditLogs.length})
      </button>
    </div>

    <!-- Tab Content -->
    <div class="detail-content">
      {#if activeTab === 'secret'}
        <div class="fields-grid">
          {#if secret.metadata.username}
            <div class="field-card">
              <span class="field-label">Username / Account</span>
              <div class="field-val-row">
                <span class="field-val mono">{secret.metadata.username}</span>
                <button class="mini-copy-btn" on:click={() => copyField('username')}>Copy</button>
              </div>
            </div>
          {/if}

          <!-- Type specific payloads -->
          {#if secret.payload.type === 'password'}
            <div class="field-card full-width">
              <span class="field-label">Encrypted Password</span>
              <div class="field-val-row">
                <span class="field-val mono password-val">
                  {showSensitive ? secret.payload.data.password : '••••••••••••••••••••••••••••••••'}
                </span>
                <button class="btn btn-primary" on:click={() => copyField('password')}>Copy Password</button>
              </div>
            </div>

          {:else if secret.payload.type === 'ssh_key'}
            <div class="field-card full-width">
              <span class="field-label">SSH Private Key</span>
              <pre class="code-block mono">{showSensitive ? secret.payload.data.private_key : '-----BEGIN OPENSSH PRIVATE KEY-----\n[HIDDEN - CLICK REVEAL TO VIEW]\n-----END OPENSSH PRIVATE KEY-----'}</pre>
              <button class="btn btn-primary mt-8" on:click={() => copyField('private_key')}>Copy Private Key</button>
            </div>
            {#if secret.payload.data.public_key}
              <div class="field-card full-width">
                <span class="field-label">SSH Public Key</span>
                <pre class="code-block mono">{secret.payload.data.public_key}</pre>
                <button class="mini-copy-btn mt-8" on:click={() => copyField('public_key')}>Copy Public Key</button>
              </div>
            {/if}

          {:else if secret.payload.type === 'certificate'}
            <div class="field-card full-width">
              <span class="field-label">Private Key</span>
              <pre class="code-block mono">{showSensitive ? secret.payload.data.private_key : '-----BEGIN RSA PRIVATE KEY-----\n[HIDDEN]\n-----END RSA PRIVATE KEY-----'}</pre>
            </div>
            <div class="field-card full-width">
              <span class="field-label">Certificate PEM</span>
              <pre class="code-block mono">{secret.payload.data.certificate_pem}</pre>
            </div>

          {:else if secret.payload.type === 'api_key'}
            <div class="field-card full-width">
              <span class="field-label">API Key</span>
              <div class="field-val-row">
                <span class="field-val mono">
                  {showSensitive ? secret.payload.data.api_key : '••••••••••••••••••••••••'}
                </span>
                <button class="btn btn-primary" on:click={() => copyField('api_key')}>Copy API Key</button>
              </div>
            </div>
            {#if secret.payload.data.api_secret}
              <div class="field-card full-width">
                <span class="field-label">API Secret</span>
                <div class="field-val-row">
                  <span class="field-val mono">
                    {showSensitive ? secret.payload.data.api_secret : '••••••••••••••••••••••••'}
                  </span>
                  <button class="mini-copy-btn" on:click={() => copyField('api_secret')}>Copy Secret</button>
                </div>
              </div>
            {/if}

          {:else if secret.payload.type === 'database_credential'}
            <div class="field-card">
              <span class="field-label">Engine</span>
              <span class="field-val mono">{secret.payload.data.engine}</span>
            </div>
            <div class="field-card">
              <span class="field-label">Host & Port</span>
              <span class="field-val mono">{secret.payload.data.host}:{secret.payload.data.port}</span>
            </div>
            <div class="field-card">
              <span class="field-label">Database</span>
              <span class="field-val mono">{secret.payload.data.database}</span>
            </div>
            <div class="field-card">
              <span class="field-label">Database Password</span>
              <div class="field-val-row">
                <span class="field-val mono">
                  {showSensitive ? secret.payload.data.password : '••••••••••••••••'}
                </span>
                <button class="mini-copy-btn" on:click={() => copyField('password')}>Copy</button>
              </div>
            </div>
            {#if secret.payload.data.connection_string}
              <div class="field-card full-width">
                <span class="field-label">Connection String (URI)</span>
                <div class="field-val-row">
                  <span class="field-val mono">
                    {showSensitive ? secret.payload.data.connection_string : 'postgresql://user:••••••••@host:5432/db'}
                  </span>
                  <button class="btn btn-primary" on:click={() => copyField('connection_string')}>Copy URI</button>
                </div>
              </div>
            {/if}
          {/if}

          {#if secret.metadata.notes}
            <div class="field-card full-width">
              <span class="field-label">Notes & Description</span>
              <p class="notes-text">{secret.metadata.notes}</p>
            </div>
          {/if}
        </div>

      {:else}
        <!-- Audit Trail Tab -->
        <div class="audit-list">
          {#if auditLogs.length === 0}
            <p class="no-audit">No audit entries recorded.</p>
          {:else}
            {#each auditLogs as log (log.id)}
              <div class="audit-entry">
                <div class="audit-left">
                  <span class="audit-action badge badge-{log.action}">{log.action.toUpperCase()}</span>
                  <span class="audit-client">{log.client}</span>
                </div>
                <span class="audit-time">{formatDate(log.timestamp)}</span>
              </div>
            {/each}
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .secret-detail-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: rgba(10, 14, 23, 0.2);
    overflow: hidden;
  }

  .no-selection {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    height: 100%;
    padding: 40px;
    gap: 12px;
  }

  .select-icon {
    width: 56px;
    height: 56px;
    color: var(--text-faint);
    opacity: 0.4;
  }

  .select-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .select-desc {
    font-size: 13px;
    color: var(--text-faint);
    max-width: 300px;
  }

  .detail-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border-subtle);
    gap: 16px;
  }

  .header-main {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .type-pill {
    font-size: 10px;
    font-weight: 700;
    color: var(--accent-emerald);
    background: rgba(16, 185, 129, 0.12);
    padding: 2px 8px;
    border-radius: 4px;
    font-family: var(--font-mono);
  }

  .version-tag {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--text-faint);
    background: rgba(255, 255, 255, 0.05);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .detail-title {
    font-size: 20px;
    font-weight: 700;
    color: #ffffff;
  }

  .detail-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--text-faint);
    font-family: var(--font-mono);
  }

  .link-url {
    color: var(--accent-cyan);
    text-decoration: none;
  }

  .link-url:hover {
    text-decoration: underline;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .detail-tabs {
    display: flex;
    border-bottom: 1px solid var(--border-subtle);
    padding: 0 24px;
    gap: 16px;
    background: rgba(10, 14, 23, 0.4);
  }

  .tab-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 13px;
    font-weight: 600;
    padding: 12px 4px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: all 0.18s ease;
  }

  .tab-btn:hover {
    color: var(--text-main);
  }

  .tab-btn.active {
    color: var(--accent-emerald);
    border-bottom-color: var(--accent-emerald);
  }

  .detail-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }

  .fields-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 16px;
  }

  .field-card {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .field-card.full-width {
    grid-column: span 2;
  }

  .field-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .field-val-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .field-val {
    font-size: 13px;
    color: var(--text-main);
    word-break: break-all;
  }

  .mono {
    font-family: var(--font-mono);
  }

  .password-val {
    font-size: 14px;
    letter-spacing: 0.1em;
  }

  .code-block {
    background: rgba(0, 0, 0, 0.4);
    padding: 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    font-size: 12px;
    max-height: 180px;
    overflow-y: auto;
    color: #cbd5e1;
  }

  .mini-copy-btn {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 4px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .mini-copy-btn:hover {
    background: rgba(255, 255, 255, 0.15);
    color: #ffffff;
  }

  .mt-8 {
    margin-top: 8px;
  }

  .notes-text {
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.6;
    white-space: pre-wrap;
  }

  .audit-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .audit-entry {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
  }

  .audit-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .audit-client {
    font-size: 12px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .audit-time {
    font-size: 11px;
    color: var(--text-faint);
    font-family: var(--font-mono);
  }

  .no-audit {
    font-size: 13px;
    color: var(--text-faint);
  }
</style>
