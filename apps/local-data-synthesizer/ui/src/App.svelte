<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import SchemaInspector from './components/SchemaInspector.svelte';
  import DataPreviewTable from './components/DataPreviewTable.svelte';
  import PrivacyScoreMeter from './components/PrivacyScoreMeter.svelte';
  import LicenseModal from './components/LicenseModal.svelte';
  import CommandPalette, { type CommandAction } from './components/CommandPalette.svelte';

  interface SynthesizerStats {
    total_schemas_analyzed: number;
    total_datasets_generated: number;
    total_rows_synthesized: number;
    avg_k_anonymity: number;
  }

  interface SyntheticDatasetConfig {
    dataset_id: string;
    name: string;
    tables: any[];
    target_rows: number;
    hmac_salt: string;
    preserve_referential_integrity: bool;
    differential_privacy_epsilon: number;
  }

  let stats: SynthesizerStats = {
    total_schemas_analyzed: 1,
    total_datasets_generated: 0,
    total_rows_synthesized: 0,
    avg_k_anonymity: 5,
  };

  let currentConfig: SyntheticDatasetConfig | null = null;
  let activeTableName: string = 'customers';
  let previewRows: any[] = [];
  let latestAuditReport: any = null;
  let isLicenseModalOpen: boolean = false;
  let licenseTier: string = 'Community';
  let isCommandPaletteOpen: boolean = false;
  let isGenerating: boolean = false;
  let targetRowsOverride: number = 70;

  $: commandActions = [
    {
      id: 'generate-synthetic-now',
      title: 'Generate Synthetic Dataset',
      subtitle: 'Synthesize multi-table dataset preserving foreign keys and differential privacy',
      category: 'Synthesis',
      icon: '⚡',
      shortcut: '⌘G',
      run: () => {
        executeSynthesis();
      },
    },
    {
      id: 'export-active-csv',
      title: 'Export Active Table CSV',
      subtitle: `Export current table (${activeTableName}) as sanitized CSV`,
      category: 'Export',
      icon: '💾',
      shortcut: '⌘E',
      run: () => {
        handleExportCsv(new CustomEvent('exportCsv', { detail: { tableName: activeTableName } }));
      },
    },
    {
      id: 'target-100-rows',
      title: 'Set Target Rows: 100 Records',
      subtitle: 'Fast staging dataset size',
      category: 'Synthesis',
      icon: '🔢',
      shortcut: '1',
      run: () => {
        targetRowsOverride = 100;
      },
    },
    {
      id: 'target-1000-rows',
      title: 'Set Target Rows: 1,000 Records',
      subtitle: 'Performance benchmark dataset size',
      category: 'Synthesis',
      icon: '🔢',
      shortcut: '2',
      run: () => {
        targetRowsOverride = 1000;
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
      stats = await invokeIpc<SynthesizerStats>('get_stats');
      currentConfig = await invokeIpc<SyntheticDatasetConfig>('get_dataset_config', {
        dataset_id: 'ds-fintech-demo',
      });
      if (currentConfig?.tables?.length) {
        activeTableName = currentConfig.tables[0].table_name;
      }
      const lic = await invokeIpc<any>('get_license_status');
      if (lic && lic.tier) {
        licenseTier = lic.tier;
      }
      // Auto-generate initial preview
      await executeSynthesis();
    } catch (e) {
      console.error('Failed to load initial synthesizer data:', e);
    }
  }

  async function executeSynthesis() {
    if (!currentConfig) return;
    isGenerating = true;
    try {
      const report = await invokeIpc<any>('synthesize_dataset', {
        dataset_id: currentConfig.dataset_id,
        target_rows_override: targetRowsOverride,
      });
      latestAuditReport = report;
      stats = await invokeIpc<SynthesizerStats>('get_stats');
      await refreshPreviewRows();
    } catch (err) {
      console.error('Synthesis failed:', err);
    } finally {
      isGenerating = false;
    }
  }

  async function refreshPreviewRows() {
    if (!currentConfig) return;
    try {
      previewRows = await invokeIpc<any[]>('get_table_preview', {
        dataset_id: currentConfig.dataset_id,
        table_name: activeTableName,
        limit: 50,
      });
    } catch (err) {
      console.error('Failed to fetch preview rows:', err);
    }
  }

  async function handleTableSelect(e: CustomEvent) {
    activeTableName = e.detail.tableName;
    await refreshPreviewRows();
  }

  async function handleExportCsv(e: CustomEvent) {
    if (!currentConfig) return;
    try {
      const res = await invokeIpc<{ csv: string }>('export_table_csv', {
        dataset_id: currentConfig.dataset_id,
        table_name: e.detail.tableName,
      });

      // Browser download trigger
      const blob = new Blob([res.csv], { type: 'text/csv;charset=utf-8;' });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.setAttribute('href', url);
      link.setAttribute('download', `${e.detail.tableName}_synthetic.csv`);
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
    } catch (err) {
      console.error('CSV export failed:', err);
    }
  }

  onMount(() => {
    loadInitialData();
  });
</script>

<svelte:window on:keydown={handleGlobalKeyDown} />

<div class="synthesizer-app">
  <!-- Top Bar -->
  <header class="app-header">
    <div class="brand">
      <span class="brand-icon">🧬</span>
      <div class="brand-text">
        <h1>LOCAL SYNTHETIC DATA & ANONYMIZATION STUDIO</h1>
        <span class="subtitle">100% Offline Schema Cloning, Referential Integrity Preserving Subsets & Differential Privacy</span>
      </div>
    </div>

    <div class="header-stats">
      <button class="cmd-palette-btn" on:click={() => (isCommandPaletteOpen = true)} title="Commands (⌘K)">
        <span class="cmd-icon">⌘</span>
        <span>Commands</span>
        <kbd>⌘K</kbd>
      </button>

      <div class="stat-pill">
        <span class="stat-label">SCHEMAS:</span>
        <span class="stat-value">{stats.total_schemas_analyzed}</span>
      </div>
      <div class="stat-pill">
        <span class="stat-label">SYNTHESIZED:</span>
        <span class="stat-value">{stats.total_rows_synthesized} rows</span>
      </div>
      <div class="stat-pill">
        <span class="stat-label">K-ANON:</span>
        <span class="stat-value">k ≥ {stats.avg_k_anonymity}</span>
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
    <!-- Upper: Privacy & Differential Privacy Guarantees -->
    <section class="privacy-section">
      <PrivacyScoreMeter auditReport={latestAuditReport} />
    </section>

    <!-- Middle: Schema Inspector -->
    {#if currentConfig}
      <section class="schema-section">
        <SchemaInspector
          tables={currentConfig.tables}
          bind:activeTableName
          on:tableSelect={handleTableSelect}
        />
      </section>

      <!-- Lower: Data Preview & Export -->
      <section class="preview-section">
        <DataPreviewTable
          tableName={activeTableName}
          rows={previewRows}
          on:exportCsv={handleExportCsv}
        />
      </section>

      <!-- Bottom Generator Control Bar -->
      <div class="generator-bar">
        <div class="config-inputs">
          <label for="rows-target">Target Rows Override:</label>
          <input
            id="rows-target"
            type="number"
            bind:value={targetRowsOverride}
            min="10"
            max="10000"
            step="10"
          />
          <span class="referential-badge">✓ Referential Integrity Active (Foreign Keys Preserved)</span>
        </div>

        <button
          class="generate-btn"
          disabled={isGenerating}
          on:click={executeSynthesis}
        >
          {isGenerating ? 'Synthesizing Gaussian & Categorical Distributions...' : 'Generate Synthetic Dataset'}
        </button>
      </div>
    {:else}
      <div class="loading-state">
        <span class="loading-spinner"></span>
        <span>Loading schema synthesizer...</span>
      </div>
    {/if}
  </main>

  <!-- Footer Status Bar -->
  <footer class="app-footer">
    <span class="status-item">
      DATASET: {currentConfig ? currentConfig.name : 'None'}
    </span>
    <span class="status-item">
      ACTIVE TABLE: <strong class="active-tag">{activeTableName}</strong>
    </span>
    <span class="status-item security">
      100% AIR-GAPPED: LOCAL SYNTHESIS WITH ZERO NETWORK EGRESS
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
    appName="Local Synthetic Data & Anonymization Studio"
    on:close={() => (isLicenseModalOpen = false)}
    on:statusChange={(e) => (licenseTier = e.detail.tier)}
  />
</div>

<style>
  .synthesizer-app {
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
    color: #38bdf8;
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
    background: rgba(56, 189, 248, 0.15);
    border-color: #38bdf8;
    color: #f8fafc;
  }

  .cmd-icon {
    color: #38bdf8;
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
    border: 1px solid #38bdf866;
    color: #38bdf8;
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

  .privacy-section {
    flex-shrink: 0;
  }

  .schema-section {
    flex: 1;
    min-height: 180px;
    display: flex;
    flex-direction: column;
  }

  .preview-section {
    flex: 1.2;
    min-height: 200px;
    display: flex;
    flex-direction: column;
  }

  .generator-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: #0f141d;
    border: 1px solid #1a2332;
    border-radius: 6px;
    padding: 8px 14px;
    font-family: monospace;
  }

  .config-inputs {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 0.75rem;
  }

  .config-inputs label {
    color: #94a3b8;
  }

  .config-inputs input {
    width: 80px;
    background: #090d14;
    border: 1px solid #28374d;
    color: #f1f5f9;
    padding: 4px 8px;
    border-radius: 4px;
    font-family: inherit;
    font-size: 0.75rem;
    outline: none;
  }

  .referential-badge {
    color: #34d399;
    font-size: 0.72rem;
    font-weight: 600;
  }

  .generate-btn {
    background: #0284c7;
    border: 1px solid #38bdf8;
    color: #ffffff;
    font-weight: 700;
    font-size: 0.78rem;
    font-family: monospace;
    padding: 8px 16px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .generate-btn:hover:not(:disabled) {
    background: #0369a1;
    box-shadow: 0 0 16px rgba(56, 189, 248, 0.4);
  }

  .generate-btn:disabled {
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

  .active-tag {
    color: #38bdf8;
  }

  .status-item.security {
    margin-left: auto;
    color: #34d399;
    font-weight: 600;
  }

  .status-item.version {
    color: #38bdf8;
  }
</style>
