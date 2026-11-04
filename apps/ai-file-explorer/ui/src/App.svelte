<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc, openNativeDialog, getDrivesAndPlaces, isDesktopApp } from './lib/ipc';
  import Header from './components/Header.svelte';
  import FileBrowser from './components/FileBrowser.svelte';
  import FilePreviewPane from './components/FilePreviewPane.svelte';
  import DuplicateFinder from './components/DuplicateFinder.svelte';
  import TreemapView from './components/TreemapView.svelte';
  import BatchRenameModal from './components/BatchRenameModal.svelte';
  import StatsDrawer from './components/StatsDrawer.svelte';
  import ContentSearch from './components/ContentSearch.svelte';
  import ScreenshotGallery from './components/ScreenshotGallery.svelte';
  import DocumentModal from './components/DocumentModal.svelte';
  import ComplianceScanner from './components/ComplianceScanner.svelte';
  import type {
    ActiveTab,
    BatchRenameResult,
    BatchRenameRule,
    ComplianceScanSummary,
    DeduplicationSummary,
    DeleteFilesResult,
    DirectoryListing,
    ExtractedDocument,
    FileItem,
    FolderStats,
    FsLocationEntry,
    RenamePreview,
    ScreenshotFingerprint,
    ScreenshotSearchResult,
    SensitiveFinding,
    TreemapNode,
  } from './types';

  // Navigation and Tab State
  let currentPath = 'H:\\rustapps';
  let searchQuery = '';
  let activeTab: ActiveTab = 'browser';
  let showStatsDrawer = false;
  let navError = '';
  let previewItem: FileItem | null = null;

  // Drives & Quick Access Places
  let drivesAndPlaces: FsLocationEntry[] = [];
  let networkPathInput = '';

  // Data states
  let currentFolderItems: FileItem[] = [];
  let listing: DirectoryListing = {
    current_path: currentPath,
    parent_path: null,
    items: [],
    total_count: 0,
    total_size_bytes: 0,
  };
  let selectedPaths: string[] = [];
  let isScanningDuplicates = false;
  let dedupSummary: DeduplicationSummary | null = null;
  let treemapNode: TreemapNode | null = null;
  let folderStats: FolderStats | null = null;

  // Batch Rename States
  let renamePreviews: RenamePreview[] = [];
  let renameResult: BatchRenameResult | null = null;
  let isExecutingRename = false;

  // Phase 6 Document Inspection Modal State
  let inspectedDoc: ExtractedDocument | null = null;
  let isDocModalOpen = false;

  // Standardized Desktop IPC Bridge
  async function invokeTauri<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
    return await invokeIpc<T>(cmd, args);
  }

  async function loadDrivesAndPlaces() {
    try {
      const list = await getDrivesAndPlaces();
      if (list && list.length > 0) {
        drivesAndPlaces = list;
        return;
      }
      drivesAndPlaces = await invokeIpc<FsLocationEntry[]>('list_drives_and_places_cmd');
    } catch (err) {
      console.error('Failed to enumerate drives and places:', err);
    }
  }

  async function handleBrowseFolder() {
    try {
      const chosen = await openNativeDialog({
        directory: true,
        title: 'Select Folder to Browse in AI File Explorer',
      });
      if (chosen && typeof chosen === 'string') {
        await loadDirectory(chosen);
      }
    } catch (err) {
      console.error('Browse folder dialog error:', err);
    }
  }

  async function handleOpenInOs(path: string) {
    try {
      await invokeTauri('open_in_os_cmd', { path });
    } catch (err) {
      console.error('Open in OS error:', err);
    }
  }

  async function loadDirectory(path: string) {
    navError = '';
    try {
      const result = await invokeTauri<DirectoryListing>('list_directory_cmd', { path });
      listing = result;
      currentFolderItems = result.items || [];
      if (result && result.current_path) {
        currentPath = result.current_path;
      } else {
        currentPath = path;
      }
      selectedPaths = [];
      searchQuery = '';

      // Index current folder entries asynchronously without blocking navigation
      invokeTauri<number>('index_directory_cmd', { path: currentPath }).catch(() => {});

      if (showStatsDrawer) {
        loadStorageStats(currentPath);
      }
      if (activeTab === 'treemap') {
        loadTreemap(currentPath);
      }
    } catch (err) {
      navError = err instanceof Error ? err.message : String(err);
      console.error('Failed to list directory:', err);
    }
  }

  $: if (showStatsDrawer && currentPath && (!folderStats || folderStats.root_path !== currentPath)) {
    loadStorageStats(currentPath);
  }

  $: if (activeTab === 'treemap' && currentPath && (!treemapNode || treemapNode.path !== currentPath)) {
    loadTreemap(currentPath);
  }

  async function handleSearch(query: string) {
    searchQuery = query;
    const trimmed = query.trim().toLowerCase();
    if (!trimmed) {
      listing = {
        ...listing,
        items: currentFolderItems,
        total_count: currentFolderItems.length,
      };
      return;
    }

    // Strip leading wildcard or dot if searching for extension e.g. "*.pdf" or ".pdf"
    const extSearch = trimmed.startsWith('*.') ? trimmed.slice(2) : trimmed.startsWith('.') ? trimmed.slice(1) : trimmed;

    // Immediate local substring match on current folder (checking both file_name and extension)
    const localHits = currentFolderItems.filter((item) =>
      item.file_name.toLowerCase().includes(trimmed) ||
      (item.extension && item.extension.toLowerCase() === extSearch)
    );

    try {
      const indexedHits = await invokeTauri<FileItem[]>('search_files_cmd', {
        query,
        limit: 100,
      });
      const seen = new Set(localHits.map((i) => i.path));
      const merged = [...localHits];
      for (const hit of indexedHits || []) {
        if (!seen.has(hit.path)) {
          seen.add(hit.path);
          merged.push(hit);
        }
      }
      listing = {
        ...listing,
        items: merged,
        total_count: merged.length,
      };
    } catch {
      listing = {
        ...listing,
        items: localHits,
        total_count: localHits.length,
      };
    }
  }

  async function loadStorageStats(path: string) {
    try {
      folderStats = await invokeTauri<FolderStats>('analyze_storage_cmd', {
        path,
        top_limit: 5,
      });
    } catch (err) {
      console.error('Stats error:', err);
    }
  }

  async function loadTreemap(path: string) {
    try {
      treemapNode = await invokeTauri<TreemapNode>('get_treemap_cmd', {
        path,
        width: 1024,
        height: 768,
        max_depth: 3,
      });
    } catch (err) {
      console.error('Treemap error:', err);
    }
  }

  async function handleStartScanDuplicates(root: string, minSize: number) {
    isScanningDuplicates = true;
    try {
      dedupSummary = await invokeTauri<DeduplicationSummary>('scan_duplicates_cmd', {
        search_roots: [root],
        min_size_bytes: minSize,
      });
    } catch (err) {
      console.error('Dedup scan error:', err);
    } finally {
      isScanningDuplicates = false;
    }
  }

  async function handleDeleteDuplicates(paths: string[]) {
    try {
      const res = await invokeTauri<DeleteFilesResult>('delete_files_cmd', { paths });
      if (res && dedupSummary) {
        const deletedSet = new Set(paths);
        const updatedGroups = dedupSummary.duplicate_groups
          .map(g => ({
            ...g,
            file_paths: g.file_paths.filter(p => !deletedSet.has(p)),
          }))
          .filter(g => g.file_paths.length > 1);

        dedupSummary = {
          ...dedupSummary,
          duplicate_groups: updatedGroups,
          total_duplicate_files: updatedGroups.reduce((acc, g) => acc + g.file_paths.length, 0),
          potential_reclaimed_bytes: Math.max(0, dedupSummary.potential_reclaimed_bytes - res.reclaimed_bytes),
        };
        await loadDirectory(currentPath);
        await loadStorageStats(currentPath);
        await loadTreemap(currentPath);
      }
    } catch (err) {
      console.error('Delete duplicates error:', err);
    }
  }

  async function handlePreviewBatchRename(rule: BatchRenameRule) {
    try {
      renamePreviews = await invokeTauri<RenamePreview[]>('preview_batch_rename_cmd', {
        paths: selectedPaths,
        rule,
      });
    } catch (err) {
      console.error('Preview error:', err);
    }
  }

  async function handleExecuteBatchRename(previews: RenamePreview[]) {
    isExecutingRename = true;
    try {
      renameResult = await invokeTauri<BatchRenameResult>('execute_batch_rename_cmd', {
        previews,
      });
      if (renameResult && !renameResult.rolled_back && !renameResult.error_message) {
        selectedPaths = [];
        renamePreviews = [];
        await loadDirectory(currentPath);
        await loadStorageStats(currentPath);
        await loadTreemap(currentPath);
      }
    } catch (err) {
      console.error('Execute error:', err);
    } finally {
      isExecutingRename = false;
    }
  }

  async function handleInspectDoc(path: string) {
    try {
      inspectedDoc = await invokeTauri<ExtractedDocument>('extract_document_cmd', { path });
      isDocModalOpen = true;
    } catch (err) {
      console.error('Inspect doc error:', err);
    }
  }

  function handleNavigateUp() {
    if (listing.parent_path) {
      loadDirectory(listing.parent_path);
    }
  }

  function handleToggleSelect(path: string) {
    if (selectedPaths.includes(path)) {
      selectedPaths = selectedPaths.filter(p => p !== path);
    } else {
      selectedPaths = [...selectedPaths, path];
    }
  }

  function handleSelectAll() {
    selectedPaths = listing.items.map(i => i.path);
  }

  function handleDeselectAll() {
    selectedPaths = [];
  }

  async function handleScanDirectoryCompliance(path: string): Promise<ComplianceScanSummary> {
    return await invokeTauri<ComplianceScanSummary>('scan_compliance_cmd', { path });
  }

  async function handleScanFileCompliance(path: string): Promise<SensitiveFinding[]> {
    const summary = await invokeTauri<ComplianceScanSummary>('scan_compliance_cmd', { path });
    return summary.findings;
  }

  function handleConnectNetworkPath() {
    if (networkPathInput.trim()) {
      loadDirectory(networkPathInput.trim());
    }
  }

  onMount(async () => {
    await loadDrivesAndPlaces();
    await loadDirectory(currentPath);
  });

  $: driveEntries = drivesAndPlaces.filter((e) => e.kind === 'drive');
  $: quickAccessEntries = drivesAndPlaces.filter((e) => e.kind !== 'drive');
