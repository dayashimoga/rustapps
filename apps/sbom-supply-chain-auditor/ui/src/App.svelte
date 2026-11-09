<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import ComplianceRadar from './components/ComplianceRadar.svelte';
  import ComponentGraphTable from './components/ComponentGraphTable.svelte';
  import LicenseModal from './components/LicenseModal.svelte';
  import CommandPalette, { type CommandAction } from './components/CommandPalette.svelte';

  interface AuditorStats {
    total_sboms_audited: number;
    total_components_analyzed: number;
    total_cves_flagged: number;
    total_copyleft_detected: number;
    avg_compliance_score: number;
  }

  interface SbomDocument {
    document_id: string;
    filename: string;
    format: string;
    spec_version: string;
    serial_number: string;
    timestamp: number;
    author: string;
    total_components: number;
    components: any[];
    ntia_minimum_elements_compliant: boolean;
    executive_order_14028_compliant: boolean;
  }

  let stats: AuditorStats = {
    total_sboms_audited: 0,
    total_components_analyzed: 0,
    total_cves_flagged: 0,
    total_copyleft_detected: 0,
    avg_compliance_score: 95.0,
  };

  let currentSbom: SbomDocument | null = null;
  let auditReport: any = null;
  let isLicenseModalOpen: boolean = false;
  let licenseTier: string = 'Community';
  let isCommandPaletteOpen: boolean = false;
  let filterQuery: string = '';

  $: commandActions = [
    {
      id: 're-audit-sbom',
      title: 'Run Offline Supply Chain Audit',
      subtitle: 'Scan components against local CVE database and evaluate copyleft risks',
      category: 'Audit',
      icon: '🛡️',
      shortcut: '⌘A',
      run: async () => {
        if (currentSbom) {
          auditReport = await invokeIpc('get_audit_report', { document_id: currentSbom.document_id });
        }
      },
    },
    {
      id: 'filter-critical-cves',
      title: 'Filter: Critical Vulnerabilities (CVE-2024-3094, etc.)',
      subtitle: 'Show components with active CVSS 9.0–10.0 exploits',
      category: 'Vulnerabilities',
      icon: '🚨',
      shortcut: 'C',
      run: () => {
        filterQuery = 'CVE';
      },
    },
    {
      id: 'filter-copyleft',
      title: 'Filter: Copyleft Licenses (GPL / AGPL)',
      subtitle: 'Show components requiring mandatory source disclosure',
      category: 'Compliance',
      icon: '⚖️',
      shortcut: 'G',
      run: () => {
        filterQuery = 'GPL';
      },
    },
    {
      id: 'clear-filter',
      title: 'Clear Filters',
      subtitle: 'Show all components in the inventory',
      category: 'Audit',
      icon: '🔄',
      shortcut: '0',
      run: () => {
        filterQuery = '';
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
      } else if (isLicenseModalOpen) {
        e.preventDefault();
        isLicenseModalOpen = false;
      }
    }
  }

  async function loadInitialData() {
    try {
      stats = await invokeIpc<AuditorStats>('get_stats');
      currentSbom = await invokeIpc<SbomDocument>('get_sbom', {
        document_id: 'doc-demo-cyclonedx',
      });
      auditReport = await invokeIpc<any>('get_audit_report', {
        document_id: 'doc-demo-cyclonedx',
      });
      const lic = await invokeIpc<any>('get_license_status');
      if (lic && lic.tier) {
        licenseTier = lic.tier;
      }
    } catch (e) {
      console.error('Failed to load initial SBOM auditor data:', e);
    }
  }

  onMount(() => {
    loadInitialData();
  });
</script>

<svelte:window on:keydown={handleGlobalKeyDown} />

<div class="sbom-auditor-app">
  <!-- Top Bar -->
  <header class="app-header">
    <div class="brand">
      <span class="brand-icon">📦</span>
      <div class="brand-text">
        <h1>AIR-GAPPED SBOM & SUPPLY-CHAIN AUDITOR</h1>
        <span class="subtitle">100% Offline CycloneDX/SPDX Ingestion, Copyleft Hazard Hunter & Executive Order 14028 Verification</span>
      </div>
    </div>

    <div class="header-stats">
      <button class="cmd-palette-btn" on:click={() => (isCommandPaletteOpen = true)} title="Commands (⌘K)">
        <span class="cmd-icon">⌘</span>
        <span>Commands</span>
        <kbd>⌘K</kbd>
      </button>

      <div class="stat-pill">
        <span class="stat-label">AUDITED:</span>
        <span class="stat-value">{stats.total_sboms_audited}</span>
      </div>
      <div class="stat-pill">
        <span class="stat-label">COMPONENTS:</span>
        <span class="stat-value">{stats.total_components_analyzed}</span>
      </div>
      <div class="stat-pill">
        <span class="stat-label">CVES:</span>
        <span class="stat-value red">{stats.total_cves_flagged}</span>
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

  <!-- Main Body -->
  <main class="app-body">
    {#if currentSbom}
      <!-- Upper Section: NTIA & EO 14028 Compliance Radar -->
      <section class="radar-section">
        <ComplianceRadar report={auditReport} />
      </section>

      <!-- Lower Section: Inventory & CVE Inspector Table -->
      <section class="table-section">
        <ComponentGraphTable
          components={currentSbom.components}
          bind:filterQuery
        />
      </section>
    {:else}
      <div class="loading-state">
        <span class="loading-spinner"></span>
        <span>Loading air-gapped SBOM supply chain engine...</span>
      </div>
    {/if}
  </main>

  <!-- Footer Status Bar -->
  <footer class="app-footer">
    <span class="status-item">
      FILE: {currentSbom ? currentSbom.filename : 'None'}
    </span>
    <span class="status-item">
      SPEC: {currentSbom ? `CycloneDX v${currentSbom.spec_version}` : 'CycloneDX 1.5'}
    </span>
    <span class="status-item">
      NTIA MINIMUM ELEMENTS: <strong class="pass-tag">{currentSbom?.ntia_minimum_elements_compliant ? 'VERIFIED PASSED' : 'NON-COMPLIANT'}</strong>
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

  <!-- Licensing Modal -->
  <LicenseModal
    isOpen={isLicenseModalOpen}
    appName="Air-Gapped SBOM & Supply-Chain Auditor"
    on:close={() => (isLicenseModalOpen = false)}
    on:statusChange={(e) => (licenseTier = e.detail.tier)}
  />
</div>

<style>
  .sbom-auditor-app {
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
    color: #f87171;
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
    background: rgba(239, 68, 68, 0.15);
    border-color: #f87171;
    color: #f8fafc;
  }

  .cmd-icon {
    color: #f87171;
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
    color: #38bdf8;
    font-weight: 600;
  }

  .stat-value.red {
    color: #ef4444;
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
    border: 1px solid #f8717166;
    color: #f87171;
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

  .radar-section {
    flex-shrink: 0;
  }

  .table-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
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

  .pass-tag {
    color: #10b981;
  }

  .status-item.security {
    margin-left: auto;
    color: #34d399;
    font-weight: 600;
  }

  .status-item.version {
    color: #f87171;
  }
</style>
