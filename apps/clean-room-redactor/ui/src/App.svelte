<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import DocumentViewer from './components/DocumentViewer.svelte';
  import PiiFindingsBar from './components/PiiFindingsBar.svelte';
  import CertificateModal from './components/CertificateModal.svelte';
  import LicenseModal from './components/LicenseModal.svelte';
  import CommandPalette, { type CommandAction } from './components/CommandPalette.svelte';

  interface RedactionStats {
    total_documents_sanitized: number;
    total_pages_sanitized: number;
    total_pii_entities_burned: number;
    total_certificates_issued: number;
    compliance_score: number;
  }

  interface DocumentPage {
    page_index: usize;
    width_pts: number;
    height_pts: number;
    text_content: string;
    pii_findings: any[];
    redaction_masks: any[];
    thumbnail_svg: string;
  }

  interface RedactionDocument {
    document_id: string;
    filename: string;
    file_size_bytes: number;
    total_pages: number;
    original_sha256: string;
    original_blake3: string;
    has_embedded_metadata: boolean;
    metadata_keys: string[];
    pages: DocumentPage[];
  }

  let stats: RedactionStats = {
    total_documents_sanitized: 0,
    total_pages_sanitized: 0,
    total_pii_entities_burned: 0,
    total_certificates_issued: 0,
    compliance_score: 99.8,
  };

  let currentDocument: RedactionDocument | null = null;
  let activePageIndex: number = 0;
  let isLicenseModalOpen: boolean = false;
  let licenseTier: string = 'Community';
  let isCommandPaletteOpen: boolean = false;
  let isCertModalOpen: boolean = false;
  let latestCertificate: any = null;
  let latestCertSvg: string = '';
  let isProcessing: boolean = false;

  // Bates Numbering Settings
  let batesPrefix: string = 'LIT-PROD-';
  let batesStartNumber: number = 1;
  let batesDigits: number = 6;
  let batesPosition: string = 'bottom_right';

  $: activePagePii = currentDocument?.pages[activePageIndex]?.pii_findings || [];

  $: commandActions = [
    {
      id: 'execute-burnout',
      title: 'Execute Clean-Room Burnout & Generate Certificate',
      subtitle: 'Permanently excise approved PII/PHI, apply Bates stamps, and sign Ed25519 audit manifest',
      category: 'Redaction',
      icon: '🛡️',
      shortcut: '⌘B',
      run: () => {
        executeSanitization();
      },
    },
    {
      id: 'load-demo-doc',
      title: 'Load Federal Settlement Agreement (Demo)',
      subtitle: '3-page confidential settlement agreement with SSNs, Credit Cards, and HIPAA MRNs',
      category: 'Redaction',
      icon: '📄',
      shortcut: 'D',
      run: async () => {
        await loadDocumentById('doc-demo-settlement');
      },
    },
    {
      id: 'view-latest-cert',
      title: 'View Cryptographic Sanitization Certificate',
      subtitle: latestCertificate ? `ID: ${latestCertificate.certificate_id}` : 'No certificate issued yet',
      category: 'Security',
      icon: '📜',
      shortcut: 'C',
      run: () => {
        if (latestCertificate) {
          isCertModalOpen = true;
        }
      },
    },
    {
      id: 'toggle-bates-top',
      title: 'Set Bates Number Position: Top Right',
      subtitle: 'Standard header position for FOIA releases',
      category: 'Bates',
      icon: '🔢',
      shortcut: 'T',
      run: () => {
        batesPosition = 'top_right';
      },
    },
    {
      id: 'toggle-bates-bottom',
      title: 'Set Bates Number Position: Bottom Right',
      subtitle: 'Standard litigation production footer position',
      category: 'Bates',
      icon: '🔢',
      shortcut: 'B',
      run: () => {
        batesPosition = 'bottom_right';
      },
    },
    {
      id: 'manage-license',
      title: 'Manage Offline Ed25519 License',
      subtitle: `Current tier: ${licenseTier.toUpperCase()}`,
      category: 'System',
      icon: '🔑',
      shortcut: 'L',
      run: () => {
        isLicenseModalOpen = true;
      },
    },
  ] as CommandAction[];

  function handleGlobalKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      isCommandPaletteOpen = !isCommandPaletteOpen;
    } else if (e.key === 'Escape') {
      if (isCommandPaletteOpen) {
        e.preventDefault();
        isCommandPaletteOpen = false;
      } else if (isCertModalOpen) {
        e.preventDefault();
        isCertModalOpen = false;
      } else if (isLicenseModalOpen) {
        e.preventDefault();
        isLicenseModalOpen = false;
      }
    }
  }

  async function loadInitialData() {
    try {
      stats = await invokeIpc<RedactionStats>('get_stats');
      await loadDocumentById('doc-demo-settlement');
      const lic = await invokeIpc<any>('get_license_status');
      if (lic && lic.tier) {
        licenseTier = lic.tier;
      }
    } catch (e) {
      console.error('Failed to load initial redactor data:', e);
    }
  }

  async function loadDocumentById(docId: string) {
    try {
      currentDocument = await invokeIpc<RedactionDocument>('get_document', {
        document_id: docId,
      });
      activePageIndex = 0;
    } catch (err) {
      console.error('Failed to load document:', err);
    }
  }

  async function handleAddMask(e: CustomEvent) {
    if (!currentDocument) return;
    try {
      await invokeIpc('add_redaction', {
        document_id: currentDocument.document_id,
        page_index: e.detail.pageIndex,
        label: e.detail.label,
        burn_style: e.detail.burnStyle,
        exemption_code: e.detail.exemptionCode,
        bbox: e.detail.bbox,
      });
      await loadDocumentById(currentDocument.document_id);
    } catch (err) {
      console.error('Failed to add manual redaction mask:', err);
    }
  }

  async function handleRemoveMask(e: CustomEvent) {
    if (!currentDocument) return;
    try {
      await invokeIpc('remove_redaction', {
        document_id: currentDocument.document_id,
        page_index: e.detail.pageIndex,
        mask_id: e.detail.maskId,
      });
      await loadDocumentById(currentDocument.document_id);
    } catch (err) {
      console.error('Failed to remove mask:', err);
    }
  }

  async function handleToggleApproval(e: CustomEvent) {
    if (!currentDocument) return;
    try {
      await invokeIpc('toggle_pii_approval', {
        document_id: currentDocument.document_id,
        page_index: e.detail.pageIndex,
        finding_id: e.detail.findingId,
        approved: e.detail.approved,
      });
      await loadDocumentById(currentDocument.document_id);
    } catch (err) {
      console.error('Failed to toggle approval:', err);
    }
  }

  async function handleBulkApprove(e: CustomEvent) {
    if (!currentDocument) return;
    const page = currentDocument.pages[e.detail.pageIndex];
    if (!page) return;

    for (const f of page.pii_findings) {
      await invokeIpc('toggle_pii_approval', {
        document_id: currentDocument.document_id,
        page_index: e.detail.pageIndex,
        finding_id: f.id,
        approved: e.detail.approved,
      }).catch(() => {});
    }
    await loadDocumentById(currentDocument.document_id);
  }

  async function executeSanitization() {
    if (!currentDocument) return;
    isProcessing = true;
    try {
      const cert = await invokeIpc<any>('apply_clean_room_redaction', {
        document_id: currentDocument.document_id,
        bates_config: {
          prefix: batesPrefix,
          suffix: '',
          start_number: batesStartNumber,
          digits: batesDigits,
          position: batesPosition,
          font_size: 10,
          include_date: false,
        },
      });

      latestCertificate = cert;

      const svgRes = await invokeIpc<{ svg: string }>('get_certificate_svg', cert);
      latestCertSvg = svgRes.svg;

      stats = await invokeIpc<RedactionStats>('get_stats');
      await loadDocumentById(currentDocument.document_id);
      isCertModalOpen = true;
    } catch (err) {
      console.error('Sanitization failed:', err);
    } finally {
      isProcessing = false;
    }
  }

  onMount(() => {
    loadInitialData();
  });