</script>

<main class="app-layout">
  <!-- Top Navigation Header -->
  <Header
    {currentPath}
    {searchQuery}
    bind:activeTab
    bind:showStatsDrawer
    onNavigate={loadDirectory}
    onSearch={handleSearch}
    onRefresh={() => loadDirectory(currentPath)}
    onNavigateUp={handleNavigateUp}
    onBrowseFolder={handleBrowseFolder}
  />

  {#if navError}
    <div class="nav-error-banner">
      <span>Unable to open location: {navError}</span>
      <button on:click={() => (navError = '')}>Dismiss</button>
    </div>
  {/if}

  <div class="workspace-body">
    <!-- Persistent Drives, Quick Access & Network Sidebar -->
    <aside class="places-sidebar glass-panel">
      <div class="sidebar-group">
        <div class="sidebar-title-row">
          <span class="sidebar-title">Drives & Volumes</span>
          <button class="sidebar-refresh-btn" on:click={loadDrivesAndPlaces} title="Refresh Drives">↻</button>
        </div>
        {#each driveEntries as drv (drv.path)}
          <button
            class="place-item {currentPath.toLowerCase().startsWith(drv.path.toLowerCase()) ? 'active' : ''}"
            on:click={() => loadDirectory(drv.path)}
            title={drv.path}
          >
            <span class="place-icon">💽</span>
            <span class="place-label">{drv.name}</span>
          </button>
        {/each}
      </div>

      <div class="sidebar-group">
        <span class="sidebar-title">Quick Access</span>
        {#each quickAccessEntries as place (place.path)}
          <button
            class="place-item {currentPath.toLowerCase() === place.path.toLowerCase() ? 'active' : ''}"
            on:click={() => loadDirectory(place.path)}
            title={place.path}
          >
            <span class="place-icon">📁</span>
            <span class="place-label">{place.name}</span>
          </button>
        {/each}
      </div>

      <div class="sidebar-group">
        <span class="sidebar-title">Network / UNC Share</span>
        <form class="network-form" on:submit|preventDefault={handleConnectNetworkPath}>
          <input
            type="text"
            class="network-input"
            bind:value={networkPathInput}
            placeholder="\\server\share or Z:\"
          />
          <button type="submit" class="network-go-btn">Connect</button>
        </form>
        <button class="browse-dialog-btn" on:click={handleBrowseFolder}>
          📂 Browse Folder...
        </button>
      </div>
    </aside>

    <div class="main-content-view">
      <!-- Active Tab View Surface -->
      {#if activeTab === 'browser'}
        <div class="browser-split-layout">
          <FileBrowser
            items={listing.items}
            {selectedPaths}
            selectedItem={previewItem}
            onSelectItem={(item) => (previewItem = item)}
            onNavigateToFolder={(p) => {
              previewItem = null;
              loadDirectory(p);
            }}
            onNavigateUp={() => {
              previewItem = null;
              handleNavigateUp();
            }}
            onToggleSelect={handleToggleSelect}
            onSelectAll={handleSelectAll}
            onDeselectAll={handleDeselectAll}
            onInspectFile={handleInspectDoc}
            onOpenInOs={handleOpenInOs}
          />
          {#if previewItem}
            <FilePreviewPane
              item={previewItem}
              onClose={() => (previewItem = null)}
              onOpenInOs={handleOpenInOs}
              onInspectDoc={handleInspectDoc}
            />
          {/if}
        </div>
      {:else if activeTab === 'content_search'}
        <ContentSearch
          onInspectDoc={handleInspectDoc}
        />
      {:else if activeTab === 'screenshots'}
        <ScreenshotGallery
          {currentPath}
          onInspectDoc={handleInspectDoc}
          onScanFolder={(path) => invokeTauri('scan_screenshots_dir_cmd', { path })}
          onSearchSimilar={(hash, maxDist, limit) =>
            invokeTauri('search_screenshots_by_hash_cmd', {
              target_hash: hash,
              max_distance: maxDist,
              limit,
            })}
          onSearchOcr={(query, limit) =>
            invokeTauri('search_screenshots_by_text_cmd', { query, limit })}
        />
      {:else if activeTab === 'duplicates'}
        <DuplicateFinder
          searchRoot={currentPath}
          summary={dedupSummary}
          isScanning={isScanningDuplicates}
          onStartScan={handleStartScanDuplicates}
          onDeleteDuplicates={handleDeleteDuplicates}
        />
      {:else if activeTab === 'treemap'}
        <TreemapView
          rootNode={treemapNode}
          onDrillDown={loadDirectory}
          onRefreshTreemap={() => loadTreemap(currentPath)}
          onInspectDoc={handleInspectDoc}
        />
      {:else if activeTab === 'batch_rename'}
        <BatchRenameModal
          selectedFiles={selectedPaths}
          previews={renamePreviews}
          {renameResult}
          isExecuting={isExecutingRename}
          onGeneratePreview={handlePreviewBatchRename}
          onExecuteRename={handleExecuteBatchRename}
          onClearSelection={handleDeselectAll}
        />
      {:else if activeTab === 'compliance'}
        <ComplianceScanner
          {currentPath}
          onScanDirectory={handleScanDirectoryCompliance}
          onScanFile={handleScanFileCompliance}
          onInspectDoc={handleInspectDoc}
        />
      {/if}
    </div>
  </div>

  <!-- Slide-Over Storage Stats Drawer -->
  <StatsDrawer
    stats={folderStats}
    isOpen={showStatsDrawer}
    onClose={() => (showStatsDrawer = false)}
  />

  <!-- Phase 6 Document Inspection Modal -->
  <DocumentModal
    doc={inspectedDoc}
    isOpen={isDocModalOpen}
    onClose={() => (isDocModalOpen = false)}
    onOpenInOs={handleOpenInOs}
  />
</main>

<style>
  .app-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .nav-error-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: 0 16px 8px 16px;
    padding: 8px 14px;
    background: rgba(239, 68, 68, 0.16);
    border: 1px solid rgba(239, 68, 68, 0.45);
    border-radius: 8px;
    color: #fca5a5;
    font-size: 12px;
  }

  .nav-error-banner button {
    background: transparent;
    border: 1px solid rgba(252, 165, 165, 0.4);
    color: #fca5a5;
    border-radius: 4px;
    padding: 2px 8px;
    font-size: 11px;
    cursor: pointer;
  }

  .workspace-body {
    display: flex;
    flex: 1;
    overflow: hidden;
    gap: 12px;
    padding: 0 16px 16px 16px;
  }

  .places-sidebar {
    width: 220px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 12px;
    overflow-y: auto;
    border-radius: var(--radius-lg);
  }

  .sidebar-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .sidebar-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .sidebar-title {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-faint);
    padding: 2px 6px;
  }

  .sidebar-refresh-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 12px;
  }

  .place-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--text-muted);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .place-item:hover {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-main);
  }

  .place-item.active {
    background: rgba(6, 182, 212, 0.14);
    border-color: rgba(6, 182, 212, 0.35);
    color: var(--accent-cyan);
    font-weight: 600;
  }

  .place-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .network-form {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 4px;
  }

  .network-input {
    background: rgba(11, 17, 30, 0.85);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    padding: 6px 8px;
    color: var(--text-main);
    font-family: var(--font-mono);
    font-size: 11px;
    outline: none;
  }

  .network-go-btn, .browse-dialog-btn {
    background: rgba(6, 182, 212, 0.16);
    border: 1px solid rgba(6, 182, 212, 0.35);
    border-radius: 6px;
    padding: 6px 10px;
    color: var(--accent-cyan);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .browse-dialog-btn {
    margin-top: 6px;
    background: rgba(255, 255, 255, 0.05);
    border-color: var(--border-subtle);
    color: var(--text-main);
  }

  .main-content-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .browser-split-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
    gap: 12px;
    height: 100%;
  }
</style>
