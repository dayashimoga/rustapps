<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import type {
    LicenseStatusReport,
    MediaInspectionReport,
    MediaSummary,
    PrivacyAuditCertificate,
    SanitizedResult,
    SanitizerStats,
  } from './lib/types';

  import CommandPalette from './components/CommandPalette.svelte';
  import LicenseModal from './components/LicenseModal.svelte';
  import UploadMediaModal from './components/UploadMediaModal.svelte';
  import PrivacyCertificateModal from './components/PrivacyCertificateModal.svelte';
  import ForensicInspectionPane from './components/ForensicInspectionPane.svelte';
  import SanitizedComparisonPane from './components/SanitizedComparisonPane.svelte';

  let stats: SanitizerStats = {
    total_files_sanitized: 0,
    total_tags_purged: 0,
    gps_leaks_prevented: 0,
    device_fingerprints_destroyed: 0,
  };

  let mediaFiles: MediaSummary[] = [];
  let currentReport: MediaInspectionReport | null = null;
  let currentResult: SanitizedResult | null = null;
  let license: LicenseStatusReport = {
    tier: 'Community',
    is_valid: true,
    features: ['basic_exif_scrub'],
  };

  let activeMode: 'inspection' | 'sanitized' = 'inspection';
  let isSanitizing = false;

  // Modals state
  let isPaletteOpen = false;
  let isLicenseOpen = false;
  let isUploadOpen = false;
  let isCertModalOpen = false;
  let generatedCert: PrivacyAuditCertificate | null = null;

  async function loadData() {
    try {
      const [s, mList, lic] = await Promise.all([
        invokeIpc<SanitizerStats>('get_stats'),
        invokeIpc<MediaSummary[]>('list_media'),
        invokeIpc<LicenseStatusReport>('get_license_status'),
      ]);
      stats = s;
      mediaFiles = mList;
      license = lic;

      if (mList.length > 0 && !currentReport) {
        await selectFile(mList[0].file_id);
      }
    } catch (e) {
      console.error('Failed to load initial sanitizer data:', e);
    }
  }

  async function selectFile(fileId: string) {
    try {
      const [rep, res] = await Promise.all([
        invokeIpc<MediaInspectionReport>('get_media_report', { file_id: fileId }),
        invokeIpc<SanitizedResult | null>('get_sanitized_result', { file_id: fileId }),
      ]);
      currentReport = rep;
      currentResult = res;
      if (res && activeMode === 'sanitized') {
        // stay in sanitized
      } else {
        activeMode = 'inspection';
      }
    } catch (e) {
      console.error('Failed to load media report:', e);
    }
  }

  async function sanitizeActiveFile() {
    if (!currentReport) return;
    isSanitizing = true;
    try {
      const res = await invokeIpc<SanitizedResult>('sanitize_file', {
        file_id: currentReport.file_id,
      });
      currentResult = res;
      activeMode = 'sanitized';
      await loadData();
    } catch (e: any) {
      alert(`Sanitization failed: ${e.message || e}`);
    } finally {
      isSanitizing = false;
    }
  }

  async function issueCertificate() {
    if (!currentReport) return;
    try {
      const cert = await invokeIpc<PrivacyAuditCertificate>('generate_privacy_certificate', {
        file_id: currentReport.file_id,
      });
      generatedCert = cert;
      isCertModalOpen = true;
    } catch (e: any) {
      alert(`Certificate issue failed: ${e.message || e}`);
    }
  }

  function handleCommand(cmdId: string) {
    switch (cmdId) {
      case 'upload_media':
        isUploadOpen = true;
        break;
      case 'sanitize_active':
        sanitizeActiveFile();
        break;
      case 'sign_certificate':
        issueCertificate();
        break;
      case 'open_license':
        isLicenseOpen = true;
        break;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      isPaletteOpen = !isPaletteOpen;
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
      e.preventDefault();
      isUploadOpen = true;
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'p') {
      e.preventDefault();
      sanitizeActiveFile();
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      issueCertificate();
    }
  }

  onMount(() => {
    loadData();
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  });
</script>

