<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import LicenseModal, { type LicenseStatusReport } from './components/LicenseModal.svelte';
  import TriageSummaryCard, { type TriageSummary } from './components/TriageSummaryCard.svelte';
  import ProcessViewer, { type ProcessArtifact } from './components/ProcessViewer.svelte';
  import SocketViewer, { type SocketArtifact } from './components/SocketViewer.svelte';
  import FileIntegrityViewer, { type FileIntegrityRecord } from './components/FileIntegrityViewer.svelte';
  import EventLogTimeline, { type LogEventRecord } from './components/EventLogTimeline.svelte';
  import CertViewer, { type CertArtifact } from './components/CertViewer.svelte';
  import CommandPalette, { type CommandAction } from './components/CommandPalette.svelte';

  interface TriagePackage {
    summary: TriageSummary;
    processes: ProcessArtifact[];
    sockets: SocketArtifact[];
    suspicious_files: FileIntegrityRecord[];
    event_logs: LogEventRecord[];
    certificates: CertArtifact[];
  }

  interface DfirStats {
    total_triage_runs: number;
    total_threats_flagged: number;
    total_files_hashed: number;
  }

  type ActiveTab = 'overview' | 'processes' | 'sockets' | 'files' | 'logs' | 'certs';

  let currentTab: ActiveTab = 'overview';
  let activePackage: TriagePackage | null = null;
  let triageRuns: TriageSummary[] = [];
  let stats: DfirStats = {
    total_triage_runs: 0,
    total_threats_flagged: 0,
    total_files_hashed: 0,
  };

  let isLicenseModalOpen = false;
  let isScanning = false;
  let errorMessage = '';

  let licenseTier: 'Free' | 'Premium' | 'Business' = 'Free';
  let isLicenseActive = false;

  let isCommandPaletteOpen = false;

  $: commandActions = [
    {
      id: 'run-sweep',
      title: 'Run Live Host Triage Sweep',
      subtitle: 'Enumerate processes, network sockets, suspicious entropy & hashes',
      category: 'Forensics',
      icon: '⚡',
      shortcut: '⌘R',
      run: () => executeTriage(),
    },
    {
      id: 'tab-overview',
      title: 'Overview & Threat Score',
      subtitle: 'View overall threat posture and anomaly counts',
      category: 'Navigation',
      icon: '📊',
      shortcut: '1',
      run: () => { currentTab = 'overview'; },
    },
    {
      id: 'tab-procs',
      title: 'Running Processes & Memory',
      subtitle: 'Inspect loaded modules, memory RSS, PID, and parent lineage',
      category: 'Navigation',
      icon: '⚙️',
      shortcut: '2',
      run: () => { currentTab = 'processes'; },
    },
    {
      id: 'tab-sockets',
      title: 'Active Network Sockets',
      subtitle: 'Inspect local/remote IP, ports, listening states, and owning PID',
      category: 'Navigation',
      icon: '🌐',
      shortcut: '3',
      run: () => { currentTab = 'sockets'; },
    },
    {
      id: 'tab-files',
      title: 'File Integrity & Shannon Entropy',
      subtitle: 'Detect packed binaries, high entropy (>7.2), and hash anomalies',
      category: 'Navigation',
      icon: '📁',
      shortcut: '4',
      run: () => { currentTab = 'files'; },
    },
    {
      id: 'tab-logs',
      title: 'Event Log Timeline',
      subtitle: 'Correlated security events, auth failures, and audit entries',
      category: 'Navigation',
      icon: '📜',
      shortcut: '5',
      run: () => { currentTab = 'logs'; },
    },
    {
      id: 'tab-certs',
      title: 'Root Certificate Authority Store',
      subtitle: 'Verify installed trust roots and untrusted certificates',
      category: 'Navigation',
      icon: '🔒',
      shortcut: '6',
      run: () => { currentTab = 'certs'; },
    },
    {
      id: 'manage-license',
      title: 'Manage Offline Ed25519 License',
      subtitle: `Current tier: ${licenseTier.toUpperCase()}`,
      category: 'System',
      icon: '🔑',
      shortcut: 'L',
      run: () => { isLicenseModalOpen = true; },
    },
  ] as CommandAction[];

  function handleGlobalKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      isCommandPaletteOpen = !isCommandPaletteOpen;
    } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'r') {
      e.preventDefault();
      executeTriage();
    } else if (e.key === 'Escape' && isCommandPaletteOpen) {
      e.preventDefault();
      isCommandPaletteOpen = false;
    }
  }

  onMount(async () => {
    await refreshData();
    await checkLicense();
  });

  async function checkLicense() {
    try {
      const rep = await invokeIpc<LicenseStatusReport>('get_license_status');
      if (rep) {
        licenseTier = rep.tier;
        isLicenseActive = rep.is_active;
      }
    } catch (e) {
      console.error('License status check error:', e);
    }
  }

  async function refreshData() {
    try {
      const runs = await invokeIpc<TriageSummary[]>('list_triage_runs');
      triageRuns = runs;

      if (runs.length > 0) {
        const latestId = runs[0].triage_id;
        await loadTriagePackage(latestId);
      } else {
        await executeTriage();
      }

      const st = await invokeIpc<DfirStats>('get_dfir_stats');
      if (st) {
        stats = st;
      }
    } catch (e: any) {
      errorMessage = e?.message || 'Failed to initialize DFIR data';
    }
  }

  async function loadTriagePackage(id: string) {
    try {
      const pkg = await invokeIpc<TriagePackage>('get_triage_package', { triage_id: id });
      activePackage = pkg;
    } catch (e: any) {
      errorMessage = e?.message || `Failed to load package ${id}`;
    }
  }

  async function executeTriage() {
    isScanning = true;
    errorMessage = '';
    try {
      const pkg = await invokeIpc<TriagePackage>('execute_triage', {});
      activePackage = pkg;
      const runs = await invokeIpc<TriageSummary[]>('list_triage_runs');
      triageRuns = runs;
      const st = await invokeIpc<DfirStats>('get_dfir_stats');
      if (st) stats = st;
    } catch (e: any) {
      errorMessage = e?.message || 'Triage sweep execution failed';
    } finally {
      isScanning = false;
    }
  }

  function handleRunSelection(e: Event) {
    const target = e.target as HTMLSelectElement;
    if (target.value) {
      loadTriagePackage(target.value);
    }
  }
