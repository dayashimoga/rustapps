<script lang="ts">
  import type { X509Certificate } from '../lib/types';

  export let certificates: X509Certificate[] = [];
  export let selectedCertId: string = '';
  export let onSelectCertificate: (certId: string) => void = () => {};

  $: rootCas = certificates.filter((c) => c.cert_type === 'root_ca');

  function getSubordinates(issuerId: string): X509Certificate[] {
    return certificates.filter((c) => c.issuer_ca_id === issuerId);
  }

  function getCertTypeBadge(type: string, isRevoked: boolean) {
    if (isRevoked) return { label: 'REVOKED', class: 'badge-revoked' };
    switch (type) {
      case 'root_ca':
        return { label: 'ROOT CA', class: 'badge-root' };
      case 'intermediate_ca':
        return { label: 'ISSUING CA', class: 'badge-sub' };
      case 'tls_server':
        return { label: 'TLS SERVER', class: 'badge-server' };
      case 'tls_client':
        return { label: 'TLS CLIENT', class: 'badge-client' };
      case 'code_signing':
        return { label: 'CODE SIGN', class: 'badge-code' };
      default:
        return { label: 'CERT', class: 'badge-default' };
    }
  }
</script>

<div class="tree-container">
  <div class="tree-header">
    <span>X.509 Trust Hierarchy ({certificates.length} nodes)</span>
  </div>

  <div class="tree-content">
    {#each rootCas as root}
      {@const badge = getCertTypeBadge(root.cert_type, root.is_revoked)}
      <div class="tree-branch">
        <!-- Root CA Node -->
        <button
          type="button"
          class="node-card level-root"
          class:selected={root.certificate_id === selectedCertId}
          on:click={() => onSelectCertificate(root.certificate_id)}
        >
          <span class="node-icon">🏛️</span>
          <div class="node-info">
            <div class="node-title">{root.subject.common_name}</div>
            <div class="node-meta">Serial: {root.serial_number_hex} • Ed25519</div>
          </div>
          <span class="badge {badge.class}">{badge.label}</span>
        </button>

        <!-- Subordinate / Intermediate CAs -->
        <div class="children-branch">
          {#each getSubordinates(root.certificate_id) as sub}
            {@const subBadge = getCertTypeBadge(sub.cert_type, sub.is_revoked)}
            <div class="sub-branch">
              <button
                type="button"
                class="node-card level-sub"
                class:selected={sub.certificate_id === selectedCertId}
                on:click={() => onSelectCertificate(sub.certificate_id)}
              >
                <span class="node-icon">🔐</span>
                <div class="node-info">
                  <div class="node-title">{sub.subject.common_name}</div>
                  <div class="node-meta">Serial: {sub.serial_number_hex}</div>
                </div>
                <span class="badge {subBadge.class}">{subBadge.label}</span>
              </button>

              <!-- End-Entity Certificates -->
              <div class="leaf-branch">
                {#each getSubordinates(sub.certificate_id) as leaf}
                  {@const leafBadge = getCertTypeBadge(leaf.cert_type, leaf.is_revoked)}
                  <button
                    type="button"
                    class="node-card level-leaf"
                    class:selected={leaf.certificate_id === selectedCertId}
                    class:is-revoked={leaf.is_revoked}
                    on:click={() => onSelectCertificate(leaf.certificate_id)}
                  >
                    <span class="node-icon">{leaf.is_revoked ? '🚫' : '📜'}</span>
                    <div class="node-info">
                      <div class="node-title">{leaf.subject.common_name}</div>
                      <div class="node-meta">
                        {leaf.subject_alt_names.length > 0 ? leaf.subject_alt_names[0] : `SN: ${leaf.serial_number_hex}`}
                      </div>
                    </div>
                    <span class="badge {leafBadge.class}">{leafBadge.label}</span>
                  </button>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {:else}
      <div class="empty-tree">No Root CAs initialized</div>
    {/each}
  </div>
</div>

<style>
  .tree-container {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: #181825;
    border-right: 1px solid #313244;
  }
  .tree-header {
    padding: 10px 16px;
    background: #11111b;
    border-bottom: 1px solid #313244;
    font-size: 12px;
    font-weight: 600;
    color: #cdd6f4;
  }
  .tree-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .tree-branch {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .children-branch {
    margin-left: 20px;
    padding-left: 12px;
    border-left: 2px dashed #313244;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .sub-branch {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .leaf-branch {
    margin-left: 20px;
    padding-left: 12px;
    border-left: 2px dashed #313244;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .node-card {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-radius: 8px;
    background: #1e1e2e;
    border: 1px solid #313244;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;
  }
  .node-card:hover {
    border-color: #89b4fa;
  }
  .node-card.selected {
    border-color: #89b4fa;
    background: #252739;
    box-shadow: 0 0 10px rgba(137, 180, 250, 0.15);
  }
  .level-root {
    border-left: 4px solid #f9e2af;
  }
  .level-sub {
    border-left: 4px solid #cba6f7;
  }
  .level-leaf {
    border-left: 4px solid #89dceb;
  }
  .node-card.is-revoked {
    border-left: 4px solid #f38ba8;
    opacity: 0.75;
  }
  .node-icon {
    font-size: 16px;
  }
  .node-info {
    flex: 1;
    overflow: hidden;
  }
  .node-title {
    font-size: 12px;
    font-weight: 600;
    color: #cdd6f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .node-meta {
    font-size: 10px;
    color: #6c7086;
    font-family: monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .badge {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 9px;
    font-weight: 700;
    font-family: monospace;
    white-space: nowrap;
  }
  .badge-root {
    background: rgba(249, 226, 175, 0.2);
    color: #f9e2af;
    border: 1px solid rgba(249, 226, 175, 0.4);
  }
  .badge-sub {
    background: rgba(203, 166, 247, 0.2);
    color: #cba6f7;
    border: 1px solid rgba(203, 166, 247, 0.4);
  }
  .badge-server {
    background: rgba(137, 220, 235, 0.2);
    color: #89dceb;
    border: 1px solid rgba(137, 220, 235, 0.4);
  }
  .badge-client {
    background: rgba(137, 180, 250, 0.2);
    color: #89b4fa;
    border: 1px solid rgba(137, 180, 250, 0.4);
  }
  .badge-code {
    background: rgba(166, 227, 161, 0.2);
    color: #a6e3a1;
    border: 1px solid rgba(166, 227, 161, 0.4);
  }
  .badge-revoked {
    background: rgba(243, 139, 168, 0.2);
    color: #f38ba8;
    border: 1px solid rgba(243, 139, 168, 0.4);
  }
  .badge-default {
    background: rgba(108, 112, 134, 0.2);
    color: #a6adc8;
  }
  .empty-tree {
    padding: 30px;
    text-align: center;
    color: #6c7086;
  }
</style>
