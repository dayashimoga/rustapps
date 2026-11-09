<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import type {
    DatabaseSchema,
    LicenseStatusReport,
    MigrationIntegrityManifest,
    MigrationPlan,
    SchemaSummary,
    StudioStats,
  } from './lib/types';

  import CommandPalette from './components/CommandPalette.svelte';
  import LicenseModal from './components/LicenseModal.svelte';
  import NewSchemaModal from './components/NewSchemaModal.svelte';
  import MigrationManifestModal from './components/MigrationManifestModal.svelte';
  import SchemaViewerPane from './components/SchemaViewerPane.svelte';
  import MigrationPlanPane from './components/MigrationPlanPane.svelte';

  let stats: StudioStats = {
    schemas_analyzed: 0,
    migrations_generated: 0,
    breaking_hazards_prevented: 0,
    safe_online_deployments: 0,
  };

  let schemas: SchemaSummary[] = [];
  let currentSchema: DatabaseSchema | null = null;
  let currentPlan: MigrationPlan | null = null;
  let license: LicenseStatusReport = {
    tier: 'Community',
    is_valid: true,
    features: ['basic_schema_diff'],
  };

  let activeMode: 'schema_explorer' | 'migration_plan' = 'migration_plan';
  let baseSchemaId = '';
  let targetSchemaId = '';

  // Modals state
  let isPaletteOpen = false;
  let isLicenseOpen = false;
  let isNewSchemaOpen = false;
  let isManifestModalOpen = false;
  let generatedManifest: MigrationIntegrityManifest | null = null;

  async function loadData() {
    try {
      const [s, sList, lic, plan] = await Promise.all([
        invokeIpc<StudioStats>('get_stats'),
        invokeIpc<SchemaSummary[]>('list_schemas'),
        invokeIpc<LicenseStatusReport>('get_license_status'),
        invokeIpc<MigrationPlan | null>('get_migration_plan'),
      ]);
      stats = s;
      schemas = sList;
      license = lic;
      if (plan) {
        currentPlan = plan;
      }

      if (sList.length >= 2) {
        baseSchemaId = sList[1].schema_id;
        targetSchemaId = sList[0].schema_id;
      } else if (sList.length === 1) {
        baseSchemaId = sList[0].schema_id;
        targetSchemaId = sList[0].schema_id;
      }

      if (sList.length > 0 && !currentSchema) {
        await selectSchema(sList[0].schema_id);
      }
    } catch (e) {
      console.error('Failed to load initial SQL diff studio data:', e);
    }
  }

  async function selectSchema(schemaId: string) {
    try {
      const sch = await invokeIpc<DatabaseSchema>('get_schema', { schema_id: schemaId });
      currentSchema = sch;
    } catch (e) {
      console.error('Failed to load schema:', e);
    }
  }

  async function runComparison() {
    if (!baseSchemaId || !targetSchemaId) {
      alert('Please select both a baseline and a target database schema.');
      return;
    }
    try {
      const plan = await invokeIpc<MigrationPlan>('compare_schemas', {
        base_id: baseSchemaId,
        target_id: targetSchemaId,
      });
      currentPlan = plan;
      activeMode = 'migration_plan';
      const newStats = await invokeIpc<StudioStats>('get_stats');
      stats = newStats;
    } catch (e: any) {
      alert(`Comparison failed: ${e.message || e}`);
    }
  }

  async function signManifest() {
    if (!currentPlan) return;
    try {
      const manifest = await invokeIpc<MigrationIntegrityManifest>('generate_integrity_manifest', {
        plan_id: currentPlan.plan_id,
      });
      generatedManifest = manifest;
      isManifestModalOpen = true;
    } catch (e: any) {
      alert(`Manifest signing failed: ${e.message || e}`);
    }
  }

  function handleCommand(cmdId: string) {
    switch (cmdId) {
      case 'parse_schema':
        isNewSchemaOpen = true;
        break;
      case 'run_diff':
        runComparison();
        break;
      case 'view_up_script':
        activeMode = 'migration_plan';
        break;
      case 'view_down_script':
        activeMode = 'migration_plan';
        break;
      case 'sign_manifest':
        signManifest();
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
      isNewSchemaOpen = true;
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd') {
      e.preventDefault();
      runComparison();
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      signManifest();
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
      <div class="logo-mark">🗄️</div>
      <div class="brand-titles">
        <h1 class="app-title">SQL Migration & Diff Studio</h1>
        <span class="tagline">Air-Gapped Schema AST Differ, Safe Migration Generator & Linter</span>
      </div>
    </div>

    <!-- Live Executive Metrics Counters -->
    <div class="metrics-bar">
      <div class="metric-item">
        <span class="m-val">{stats.schemas_analyzed}</span>
        <span class="m-lbl">Schemas</span>
      </div>
      <div class="metric-divider"></div>
      <div class="metric-item">
        <span class="m-val">{stats.migrations_generated}</span>
        <span class="m-lbl">Plans Built</span>
      </div>
      <div class="metric-divider"></div>
      <div class="metric-item crit">
        <span class="m-val">{stats.breaking_hazards_prevented}</span>
        <span class="m-lbl">Hazards Caught</span>
      </div>
      <div class="metric-divider"></div>
      <div class="metric-item safe">
        <span class="m-val">{stats.safe_online_deployments}</span>
        <span class="m-lbl">Safe Online Ops</span>
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
          class:active={activeMode === 'migration_plan'}
          on:click={() => (activeMode = 'migration_plan')}
        >
          ⚖️ Migration Diff Studio
        </button>
        <button
          type="button"
          class="mode-btn"
          class:active={activeMode === 'schema_explorer'}
          on:click={() => (activeMode = 'schema_explorer')}
        >
          📋 Schema Explorer
        </button>
      </div>

      {#if activeMode === 'migration_plan'}
        <div class="schema-selectors">
          <div class="sel-group">
            <span class="sel-lbl">Baseline (v1):</span>
            <select bind:value={baseSchemaId}>
              {#each schemas as s}
                <option value={s.schema_id}>{s.title} ({s.dialect})</option>
              {/each}
            </select>
          </div>

          <span class="sep">➔</span>

          <div class="sel-group">
            <span class="sel-lbl">Target (v2):</span>
            <select bind:value={targetSchemaId}>
              {#each schemas as s}
                <option value={s.schema_id}>{s.title} ({s.dialect})</option>
              {/each}
            </select>
          </div>
        </div>
      {:else}
        <div class="schema-selectors">
          <div class="sel-group">
            <span class="sel-lbl">Active Schema:</span>
            <select
              value={currentSchema?.schema_id || ''}
              on:change={(e) => selectSchema(e.currentTarget.value)}
            >
              {#each schemas as s}
                <option value={s.schema_id}>{s.title} ({s.dialect})</option>
              {/each}
            </select>
          </div>
        </div>
      {/if}
    </div>

    <div class="ribbon-actions">
      <button type="button" class="btn action-btn primary" on:click={() => (isNewSchemaOpen = true)}>
        <span class="btn-icon">➕</span> Import DDL Schema
      </button>
      {#if activeMode === 'migration_plan'}
        <button type="button" class="btn action-btn compare" on:click={runComparison}>
          <span class="btn-icon">⚡</span> Run Comparison
        </button>
      {/if}
    </div>
  </div>

  <!-- Main Workspace Pane -->
  <main class="main-workspace">
    {#if activeMode === 'migration_plan'}
      <MigrationPlanPane plan={currentPlan} onSignManifest={signManifest} />
    {:else}
      <SchemaViewerPane schema={currentSchema} />
    {/if}
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

  <NewSchemaModal
    isOpen={isNewSchemaOpen}
    onClose={() => (isNewSchemaOpen = false)}
    onSchemaParsed={async (s) => {
      await loadData();
      await selectSchema(s.schema_id);
    }}
  />

  <MigrationManifestModal
    isOpen={isManifestModalOpen}
    manifest={generatedManifest}
    onClose={() => (isManifestModalOpen = false)}
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

  .schema-selectors {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .sel-group {
    display: flex;
    align-items: center;
    gap: 6px;
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
    padding: 4px 8px;
    border-radius: 5px;
    outline: none;
  }

  .sep {
    color: #64748b;
    font-size: 0.8rem;
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

  .action-btn.compare {
    background: #059669;
    border-color: #10b981;
    color: #fff;
  }

  .action-btn.compare:hover {
    background: #047857;
  }

  .main-workspace {
    flex: 1;
    display: flex;
    overflow: hidden;
  }
</style>
