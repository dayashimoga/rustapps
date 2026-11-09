<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import type { X509Certificate, PkiStats, ShamirCeremony } from './lib/types';
  import CaHierarchyTree from './components/CaHierarchyTree.svelte';
  import CertificateDetailPane from './components/CertificateDetailPane.svelte';
  import SignCsrModal from './components/SignCsrModal.svelte';
  import ShamirCeremonyModal from './components/ShamirCeremonyModal.svelte';
  import CommandPalette from './components/CommandPalette.svelte';
  import LicenseModal from './components/LicenseModal.svelte';

  let stats: PkiStats | null = null;
  let certificates: X509Certificate[] = [];
  let selectedCertId: string = '';
  let selectedCertificate: X509Certificate | null = null;
  let shamirCeremonies: ShamirCeremony[] = [];
  let licenseStatus: any = null;

  let isCommandPaletteOpen = false;
  let isLicenseModalOpen = false;
  let isSignCsrModalOpen = false;
  let isShamirModalOpen = false;
  let isLoading = true;

  $: caCertificates = certificates.filter((c) => c.is_ca);

  onMount(async () => {
    await loadInitialData();
    window.addEventListener('keydown', handleGlobalKeyDown);
    return () => window.removeEventListener('keydown', handleGlobalKeyDown);
  });

  function handleGlobalKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      isCommandPaletteOpen = !isCommandPaletteOpen;
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
      e.preventDefault();
      isSignCsrModalOpen = true;
    }
  }

  async function loadInitialData() {
    isLoading = true;
    try {
      stats = await invokeIpc<PkiStats>('get_stats', {});
      certificates = await invokeIpc<X509Certificate[]>('get_hierarchy', {});
      shamirCeremonies = await invokeIpc<ShamirCeremony[]>('get_shamir_ceremonies', {});
      licenseStatus = await invokeIpc<any>('get_license_status', {});

      if (certificates.length > 0 && !selectedCertId) {
        await selectCertificate(certificates[0].certificate_id);
      }
    } catch (e) {
      console.error('Failed to load PKI studio state', e);
    } finally {
      isLoading = false;
    }
  }

  async function selectCertificate(certId: string) {
    selectedCertId = certId;
    try {
      selectedCertificate = await invokeIpc<X509Certificate>('get_certificate_detail', {
        certificate_id: certId,
      });
    } catch {
      selectedCertificate = certificates.find((c) => c.certificate_id === certId) || null;
    }
  }

  async function handleRevoke(certId: string) {
    try {
      await invokeIpc('revoke_certificate', {
        certificate_id: certId,
        reason: 'KeyCompromise',
      });
      await loadInitialData();
      await selectCertificate(certId);
    } catch (e) {
      console.error('Failed to revoke certificate', e);
    }
  }

  function handleCommand(cmdId: string) {
    if (cmdId === 'sign_csr') {
      isSignCsrModalOpen = true;
    } else if (cmdId === 'open_shamir') {
      isShamirModalOpen = true;
    } else if (cmdId === 'open_license') {
      isLicenseModalOpen = true;
    }
  }
</script>

