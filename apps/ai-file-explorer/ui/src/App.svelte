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
  import CommandPalette, { type CommandAction } from './components/CommandPalette.svelte';
  import type {
    ActiveTab,
    BatchRenameResult,
    BatchRenameRule,
    ComplianceScanSummary,
    CopyFilesResult,
    DeduplicationSummary,
    DeleteFilesResult,
    DirectoryListing,
    ExtractedDocument,
    FileItem,
    FolderStats,
    FsLocationEntry,
    MoveFilesResult,
    RenamePreview,
    ScreenshotFingerprint,
    ScreenshotSearchResult,
    SensitiveFinding,
    SmartFolder,
    TreemapNode,
  } from './types';

  // Navigation and Tab State
  let currentPath = 'H:\\rustapps';
  let searchQuery = '';
  let activeTab: ActiveTab = 'browser';
  let showStatsDrawer = false;
  let navError = '';
  let previewItem: FileItem | null = null;

  // Dual-Pane Commander Mode State
  let isDualPaneMode = false;
  let activePane: 'left' | 'right' = 'left';
  let rightPath = 'H:\\rustapps';
  let rightFolderItems: FileItem[] = [];
  let rightListing: DirectoryListing = {
    current_path: rightPath,
    parent_path: null,
    items: [],
    total_count: 0,
    total_size_bytes: 0,
  };
  let rightSelectedPaths: string[] = [];
  let rightPreviewItem: FileItem | null = null;
  let commanderFeedback = '';

  // Smart Virtual Folders State
  let smartFolders: SmartFolder[] = [];
  let activeSmartFolder: SmartFolder | null = null;
  let isSmartFolderMode = false;
  let isLoadingSmartFolder = false;

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
  let isCommandPaletteOpen = false;

  $: commandActions = [
    {
      id: 'tab-browser',
      title: 'Explorer & File Grid',
      subtitle: 'Browse local directories and preview files',
      category: 'Navigation',
      icon: '📁',
      shortcut: 'Ctrl+1',
      run: () => { activeTab = 'browser'; },
    },
    {
      id: 'tab-content-search',
      title: 'Content & Semantic Search',
      subtitle: 'Tantivy full-text & vector search across documents',
      category: 'Navigation',
      icon: '🔍',
      shortcut: 'Ctrl+2',
      run: () => { activeTab = 'content_search'; },
    },
    {
      id: 'tab-screenshots',
      title: 'Screenshot OCR Gallery',
      subtitle: 'Visual perceptual hash & OCR search',
      category: 'Navigation',
      icon: '🖼️',
      shortcut: 'Ctrl+3',
      run: () => { activeTab = 'screenshots'; },
    },
    {
      id: 'tab-duplicates',
      title: 'Duplicate File Finder',
      subtitle: 'BLAKE3 content hashing deduplication',
      category: 'Navigation',
      icon: '👥',
      shortcut: 'Ctrl+4',
      run: () => { activeTab = 'duplicates'; },
    },
    {
      id: 'tab-treemap',
      title: 'Disk Treemap Visualizer',
      subtitle: 'Hierarchical space consumption visualizer',
      category: 'Navigation',
      icon: '🗺️',
      shortcut: 'Ctrl+5',
      run: () => { activeTab = 'treemap'; },
    },
    {
      id: 'tab-batch-rename',
      title: 'Batch File Renamer',
      subtitle: 'Regex, prefixes, numbering transformations',
      category: 'Navigation',
      icon: '✏️',
      shortcut: 'Ctrl+6',
      run: () => { activeTab = 'batch_rename'; },
    },
    {
      id: 'tab-compliance',
      title: 'Data Compliance Scanner',
      subtitle: 'PII, secret leak, and compliance pattern detection',
      category: 'Navigation',
      icon: '🛡️',
      shortcut: 'Ctrl+7',
      run: () => { activeTab = 'compliance'; },
    },
    {
      id: 'act-browse',
      title: 'Browse Folder...',
      subtitle: 'Open native folder picker dialog',
      category: 'Actions',
      icon: '📂',
      shortcut: 'Ctrl+O',
      run: handleBrowseFolder,
    },
    {
      id: 'act-refresh',
      title: 'Refresh Current Directory',
      subtitle: 'Re-scan filesystem entries',
      category: 'Actions',
      icon: '↻',
      shortcut: 'Ctrl+R',
      run: () => loadDirectory(currentPath),
    },
    {
      id: 'act-nav-up',
      title: 'Navigate to Parent Folder',
      subtitle: 'Go up one directory level',
      category: 'Navigation',
      icon: '⬆️',
      shortcut: 'Alt+Up',
      run: handleNavigateUp,
    },
    {
      id: 'act-stats',
      title: 'Toggle Storage Analytics',
      subtitle: 'Open slide-over folder statistics drawer',
      category: 'Actions',
      icon: '📊',
      shortcut: 'Ctrl+S',
      run: () => { showStatsDrawer = !showStatsDrawer; },
    },
    {
      id: 'act-select-all',
      title: 'Select All Files',
      subtitle: 'Select all items in current folder',
      category: 'Selection',
      icon: '☑️',
      shortcut: 'Ctrl+A',
      run: handleSelectAll,
    },
    {
      id: 'act-deselect-all',
      title: 'Deselect All',
      subtitle: 'Clear current item selection',
      category: 'Selection',
      icon: '⏹️',
      shortcut: 'Ctrl+D',
      run: handleDeselectAll,
    },
    ...drivesAndPlaces.map((entry) => ({
      id: `drive-${entry.path}`,
      title: `Jump to ${entry.name}`,
      subtitle: entry.path,
      category: 'Drives & Places',
      icon: entry.kind === 'drive' ? '💽' : '📁',
      shortcut: '',
      run: () => loadDirectory(entry.path),
    })),
  ];

  function handleGlobalKeyDown(e: KeyboardEvent) {
    const isCtrl = e.ctrlKey || e.metaKey;

    if (isCtrl && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      isCommandPaletteOpen = !isCommandPaletteOpen;
      return;
    }

    if (e.key === 'Escape') {
      if (isCommandPaletteOpen) {
        e.preventDefault();
        isCommandPaletteOpen = false;
        return;
      }
      if (isDocModalOpen) {
        e.preventDefault();
        isDocModalOpen = false;
        return;
      }
      if (previewItem) {
        e.preventDefault();
        previewItem = null;
        return;
      }
      if (showStatsDrawer) {
        e.preventDefault();
        showStatsDrawer = false;
        return;
      }
      return;
    }

    // Ignore other shortcuts when typing in inputs/textareas
    const activeTagName = (document.activeElement?.tagName || '').toLowerCase();
    const isInputActive = activeTagName === 'input' || activeTagName === 'textarea';
    if (isInputActive) return;

    if (isCtrl && e.key.toLowerCase() === 'o') {
      e.preventDefault();
      handleBrowseFolder();
    } else if (isCtrl && e.key.toLowerCase() === 'd') {
      e.preventDefault();
      isDualPaneMode = !isDualPaneMode;
      if (isDualPaneMode && rightFolderItems.length === 0) {
        loadRightDirectory(currentPath);
      }
    } else if (isCtrl && e.key.toLowerCase() === 'r') {
      e.preventDefault();
      loadDirectory(currentPath);
      if (isDualPaneMode) loadRightDirectory(rightPath);
    } else if (isCtrl && e.key.toLowerCase() === 's') {
      e.preventDefault();
      showStatsDrawer = !showStatsDrawer;
    } else if (isCtrl && e.key >= '1' && e.key <= '7') {
      e.preventDefault();
      const tabs: ActiveTab[] = ['browser', 'content_search', 'screenshots', 'duplicates', 'treemap', 'batch_rename', 'compliance'];
      const targetTab = tabs[parseInt(e.key, 10) - 1];
      if (targetTab) activeTab = targetTab;
    } else if (isDualPaneMode && e.key === 'Tab') {
      e.preventDefault();
      activePane = activePane === 'left' ? 'right' : 'left';
    } else if (isDualPaneMode && e.key === 'F5') {
      e.preventDefault();
      handleCommanderCopy();
    } else if (isDualPaneMode && e.key === 'F6') {
      e.preventDefault();
      handleCommanderMove();
    } else if (isDualPaneMode && e.key === 'F7') {
      e.preventDefault();
      handleCommanderNewFolder();
    } else if (isDualPaneMode && e.key === 'F8') {
      e.preventDefault();
      handleCommanderDelete();
    } else if ((e.altKey && e.key === 'ArrowUp') || e.key === 'Backspace') {
      e.preventDefault();
      handleNavigateUp();
    }
  }

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

  async function loadRightDirectory(path: string) {
    try {
      const result = await invokeTauri<DirectoryListing>('list_directory_cmd', { path });
      rightListing = result;
      rightFolderItems = result.items || [];
      if (result && result.current_path) {
        rightPath = result.current_path;
      } else {
        rightPath = path;
      }
      rightSelectedPaths = [];
    } catch (err) {
      console.error('Failed to list right directory:', err);
    }
  }

  async function handleCommanderCopy() {
    commanderFeedback = '';
    const srcPaths = activePane === 'left' ? selectedPaths : rightSelectedPaths;
    const destDir = activePane === 'left' ? rightPath : currentPath;

    if (srcPaths.length === 0) {
      commanderFeedback = 'No files selected to copy. Select files in the active pane first.';
      return;
    }

    try {
      const res = await invokeTauri<CopyFilesResult>('copy_files_cmd', {
        sources: srcPaths,
        destination_dir: destDir,
      });
      commanderFeedback = `Copied ${res.copied_count} item(s) (${(res.total_bytes / 1024).toFixed(1)} KB) to ${destDir}`;
      await loadDirectory(currentPath);
      await loadRightDirectory(rightPath);
    } catch (err) {
      commanderFeedback = `Copy failed: ${err instanceof Error ? err.message : String(err)}`;
    }
  }

  async function handleCommanderMove() {
    commanderFeedback = '';
    const srcPaths = activePane === 'left' ? selectedPaths : rightSelectedPaths;
    const destDir = activePane === 'left' ? rightPath : currentPath;

    if (srcPaths.length === 0) {
      commanderFeedback = 'No files selected to move. Select files in the active pane first.';
      return;
    }

    try {
      const res = await invokeTauri<MoveFilesResult>('move_files_cmd', {
        sources: srcPaths,
        destination_dir: destDir,
      });
      commanderFeedback = `Moved ${res.moved_count} item(s) to ${destDir}`;
      if (activePane === 'left') selectedPaths = [];
      else rightSelectedPaths = [];
      await loadDirectory(currentPath);
      await loadRightDirectory(rightPath);
    } catch (err) {
      commanderFeedback = `Move failed: ${err instanceof Error ? err.message : String(err)}`;
    }
  }

  async function handleCommanderNewFolder() {
    const parentDir = activePane === 'left' ? currentPath : rightPath;
    const folderName = prompt('Enter new folder name:', 'New Folder');
    if (!folderName || !folderName.trim()) return;

    try {
      await invokeTauri<string>('create_directory_cmd', {
        parent_dir: parentDir,
        dir_name: folderName.trim(),
      });
      if (activePane === 'left') await loadDirectory(currentPath);
      else await loadRightDirectory(rightPath);
      commanderFeedback = `Created folder "${folderName.trim()}"`;
    } catch (err) {
      commanderFeedback = `Create folder failed: ${err instanceof Error ? err.message : String(err)}`;
    }
  }

  async function handleCommanderDelete() {
    const srcPaths = activePane === 'left' ? selectedPaths : rightSelectedPaths;
    if (srcPaths.length === 0) {
      commanderFeedback = 'No files selected to delete in active pane.';
      return;
    }

    const confirmed = confirm(`Are you sure you want to permanently delete ${srcPaths.length} selected item(s)?`);
    if (!confirmed) return;

    try {
      const res = await invokeTauri<DeleteFilesResult>('delete_files_cmd', {
        paths: srcPaths,
      });
      commanderFeedback = `Deleted ${res.deleted_count} item(s)`;
      if (activePane === 'left') {
        selectedPaths = [];
        await loadDirectory(currentPath);
      } else {
        rightSelectedPaths = [];
        await loadRightDirectory(rightPath);
      }
    } catch (err) {
      commanderFeedback = `Delete failed: ${err instanceof Error ? err.message : String(err)}`;
    }
  }

  async function loadSmartFolders() {
    try {
      smartFolders = await invokeTauri<SmartFolder[]>('get_smart_folders_cmd');
    } catch (err) {
      console.error('Failed to load smart folders:', err);
    }
  }

  async function selectSmartFolder(folder: SmartFolder) {
    activeSmartFolder = folder;
    isSmartFolderMode = true;
    isLoadingSmartFolder = true;
    navError = '';
    activeTab = 'browser';
    try {
      const results = await invokeTauri<FileItem[]>('query_smart_folder_cmd', {
        id: folder.id,
        path_context: currentPath,
        limit: 250,
      });
      currentFolderItems = results || [];
      listing = {
        current_path: `Smart: ${folder.name}`,
        parent_path: null,
        items: results,
        total_count: results.length,
        total_size_bytes: results.reduce((acc, it) => acc + it.size_bytes, 0),
      };
      selectedPaths = [];
      previewItem = null;
    } catch (err) {
      navError = `Smart folder query failed: ${err instanceof Error ? err.message : String(err)}`;
    } finally {
      isLoadingSmartFolder = false;
    }
  }

  function exitSmartFolderMode() {
    isSmartFolderMode = false;
    activeSmartFolder = null;
    loadDirectory(currentPath);
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
    await loadSmartFolders();
    await loadDirectory(currentPath);
    await loadRightDirectory(currentPath);
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
    bind:isDualPaneMode
    onToggleDualPane={() => {
      isDualPaneMode = !isDualPaneMode;
      if (isDualPaneMode && rightFolderItems.length === 0) {
        loadRightDirectory(currentPath);
      }
    }}
    onNavigate={loadDirectory}
    onSearch={handleSearch}
    onRefresh={() => {
      loadDirectory(currentPath);
      if (isDualPaneMode) loadRightDirectory(rightPath);
    }}
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
            class="place-item {!isSmartFolderMode && currentPath.toLowerCase().startsWith(drv.path.toLowerCase()) ? 'active' : ''}"
            on:click={() => {
              isSmartFolderMode = false;
              loadDirectory(drv.path);
            }}
            title={drv.path}
          >
            <span class="place-icon">💽</span>
            <span class="place-label">{drv.name}</span>
          </button>
        {/each}
      </div>

      <!-- Smart Virtual Folders -->
      <div class="sidebar-group">
        <div class="sidebar-title-row">
          <span class="sidebar-title">Smart Folders</span>
          {#if isSmartFolderMode}
            <button class="sidebar-clear-btn" on:click={exitSmartFolderMode} title="Return to normal view">✕ Normal</button>
          {/if}
        </div>
        {#each smartFolders as sf (sf.id)}
          <button
            class="place-item smart-folder-item {isSmartFolderMode && activeSmartFolder?.id === sf.id ? 'active-smart' : ''}"
            on:click={() => selectSmartFolder(sf)}
            title="{sf.name}: {sf.description}"
          >
            <span class="place-icon">{sf.icon}</span>
            <span class="place-label">{sf.name}</span>
          </button>
        {/each}
      </div>

      <div class="sidebar-group">
        <span class="sidebar-title">Quick Access</span>
        {#each quickAccessEntries as place (place.path)}
          <button
            class="place-item {!isSmartFolderMode && currentPath.toLowerCase() === place.path.toLowerCase() ? 'active' : ''}"
            on:click={() => {
              isSmartFolderMode = false;
              loadDirectory(place.path);
            }}
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
      {#if isSmartFolderMode && activeSmartFolder}
        <div class="smart-folder-banner glass-panel">
          <div class="smart-banner-info">
            <span class="smart-banner-icon">{activeSmartFolder.icon}</span>
            <div>
              <div class="smart-banner-title">Smart Virtual Folder: {activeSmartFolder.name}</div>
              <div class="smart-banner-desc">{activeSmartFolder.description} • {listing.total_count} matching files</div>
            </div>
          </div>
          <button class="btn btn-secondary smart-exit-btn" on:click={exitSmartFolderMode}>
            Back to Directory
          </button>
        </div>
      {/if}

      <!-- Active Tab View Surface -->
      {#if activeTab === 'browser'}
        {#if isDualPaneMode}
          <!-- Dual-Pane Commander View -->
          <div class="commander-container">
            <div class="commander-panes">
              <!-- Left Pane -->
              <div class="commander-pane {activePane === 'left' ? 'pane-active' : ''}">
                <div class="pane-path-bar">
                  <span class="pane-tag">L:</span>
                  <input
                    type="text"
                    class="pane-path-input"
                    value={currentPath}
                    on:change={(e) => loadDirectory(e.currentTarget.value)}
                  />
                  <button class="icon-btn-sm" on:click={handleNavigateUp} title="Up">⬆</button>
                  <button class="icon-btn-sm" on:click={() => loadDirectory(currentPath)} title="Refresh">↻</button>
                </div>
                <FileBrowser
                  paneTitle="Left Pane"
                  isActivePane={activePane === 'left'}
                  onActivatePane={() => (activePane = 'left')}
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
              </div>

              <!-- Right Pane -->
              <div class="commander-pane {activePane === 'right' ? 'pane-active' : ''}">
                <div class="pane-path-bar">
                  <span class="pane-tag">R:</span>
                  <input
                    type="text"
                    class="pane-path-input"
                    value={rightPath}
                    on:change={(e) => loadRightDirectory(e.currentTarget.value)}
                  />
                  <button class="icon-btn-sm" on:click={() => {
                    if (rightListing.parent_path) loadRightDirectory(rightListing.parent_path);
                  }} title="Up">⬆</button>
                  <button class="icon-btn-sm" on:click={() => loadRightDirectory(rightPath)} title="Refresh">↻</button>
                </div>
                <FileBrowser
                  paneTitle="Right Pane"
                  isActivePane={activePane === 'right'}
                  onActivatePane={() => (activePane = 'right')}
                  items={rightListing.items}
                  selectedPaths={rightSelectedPaths}
                  selectedItem={rightPreviewItem}
                  onSelectItem={(item) => (rightPreviewItem = item)}
                  onNavigateToFolder={(p) => {
                    rightPreviewItem = null;
                    loadRightDirectory(p);
                  }}
                  onNavigateUp={() => {
                    rightPreviewItem = null;
                    if (rightListing.parent_path) loadRightDirectory(rightListing.parent_path);
                  }}
                  onToggleSelect={(p) => {
                    if (rightSelectedPaths.includes(p)) {
                      rightSelectedPaths = rightSelectedPaths.filter(x => x !== p);
                    } else {
                      rightSelectedPaths = [...rightSelectedPaths, p];
                    }
                  }}
                  onSelectAll={() => {
                    rightSelectedPaths = rightListing.items.map(i => i.path);
                  }}
                  onDeselectAll={() => {
                    rightSelectedPaths = [];
                  }}
                  onInspectFile={handleInspectDoc}
                  onOpenInOs={handleOpenInOs}
                />
              </div>
            </div>

            <!-- Commander Function Keys Footer -->
            <div class="commander-footer glass-panel">
              <button class="commander-fn-btn" on:click={() => (activePane = activePane === 'left' ? 'right' : 'left')}>
                <kbd>Tab</kbd> Switch Pane
              </button>
              <button class="commander-fn-btn copy-btn" on:click={handleCommanderCopy}>
                <kbd>F5</kbd> Copy to {activePane === 'left' ? 'Right' : 'Left'}
              </button>
              <button class="commander-fn-btn move-btn" on:click={handleCommanderMove}>
                <kbd>F6</kbd> Move to {activePane === 'left' ? 'Right' : 'Left'}
              </button>
              <button class="commander-fn-btn newdir-btn" on:click={handleCommanderNewFolder}>
                <kbd>F7</kbd> New Folder
              </button>
              <button class="commander-fn-btn del-btn" on:click={handleCommanderDelete}>
                <kbd>F8</kbd> Delete
              </button>
              {#if commanderFeedback}
                <span class="commander-feedback-pill">{commanderFeedback}</span>
              {/if}
            </div>
          </div>
        {:else}
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
        {/if}
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

  <!-- Global Command Palette -->
  <CommandPalette
    isOpen={isCommandPaletteOpen}
    actions={commandActions}
    on:close={() => (isCommandPaletteOpen = false)}
  />
</main>

<svelte:window on:keydown={handleGlobalKeyDown} />

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

  /* Smart Folders */
  .smart-folder-item {
    transition: all 0.15s ease;
  }
  .smart-folder-item.active-smart {
    background: rgba(6, 182, 212, 0.22);
    border: 1px solid rgba(6, 182, 212, 0.55);
    color: var(--accent-cyan);
    font-weight: 700;
  }
  .sidebar-clear-btn {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.35);
    color: #ef4444;
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 4px;
    cursor: pointer;
  }
  .smart-folder-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 16px;
    margin-bottom: 12px;
    border-left: 3px solid var(--accent-cyan);
    background: rgba(6, 182, 212, 0.08);
  }
  .smart-banner-info {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .smart-banner-icon {
    font-size: 24px;
  }
  .smart-banner-title {
    font-weight: 700;
    font-size: 14px;
    color: var(--text-main);
  }
  .smart-banner-desc {
    font-size: 12px;
    color: var(--text-muted);
  }
  .smart-exit-btn {
    font-size: 11px;
    padding: 4px 10px;
  }

  /* Commander Dual-Pane */
  .commander-container {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
    height: 100%;
    gap: 8px;
  }
  .commander-panes {
    display: flex;
    flex: 1;
    gap: 12px;
    overflow: hidden;
  }
  .commander-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: hidden;
    border-radius: var(--radius-md);
    transition: all 0.2s ease;
  }
  .commander-pane.pane-active {
    box-shadow: 0 0 14px rgba(6, 182, 212, 0.18);
  }
  .pane-path-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(15, 23, 42, 0.85);
    border: 1px solid var(--border-subtle);
    padding: 4px 8px;
    border-radius: 6px;
  }
  .pane-tag {
    font-weight: 800;
    font-size: 12px;
    color: var(--accent-cyan);
  }
  .pane-path-input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text-main);
    font-family: var(--font-mono);
    font-size: 11px;
    outline: none;
  }
  .icon-btn-sm {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: var(--text-main);
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 11px;
    cursor: pointer;
  }
  .icon-btn-sm:hover {
    background: rgba(255, 255, 255, 0.15);
  }
  .commander-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: var(--radius-md);
    background: rgba(15, 23, 42, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }
  .commander-fn-btn {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: var(--text-main);
    border-radius: 5px;
    padding: 5px 10px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
    transition: all 0.15s ease;
  }
  .commander-fn-btn kbd {
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 3px;
    padding: 1px 4px;
    font-size: 10px;
    color: var(--accent-cyan);
  }
  .commander-fn-btn:hover {
    background: rgba(255, 255, 255, 0.14);
  }
  .commander-fn-btn.copy-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
  }
  .commander-fn-btn.move-btn:hover {
    border-color: #3b82f6;
    color: #3b82f6;
  }
  .commander-fn-btn.newdir-btn:hover {
    border-color: #10b981;
    color: #10b981;
  }
  .commander-fn-btn.del-btn:hover {
    border-color: #ef4444;
    color: #ef4444;
  }
  .commander-feedback-pill {
    margin-left: auto;
    font-size: 11px;
    color: var(--accent-cyan);
    font-family: var(--font-mono);
    background: rgba(6, 182, 212, 0.12);
    padding: 3px 8px;
    border-radius: 4px;
  }
</style>
