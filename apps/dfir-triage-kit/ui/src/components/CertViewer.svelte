<script lang="ts">
  export interface CertArtifact {
    subject: string;
    issuer: string;
    serial_number: string;
    sha256_fingerprint: string;
    not_before: number;
    not_after: number;
    is_expired: boolean;
    is_self_signed: boolean;
  }

  export let certificates: CertArtifact[] = [];

  let search = '';

  function formatDate(ts: number): string {
    return new Date(ts * 1000).toLocaleDateString();
  }

  $: filteredCerts = certificates.filter((c) => {
    if (!search) return true;
    const q = search.toLowerCase();
    return (
      c.subject.toLowerCase().includes(q) ||
      c.issuer.toLowerCase().includes(q) ||
      c.serial_number.toLowerCase().includes(q)
    );
  });
</script>

<div class="cert-container">
  <div class="controls-bar">
    <div class="search-box">
      <span class="icon">🔏</span>
      <input
        type="text"
        bind:value={search}
        placeholder="Filter by subject, issuer, serial..."
      />
      {#if search}
        <button class="clear-btn" on:click={() => (search = '')}>✕</button>
      {/if}
    </div>
    <span class="count-tag">Showing {filteredCerts.length} certificates</span>
  </div>

  <div class="cert-grid">
    {#if filteredCerts.length === 0}
      <div class="empty-certs">No certificates found.</div>
    {:else}
      {#each filteredCerts as cert}
        <div class="cert-card" class:expired={cert.is_expired}>
          <div class="cert-header">
            <span class="subject" title={cert.subject}>{cert.subject}</span>
            <div class="badges">
              {#if cert.is_self_signed}
                <span class="badge self-signed">SELF-SIGNED</span>
              {/if}
              {#if cert.is_expired}
                <span class="badge expired">EXPIRED</span>
              {:else}
                <span class="badge valid">VALID</span>
              {/if}
            </div>
          </div>

          <div class="cert-meta">
            <div class="meta-row">
              <span class="label">Issuer</span>
              <span class="val" title={cert.issuer}>{cert.issuer}</span>
            </div>
            <div class="meta-row">
              <span class="label">Serial</span>
              <span class="val mono">{cert.serial_number}</span>
            </div>
            <div class="meta-row">
              <span class="label">Fingerprint (SHA-256)</span>
              <span class="val mono">{cert.sha256_fingerprint.slice(0, 24)}...</span>
            </div>
            <div class="meta-row">
              <span class="label">Validity Window</span>
              <span class="val dates">{formatDate(cert.not_before)} → {formatDate(cert.not_after)}</span>
            </div>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .cert-container {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .controls-bar {
    padding: 12px 16px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
  }

  .search-box {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 10px;
    width: 360px;
    max-width: 100%;
  }

  .search-box input {
    background: transparent;
    border: none;
    color: var(--text-main);
    font-size: 0.85rem;
    width: 100%;
    outline: none;
  }

  .clear-btn {
    background: transparent;
    border: none;
    color: var(--text-sub);
    font-size: 0.8rem;
  }

  .count-tag {
    font-size: 0.78rem;
    color: var(--text-sub);
  }

  .cert-grid {
    padding: 16px;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 14px;
    max-height: calc(100vh - 360px);
    overflow-y: auto;
  }

  .cert-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .cert-card.expired {
    border-color: rgba(239, 68, 68, 0.4);
    background: rgba(239, 68, 68, 0.05);
  }

  .cert-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 10px;
  }

  .subject {
    font-weight: 700;
    font-size: 0.88rem;
    color: var(--text-main);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .badges {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .badge {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.68rem;
    font-weight: 700;
  }

  .badge.valid {
    background: rgba(16, 185, 129, 0.15);
    color: var(--accent-green);
  }

  .badge.self-signed {
    background: rgba(6, 182, 212, 0.15);
    color: var(--accent-cyan);
  }

  .badge.expired {
    background: rgba(239, 68, 68, 0.2);
    color: var(--accent-red);
  }

  .cert-meta {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .meta-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.78rem;
    gap: 8px;
  }

  .label {
    color: var(--text-sub);
    flex-shrink: 0;
  }

  .val {
    color: var(--text-muted);
    text-align: right;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mono {
    font-family: var(--font-mono);
  }

  .empty-certs {
    padding: 30px;
    text-align: center;
    color: var(--text-sub);
    grid-column: 1 / -1;
  }
</style>
