<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import LicenseModal, { type LicenseStatusReport } from './components/LicenseModal.svelte';
  import DataTable from './components/DataTable.svelte';
  import DataChart, { type ChartDataPoint } from './components/DataChart.svelte';
  import ColumnInspector, { type ColumnStatistics } from './components/ColumnInspector.svelte';
  import CommandPalette, { type CommandAction } from './components/CommandPalette.svelte';

  interface ColumnMetadata {
    name: string;
    data_type: string;
    null_count: number;
    is_numeric: boolean;
  }

  interface SchemaSnapshot {
    dataset_id: string;
    name: string;
    file_path: string;
    format: string;
    columns: ColumnMetadata[];
    total_rows: number;
    total_bytes: number;
    last_scanned: number;
  }

  interface WorkbenchStats {
    total_datasets: number;
    total_cached_rows: number;
    total_bytes_analyzed: number;
  }

  interface DataFilter {
    column: string;
    operator: string;
    value: string;
  }

  interface QueryPageResult {
    columns: string[];
    rows: string[][];
    total_matching_rows: number;
    execution_time_ms: number;
  }

  interface ChartResult {
    points: ChartDataPoint[];
    x_label: string;
    y_label: string;
    computed_in_ms: number;
  }

  let datasets: SchemaSnapshot[] = [];
  let activeDatasetId: string = '';
  let activeSchema: SchemaSnapshot | null = null;
  let stats: WorkbenchStats = {
    total_datasets: 0,
    total_cached_rows: 0,
    total_bytes_analyzed: 0,
  };

  let activeView: 'table' | 'chart' = 'table';
  let isLicenseModalOpen: boolean = false;
  let licenseTier: string = 'FREE';

  // Table State
  let tableColumns: string[] = [];
  let tableRows: string[][] = [];
  let totalMatchingRows: number = 0;
  let pageIndex: number = 0;
  let pageSize: number = 25;
  let sortColumn: string | null = null;
  let sortDirection: 'Asc' | 'Desc' = 'Asc';
  let searchQuery: string = '';
  let filters: DataFilter[] = [];
  let isLoadingTable: boolean = false;

  // Filter creation state
  let newFilterCol: string = '';
  let newFilterOp: string = 'Eq';
  let newFilterVal: string = '';
  let showAddFilter: boolean = false;

  // Chart State
  let chartType: 'Bar' | 'Line' | 'Scatter' | 'Histogram' = 'Bar';
  let chartXCol: string = '';
  let chartYCol: string = '';
  let chartAgg: 'Count' | 'Sum' | 'Avg' | 'Min' | 'Max' = 'Count';
  let chartPoints: ChartDataPoint[] = [];
  let chartExecutionMs: number = 0;
  let isLoadingChart: boolean = false;

  // Column Inspector State
  let activeColStats: ColumnStatistics | null = null;
  let isInspectorOpen: boolean = false;

  // Command Palette State (Linear/Raycast UX)
  let isCommandPaletteOpen: boolean = false;

  $: commandActions = [
    {
      id: 'open-demo',
      title: 'Load Demo Sales Dataset',
      subtitle: 'Instantly populate workbench with 1,000 synthetic transaction records',
      category: 'Dataset',
      icon: '⚡',
      shortcut: '⌘D',
      run: () => handleSeedDemo(),
    },
    {
      id: 'open-file',
      title: 'Open Local Parquet / Arrow File',
      subtitle: 'Browse local filesystem using native dialog',
      category: 'File',
      icon: '📂',
      shortcut: '⌘O',
      run: () => handleOpenDataset(),
    },
    {
      id: 'view-table',
      title: 'Switch to Data Grid View',
      subtitle: 'Examine rows, paginate, and apply column filters',
      category: 'View',
      icon: '📊',
      shortcut: 'T',
      run: () => { handleViewToggle('table'); },
    },
    {
      id: 'view-chart',
      title: 'Switch to Chart Analytics View',
      subtitle: 'Aggregate metrics and render bar, line, and scatter plots',
      category: 'View',
      icon: '📈',
      shortcut: 'C',
      run: () => { handleViewToggle('chart'); },
    },
    {
      id: 'open-license',
      title: 'Manage Offline License & Entitlements',
      subtitle: `Current tier: ${licenseTier}`,
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
    } else if (e.key === 'Escape' && isCommandPaletteOpen) {
      e.preventDefault();
      isCommandPaletteOpen = false;
    }
  }

  onMount(async () => {
    await loadInitialData();
  });

  async function loadInitialData() {
    await refreshStats();
    await loadDatasets();
    try {
      const lic = await invokeIpc<LicenseStatusReport>('get_license_status');
      if (lic) licenseTier = lic.tier.toUpperCase();
    } catch (e) {
      console.warn('License status fetch error:', e);
    }
  }

  async function refreshStats() {
    try {
      const res = await invokeIpc<WorkbenchStats>('get_workbench_stats');
      if (res) stats = res;
    } catch (e) {
      console.error('Failed to get stats:', e);
    }
  }

  async function loadDatasets() {
    try {
      const list = await invokeIpc<SchemaSnapshot[]>('list_datasets');
      datasets = list || [];
      if (datasets.length > 0 && !activeDatasetId) {
        selectDataset(datasets[0].dataset_id);
      } else if (datasets.length === 0) {
        // Auto-seed demonstration dataset
        await handleSeedDemo();
      }
    } catch (e) {
      console.error('Failed to load datasets:', e);
    }
  }

  async function handleSeedDemo() {
    try {
      const schema = await invokeIpc<SchemaSnapshot>('seed_demo_dataset');
      if (schema) {
        await loadDatasets();
        selectDataset(schema.dataset_id);
      }
    } catch (e) {
      console.error('Failed to seed demo dataset:', e);
    }
  }

  function selectDataset(id: string) {
    activeDatasetId = id;
    activeSchema = datasets.find((d) => d.dataset_id === id) || null;
    pageIndex = 0;
    sortColumn = null;
    filters = [];
    if (activeSchema && activeSchema.columns.length > 0) {
      chartXCol = activeSchema.columns[0].name;
      const numCol = activeSchema.columns.find((c) => c.is_numeric);
      chartYCol = numCol ? numCol.name : activeSchema.columns[0].name;
      newFilterCol = activeSchema.columns[0].name;
    }
    executeQuery();
    if (activeView === 'chart') {
      executeChart();
    }
  }

  async function executeQuery() {
    if (!activeDatasetId) return;
    isLoadingTable = true;
    try {
      const queryPayload = {
        dataset_id: activeDatasetId,
        filters,
        sort: sortColumn ? { column: sortColumn, direction: sortDirection } : null,
        search_query: searchQuery.trim() || null,
        limit: pageSize,
        offset: pageIndex * pageSize,
      };

      const result = await invokeIpc<QueryPageResult>('query_dataset', { query: queryPayload });
      if (result) {
        tableColumns = result.columns;
        tableRows = result.rows;
        totalMatchingRows = result.total_matching_rows;
      }
    } catch (e) {
      console.error('Query execution error:', e);
    } finally {
      isLoadingTable = false;
    }
  }

  async function executeChart() {
    if (!activeDatasetId || !chartXCol) return;
    isLoadingChart = true;
    try {
      const chartPayload = {
        dataset_id: activeDatasetId,
        chart_type: chartType,
        x_column: chartXCol,
        y_column: chartYCol || null,
        aggregation: chartAgg,
        limit: 15,
      };

      const result = await invokeIpc<ChartResult>('compute_chart_data', { chart: chartPayload });
      if (result) {
        chartPoints = result.points;
        chartExecutionMs = result.computed_in_ms;
      }
    } catch (e) {
      console.error('Chart computation error:', e);
    } finally {
      isLoadingChart = false;
    }
  }

  function handlePageChange(e: CustomEvent<number>) {
    pageIndex = e.detail;
    executeQuery();
  }

  function handleSortChange(e: CustomEvent<{ column: string; direction: 'Asc' | 'Desc' }>) {
    sortColumn = e.detail.column;
    sortDirection = e.detail.direction;
    pageIndex = 0;
    executeQuery();
  }

  async function handleColumnSelect(e: CustomEvent<string>) {
    const colName = e.detail;
    try {
      const statsRes = await invokeIpc<ColumnStatistics>('compute_column_stats', {
        dataset_id: activeDatasetId,
        column_name: colName,
      });
      if (statsRes) {
        activeColStats = statsRes;
        isInspectorOpen = true;
      }
    } catch (err) {
      console.error('Column stats error:', err);
    }
  }

  function handleAddFilter() {
    if (!newFilterCol) return;
    filters = [
      ...filters,
      {
        column: newFilterCol,
        operator: newFilterOp,
        value: newFilterVal.trim(),
      },
    ];
    newFilterVal = '';
    showAddFilter = false;
    pageIndex = 0;
    executeQuery();
  }

  function removeFilter(index: number) {
    filters = filters.filter((_, i) => i !== index);
    pageIndex = 0;
    executeQuery();
  }

  function handleViewToggle(view: 'table' | 'chart') {
    activeView = view;
    if (view === 'chart') {
      executeChart();
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }
</script>

<div class="workbench-layout">
  <!-- Left Control Sidebar -->
  <aside class="sidebar">
    <!-- Top Branding -->
    <div class="sidebar-header">
      <div class="brand">
        <span class="brand-icon">📊</span>
        <div class="brand-text">
          <span class="brand-title">PARQUET WORKBENCH</span>
          <span class="airgap-badge">100% AIR-GAPPED</span>
        </div>
      </div>
      <button class="tier-badge" on:click={() => (isLicenseModalOpen = true)}>
        [{licenseTier}]
      </button>
    </div>

    <!-- Quick Stats -->
    <div class="sidebar-stats">
      <div class="s-stat">
        <span class="s-val">{stats.total_datasets}</span>
        <span class="s-lbl">Files</span>
      </div>
      <div class="s-stat">
        <span class="s-val">{stats.total_cached_rows.toLocaleString()}</span>
        <span class="s-lbl">Rows</span>
      </div>
      <div class="s-stat">
        <span class="s-val">{formatBytes(stats.total_bytes_analyzed)}</span>
        <span class="s-lbl">Size</span>
      </div>
    </div>

    <!-- Datasets Section -->
    <div class="section-title">
      <span>LOADED DATASETS</span>
      <button class="btn-sm-action" on:click={handleSeedDemo} title="Seed Demonstration Dataset">
        ⚡ Demo
      </button>
    </div>

    <div class="datasets-list">
      {#each datasets as ds}
        <div
          class="ds-item"
          class:active={ds.dataset_id === activeDatasetId}
          on:click={() => selectDataset(ds.dataset_id)}
          on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && selectDataset(ds.dataset_id)}
          role="button"
          tabindex="0"
        >
          <div class="ds-row">
            <span class="ds-icon">📁</span>
            <span class="ds-name">{ds.name}</span>
            <span class="ds-format">{ds.format.toUpperCase()}</span>
          </div>
          <div class="ds-sub">
            <span>{ds.total_rows.toLocaleString()} rows</span>
            <span>•</span>
            <span>{ds.columns.length} cols</span>
          </div>
        </div>
      {/each}
    </div>

    <!-- Active Filters Section -->
    <div class="section-title">
      <span>FILTERS & PREDICATES</span>
      <button class="btn-sm-action" on:click={() => (showAddFilter = !showAddFilter)}>
        + Add
      </button>
    </div>

    {#if showAddFilter}
      <div class="add-filter-card">
        <select bind:value={newFilterCol} class="styled-select">
          {#if activeSchema}
            {#each activeSchema.columns as col}
              <option value={col.name}>{col.name}</option>
            {/each}
          {/if}
        </select>
        <select bind:value={newFilterOp} class="styled-select">
          <option value="Eq">Equals (=)</option>
          <option value="Neq">Not Equal (!=)</option>
          <option value="Gt">Greater Than (&gt;)</option>
          <option value="Lt">Less Than (&lt;)</option>
          <option value="Contains">Contains Text</option>
        </select>
        <input
          type="text"
          bind:value={newFilterVal}
          placeholder="Filter value..."
          class="styled-input"
          on:keydown={(e) => e.key === 'Enter' && handleAddFilter()}
        />
        <div class="filter-btn-row">
          <button class="btn-sm btn-secondary" on:click={() => (showAddFilter = false)}>Cancel</button>
          <button class="btn-sm btn-primary" on:click={handleAddFilter}>Apply</button>
        </div>
      </div>
    {/if}

    <div class="filters-list">
      {#if filters.length === 0}
        <span class="empty-hint">No active filters applied.</span>
      {:else}
        {#each filters as f, idx}
          <div class="filter-pill">
            <span class="f-col">{f.column}</span>
            <span class="f-op">{f.operator}</span>
            <span class="f-val">"{f.value}"</span>
            <button class="btn-rem-f" on:click={() => removeFilter(idx)}>✕</button>
          </div>
        {/each}
      {/if}
    </div>

    <!-- Active Schema Columns -->
    {#if activeSchema}
      <div class="section-title">
        <span>COLUMNS ({activeSchema.columns.length})</span>
      </div>
      <div class="columns-list">
        {#each activeSchema.columns as col}
          <div
            class="col-item"
            on:click={() => handleColumnSelect(new CustomEvent('columnSelect', { detail: col.name }))}
            on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && handleColumnSelect(new CustomEvent('columnSelect', { detail: col.name }))}
            role="button"
            tabindex="0"
          >
            <span class="col-name">{col.name}</span>
            <span class="col-type">{col.data_type}</span>
          </div>
        {/each}
      </div>
    {/if}
  </aside>

  <!-- Main Viewport -->
  <main class="main-content">
    {#if activeSchema}
      <!-- Workspace Header -->
      <header class="workspace-header">
        <div class="search-wrap">
          <span class="search-icon">🔍</span>
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search matching text rows..."
            class="search-input"
            on:input={() => { pageIndex = 0; executeQuery(); }}
          />
        </div>

        <button class="cmd-palette-btn" on:click={() => (isCommandPaletteOpen = true)} title="Command Palette (⌘K)">
          <span>Commands</span>
          <kbd>⌘K</kbd>
        </button>

        <div class="view-switch-tabs">
          <button
            class="vtab-btn"
            class:active={activeView === 'table'}
            on:click={() => handleViewToggle('table')}
          >
            📑 Virtualized Table
          </button>
          <button
            class="vtab-btn"
            class:active={activeView === 'chart'}
            on:click={() => handleViewToggle('chart')}
          >
            📈 Offline Chart
          </button>
        </div>
      </header>

      <!-- View Content -->
      <div class="workspace-viewport">
        {#if activeView === 'table'}
          <DataTable
            columns={tableColumns}
            rows={tableRows}
            totalRows={totalMatchingRows}
            {pageSize}
            {pageIndex}
            {sortColumn}
            {sortDirection}
            isLoading={isLoadingTable}
            on:pageChange={handlePageChange}
            on:sortChange={handleSortChange}
            on:columnSelect={handleColumnSelect}
          />
        {:else if activeView === 'chart'}
          <div class="chart-screen">
            <!-- Chart Controls Bar -->
            <div class="chart-controls-bar">
              <div class="c-ctrl">
                <label for="c-type">Chart Type</label>
                <select id="c-type" bind:value={chartType} on:change={executeChart} class="styled-select">
                  <option value="Bar">Bar Chart</option>
                  <option value="Line">Line Chart</option>
                  <option value="Scatter">Scatter Plot</option>
                  <option value="Histogram">Histogram</option>
                </select>
              </div>

              <div class="c-ctrl">
                <label for="c-x">X Dimension</label>
                <select id="c-x" bind:value={chartXCol} on:change={executeChart} class="styled-select">
                  {#each activeSchema.columns as col}
                    <option value={col.name}>{col.name}</option>
                  {/each}
                </select>
              </div>

              <div class="c-ctrl">
                <label for="c-agg">Aggregation</label>
                <select id="c-agg" bind:value={chartAgg} on:change={executeChart} class="styled-select">
                  <option value="Count">COUNT(*)</option>
                  <option value="Sum">SUM</option>
                  <option value="Avg">AVG</option>
                  <option value="Min">MIN</option>
                  <option value="Max">MAX</option>
                </select>
              </div>

              {#if chartAgg !== 'Count'}
                <div class="c-ctrl">
                  <label for="c-y">Y Metric</label>
                  <select id="c-y" bind:value={chartYCol} on:change={executeChart} class="styled-select">
                    {#each activeSchema.columns.filter((c) => c.is_numeric) as col}
                      <option value={col.name}>{col.name}</option>
                    {/each}
                  </select>
                </div>
              {/if}

              <button class="btn-refresh-chart" on:click={executeChart}>
                ↻ Refresh
              </button>
            </div>

            <div class="chart-display-area">
              <DataChart
                points={chartPoints}
                {chartType}
                xLabel={chartXCol}
                yLabel={`${chartAgg}(${chartAgg === 'Count' ? 'Rows' : chartYCol})`}
                executionTimeMs={chartExecutionMs}
              />
            </div>
          </div>
        {/if}
      </div>
    {:else}
      <div class="no-dataset-view">
        <span class="no-ds-icon">📊</span>
        <h2>Local Database & Parquet Analytics Workbench</h2>
        <p>100% Offline Multi-Gigabyte Data Query & Visualization Studio</p>
        <button class="btn btn-primary" on:click={handleSeedDemo}>
          ⚡ Load Demonstration Dataset
        </button>
      </div>
    {/if}
  </main>
</div>

<!-- Offline Licensing Modal -->
<LicenseModal
  isOpen={isLicenseModalOpen}
  appName="Parquet Analytics Workbench"
  on:close={() => (isLicenseModalOpen = false)}
  on:statusChange={(e) => (licenseTier = e.detail.tier.toUpperCase())}
/>

<!-- Column Statistics Inspector -->
<ColumnInspector
  stats={activeColStats}
  isOpen={isInspectorOpen}
  on:close={() => (isInspectorOpen = false)}
  on:setChartAxis={(e) => {
    chartXCol = e.detail.column;
    activeView = 'chart';
    isInspectorOpen = false;
    executeChart();
  }}
/>

<svelte:window on:keydown={handleGlobalKeyDown} />

<!-- Raycast / Linear Command Palette -->
<CommandPalette
  isOpen={isCommandPaletteOpen}
  actions={commandActions}
  on:close={() => (isCommandPaletteOpen = false)}
/>

<style>
  .workbench-layout {
    display: flex;
    width: 100vw;
    height: 100vh;
    background: #0b0f19;
    overflow: hidden;
  }
  .sidebar {
    width: 290px;
    background: #111827;
    border-right: 1px solid rgba(75, 85, 99, 0.4);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }
  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px;
    border-bottom: 1px solid rgba(75, 85, 99, 0.3);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .brand-icon {
    font-size: 1.4rem;
  }
  .brand-text {
    display: flex;
    flex-direction: column;
  }
  .brand-title {
    font-weight: 800;
    font-size: 0.85rem;
    letter-spacing: 0.05em;
    color: #f8fafc;
  }
  .airgap-badge {
    font-size: 0.65rem;
    color: #06b6d4;
    font-weight: 600;
  }
  .tier-badge {
    background: #1e293b;
    border: 1px solid #334155;
    color: #38bdf8;
    font-size: 0.75rem;
    font-weight: 700;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
  }
  .tier-badge:hover {
    background: #334155;
  }
  .sidebar-stats {
    display: flex;
    padding: 10px 16px;
    background: rgba(15, 23, 42, 0.5);
    border-bottom: 1px solid rgba(75, 85, 99, 0.2);
  }
  .s-stat {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }
  .s-val {
    font-size: 0.95rem;
    font-weight: 700;
    color: #e2e8f0;
  }
  .s-lbl {
    font-size: 0.65rem;
    color: #64748b;
    text-transform: uppercase;
  }
  .section-title {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px 6px;
    font-size: 0.7rem;
    font-weight: 700;
    color: #64748b;
    letter-spacing: 0.05em;
  }
  .btn-sm-action {
    background: none;
    border: none;
    color: #06b6d4;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
  }
  .datasets-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px 8px;
    max-height: 160px;
    overflow-y: auto;
    border-bottom: 1px solid rgba(75, 85, 99, 0.2);
  }
  .ds-item {
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
    color: #cbd5e1;
    transition: background 0.1s;
  }
  .ds-item:hover {
    background: #1e293b;
  }
  .ds-item.active {
    background: #1e293b;
    border-left: 3px solid #06b6d4;
    color: #ffffff;
  }
  .ds-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .ds-icon {
    font-size: 0.9rem;
  }
  .ds-name {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 600;
  }
  .ds-format {
    background: rgba(6, 182, 212, 0.15);
    color: #06b6d4;
    font-size: 0.65rem;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: 4px;
  }
  .ds-sub {
    font-size: 0.7rem;
    color: #64748b;
    margin-top: 2px;
    display: flex;
    gap: 6px;
  }
  .add-filter-card {
    background: #1e293b;
    padding: 10px;
    margin: 4px 12px;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .styled-select, .styled-input {
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 4px;
    padding: 6px 8px;
    color: #f8fafc;
    font-size: 0.8rem;
  }
  .styled-select:focus, .styled-input:focus {
    outline: none;
    border-color: #06b6d4;
  }
  .filter-btn-row {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 4px;
  }
  .btn-sm {
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
    border: none;
  }
  .btn-primary {
    background: #06b6d4;
    color: #ffffff;
  }
  .btn-secondary {
    background: #334155;
    color: #cbd5e1;
  }
  .filters-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 4px 12px;
    max-height: 120px;
    overflow-y: auto;
    border-bottom: 1px solid rgba(75, 85, 99, 0.2);
  }
  .empty-hint {
    font-size: 0.75rem;
    color: #64748b;
    padding: 4px 0;
  }
  .filter-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #1e293b;
    border: 1px solid #334155;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-family: var(--font-mono);
  }
  .f-col {
    color: #38bdf8;
    font-weight: 600;
  }
  .f-op {
    color: #64748b;
  }
  .f-val {
    color: #a5f3fc;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .btn-rem-f {
    background: none;
    border: none;
    color: #ef4444;
    cursor: pointer;
    font-size: 0.75rem;
  }
  .columns-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .col-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 5px 8px;
    border-radius: 4px;
    font-size: 0.78rem;
    color: #cbd5e1;
    cursor: pointer;
  }
  .col-item:hover {
    background: #1e293b;
    color: #38bdf8;
  }
  .col-type {
    font-size: 0.65rem;
    color: #64748b;
    font-family: var(--font-mono);
  }
  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
  .workspace-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: #111827;
    border-bottom: 1px solid rgba(75, 85, 99, 0.4);
  }
  .search-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 6px;
    padding: 6px 12px;
    width: 380px;
  }
  .cmd-palette-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: #94a3b8;
    padding: 6px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    transition: all 0.15s ease;
  }
  .cmd-palette-btn:hover {
    background: rgba(99, 102, 241, 0.15);
    border-color: rgba(99, 102, 241, 0.4);
    color: #e2e8f0;
  }
  .cmd-palette-btn kbd {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.15);
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 10px;
    color: #a5b4fc;
  }
  .search-icon {
    font-size: 0.85rem;
    opacity: 0.6;
  }
  .search-input {
    background: transparent;
    border: none;
    color: #f8fafc;
    font-size: 0.85rem;
    width: 100%;
  }
  .search-input:focus {
    outline: none;
  }
  .view-switch-tabs {
    display: flex;
    gap: 6px;
  }
  .vtab-btn {
    background: #1e293b;
    border: 1px solid #334155;
    color: #94a3b8;
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }
  .vtab-btn:hover {
    color: #e2e8f0;
  }
  .vtab-btn.active {
    background: #06b6d4;
    color: #0b0f19;
    border-color: #06b6d4;
  }
  .workspace-viewport {
    flex: 1;
    overflow: hidden;
  }
  .chart-screen {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .chart-controls-bar {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    padding: 12px 16px;
    background: #111827;
    border-bottom: 1px solid rgba(75, 85, 99, 0.3);
  }
  .c-ctrl {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .c-ctrl label {
    font-size: 0.75rem;
    color: #94a3b8;
    font-weight: 500;
  }
  .btn-refresh-chart {
    background: #1e293b;
    border: 1px solid #334155;
    color: #cbd5e1;
    padding: 7px 14px;
    border-radius: 4px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }
  .btn-refresh-chart:hover {
    background: #334155;
    color: #ffffff;
  }
  .chart-display-area {
    flex: 1;
    overflow: hidden;
  }
  .no-dataset-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: #64748b;
  }
  .no-ds-icon {
    font-size: 3rem;
  }
</style>
