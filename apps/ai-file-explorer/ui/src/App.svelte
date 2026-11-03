<script lang="ts">
  import { onMount } from 'svelte';
  import Header from './components/Header.svelte';
  import FileBrowser from './components/FileBrowser.svelte';
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
    RenamePreview,
    ScreenshotFingerprint,
    ScreenshotSearchResult,
    SensitiveFinding,
    TreemapNode,
  } from './types';

  // Navigation and Tab State
  let currentPath = 'H:/rustapps';
  let searchQuery = '';
  let activeTab: ActiveTab = 'browser';
  let showStatsDrawer = false;

  // Data states
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
        }
      } catch {
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

    throw new Error(`Unhandled mock Tauri command: ${cmd}`);
  }

  async function loadDirectory(path: string) {
    currentPath = path;
    try {
      listing = await invokeTauri<DirectoryListing>('list_directory_cmd', { path });
      if (listing && listing.current_path) {
        currentPath = listing.current_path;
      }
      selectedPaths = [];
      await loadStorageStats(currentPath);
      await loadTreemap(currentPath);
    } catch (err) {
      console.error('Failed to list directory:', err);
    }
  }

  async function handleSearch(query: string) {
    searchQuery = query;
    if (!query.trim()) {
      await loadDirectory(currentPath);
      return;
    }

    try {
      const hits = await invokeTauri<FileItem[]>('search_files_cmd', {
        query,
        limit: 100,
      });
      listing = {
        ...listing,
        items: hits,
      };
    } catch (err) {
      console.error('Search error:', err);
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

  onMount(() => {
    loadDirectory(currentPath);
  });
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
  />

  <div class="main-content-view">
    <!-- Active Tab View Surface -->
    {#if activeTab === 'browser'}
      <FileBrowser
        items={listing.items}
        {selectedPaths}
        onNavigateToFolder={loadDirectory}
        onNavigateUp={handleNavigateUp}
        onToggleSelect={handleToggleSelect}
        onSelectAll={handleSelectAll}
        onDeselectAll={handleDeselectAll}
      />
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
  />
</main>

<style>
  .app-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .main-content-view {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }
</style>