</script>

<svelte:window on:keydown={handleGlobalKeyDown} />

<div class="clean-room-app">
  <!-- Top Bar -->
  <header class="app-header">
    <div class="brand">
      <span class="brand-icon">🛡️</span>
      <div class="brand-text">
        <h1>OFFLINE CLEAN-ROOM DOCUMENT REDACTOR</h1>
        <span class="subtitle">100% Offline Deep Stream Burnout, PII/PHI Detection & Ed25519 Cryptographic Certificates</span>
      </div>
    </div>

    <div class="header-stats">
      <button class="cmd-palette-btn" on:click={() => (isCommandPaletteOpen = true)} title="Commands (⌘K)">
        <span class="cmd-icon">⌘</span>
        <span>Commands</span>
        <kbd>⌘K</kbd>
      </button>

      {#if latestCertificate}
        <button class="cert-btn" on:click={() => (isCertModalOpen = true)}>
          📜 View Certificate
        </button>
      {/if}

      <div class="stat-pill">
        <span class="stat-label">SANITIZED:</span>
        <span class="stat-value">{stats.total_documents_sanitized} docs</span>
      </div>
      <div class="stat-pill">
        <span class="stat-label">BURNED:</span>
        <span class="stat-value">{stats.total_pii_entities_burned} entities</span>
      </div>
      <div class="stat-pill air-gap">
        <span class="dot"></span>
        <span>AIR-GAPPED</span>
      </div>
      <button class="license-btn" on:click={() => (isLicenseModalOpen = true)}>
        TIER: {licenseTier.toUpperCase()}
      </button>
    </div>
  </header>

  <!-- Main Document Workspace -->
  <main class="app-body">
    {#if currentDocument}
      <!-- Upper Section: Document Viewer & Manual Redactions -->
      <section class="viewer-section">
        <DocumentViewer
          pages={currentDocument.pages}
          bind:activePageIndex
          on:addMask={handleAddMask}
          on:removeMask={handleRemoveMask}
        />
      </section>

      <!-- Lower Section: AI PII Findings Bar & Bates Config -->
      <section class="bottom-tools-section">
        <div class="pii-panel-wrap">
          <PiiFindingsBar
            piiFindings={activePagePii}
            pageIndex={activePageIndex}
            on:toggleApproval={handleToggleApproval}
            on:bulkApprove={handleBulkApprove}
          />
        </div>

        <div class="bates-panel">
          <div class="bates-header">
            <span class="bates-title">BATES STAMPING & PRODUCTION CONFIG</span>
          </div>
          <div class="bates-inputs">
            <div class="bates-field">
              <label for="bates-prefix">Prefix:</label>
              <input id="bates-prefix" type="text" bind:value={batesPrefix} placeholder="LIT-CONF-" />
            </div>
            <div class="bates-field">
              <label for="bates-start">Start #:</label>
              <input id="bates-start" type="number" bind:value={batesStartNumber} min="1" />
            </div>
            <div class="bates-field">
              <label for="bates-pos">Position:</label>
              <select id="bates-pos" bind:value={batesPosition}>
                <option value="bottom_right">Bottom Right (Standard)</option>
                <option value="bottom_center">Bottom Center</option>
                <option value="bottom_left">Bottom Left</option>
                <option value="top_right">Top Right (FOIA)</option>
                <option value="top_center">Top Center</option>
                <option value="top_left">Top Left</option>
              </select>
            </div>
          </div>

          <button
            class="execute-btn"
            disabled={isProcessing}
            on:click={executeSanitization}
          >
            {isProcessing ? 'Burning Streams & Signing Certificate...' : 'Execute Clean-Room Burnout & Sign Certificate'}
          </button>
        </div>
      </section>
    {:else}
      <div class="loading-state">
        <span class="loading-spinner"></span>
        <span>Loading clean-room document workspace...</span>
      </div>
    {/if}
  </main>

  <!-- Footer Status Bar -->
  <footer class="app-footer">
    <span class="status-item">
      FILE: {currentDocument ? currentDocument.filename : 'None'}
    </span>
    <span class="status-item">
      PAGES: {currentDocument ? currentDocument.total_pages : 0}
    </span>
    <span class="status-item">
      METADATA STRIP: <strong class="meta-status">{currentDocument?.has_embedded_metadata ? 'PENDING BURNOUT' : 'STRIPPED CLEAN'}</strong>
    </span>
    <span class="status-item security">
      100% AIR-GAPPED: ZERO NETWORK EGRESS ASSURED
    </span>
    <span class="status-item version">v{0.1}</span>
  </footer>

  <!-- Command Palette -->
  <CommandPalette
    isOpen={isCommandPaletteOpen}
    actions={commandActions}
    onClose={() => (isCommandPaletteOpen = false)}
  />

  <!-- Certificate Modal -->
  <CertificateModal
    isOpen={isCertModalOpen}
    certificate={latestCertificate}
    certSvg={latestCertSvg}
    on:close={() => (isCertModalOpen = false)}
  />

  <!-- Licensing Modal -->
  <LicenseModal
    isOpen={isLicenseModalOpen}
    appName="Offline Clean-Room Document Redactor"
    on:close={() => (isLicenseModalOpen = false)}
    on:statusChange={(e) => (licenseTier = e.detail.tier)}
  />
</div>

<style>
  .clean-room-app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #090d14;
    color: #f0f6fc;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
  }

  .app-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 18px;
    background: #0f141d;
    border-bottom: 1px solid #1a2332;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .brand-icon {
    font-size: 22px;
  }

  .brand-text h1 {
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.8px;
    color: #10b981;
    font-family: monospace;
    margin: 0;
  }

  .subtitle {
    font-size: 10px;
    color: #8b949e;
  }

  .header-stats {
    display: flex;
    align-items: center;
    gap: 10px;
    font-family: monospace;
    font-size: 11px;
  }

  .cmd-palette-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: #161e2b;
    border: 1px solid #232d3d;
    color: #cbd5e1;
    font-size: 0.76rem;
    font-weight: 500;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .cmd-palette-btn:hover {
    background: rgba(16, 185, 129, 0.15);
    border-color: #10b981;
    color: #f8fafc;
  }

  .cmd-icon {
    color: #10b981;
    font-weight: 700;
  }

  .cmd-palette-btn kbd {
    background: #090d14;
    border: 1px solid #28374d;
    padding: 1px 4px;
    border-radius: 3px;
    font-size: 0.65rem;
    color: #94a3b8;
  }

  .cert-btn {
    background: #14221c;
    border: 1px solid #10b98166;
    color: #34d399;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 11px;
    font-family: monospace;
    cursor: pointer;
    font-weight: 600;
  }

  .cert-btn:hover {
    background: #1b3329;
    border-color: #10b981;
  }

  .stat-pill {
    display: flex;
    gap: 6px;
    background: #161e2b;
    border: 1px solid #232d3d;
    padding: 4px 8px;
    border-radius: 4px;
  }

  .stat-label {
    color: #8b949e;
  }

  .stat-value {
    color: #10b981;
    font-weight: 600;
  }

  .air-gap {
    background: #10b98115;
    border-color: #10b98144;
    color: #34d399;
    font-weight: 600;
    align-items: center;
  }

  .dot {
    width: 6px;
    height: 6px;
    background: #34d399;
    border-radius: 50%;
  }

  .license-btn {
    background: #192230;
    border: 1px solid #10b98166;
    color: #34d399;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 700;
    font-family: monospace;
    cursor: pointer;
  }

  .license-btn:hover {
    background: #233145;
  }

  .app-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    padding: 8px 10px;
    gap: 8px;
  }

  .viewer-section {
    flex: 1.4;
    display: flex;
    overflow: hidden;
    min-height: 300px;
  }

  .bottom-tools-section {
    flex: 0.85;
    display: flex;
    gap: 8px;
    min-height: 180px;
  }

  .pii-panel-wrap {
    flex: 1.5;
    display: flex;
    flex-direction: column;
  }

  .bates-panel {
    flex: 1;
    background: #0f141d;
    border: 1px solid #1a2332;
    border-radius: 8px;
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    font-family: monospace;
  }

  .bates-header {
    border-bottom: 1px solid #1a2332;
    padding-bottom: 6px;
  }

  .bates-title {
    font-size: 0.72rem;
    font-weight: 700;
    color: #fbbf24;
  }

  .bates-inputs {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 8px 0;
  }

  .bates-field {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.75rem;
  }

  .bates-field label {
    width: 65px;
    color: #94a3b8;
  }

  .bates-field input,
  .bates-field select {
    flex: 1;
    background: #090d14;
    border: 1px solid #28374d;
    color: #f1f5f9;
    padding: 4px 8px;
    border-radius: 4px;
    font-family: inherit;
    font-size: 0.72rem;
    outline: none;
  }

  .execute-btn {
    background: #059669;
    border: 1px solid #10b981;
    color: #ffffff;
    font-weight: 700;
    font-size: 0.78rem;
    font-family: monospace;
    padding: 8px 12px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .execute-btn:hover:not(:disabled) {
    background: #10b981;
    box-shadow: 0 0 16px rgba(16, 185, 129, 0.4);
  }

  .execute-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .loading-state {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
    color: #8b949e;
    font-family: monospace;
    gap: 12px;
  }

  .app-footer {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 4px 16px;
    background: #0b0f17;
    border-top: 1px solid #161e2b;
    font-size: 10px;
    font-family: monospace;
    color: #8b949e;
  }

  .meta-status {
    color: #10b981;
  }

  .status-item.security {
    margin-left: auto;
    color: #34d399;
    font-weight: 600;
  }

  .status-item.version {
    color: #10b981;
  }
</style>
