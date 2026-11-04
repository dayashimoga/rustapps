<script lang="ts">
  import { onMount } from 'svelte';
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

  // Tauri IPC Bridge
  async function invokeTauri<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
    const tauri = (window as unknown as {
      __TAURI__?: {
        invoke?: (cmd: string, args: Record<string, unknown>) => Promise<T>;
        core?: { invoke: (cmd: string, args: Record<string, unknown>) => Promise<T> };
      };
    }).__TAURI__;
    if (tauri?.core?.invoke) {
      return await tauri.core.invoke(cmd, args);
    }
    if (tauri?.invoke) {
      return await tauri.invoke(cmd, args);
    }
    if (typeof fetch !== 'undefined' && typeof window !== 'undefined' && window.location && window.location.protocol.startsWith('http')) {
      try {
        const response = await fetch('/__ipc/' + encodeURIComponent(cmd), {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(args || {}),
        });
        if (response.ok) {
          const payload = await response.json();
          if (payload && '__ipc_data' in payload) {
            return payload.__ipc_data as T;
          }
        } else {
          const errPayload = await response.json().catch(() => null);
          if (errPayload && errPayload.__ipc_error) {
            throw new Error(errPayload.__ipc_error);
          }
          throw new Error(`Command "${cmd}" failed with status ${response.status}`);
        }
      } catch (e: any) {
        if (e && e.message && !e.message.includes('Failed to fetch')) {
          throw e;
        }
        // Fallback for headless unit test environments
      }
    }

    // Comprehensive in-browser development mock fallback
    if (cmd === 'list_directory_cmd') {
      const mockItems: FileItem[] = [
        { path: `${currentPath}/shared`, file_name: 'shared', extension: null, size_bytes: 0, is_dir: true, modified_timestamp_secs: 1729300000, is_readonly: false },
        { path: `${currentPath}/apps`, file_name: 'apps', extension: null, size_bytes: 0, is_dir: true, modified_timestamp_secs: 1729300001, is_readonly: false },
        { path: `${currentPath}/docs`, file_name: 'docs', extension: null, size_bytes: 0, is_dir: true, modified_timestamp_secs: 1729300002, is_readonly: false },
        { path: `${currentPath}/Cargo.toml`, file_name: 'Cargo.toml', extension: 'toml', size_bytes: 2118, is_dir: false, modified_timestamp_secs: 1729300003, is_readonly: false },
        { path: `${currentPath}/README.md`, file_name: 'README.md', extension: 'md', size_bytes: 4096, is_dir: false, modified_timestamp_secs: 1729300004, is_readonly: false },
        { path: `${currentPath}/photo_dup_1.jpg`, file_name: 'photo_dup_1.jpg', extension: 'jpg', size_bytes: 1048576, is_dir: false, modified_timestamp_secs: 1729300005, is_readonly: false },
        { path: `${currentPath}/photo_dup_2.jpg`, file_name: 'photo_dup_2.jpg', extension: 'jpg', size_bytes: 1048576, is_dir: false, modified_timestamp_secs: 1729300006, is_readonly: false },
      ];
      return {
        current_path: currentPath,
        parent_path: 'H:/',
        items: mockItems,
        total_count: mockItems.length,
        total_size_bytes: mockItems.reduce((acc, i) => acc + i.size_bytes, 0),
      } as unknown as T;
    }

    if (cmd === 'search_files_cmd') {
      const query = (args.query as string || '').toLowerCase();
      const filtered = listing.items.filter(i => i.file_name.toLowerCase().includes(query));
      return filtered as unknown as T;
    }

    if (cmd === 'scan_duplicates_cmd') {
      return {
        total_scanned_files: 42,
        total_scanned_bytes: 25000000,
        duplicate_groups: [
          {
            file_size: 1048576,
            blake3_hash: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08',
            file_paths: [`${currentPath}/photo_dup_1.jpg`, `${currentPath}/photo_dup_2.jpg`],
            wasted_bytes: 1048576,
          },
        ],
        total_duplicate_files: 2,
        potential_reclaimed_bytes: 1048576,
        elapsed_millis: 14,
      } as unknown as T;
    }

    if (cmd === 'analyze_storage_cmd') {
      return {
        root_path: currentPath,
        total_files: 38,
        total_dirs: 12,
        total_size_bytes: 145000000,
        extension_counts: { rs: 24, svelte: 8, md: 4, toml: 2 },
        extension_sizes: { rs: 95000000, svelte: 30000000, md: 15000000, toml: 5000000 },
        largest_files: [
          { path: `${currentPath}/target/debug/app.exe`, file_name: 'app.exe', extension: 'exe', size_bytes: 45000000, is_dir: false, modified_timestamp_secs: 1729300000, is_readonly: false },
          { path: `${currentPath}/target/debug/lib.rlib`, file_name: 'lib.rlib', extension: 'rlib', size_bytes: 25000000, is_dir: false, modified_timestamp_secs: 1729300000, is_readonly: false },
        ],
      } as unknown as T;
    }

    if (cmd === 'get_treemap_cmd') {
      return {
        name: 'rustapps',
        path: currentPath,
        size_bytes: 145000000,
        is_dir: true,
        bounds: { x: 0, y: 0, width: (args.width as number) || 1024, height: (args.height as number) || 768 },
        children: [
          {
            name: 'apps',
            path: `${currentPath}/apps`,
            size_bytes: 85000000,
            is_dir: true,
            bounds: { x: 0, y: 0, width: 600, height: 768 },
            children: [],
          },
          {
            name: 'shared',
            path: `${currentPath}/shared`,
            size_bytes: 45000000,
            is_dir: true,
            bounds: { x: 600, y: 0, width: 424, height: 450 },
            children: [],
          },
          {
            name: 'docs',
            path: `${currentPath}/docs`,
            size_bytes: 15000000,
            is_dir: true,
            bounds: { x: 600, y: 450, width: 424, height: 318 },
            children: [],
          },
        ],
      } as unknown as T;
    }

    if (cmd === 'preview_batch_rename_cmd') {
      const paths = (args.paths as string[]) || [];
      const rule = args.rule as BatchRenameRule;
      const previews: RenamePreview[] = paths.map((p, idx) => {
        const parts = p.split('/');
        const originalName = parts.pop() || '';
        let newName = originalName;

        if (rule.type === 'RegexReplace') {
          try {
            const re = new RegExp(rule.config.pattern, 'g');
            newName = originalName.replace(re, rule.config.replacement);
          } catch {
            newName = originalName;
          }
        } else if (rule.type === 'PrefixSuffix') {
          const dot = originalName.lastIndexOf('.');
          const stem = dot !== -1 ? originalName.slice(0, dot) : originalName;
          const ext = dot !== -1 ? originalName.slice(dot) : '';
          newName = `${rule.config.prefix}${stem}${rule.config.suffix}${ext}`;
        } else if (rule.type === 'Numbering') {
          const dot = originalName.lastIndexOf('.');
          const ext = dot !== -1 ? originalName.slice(dot) : '';
          const numStr = (rule.config.start_number + idx).toString().padStart(rule.config.pad_digits, '0');
          newName = `${rule.config.prefix}${numStr}${ext}`;
        } else if (rule.type === 'ToLowerCase') {
          newName = originalName.toLowerCase();
        } else if (rule.type === 'ToUpperCase') {
          newName = originalName.toUpperCase();
        }

        const dir = parts.join('/');
        const proposedPath = `${dir}/${newName}`;
        return {
          original_path: p,
          proposed_path: proposedPath,
          has_conflict: false,
          conflict_reason: null,
        };
      });
      return previews as unknown as T;
    }

    if (cmd === 'execute_batch_rename_cmd') {
      const previews = (args.previews as RenamePreview[]) || [];
      return {
        total_requested: previews.length,
        successful_renames: previews.length,
        rolled_back: false,
        error_message: null,
      } as unknown as T;
    }

    if (cmd === 'delete_files_cmd') {
      const paths = (args.paths as string[]) || [];
      return {
        deleted_count: paths.length,
        reclaimed_bytes: paths.length * 1048576,
        failed_paths: [],
      } as unknown as T;
    }

    if (cmd === 'extract_document_cmd') {
      const docPath = args.path as string;
      const fileName = docPath.split('/').pop() || 'document.md';
      return {
        path: docPath,
        file_name: fileName,
        extension: 'md',
        metadata: {
          title: 'System Architecture Specification',
          author: 'Lead Architect',
          page_count: 5,
          word_count: 1420,
          format: 'Markdown Document',
          creation_date: '2026-10-19',
        },
        content_text: `# System Architecture\n\nHigh-performance offline document indexing using Tantivy and 8-bit quantized embeddings.\n\nAll cryptographic keys are scrubbed from RAM on drop via zeroize.\nStrictly zero network egress is permitted in core binaries.`,
        language: 'markdown',
        extracted_at_secs: 1729300000,
      } as unknown as T;
    }

    if (cmd === 'scan_compliance_cmd') {
      const scanPath = (args.path as string) || currentPath;
      const mockSummary: ComplianceScanSummary = {
        scanned_files_count: 18,
        findings_count: 2,
        critical_count: 1,
        high_count: 1,
        medium_count: 0,
        scan_duration_ms: 12.4,
        findings: [
          {
            id: 'find-1',
            data_type: 'private_key',
            severity: 'critical',
            file_path: `${scanPath}/id_rsa`,
            line_number: 1,
            redacted_preview: '-----BEGIN RSA PRIVATE KEY-----[REDACTED]-----END RSA PRIVATE KEY-----',
          },
          {
            id: 'find-2',
            data_type: 'ssn',
            severity: 'high',
            file_path: `${scanPath}/employees.csv`,
            line_number: 42,
            redacted_preview: 'Employee ID: 410, SSN: XXX-XX-6789, Status: Active',
          },
        ],
      };
      return mockSummary as unknown as T;
    }

    if (cmd === 'scan_screenshots_dir_cmd') {
      const mockScreenshots: ScreenshotFingerprint[] = [
        {
          path: `${currentPath}/vscode_terminal_error.png`,
          dhash: 0xa5a5_5a5a_3333_cccc,
          ocr_text: 'cargo test --workspace --all-targets\nerror: connection timeout to PostgreSQL on port 5432\nfatal: worker process aborted unexpectedly',
          dimensions: [1920, 1080],
        },
      ];
      return mockScreenshots as unknown as T;
    }

    if (cmd === 'search_screenshots_by_hash_cmd') {
      const results: ScreenshotSearchResult[] = [
        {
          path: `${currentPath}/vscode_terminal_error.png`,
          dhash: 0xa5a5_5a5a_3333_cccc,
          hamming_distance: 0,
          similarity_pct: 100.0,
          ocr_snippet: 'cargo test --workspace --all-targets\nerror: connection timeout',
        },
      ];
      return results as unknown as T;
    }

    if (cmd === 'search_screenshots_by_text_cmd') {
      const q = ((args.query as string) || '').toLowerCase();
      const results: ScreenshotSearchResult[] = [
        {
          path: `${currentPath}/vscode_terminal_error.png`,
          dhash: 0xa5a5_5a5a_3333_cccc,
          hamming_distance: 0,
          similarity_pct: 100.0,
          ocr_snippet: `cargo test --workspace (Matched: ${q})`,
        },
      ];
      return results as unknown as T;
    }

    if (cmd === 'list_drives_and_places_cmd') {
      const places: FsLocationEntry[] = [
        { name: 'Local Disk (C:)', path: 'C:\\', kind: 'drive', available: true },
        { name: 'Local Disk (H:)', path: 'H:\\', kind: 'drive', available: true },
        { name: 'Workspace', path: 'H:\\rustapps', kind: 'quick_access', available: true },
      ];
      return places as unknown as T;
    }

    if (cmd === 'index_directory_cmd' || cmd === 'open_in_os_cmd') {
      return true as unknown as T;
    }

    throw new Error(`Unhandled mock Tauri command: ${cmd}`);
  }

  async function loadDrivesAndPlaces() {
    try {
      const tauriFs = (window as unknown as {
        __TAURI__?: { fs?: { getDrivesAndPlaces?: () => Promise<FsLocationEntry[]> } };
      }).__TAURI__?.fs;
      if (tauriFs?.getDrivesAndPlaces) {
        const list = await tauriFs.getDrivesAndPlaces();
        if (list && list.length > 0) {
          drivesAndPlaces = list;
          return;
        }
      }
      drivesAndPlaces = await invokeTauri<FsLocationEntry[]>('list_drives_and_places_cmd');
    } catch (err) {
      console.error('Failed to enumerate drives and places:', err);
    }
  }

  async function handleBrowseFolder() {
    try {
      const dialog = (window as unknown as {
        __TAURI__?: { dialog?: { open?: (opts: Record<string, unknown>) => Promise<string | null> } };
      }).__TAURI__?.dialog;
      if (dialog?.open) {
        const chosen = await dialog.open({
          directory: true,
          title: 'Select Folder to Browse in AI File Explorer',
        });
        if (chosen && typeof chosen === 'string') {
          await loadDirectory(chosen);
        }
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