<div class="app-shell">
  <!-- Top Application Bar -->
  <header class="app-header">
    <div class="brand">
      <div class="logo-mark">🔏</div>
      <div class="brand-titles">
        <h1 class="app-title">Privacy Media Sanitizer</h1>
        <span class="tagline">Forensic Air-Gapped EXIF, GPS & Document Metadata Scrubber</span>
      </div>
    </div>

    <!-- Live Executive Metrics Counters -->
    <div class="metrics-bar">
      <div class="metric-item">
        <span class="m-val">{stats.total_files_sanitized}</span>
        <span class="m-lbl">Clean Files</span>
      </div>
      <div class="metric-divider"></div>
      <div class="metric-item">
        <span class="m-val">{stats.total_tags_purged}</span>
        <span class="m-lbl">Tags Purged</span>
      </div>
      <div class="metric-divider"></div>
      <div class="metric-item crit">
        <span class="m-val">{stats.gps_leaks_prevented}</span>
        <span class="m-lbl">GPS Leaks Defeated</span>
      </div>
      <div class="metric-divider"></div>
      <div class="metric-item safe">
        <span class="m-val">{stats.device_fingerprints_destroyed}</span>
        <span class="m-lbl">Device IDs Eradicated</span>
      </div>
    </div>

    <!-- Status badges and tools -->
    <div class="header-tools">
      <span class="airgap-badge" title="Zero Telemetry & 100% Offline Autonomy">
        <span class="dot"></span> Offline Air-Gapped
      </span>

      <button type="button" class="license-pill" on:click={() => (isLicenseOpen = true)}>
        <span class="lic-icon">🔑</span>
        <span>{license.tier}</span>
      </button>

      <button
        type="button"
        class="palette-btn"
        on:click={() => (isPaletteOpen = true)}
        title="Open Command Palette (Ctrl+K)"
      >
        <span>Ctrl+K</span>
      </button>
    </div>
  </header>

  <!-- Sub-header Ribbon -->
  <div class="action-ribbon">
    <div class="ribbon-left">
      <div class="mode-toggles">
        <button
          type="button"
          class="mode-btn"
          class:active={activeMode === 'inspection'}
          on:click={() => (activeMode = 'inspection')}
        >
          🔍 Metadata Inspection
        </button>
        <button
          type="button"
          class="mode-btn"
          class:active={activeMode === 'sanitized'}
          on:click={() => (activeMode = 'sanitized')}
        >
          🛡️ Zero-Trace Comparison
        </button>
      </div>

      <div class="file-dropdown-wrap">
        <span class="sel-lbl">Target File:</span>
        <select
          value={currentReport?.file_id || ''}
          on:change={(e) => selectFile(e.currentTarget.value)}
        >
          {#each mediaFiles as m}
            <option value={m.file_id}>
              {m.filename} ({m.media_type.toUpperCase()}) — {m.is_sanitized ? 'CLEAN' : 'DIRTY'}
            </option>
          {/each}
        </select>
      </div>
    </div>

    <div class="ribbon-actions">
      <button type="button" class="btn action-btn primary" on:click={() => (isUploadOpen = true)}>
        <span class="btn-icon">➕</span> Inspect New Media
      </button>
      <button
        type="button"
        class="btn action-btn sanitize"
        on:click={sanitizeActiveFile}
        disabled={isSanitizing}
      >
        <span class="btn-icon">⚡</span> Purge Metadata
      </button>
      {#if currentResult}
        <button type="button" class="btn action-btn cert" on:click={issueCertificate}>
          <span class="btn-icon">🔏</span> Issue Certificate
        </button>
      {/if}
    </div>
  </div>

  <!-- Main Workspace Pane -->
  <main class="main-workspace">
    <!-- Left Files Sidebar -->
    <aside class="media-sidebar">
      <div class="sidebar-hdr">REGISTERED MEDIA REPOSITORY</div>
      <div class="media-list">
        {#each mediaFiles as m}
          <div
            class="media-item"
            class:selected={currentReport?.file_id === m.file_id}
            on:click={() => selectFile(m.file_id)}
            on:keydown={(e) => e.key === 'Enter' && selectFile(m.file_id)}
            role="button"
            tabindex="0"
          >
            <div class="item-top">
              <span class="item-name font-semibold">{m.filename}</span>
              <span class="clean-badge" class:sanitized={m.is_sanitized}>
                {m.is_sanitized ? 'CLEAN' : 'DIRTY'}
              </span>
            </div>
            <div class="item-bottom">
              <span class="item-specs">
                {m.media_type.toUpperCase()} • {(m.size_bytes / 1024).toFixed(1)} KB
              </span>
              {#if m.has_gps}
                <span class="gps-badge" title="GPS Coordinates Embedded">📍 GPS</span>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </aside>

    <!-- Center/Right Active Mode Pane -->
    <section class="content-workspace">
      {#if activeMode === 'inspection'}
        <ForensicInspectionPane
          report={currentReport}
          onSanitize={sanitizeActiveFile}
          {isSanitizing}
        />
      {:else}
        <SanitizedComparisonPane
          report={currentReport}
          result={currentResult}
          onSignCertificate={issueCertificate}
        />
      {/if}
    </section>
  </main>

  <!-- Modals -->
  <CommandPalette
    isOpen={isPaletteOpen}
    onClose={() => (isPaletteOpen = false)}
    onSelectCommand={handleCommand}
  />

  <LicenseModal
    isOpen={isLicenseOpen}
    onClose={() => (isLicenseOpen = false)}
    onLicenseUpdated={loadData}
  />

  <UploadMediaModal
    isOpen={isUploadOpen}
    onClose={() => (isUploadOpen = false)}
    onMediaInspected={async (rep) => {
      await loadData();
      await selectFile(rep.file_id);
    }}
  />

  <PrivacyCertificateModal
    isOpen={isCertModalOpen}
    certificate={generatedCert}
    onClose={() => (isCertModalOpen = false)}
  />
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: #030712;
    color: #f3f4f6;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
    overflow: hidden;
    user-select: none;
  }

  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background: #030712;
  }

  .app-header {
    height: 56px;
    background: #090e1a;
    border-bottom: 1px solid #1a2338;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 18px;
    z-index: 10;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .logo-mark {
    font-size: 1.5rem;
  }

  .brand-titles {
    display: flex;
    flex-direction: column;
  }

  .app-title {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 700;
    color: #f8fafc;
    letter-spacing: -0.01em;
  }

  .tagline {
    font-size: 0.72rem;
    color: #64748b;
    font-weight: 500;
  }

  .metrics-bar {
    display: flex;
    align-items: center;
    gap: 16px;
    background: #060a12;
    padding: 6px 16px;
    border-radius: 8px;
    border: 1px solid #151f33;
  }

  .metric-item {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .m-val {
    font-size: 1rem;
    font-weight: 800;
    color: #38bdf8;
    line-height: 1.1;
  }

  .metric-item.crit .m-val { color: #ef4444; }
  .metric-item.safe .m-val { color: #10b981; }

  .m-lbl {
    font-size: 0.65rem;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .metric-divider {
    width: 1px;
    height: 22px;
    background: #1e293b;
  }

  .header-tools {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .airgap-badge {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.3);
    color: #34d399;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 4px 10px;
    border-radius: 9999px;
  }

  .airgap-badge .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #10b981;
    box-shadow: 0 0 6px #10b981;
  }

  .license-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #132338;
    border: 1px solid #1e3a8a;
    color: #60a5fa;
    font-size: 0.76rem;
    font-weight: 600;
    padding: 4px 10px;
    border-radius: 6px;
    cursor: pointer;
  }

  .palette-btn {
    background: #1e293b;
    border: 1px solid #334155;
    color: #94a3b8;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.75rem;
    font-family: monospace;
    cursor: pointer;
  }

  .action-ribbon {
    height: 48px;
    background: #060a12;
    border-bottom: 1px solid #151f33;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 18px;
  }

  .ribbon-left {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .mode-toggles {
    display: flex;
    background: #0d121f;
    border: 1px solid #1f293d;
    border-radius: 6px;
    padding: 2px;
  }

  .mode-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 0.8rem;
    font-weight: 600;
    padding: 5px 12px;
    border-radius: 4px;
    cursor: pointer;
  }

  .mode-btn.active {
    background: #1e293b;
    color: #f1f5f9;
  }

  .file-dropdown-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .sel-lbl {
    font-size: 0.78rem;
    color: #94a3b8;
    font-weight: 500;
  }

  select {
    background: #0d121f;
    border: 1px solid #1f293d;
    color: #f1f5f9;
    font-size: 0.82rem;
    padding: 4px 10px;
    border-radius: 5px;
    outline: none;
    min-width: 260px;
  }

  .ribbon-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid #1f293d;
    background: #0d121f;
    color: #cbd5e1;
    transition: background 0.15s ease;
  }

  .action-btn:hover {
    background: #1e293b;
  }

  .action-btn.primary {
    background: #2563eb;
    border-color: #3b82f6;
    color: #fff;
  }

  .action-btn.primary:hover {
    background: #1d4ed8;
  }

  .action-btn.sanitize {
    background: #059669;
    border-color: #10b981;
    color: #fff;
  }

  .action-btn.sanitize:hover:not(:disabled) {
    background: #047857;
  }

  .action-btn.cert {
    background: rgba(109, 40, 217, 0.2);
    border-color: #6d28d9;
    color: #c4b5fd;
  }

  .action-btn.cert:hover {
    background: rgba(109, 40, 217, 0.35);
  }

  .main-workspace {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .media-sidebar {
    width: 280px;
    min-width: 240px;
    background: #070b14;
    border-right: 1px solid #151f33;
    display: flex;
    flex-direction: column;
  }

  .sidebar-hdr {
    padding: 10px 14px;
    font-size: 0.72rem;
    font-weight: 700;
    color: #64748b;
    border-bottom: 1px solid #151f33;
    letter-spacing: 0.05em;
  }

  .media-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .media-item {
    background: #0a0f1d;
    border: 1px solid #172033;
    border-radius: 6px;
    padding: 10px 12px;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .media-item:hover {
    background: #101726;
  }

  .media-item.selected {
    background: #131c30;
    border-color: #2563eb;
  }

  .item-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .item-name {
    font-size: 0.85rem;
    color: #f1f5f9;
    max-width: 170px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .clean-badge {
    font-size: 0.65rem;
    font-weight: 800;
    padding: 1px 6px;
    border-radius: 3px;
    background: rgba(239, 68, 68, 0.2);
    color: #f87171;
  }

  .clean-badge.sanitized {
    background: rgba(16, 185, 129, 0.2);
    color: #34d399;
  }

  .item-bottom {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .item-specs {
    font-size: 0.72rem;
    color: #64748b;
  }

  .gps-badge {
    background: rgba(245, 158, 11, 0.15);
    color: #f59e0b;
    font-size: 0.65rem;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: 3px;
  }

  .content-workspace {
    flex: 1;
    background: #030712;
    overflow: hidden;
  }

  .font-semibold { font-weight: 600; }
</style>