</script>

<svelte:window on:keydown={handleGlobalKeyDown} />

<div class="app-layout">
  <!-- Top App Bar -->
  <header class="app-header">
    <div class="header-branding">
      <div class="brand-logo">🛡️</div>
      <div class="brand-text">
        <h1 class="brand-title">Offline DFIR Triage Kit</h1>
        <span class="brand-sub">Host Forensics, Memory Anomalies & Incident Response</span>
      </div>
      <div class="airgap-badge">
        <span class="pulse-dot"></span>
        <span>100% OFFLINE AIR-GAPPED</span>
      </div>
    </div>

    <div class="header-actions">
      <button class="cmd-palette-btn" on:click={() => (isCommandPaletteOpen = true)} title="Commands (⌘K)">
        <span class="cmd-icon">⌘</span>
        <span>Commands</span>
        <kbd>⌘K</kbd>
      </button>

      {#if triageRuns.length > 0}
        <div class="run-selector-wrap">
          <label for="run-select">Run:</label>
          <select
            id="run-select"
            class="run-select"
            value={activePackage?.summary.triage_id || ''}
            on:change={handleRunSelection}
          >
            {#each triageRuns as run}
              <option value={run.triage_id}>
                {new Date(run.collected_at * 1000).toLocaleTimeString()} - {run.triage_id.slice(0, 15)}... (Score: {run.threat_score})
              </option>
            {/each}
          </select>
        </div>
      {/if}

      <button
        class="license-badge-btn"
        class:licensed={isLicenseActive}
        on:click={() => (isLicenseModalOpen = true)}
      >
        <span>🔑</span>
        <span>{licenseTier.toUpperCase()}</span>
      </button>
    </div>
  </header>

  <!-- Error Banner -->
  {#if errorMessage}
    <div class="global-error">
      <span>⚠️ {errorMessage}</span>
      <button class="dismiss-btn" on:click={() => (errorMessage = '')}>✕</button>
    </div>
  {/if}

  <!-- Navigation Tabs -->
  <nav class="nav-bar">
    <button
      class="nav-tab"
      class:active={currentTab === 'overview'}
      on:click={() => (currentTab = 'overview')}
    >
      <span class="tab-icon">📊</span>
      <span>Overview & Score</span>
    </button>
    <button
      class="nav-tab"
      class:active={currentTab === 'processes'}
      on:click={() => (currentTab = 'processes')}
    >
      <span class="tab-icon">⚙️</span>
      <span>Processes ({activePackage?.processes.length || 0})</span>
    </button>
    <button
      class="nav-tab"
      class:active={currentTab === 'sockets'}
      on:click={() => (currentTab = 'sockets')}
    >
      <span class="tab-icon">🌐</span>
      <span>Network Sockets ({activePackage?.sockets.length || 0})</span>
    </button>
    <button
      class="nav-tab"
      class:active={currentTab === 'files'}
      on:click={() => (currentTab = 'files')}
    >
      <span class="tab-icon">📁</span>
      <span>File Integrity & Entropy ({activePackage?.suspicious_files.length || 0})</span>
    </button>
    <button
      class="nav-tab"
      class:active={currentTab === 'logs'}
      on:click={() => (currentTab = 'logs')}
    >
      <span class="tab-icon">📜</span>
      <span>Event Logs ({activePackage?.event_logs.length || 0})</span>
    </button>
    <button
      class="nav-tab"
      class:active={currentTab === 'certs'}
      on:click={() => (currentTab = 'certs')}
    >
      <span class="tab-icon">🔏</span>
      <span>Certificates ({activePackage?.certificates.length || 0})</span>
    </button>
  </nav>

  <!-- Main View Area -->
  <main class="main-content">
    {#if currentTab === 'overview'}
      <div class="overview-view">
        <TriageSummaryCard
          summary={activePackage?.summary || null}
          {isScanning}
          on:runTriage={executeTriage}
        />

        <div class="summary-details-grid">
          <div class="detail-card">
            <h3>Investigation Quick Insights</h3>
            <ul class="insights-list">
              <li>
                <strong>Target Host:</strong>
                <span>{activePackage?.summary.host_name || 'LOCAL-SYSTEM'} ({activePackage?.summary.os_type || 'Windows'})</span>
              </li>
              <li>
                <strong>Total Triage Runs Logged:</strong>
                <span>{stats.total_triage_runs} runs</span>
              </li>
              <li>
                <strong>Total Flagged Threats:</strong>
                <span class:danger={stats.total_threats_flagged > 0}>{stats.total_threats_flagged} anomalies</span>
              </li>
              <li>
                <strong>Total Files Hashed (BLAKE3/SHA256):</strong>
                <span>{stats.total_files_hashed} files</span>
              </li>
            </ul>
          </div>

          <div class="detail-card">
            <h3>Air-Gap Security Assurance</h3>
            <p class="assurance-text">
              This triage package was collected entirely offline. Zero network packets or telemetry events have been transmitted outside the local machine. All cryptographic hashes, Shannon entropy computations, and SQLite audits are executed locally with zero-mock invariants.
            </p>
          </div>
        </div>
      </div>
    {:else if currentTab === 'processes'}
      <ProcessViewer processes={activePackage?.processes || []} />
    {:else if currentTab === 'sockets'}
      <SocketViewer sockets={activePackage?.sockets || []} />
    {:else if currentTab === 'files'}
      <FileIntegrityViewer files={activePackage?.suspicious_files || []} />
    {:else if currentTab === 'logs'}
      <EventLogTimeline logs={activePackage?.event_logs || []} />
    {:else if currentTab === 'certs'}
      <CertViewer certificates={activePackage?.certificates || []} />
    {/if}
  </main>

  <!-- Bottom Status Bar -->
  <footer class="app-footer">
    <div class="footer-left">
      <span class="status-indicator online"></span>
      <span class="status-text">SQLite WAL Persistence Active: <code>dfir_triage.db</code></span>
    </div>
    <div class="footer-right">
      <span>Triage ID: <code>{activePackage?.summary.triage_id || 'NONE'}</code></span>
      <span class="divider">|</span>
      <span>Antigravity Engine v0.1.0</span>
    </div>
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
    appName="Offline DFIR Triage Kit"
    on:close={() => (isLicenseModalOpen = false)}
    on:statusChange={(e) => {
      licenseTier = e.detail.tier;
      isLicenseActive = e.detail.is_active;
    }}
  />
</div>

<style>
  .app-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
  }

  .app-header {
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    padding: 12px 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .header-branding {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .brand-logo {
    font-size: 1.8rem;
    line-height: 1;
  }

  .brand-text {
    display: flex;
    flex-direction: column;
  }

  .brand-title {
    font-size: 1.15rem;
    font-weight: 700;
    color: var(--text-main);
  }

  .brand-sub {
    font-size: 0.75rem;
    color: var(--text-sub);
  }

  .airgap-badge {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.3);
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 0.68rem;
    font-weight: 700;
    color: var(--accent-green);
    letter-spacing: 0.5px;
  }

  .pulse-dot {
    width: 6px;
    height: 6px;
    background: var(--accent-green);
    border-radius: 50%;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .cmd-palette-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 0.8rem;
    font-weight: 500;
    padding: 4px 10px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .cmd-palette-btn:hover {
    background: rgba(239, 68, 68, 0.12);
    border-color: #ef4444;
    color: var(--text-main);
  }

  .cmd-icon {
    color: #ef4444;
    font-weight: 700;
  }

  .cmd-palette-btn kbd {
    background: #0f172a;
    border: 1px solid #334155;
    padding: 1px 5px;
    border-radius: 4px;
    font-size: 0.7rem;
    color: var(--text-muted);
    font-family: inherit;
  }

  .run-selector-wrap {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .run-select {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 8px;
    color: var(--text-main);
    font-family: var(--font-mono);
    font-size: 0.78rem;
  }

  .license-badge-btn {
    background: var(--bg-card);
    border: 1px solid var(--border);
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .license-badge-btn.licensed {
    border-color: var(--accent-green);
    color: var(--accent-green);
    background: rgba(16, 185, 129, 0.1);
  }

  .global-error {
    background: rgba(239, 68, 68, 0.15);
    border-bottom: 1px solid var(--accent-red);
    padding: 8px 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: #fca5a5;
    font-size: 0.82rem;
  }

  .dismiss-btn {
    background: transparent;
    border: none;
    color: #fca5a5;
    font-size: 0.9rem;
  }

  .nav-bar {
    background: #0d121e;
    border-bottom: 1px solid var(--border);
    display: flex;
    gap: 4px;
    padding: 0 16px;
  }

  .nav-tab {
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 10px 16px;
    font-size: 0.83rem;
    font-weight: 600;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 8px;
    transition: all 0.15s ease;
  }

  .nav-tab:hover {
    color: var(--text-main);
  }

  .nav-tab.active {
    color: var(--accent-blue);
    border-bottom-color: var(--accent-blue);
  }

  .tab-icon {
    font-size: 0.95rem;
  }

  .main-content {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .overview-view {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .summary-details-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  @media (max-width: 860px) {
    .summary-details-grid {
      grid-template-columns: 1fr;
    }
  }

  .detail-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .detail-card h3 {
    font-size: 0.95rem;
    color: var(--text-main);
  }

  .insights-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 0.82rem;
  }

  .insights-list li {
    display: flex;
    justify-content: space-between;
    color: var(--text-muted);
    border-bottom: 1px dashed var(--border);
    padding-bottom: 4px;
  }

  .insights-list span {
    font-family: var(--font-mono);
    color: var(--text-main);
  }

  .insights-list span.danger {
    color: var(--accent-red);
    font-weight: 700;
  }

  .assurance-text {
    font-size: 0.82rem;
    line-height: 1.5;
    color: var(--text-muted);
  }

  .app-footer {
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    padding: 6px 18px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.72rem;
    color: var(--text-sub);
  }

  .footer-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-indicator.online {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-green);
  }

  .footer-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  code {
    font-family: var(--font-mono);
    color: var(--text-muted);
  }

  .divider {
    color: var(--border);
  }
</style>

