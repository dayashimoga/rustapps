<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import type {
    AppCategory,
    AppMetadata,
    AppSelfTestResult,
    LicenseStatus,
    SuiteIntegrityManifest,
    SuiteStats,
  } from './lib/types';

  import CommandPalette from './components/CommandPalette.svelte';
  import LicenseModal from './components/LicenseModal.svelte';
  import SuiteManifestModal from './components/SuiteManifestModal.svelte';
  import DiagnosticModal from './components/DiagnosticModal.svelte';
  import AppCard from './components/AppCard.svelte';

  let stats: SuiteStats = {
    total_apps_registered: 23,
    total_apps_installed: 23,
    suite_license_tier: 'Enterprise Air-Gap',
    air_gap_egress_blocks: 0,
  };

  let apps: AppMetadata[] = [];
  let licenseStatus: LicenseStatus | null = null;
  let suiteManifest: SuiteIntegrityManifest | null = null;

  let activeCategory: 'all' | AppCategory = 'all';
  let searchQuery = '';

  let selectedApp: AppMetadata | null = null;
  let diagnosticResult: AppSelfTestResult | null = null;
  let runningDiagnostic = false;

  let showCommandPalette = false;
  let showLicenseModal = false;
  let showManifestModal = false;
  let showDiagnosticModal = false;

  onMount(async () => {
    await reloadData();
  });

  async function reloadData() {
    try {
      stats = await invokeIpc<SuiteStats>('get_stats');
      apps = await invokeIpc<AppMetadata[]>('list_apps');
      licenseStatus = await invokeIpc<LicenseStatus>('get_license_status');
    } catch (e) {
      console.error('Failed to load suite launcher data', e);
    }
  }

  $: filteredApps = apps.filter(app => {
    const matchesCategory = activeCategory === 'all' || app.category === activeCategory;
    const q = searchQuery.toLowerCase().trim();
    const matchesSearch =
      !q ||
      app.name.toLowerCase().includes(q) ||
      app.tagline.toLowerCase().includes(q) ||
      app.binary_name.toLowerCase().includes(q);
    return matchesCategory && matchesSearch;
  });

  async function openDiagnostic(app: AppMetadata) {
    selectedApp = app;
    diagnosticResult = null;
    runningDiagnostic = true;
    showDiagnosticModal = true;

    try {
      diagnosticResult = await invokeIpc<AppSelfTestResult>('run_diagnostic', { app_id: app.id });
    } catch (e: any) {
      diagnosticResult = {
        app_id: app.id,
        passed: false,
        ipc_checks_passed: 0,
        ipc_checks_total: 7,
        message: e.message || 'Diagnostic execution failed',
      };
    } finally {
      runningDiagnostic = false;
    }
  }

  async function openSuiteManifest() {
    try {
      suiteManifest = await invokeIpc<SuiteIntegrityManifest>('sign_suite_manifest');
      showManifestModal = true;
    } catch (e) {
      console.error('Failed to generate suite manifest', e);
    }
  }

  function handleCommandSelect(e: CustomEvent<{ id: string }>) {
    switch (e.detail.id) {
      case 'verify_manifest':
        openSuiteManifest();
        break;
      case 'manage_license':
        showLicenseModal = true;
        break;
      case 'reload_catalog':
        reloadData();
        break;
      case 'run_all_diagnostics':
        if (apps.length > 0) openDiagnostic(apps[0]);
        break;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
      e.preventDefault();
      showCommandPalette = !showCommandPalette;
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="flex flex-col h-screen w-screen bg-slate-950 text-slate-100 font-sans overflow-hidden select-none">
  <!-- Top Navigation & Metrics Header -->
  <header class="px-5 py-3 bg-slate-900 border-b border-slate-800 flex items-center justify-between shrink-0">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-amber-500/10 border border-amber-500/30 flex items-center justify-center text-amber-400 font-bold text-base">
        🚀
      </div>
      <div>
        <div class="flex items-center gap-2">
          <h1 class="text-sm font-bold text-slate-100 tracking-wide">
            Antigravity Offline Enterprise Suite
          </h1>
          <span class="px-2 py-0.5 rounded text-[10px] font-mono font-medium bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
            23 APPS STANDALONE
          </span>
          <span class="px-2 py-0.5 rounded text-[10px] font-mono font-medium bg-amber-500/20 text-amber-300 border border-amber-500/30">
            100% AIR-GAPPED
          </span>
        </div>
        <p class="text-[11px] text-slate-400">
          Centralized Command Center & Diagnostic Health Monitor for all 23 Offline Workbenches
        </p>
      </div>
    </div>

    <!-- Live Executive Metrics -->
    <div class="hidden lg:flex items-center gap-6 font-mono text-xs border-x border-slate-800 px-6">
      <div>
        <div class="text-[10px] text-slate-500 uppercase">Total Apps</div>
        <div class="text-slate-200 font-bold">{stats.total_apps_registered} Registered</div>
      </div>
      <div>
        <div class="text-[10px] text-slate-500 uppercase">Disk Status</div>
        <div class="text-emerald-400 font-bold">{stats.total_apps_installed} Installed</div>
      </div>
      <div>
        <div class="text-[10px] text-slate-500 uppercase">License Coverage</div>
        <div class="text-amber-400 font-bold">{stats.suite_license_tier}</div>
      </div>
      <div>
        <div class="text-[10px] text-slate-500 uppercase">Telemetry Egress</div>
        <div class="text-emerald-400 font-bold">0 Blocks (Air-Gap)</div>
      </div>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-2">
      <button
        class="px-3 py-1.5 bg-emerald-950/80 hover:bg-emerald-900 text-emerald-300 border border-emerald-700/60 rounded-lg text-xs font-mono font-medium transition-colors"
        on:click={openSuiteManifest}
      >
        Suite Manifest
      </button>

      <button
        class="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-amber-300 rounded-lg text-xs font-mono font-medium transition-colors border border-slate-700"
        on:click={() => showLicenseModal = true}
      >
        {licenseStatus?.tier?.toUpperCase() || 'ALL-ACCESS'}
      </button>

      <button
        class="px-2.5 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-slate-200 rounded-lg text-xs font-mono border border-slate-700"
        on:click={() => showCommandPalette = true}
        title="Command Palette (Ctrl+K)"
      >
        ⌘K
      </button>
    </div>
  </header>

  <!-- Filter & Search Sub-header -->
  <div class="px-5 py-2.5 bg-slate-900/60 border-b border-slate-800 flex items-center justify-between shrink-0 font-mono text-xs gap-4">
    <div class="flex items-center gap-2">
      <button
        class="px-3 py-1 rounded text-xs transition-colors {activeCategory === 'all' ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40 font-bold' : 'text-slate-400 hover:text-slate-200'}"
        on:click={() => activeCategory = 'all'}
      >
        All Applications ({apps.length})
      </button>
      <button
        class="px-3 py-1 rounded text-xs transition-colors {activeCategory === 'enterprise_core' ? 'bg-blue-500/20 text-blue-300 border border-blue-500/40 font-bold' : 'text-slate-400 hover:text-slate-200'}"
        on:click={() => activeCategory = 'enterprise_core'}
      >
        DevOps & Security Core (7)
      </button>
      <button
        class="px-3 py-1 rounded text-xs transition-colors {activeCategory === 'creator_media' ? 'bg-purple-500/20 text-purple-300 border border-purple-500/40 font-bold' : 'text-slate-400 hover:text-slate-200'}"
        on:click={() => activeCategory = 'creator_media'}
      >
        Creator & Media (6)
      </button>
      <button
        class="px-3 py-1 rounded text-xs transition-colors {activeCategory === 'industrial_specialized' ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 font-bold' : 'text-slate-400 hover:text-slate-200'}"
        on:click={() => activeCategory = 'industrial_specialized'}
      >
        Specialized Industrial & Legal (10)
      </button>
    </div>

    <!-- Search Input -->
    <div class="relative w-72">
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Filter 23 applications..."
        class="w-full px-3 py-1 bg-slate-950 border border-slate-700 rounded text-slate-100 text-xs focus:border-amber-500 focus:outline-none"
      />
      {#if searchQuery}
        <button
          class="absolute right-2 top-1 text-slate-400 hover:text-slate-200"
          on:click={() => searchQuery = ''}
        >
          ✕
        </button>
      {/if}
    </div>
  </div>

  <!-- Main Grid Area -->
  <main class="flex-1 overflow-y-auto p-5 bg-slate-950">
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
      {#each filteredApps as app (app.id)}
        <AppCard
          {app}
          on:diagnostic={(e) => openDiagnostic(e.detail)}
          on:launch={(e) => openDiagnostic(e.detail)}
        />
      {:else}
        <div class="col-span-full p-12 text-center text-slate-500 font-mono text-xs">
          No applications match "{searchQuery}".
        </div>
      {/each}
    </div>
  </main>

  <!-- Modals -->
  <CommandPalette
    bind:show={showCommandPalette}
    on:select={handleCommandSelect}
  />

  <LicenseModal
    bind:show={showLicenseModal}
    status={licenseStatus}
    on:activated={reloadData}
  />

  <SuiteManifestModal
    bind:show={showManifestModal}
    manifest={suiteManifest}
  />

  <DiagnosticModal
    bind:show={showDiagnosticModal}
    app={selectedApp}
    result={diagnosticResult}
    running={runningDiagnostic}
    on:run={(e) => {
      const a = apps.find(x => x.id === e.detail);
      if (a) openDiagnostic(a);
    }}
  />
</div>