<div class="pki-root">
  <!-- Top Application Bar -->
  <header class="app-header">
    <div class="brand">
      <span class="brand-badge">PKI</span>
      <span class="brand-title">Crypto PKI & CA Studio</span>
      <span class="air-gap-pill">100% AIR-GAPPED</span>
    </div>

    <!-- Header Actions -->
    <div class="header-actions">
      <button
        type="button"
        class="btn-primary"
        on:click={() => (isSignCsrModalOpen = true)}
      >
        <span class="icon">✍️</span>
        <span>Issue & Sign (CSR)</span>
      </button>

      <button
        type="button"
        class="btn-tool"
        on:click={() => (isShamirModalOpen = true)}
      >
        <span class="icon">🛡️</span>
        <span>Shamir Ceremony</span>
      </button>

      <button
        type="button"
        class="btn-tool"
        on:click={() => (isLicenseModalOpen = true)}
      >
        <span class="icon">🔑</span>
        <span>{licenseStatus?.tier ? licenseStatus.tier.split(' ')[0] : 'License'}</span>
      </button>

      <button
        type="button"
        class="btn-cmd"
        on:click={() => (isCommandPaletteOpen = true)}
        title="Command Palette (Ctrl+K)"
      >
        <kbd>Ctrl+K</kbd>
      </button>
    </div>
  </header>

  <!-- Metrics & Security Status Ribbon -->
  {#if stats}
    <div class="metrics-ribbon">
      <div class="meta-item">
        <span class="lbl">Certificate Authorities:</span>
        <span class="val highlight">{stats.total_cas} CAs</span>
      </div>
      <div class="meta-item">
        <span class="lbl">Total Issued:</span>
        <span class="val">{stats.total_issued_certificates}</span>
      </div>
      <div class="meta-item">
        <span class="lbl">Active Certificates:</span>
        <span class="val text-success">{stats.active_certificates}</span>
      </div>
      <div class="meta-item">
        <span class="lbl">Revoked:</span>
        <span class="val text-danger">{stats.revoked_certificates}</span>
      </div>
      <div class="meta-item">
        <span class="lbl">Active CRLs:</span>
        <span class="val">{stats.active_crls}</span>
      </div>
      <div class="meta-item">
        <span class="lbl">Shamir Ceremonies:</span>
        <span class="val text-shamir">{stats.shamir_ceremonies_count} M-of-N</span>
      </div>

      <div class="spacer"></div>

      <div class="cipher-pills">
        <span class="cipher-pill">Ed25519</span>
        <span class="cipher-pill">RSA 4096</span>
        <span class="cipher-pill">ECDSA P-256</span>
      </div>
    </div>
  {/if}

  <!-- Main Split Layout -->
  <main class="pki-body">
    {#if isLoading}
      <div class="loading-state">Initializing air-gapped PKI cryptographic hierarchy...</div>
    {:else}
      <div class="split-view">
        <div class="tree-pane">
          <CaHierarchyTree
            {certificates}
            {selectedCertId}
            onSelectCertificate={selectCertificate}
          />
        </div>
        <div class="detail-pane">
          <CertificateDetailPane
            certificate={selectedCertificate}
            onRevoke={handleRevoke}
          />
        </div>
      </div>
    {/if}
  </main>

  <!-- Modals -->
  <CommandPalette
    isOpen={isCommandPaletteOpen}
    onClose={() => (isCommandPaletteOpen = false)}
    onSelectCommand={handleCommand}
  />

  <LicenseModal
    isOpen={isLicenseModalOpen}
    onClose={() => (isLicenseModalOpen = false)}
    onLicenseUpdated={loadInitialData}
  />

  <SignCsrModal
    isOpen={isSignCsrModalOpen}
    cas={caCertificates}
    onClose={() => (isSignCsrModalOpen = false)}
    onCertIssued={loadInitialData}
  />

  <ShamirCeremonyModal
    isOpen={isShamirModalOpen}
    ceremony={shamirCeremonies[0] || null}
    onClose={() => (isShamirModalOpen = false)}
  />
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    background: #11111b;
    color: #cdd6f4;
    overflow: hidden;
    user-select: none;
  }
  .pki-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
  }
  .app-header {
    height: 48px;
    background: #181825;
    border-bottom: 1px solid #313244;
    display: flex;
    align-items: center;
    padding: 0 16px;
    gap: 16px;
    z-index: 100;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .brand-badge {
    background: #f9e2af;
    color: #11111b;
    font-weight: 900;
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: monospace;
  }
  .brand-title {
    font-size: 13px;
    font-weight: 700;
    color: #cdd6f4;
  }
  .air-gap-pill {
    background: #1e1e2e;
    color: #a6e3a1;
    font-size: 9px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 10px;
    border: 1px solid rgba(166, 227, 161, 0.4);
    letter-spacing: 0.5px;
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
  }
  .btn-primary {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #89b4fa;
    color: #11111b;
    border: none;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .btn-tool {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #1e1e2e;
    border: 1px solid #313244;
    color: #cdd6f4;
    padding: 6px 10px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
  }
  .btn-tool:hover {
    background: #313244;
  }
  .btn-cmd {
    background: transparent;
    border: none;
    cursor: pointer;
  }
  .btn-cmd kbd {
    background: #313244;
    color: #89b4fa;
    padding: 3px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-family: monospace;
  }
  .metrics-ribbon {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 6px 16px;
    background: #181825;
    border-bottom: 1px solid #313244;
    font-size: 11px;
  }
  .meta-item {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .meta-item .lbl {
    color: #6c7086;
  }
  .meta-item .val {
    color: #cdd6f4;
    font-weight: 500;
  }
  .highlight {
    color: #f9e2af !important;
    font-weight: 700 !important;
  }
  .text-success {
    color: #a6e3a1 !important;
  }
  .text-danger {
    color: #f38ba8 !important;
  }
  .text-shamir {
    color: #cba6f7 !important;
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  .cipher-pills {
    display: flex;
    gap: 6px;
  }
  .cipher-pill {
    background: #11111b;
    border: 1px solid #313244;
    color: #a6adc8;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-family: monospace;
  }
  .pki-body {
    flex: 1;
    overflow: hidden;
    display: flex;
  }
  .split-view {
    display: flex;
    width: 100%;
    height: 100%;
  }
  .tree-pane {
    width: 380px;
    height: 100%;
    overflow: hidden;
  }
  .detail-pane {
    flex: 1;
    height: 100%;
    overflow: hidden;
  }
  .loading-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #6c7086;
    font-size: 14px;
  }
</style>
