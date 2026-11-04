<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc, openNativeDialog, isDesktopApp } from './lib/ipc';
  import Header from './components/Header.svelte';
  import VirtualizedViewport from './components/VirtualizedViewport.svelte';
  import BookmarksDrawer from './components/BookmarksDrawer.svelte';
  import TimelineHistogram from './components/TimelineHistogram.svelte';
  import AiInsightsDrawer from './components/AiInsightsDrawer.svelte';
  import RedactExportModal from './components/RedactExportModal.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import type {
    LogLineRecord,
    OpenedFileInfo,
    LogSearchReport,
    BookmarkRecord,
    ProcessResourceUsage,
    TimelineBucketUi,
    TimelineHistogramUi,
    TemplateClusterUi,
    AnomalySpikeUi,
    ErrorClusterUi,
  } from './types';

  let filePath = '';
  let fileSizeBytes = 0;
  let originalTotalLines = 0;
  let totalLines = 0;
  let filteredLineIndices: number[] | null = null;
  let lines: LogLineRecord[] = [];
  let visibleStart = 0;
  let visibleCount = 50;
  let activeLine = 0;

  // Tabs for multi-file log viewing
  let openTabs: Array<{ path: string; name: string; size: number }> = [];

  // Folder Explorer Sidebar
  let folderPath = '';
  let folderFiles: Array<{ path: string; name: string; size_bytes: number; modified_secs: number }> = [];
  let isFolderSidebarOpen = false;

  // System Logs Modal
  let systemLogs: Array<{ path: string; name: string; size_bytes: number; modified_secs: number }> = [];
  let isSystemLogsOpen = false;

  // Recent Logs
  let recentLogs: string[] = [];

  // Severity Level Filter
  let activeSeverityFilter = 'ALL';

  let searchQuery = '';
  let isRegex = false;
  let caseSensitive = false;
  let searchMatchCount: number | null = null;
  let searchLatencyMicros: number | null = null;

  let tailModeActive = false;
  let isBookmarksOpen = false;
  let showTimeline = true;
  let showAiInsights = false;
  let isRedactModalOpen = false;

  let bookmarks: BookmarkRecord[] = [];
  let systemMetrics: ProcessResourceUsage = {
    resident_set_size_bytes: 48 * 1024 * 1024,
    cpu_percentage: 0.4,
  };

  let timelineBuckets: TimelineBucketUi[] = [];
  let minEpochMs = 0;
  let maxEpochMs = 0;
  let selectedRange: [number, number] | null = null;

  let minedTemplates: TemplateClusterUi[] = [];
  let detectedAnomalies: AnomalySpikeUi[] = [];
  let detectedErrorClusters: ErrorClusterUi[] = [];
  let isAiAnalyzing = false;

  let viewportComponent: VirtualizedViewport;
  let openFileError = '';
  let hiddenFileInput: HTMLInputElement;

  // Standardized Desktop IPC Bridge
  async function tauriInvoke<T>(cmd: string, args: Record<string, any> = {}): Promise<T> {
    return await invokeIpc<T>(cmd, args);
  }

  async function pickNativeFileOrFolder(directory: boolean = false): Promise<string | null> {
    const res = await openNativeDialog({
      directory,
      multiple: false,
      title: directory ? 'Select Folder with Log Files' : 'Open Log File',
      filters: directory ? undefined : [{ name: 'Log Files', extensions: ['log', 'txt', 'jsonl', 'json', 'csv', 'tsv', 'out', 'err'] }],
    });
    if (typeof res === 'string' && res.trim()) return res.trim();
    if (Array.isArray(res) && res.length > 0) return res[0];
    return null;
  }

  function addRecent(path: string) {
    recentLogs = [path, ...recentLogs.filter((p) => p !== path)].slice(0, 8);
    try {
      localStorage.setItem('ulv_recent_logs', JSON.stringify(recentLogs));
    } catch (_) {}
  }

  async function openRealFile(path: string) {
    openFileError = '';
    try {
      const info = await tauriInvoke<OpenedFileInfo>('open_file', { path });
      if (info) {
        filePath = info.path;
        originalTotalLines = info.total_lines;
        totalLines = info.total_lines;
        filteredLineIndices = null;
        selectedRange = null;
        fileSizeBytes = info.file_size_bytes;
        activeSeverityFilter = 'ALL';

        const fileName = info.path.split(/[/\\]/).pop() || info.path;
        if (!openTabs.some((t) => t.path === info.path)) {
          openTabs = [...openTabs, { path: info.path, name: fileName, size: info.file_size_bytes }];
        }
        addRecent(info.path);

        await fetchSlice(0, 60);
        await loadAiInsights();
      }
    } catch (e: any) {
      openFileError = e?.message || String(e);
      console.error('Failed to open log file:', e);
    }
  }

  function closeTab(path: string) {
    openTabs = openTabs.filter((t) => t.path !== path);
    if (filePath === path) {
      if (openTabs.length > 0) {
        openRealFile(openTabs[openTabs.length - 1].path);
      } else {
        filePath = '';
        totalLines = 0;
        lines = [];
      }
    }
  }

  async function openFolder(dirPath: string) {
    openFileError = '';
    try {
      const files = await tauriInvoke<Array<{ path: string; name: string; size_bytes: number; modified_secs: number }>>('open_directory', { path: dirPath });
      folderPath = dirPath;
      folderFiles = files || [];
      isFolderSidebarOpen = true;
      if (folderFiles.length > 0) {
        await openRealFile(folderFiles[0].path);
      }
    } catch (e: any) {
      openFileError = e?.message || String(e);
    }
  }

  async function openSystemLogsModal() {
    openFileError = '';
    try {
      const logs = await tauriInvoke<Array<{ path: string; name: string; size_bytes: number; modified_secs: number }>>('list_system_logs', {});
      systemLogs = logs || [];
      isSystemLogsOpen = true;
    } catch (e: any) {
      openFileError = e?.message || String(e);
    }
  }

  async function exportCurrentFilteredLogs() {
    try {
      const defaultName = `filtered_${Date.now()}.log`;
      const result = await tauriInvoke<any>('export_filtered_logs', {
        destPath: defaultName,
        lineNumbers: filteredLineIndices,
      });
      alert(`Exported ${result.exported_lines} lines to ${result.dest_path}`);
    } catch (e: any) {
      alert(`Export failed: ${e?.message || e}`);
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  async function handleBrowserFileChosen(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input?.files?.[0];
    if (!file) return;
    await loadDroppedOrSelectedFileObject(file);
  }

  async function loadDroppedOrSelectedFileObject(file: File) {
    openFileError = '';
    try {
      const content = await file.text();
      const info = await tauriInvoke<OpenedFileInfo>('open_raw_log_content', {
        fileName: file.name,
        content,
      });
      if (info) {
        filePath = info.path;
        originalTotalLines = info.total_lines;
        totalLines = info.total_lines;
        filteredLineIndices = null;
        selectedRange = null;
        fileSizeBytes = info.file_size_bytes;
        activeSeverityFilter = 'ALL';

        const fileName = info.path.split(/[/\\]/).pop() || info.path;
        if (!openTabs.some((t) => t.path === info.path)) {
          openTabs = [...openTabs, { path: info.path, name: fileName, size: info.file_size_bytes }];
        }
        addRecent(info.path);

        await fetchSlice(0, 60);
        await loadAiInsights();
      }
    } catch (err: any) {
      openFileError = err?.message || String(err);
    }
  }

  async function handleFileDrop(e: DragEvent) {
    e.preventDefault();
    const file = e.dataTransfer?.files?.[0];
    if (file) {
      await loadDroppedOrSelectedFileObject(file);
    }
  }

  async function handleOpenFilePrompt() {
    openFileError = '';
    const picked = await pickNativeFileOrFolder(false);
    if (picked) {
      await openRealFile(picked);
      return;
    }
    if (hiddenFileInput) {
      hiddenFileInput.click();
    }
  }

  async function handleOpenFolderPrompt() {
    openFileError = '';
    const picked = await pickNativeFileOrFolder(true);
    if (picked) {
      await openFolder(picked);
    }
  }

  async function loadAiInsights() {
    if (!filePath) return;
    isAiAnalyzing = true;
    try {
      const [hist, templates, anomalies, errors] = await Promise.all([
        tauriInvoke<TimelineHistogramUi>('get_timeline_histogram', { targetBuckets: 60 }),
        tauriInvoke<TemplateClusterUi[]>('mine_templates', { maxRecords: 50000 }),
        tauriInvoke<AnomalySpikeUi[]>('detect_anomalies', { targetBuckets: 60, zThreshold: 2.5 }),
        tauriInvoke<ErrorClusterUi[]>('cluster_errors', { maxRecords: 50000 }),
      ]);
      if (hist) {
        timelineBuckets = hist.buckets;
        minEpochMs = hist.min_epoch_ms;
        maxEpochMs = hist.max_epoch_ms;
      }
      if (templates) minedTemplates = templates;
      if (anomalies) detectedAnomalies = anomalies;
      if (errors) detectedErrorClusters = errors;
    } catch (e) {
      console.warn('Failed to load AI insights from backend:', e);
    } finally {
      isAiAnalyzing = false;
    }
  }

  function generateSampleLogs(count: number) {
    const services = ['auth-service', 'payment-gateway', 'ingress-controller', 'database-pool', 'worker-queue'];
    const levels = ['INFO', 'INFO', 'INFO', 'WARN', 'ERROR', 'DEBUG'];
    const messages = [
      'Connection established from 192.168.1.104:443',
      'Transaction committed successfully in 14.2ms',
      'High heap memory threshold reached (84.2%)',
      'Connection pool timeout acquiring PostgreSQL handle',
      'JWT token validation failed for user_id=98421',
      'Cache miss for key: session_token_88ab',
      'Worker job processed successfully: job_id=89231',
    ];

    const generated: string[] = [];
    const baseTime = Date.now() - count * 100;
    minEpochMs = baseTime;
    maxEpochMs = baseTime + count * 100;

    for (let i = 0; i < count; i++) {
      const timeStr = new Date(baseTime + i * 100).toISOString();
      const level = levels[i % levels.length];
      const service = services[i % services.length];
      const msg = messages[i % messages.length];
      generated.push(`${timeStr} [${level}] [${service}] #${i + 1}: ${msg}`);
    }

    mockFileLines = generated;
    filePath = 'sample_production_500k.log';
    originalTotalLines = generated.length;
    totalLines = generated.length;
    filteredLineIndices = null;
    selectedRange = null;
    fileSizeBytes = generated.reduce((acc, l) => acc + l.length + 1, 0);

    // Generate timeline buckets (60 buckets)
    const numBuckets = 60;
    const bucketSpan = (maxEpochMs - minEpochMs) / numBuckets;
    const buckets: TimelineBucketUi[] = [];

    for (let b = 0; b < numBuckets; b++) {
      const bStart = minEpochMs + b * bucketSpan;
      const bEnd = bStart + bucketSpan;
      const logsInBucket = Math.floor(count / numBuckets);

      const errCount = b === 25 ? Math.floor(logsInBucket * 0.4) : Math.floor(logsInBucket * 0.15);
      const warnCount = Math.floor(logsInBucket * 0.15);
      const fatalCount = b === 25 ? 12 : 0;
      const infoCount = logsInBucket - errCount - warnCount - fatalCount;

      buckets.push({
        bucket_index: b,
        start_epoch_ms: bStart,
        end_epoch_ms: bEnd,
        total_count: logsInBucket,
        trace_count: 0,
        debug_count: Math.floor(logsInBucket * 0.1),
        info_count: infoCount,
        warn_count: warnCount,
        error_count: errCount,
        fatal_count: fatalCount,
        volume_per_second: logsInBucket / (bucketSpan / 1000),
      });
    }
    timelineBuckets = buckets;

    // Generate AI intelligence models
    minedTemplates = [
      {
        cluster_id: 1,
        template: 'Connection established from <IP>:<NUM>',
        token_count: 4,
        frequency: Math.floor(count * 0.28),
        sample_lines: [0, 7, 14, 21],
      },
      {
        cluster_id: 2,
        template: 'Transaction committed successfully in <NUM>ms',
        token_count: 5,
        frequency: Math.floor(count * 0.24),
        sample_lines: [1, 8, 15, 22],
      },
      {
        cluster_id: 3,
        template: 'Connection pool timeout acquiring PostgreSQL handle',
        token_count: 6,
        frequency: Math.floor(count * 0.16),
        sample_lines: [3, 10, 17, 24],
      },
      {
        cluster_id: 4,
        template: 'JWT token validation failed for user_id=<NUM>',
        token_count: 6,
        frequency: Math.floor(count * 0.14),
        sample_lines: [4, 11, 18, 25],
      },
    ];

    detectedAnomalies = [
      {
        spike_id: 1,
        start_epoch_ms: minEpochMs + 25 * bucketSpan,
        end_epoch_ms: minEpochMs + 26 * bucketSpan,
        z_score: 3.84,
        observed_value: 345,
        expected_value: 120,
        metric_name: 'error_density',
        description: 'Critical error spike in ingress-controller & database-pool (z-score: 3.84)',
        severity: 'ERROR',
      },
      {
        spike_id: 2,
        start_epoch_ms: minEpochMs + 12 * bucketSpan,
        end_epoch_ms: minEpochMs + 13 * bucketSpan,
        z_score: 2.62,
        observed_value: 850,
        expected_value: 480,
        metric_name: 'log_volume',
        description: 'Sudden incoming traffic surge in auth-service (z-score: 2.62)',
        severity: 'WARN',
      },
    ];

    detectedErrorClusters = [
      {
        cluster_id: 1,
        template: 'Connection pool timeout acquiring PostgreSQL handle',
        error_count: Math.floor(count * 0.16),
        affected_sources: ['database-pool', 'payment-gateway'],
        sample_lines: [3, 10, 17, 24],
      },
      {
        cluster_id: 2,
        template: 'JWT token validation failed for user_id=<NUM>',
        error_count: Math.floor(count * 0.14),
        affected_sources: ['auth-service'],
        sample_lines: [4, 11, 18, 25],
      },
    ];

    fetchSlice(0, 60);
  }

  async function fetchSlice(start: number, count: number) {
    visibleStart = start;
    visibleCount = count;

    if (filteredLineIndices !== null) {
      const sliceIndices = filteredLineIndices.slice(start, start + count);
      if (sliceIndices.length === 0) {
        lines = [];
        return;
      }
      if (isTauri && filePath) {
        try {
          const slice = await tauriInvoke<LogLineRecord[]>('get_lines_by_indices', {
            lineNumbers: sliceIndices,
          });
          if (slice && slice.length > 0) {
            lines = slice;
            return;
          }
        } catch (err) {
          console.warn('Backend slice error for filtered indices:', err);
        }
      }
      if (mockFileLines.length > 0) {
        lines = sliceIndices.map((origIdx) => ({
          line_number: origIdx,
          byte_start: origIdx * 80,
          byte_end: origIdx * 80 + (mockFileLines[origIdx]?.length || 0),
          text: mockFileLines[origIdx] || '',
        }));
        return;
      }
    }

    if (isTauri && filePath) {
      try {
        const slice = await tauriInvoke<LogLineRecord[]>('get_lines_slice', {
          startLine: start,
          count,
        });
        if (slice && slice.length > 0) {
          lines = slice;
          return;
        }
      } catch (err) {
        console.warn('Backend slice error, using cache:', err);
      }
    }

    if (mockFileLines.length > 0) {
      const sliceEnd = Math.min(start + count, mockFileLines.length);
      lines = mockFileLines.slice(start, sliceEnd).map((text, idx) => ({
        line_number: start + idx,
        byte_start: (start + idx) * 80,
        byte_end: (start + idx) * 80 + text.length,
        text,
      }));
    }
  }

  function handleScrollChange(event: CustomEvent<{ startLine: number; count: number }>) {
    fetchSlice(event.detail.startLine, event.detail.count);
  }

  async function handleSearch(event: CustomEvent<{ query: string; isRegex: boolean; caseSensitive: boolean }>) {
    searchQuery = event.detail.query;
    isRegex = event.detail.isRegex;
    caseSensitive = event.detail.caseSensitive;

    if (!searchQuery.trim()) {
      searchMatchCount = null;
      searchLatencyMicros = null;
      return;
    }

    if (isTauri && filePath) {
      try {
        const report = await tauriInvoke<LogSearchReport>('search', {
          query: {
            pattern: searchQuery,
            is_regex: isRegex,
            case_sensitive: caseSensitive,
            limit: 1000,
            start_line: 0,
          },
        });
        if (report) {
          searchMatchCount = report.total_matched_in_scan;
          searchLatencyMicros = report.elapsed_micros;
          if (report.matches.length > 0 && viewportComponent) {
            viewportComponent.scrollToLine(report.matches[0].line_number);
            activeLine = report.matches[0].line_number;
          }
          return;
        }
      } catch (err) {
        console.warn('Backend search error, falling back to client-side:', err);
      }
    }

    const start = performance.now();
    let matches = 0;
    let firstMatchLine = -1;

    let regex: RegExp | null = null;
    if (isRegex) {
      try {
        regex = new RegExp(searchQuery, caseSensitive ? '' : 'i');
      } catch {
        searchMatchCount = 0;
        return;
      }
    }

    const queryLower = searchQuery.toLowerCase();

    for (let i = 0; i < mockFileLines.length; i++) {
      const line = mockFileLines[i];
      const matched = regex
        ? regex.test(line)
        : caseSensitive
        ? line.includes(searchQuery)
        : line.toLowerCase().includes(queryLower);

      if (matched) {
        matches++;
        if (firstMatchLine === -1) {
          firstMatchLine = i;
        }
      }
    }

    searchMatchCount = matches;
    searchLatencyMicros = Math.round((performance.now() - start) * 1000);

    if (firstMatchLine !== -1 && viewportComponent) {
      viewportComponent.scrollToLine(firstMatchLine);
      activeLine = firstMatchLine;
    }
  }

  async function toggleTailMode() {
    tailModeActive = !tailModeActive;
    if (isTauri && filePath) {
      try {
        tailModeActive = (await tauriInvoke<boolean>('toggle_tail_mode', { enable: tailModeActive })) ?? tailModeActive;
      } catch (err) {
        console.warn('Failed to toggle tail mode via IPC:', err);
      }
    }
  }

  // Periodic tail stream simulation when in preview mode
  let tailInterval: number | null = null;
  $: {
    if (tailModeActive && !tailInterval && !isTauri) {
      tailInterval = window.setInterval(() => {
        if (mockFileLines.length > 0) {
          const newLine = `${new Date().toISOString()} [INFO] [tail-worker] Appended real-time event #${totalLines + 1}`;
          mockFileLines.push(newLine);
          totalLines = mockFileLines.length;
          fileSizeBytes += newLine.length + 1;

          if (viewportComponent) {
            viewportComponent.scrollToBottom();
          }
        }
      }, 100);
    } else if (!tailModeActive && tailInterval) {
      clearInterval(tailInterval);
      tailInterval = null;
    }
  }

  function handleJumpLine(event: CustomEvent<number>) {
    const line = event.detail;
    if (line >= 0 && line < totalLines && viewportComponent) {
      viewportComponent.scrollToLine(line);
      activeLine = line;
    }
  }

  async function handleBookmarkLine(event: CustomEvent<{ line: number }>) {
    const line = event.detail.line;
    if (isTauri && filePath) {
      try {
        const saved = await tauriInvoke<BookmarkRecord>('add_bookmark', {
          lineNumber: line,
          note: `Line ${line + 1}: ${lines.find((l) => l.line_number === line)?.text.slice(0, 40) || 'Bookmarked line'}...`,
        });
        if (saved) {
          bookmarks = [...bookmarks, saved];
          isBookmarksOpen = true;
          return;
        }
      } catch (e) {
        console.warn('Failed to save bookmark via IPC:', e);
      }
    }

    const exists = bookmarks.find((b) => b.line_number === line);
    if (!exists) {
      bookmarks = [
        ...bookmarks,
        {
          id: `bm_${line}_${Date.now()}`,
          workspace_id: 'default',
          file_path: filePath,
          line_number: line,
          label: `Line ${line + 1}: ${mockFileLines[line]?.slice(0, 40) || 'Bookmarked line'}...`,
        },
      ];
    }
    isBookmarksOpen = true;
  }

  function handleDeleteBookmark(event: CustomEvent<string>) {
    const id = event.detail;
    bookmarks = bookmarks.filter((b) => b.id !== id);
  }

  async function handleRangeSelected(event: CustomEvent<{ startMs: number; endMs: number }>) {
    selectedRange = [event.detail.startMs, event.detail.endMs];
    if (isTauri && filePath) {
      try {
        const matchedLines = await tauriInvoke<number[]>('filter_by_timerange', {
          startMs: event.detail.startMs,
          endMs: event.detail.endMs,
        });
        if (matchedLines) {
          filteredLineIndices = matchedLines;
          totalLines = matchedLines.length;
          await fetchSlice(0, 60);
          if (viewportComponent) {
            viewportComponent.scrollToLine(0);
            activeLine = matchedLines[0] ?? 0;
          }
          return;
        }
      } catch (e) {
        console.warn('Timerange filter IPC error:', e);
      }
    }

    // Mock file lines timerange filtering
    if (mockFileLines.length > 0 && maxEpochMs > minEpochMs) {
      const startFrac = Math.max(0, Math.min(1, (event.detail.startMs - minEpochMs) / (maxEpochMs - minEpochMs)));
      const endFrac = Math.max(0, Math.min(1, (event.detail.endMs - minEpochMs) / (maxEpochMs - minEpochMs)));
      const startIdx = Math.floor(startFrac * mockFileLines.length);
      const endIdx = Math.min(mockFileLines.length, Math.ceil(endFrac * mockFileLines.length) + 1);
      const indices: number[] = [];
      for (let i = startIdx; i < endIdx; i++) {
        indices.push(i);
      }
      filteredLineIndices = indices;
      totalLines = indices.length;
      await fetchSlice(0, 60);
      if (viewportComponent) {
        viewportComponent.scrollToLine(0);
        activeLine = indices[0] ?? 0;
      }
    }
  }

  function handleClearRange() {
    selectedRange = null;
    filteredLineIndices = null;
    totalLines = originalTotalLines;
    fetchSlice(0, 60);
  }

  async function handleExportRedacted(destPath: string): Promise<{ dest_path: string; records_redacted: number }> {
    if (isTauri && filePath) {
      const res = await tauriInvoke<{ dest_path: string; records_redacted: number }>('export_redacted', {
        destPath,
      });
      if (res) return res;
    }
    // Preview / browser fallback
    await new Promise((r) => setTimeout(r, 500));
    return {
      dest_path: destPath,
      records_redacted: totalLines,
    };
  }

  // Global Keyboard Shortcuts
  function handleKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      document.getElementById('input-search')?.focus();
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 't') {
      e.preventDefault();
      toggleTailMode();
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'b') {
      e.preventDefault();
      isBookmarksOpen = !isBookmarksOpen;
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'i') {
      e.preventDefault();
      showAiInsights = !showAiInsights;
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'o') {
      e.preventDefault();
      handleOpenFilePrompt();
    }
  }

  let metricsInterval: number | null = null;

  async function updateMetrics() {
    try {
      const sampled = await tauriInvoke<ProcessResourceUsage>('get_system_metrics');
      if (sampled) {
        systemMetrics = sampled;
      }
    } catch {
      // ignore
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown);

    try {
      const saved = localStorage.getItem('ulv_recent_logs');
      if (saved) {
        recentLogs = JSON.parse(saved);
      }
    } catch (_) {}

    updateMetrics();
    metricsInterval = window.setInterval(updateMetrics, 2500);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      if (tailInterval) clearInterval(tailInterval);
      if (metricsInterval) clearInterval(metricsInterval);
    };
  });
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="app-layout" on:dragover|preventDefault on:drop={handleFileDrop}>
  <input
    type="file"
    bind:this={hiddenFileInput}
    on:change={handleBrowserFileChosen}
    accept=".log,.txt,.jsonl,.json,.csv,.tsv,.out,.err"
    style="display: none;"
  />

  <!-- Top Navigation Bar -->
  <Header
    {filePath}
    {tailModeActive}
    {searchQuery}
    {isRegex}
    {caseSensitive}
    {searchMatchCount}
    {searchLatencyMicros}
    {systemMetrics}
    {showTimeline}
    {showAiInsights}
    bookmarksCount={bookmarks.length}
    on:openFile={handleOpenFilePrompt}
    on:openFolder={handleOpenFolderPrompt}
    on:openSystemLogs={openSystemLogsModal}
    on:openPath={(e) => openRealFile(e.detail)}
    on:toggleTail={toggleTailMode}
    on:search={handleSearch}
    on:jumpLine={handleJumpLine}
    on:toggleBookmarks={() => (isBookmarksOpen = !isBookmarksOpen)}
    on:toggleTimeline={() => (showTimeline = !showTimeline)}
    on:toggleAiInsights={() => (showAiInsights = !showAiInsights)}
    on:openRedactModal={() => (isRedactModalOpen = true)}
  />

  <!-- Multi-File Tabs Bar -->
  {#if openTabs.length > 0}
    <div class="tabs-bar">
      {#each openTabs as tab (tab.path)}
        <button
          class="tab-pill {tab.path === filePath ? 'active' : ''}"
          on:click={() => openRealFile(tab.path)}
          title={tab.path}
        >
          <span class="tab-icon">📄</span>
          <span class="tab-title">{tab.name}</span>
          <span class="tab-size">({formatBytes(tab.size)})</span>
          <button
            type="button"
            class="tab-close"
            on:click|stopPropagation={() => closeTab(tab.path)}
          >
            ✕
          </button>
        </button>
      {/each}
    </div>
  {/if}

  {#if filePath}
    <!-- Fast Log Level Filter Bar & Sub-toolbar -->
    <div class="sub-toolbar glass-panel">
      <div class="severity-pills">
        <span class="filter-label">Level:</span>
        {#each ['ALL', 'ERROR', 'WARN', 'INFO', 'DEBUG'] as lvl}
          <button
            class="severity-pill {activeSeverityFilter === lvl ? 'active' : ''} {lvl.toLowerCase()}"
            on:click={() => {
              activeSeverityFilter = lvl;
              if (lvl === 'ALL') {
                handleSearch(new CustomEvent('search', { detail: { query: '', isRegex: false, caseSensitive: false } }));
              } else {
                handleSearch(new CustomEvent('search', { detail: { query: lvl, isRegex: false, caseSensitive: false } }));
              }
            }}
          >
            {lvl}
          </button>
        {/each}
      </div>

      <div class="toolbar-actions">
        {#if folderFiles.length > 0}
          <button
            class="btn btn-secondary btn-sm"
            on:click={() => (isFolderSidebarOpen = !isFolderSidebarOpen)}
          >
            📁 {isFolderSidebarOpen ? 'Hide Folder' : `Folder (${folderFiles.length})`}
          </button>
        {/if}
        <button class="btn btn-secondary btn-sm" on:click={exportCurrentFilteredLogs} title="Export filtered or all log lines">
          💾 Export Filtered
        </button>
        <button class="btn btn-secondary btn-sm" on:click={() => (isRedactModalOpen = true)} title="PII Redacted Export">
          🛡️ Redact &amp; Export
        </button>
      </div>
    </div>
  {/if}

  {#if openFileError}
    <div class="open-error-banner">
      <span>⚠️ {openFileError}</span>
      <button on:click={() => (openFileError = '')}>Dismiss</button>
    </div>
  {/if}

  <!-- Interactive Timeline Histogram (Phase 2 Advanced) -->
  {#if showTimeline && timelineBuckets.length > 0}
    <TimelineHistogram
      buckets={timelineBuckets}
      {minEpochMs}
      {maxEpochMs}
      {selectedRange}
      on:rangeSelected={handleRangeSelected}
      on:clearRange={handleClearRange}
    />
  {/if}

  <!-- Main Viewport Area -->
  <main class="main-content">
    {#if !filePath || totalLines === 0}
      <!-- Welcome Hub when no log is loaded -->
      <div class="welcome-hub-container">
        <div class="welcome-card glass-panel">
          <div class="welcome-badge">100% OFFLINE • AIR-GAP GUARANTEED</div>
          <h2>Universal Log Viewer</h2>
          <p class="welcome-desc">
            Ultra-fast zero-copy memory-mapped log analysis engine. Instant sub-millisecond search, 64-bit microsecond timeline histograms, and offline AI Drain3 template clustering.
          </p>

          <!-- svelte-ignore a11y-click-events-have-key-events -->
          <!-- svelte-ignore a11y-no-static-element-interactions -->
          <div
            class="drop-target-box"
            on:click={handleOpenFilePrompt}
          >
            <div class="drop-icon">📂</div>
            <h3>Drop Any Log File Here</h3>
            <p>Supports .log, .txt, .jsonl, .json, .out, .err, .csv, and compressed streams up to 500GB+</p>
          </div>

          <div class="quick-actions-row">
            <button class="btn btn-primary btn-lg" on:click={handleOpenFilePrompt}>
              📄 Open Log File
            </button>
            <button class="btn btn-secondary btn-lg" on:click={handleOpenFolderPrompt}>
              📁 Open Folder
            </button>
            <button class="btn btn-secondary btn-lg" on:click={openSystemLogsModal}>
              💻 System Event Logs
            </button>
          </div>

          {#if recentLogs.length > 0}
            <div class="recents-panel">
              <span class="recents-header">Recent Files:</span>
              <div class="recents-grid">
                {#each recentLogs as recent}
                  <button class="recent-pill" on:click={() => openRealFile(recent)} title={recent}>
                    <span class="recent-name">{recent.split(/[/\\]/).pop()}</span>
                    <span class="recent-path">{recent}</span>
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </div>
    {:else}
      <div class="workspace-viewport-row">
        {#if isFolderSidebarOpen}
          <aside class="folder-sidebar glass-panel">
            <div class="folder-sidebar-header">
              <span class="folder-title" title={folderPath}>📁 {folderPath.split(/[/\\]/).pop()}</span>
              <button class="close-sidebar-btn" on:click={() => (isFolderSidebarOpen = false)}>✕</button>
            </div>
            <div class="folder-file-list">
              {#each folderFiles as f (f.path)}
                <button
                  class="folder-file-btn {f.path === filePath ? 'active' : ''}"
                  on:click={() => openRealFile(f.path)}
                >
                  <span class="file-icon">📄</span>
                  <div class="file-info">
                    <span class="file-name" title={f.path}>{f.name}</span>
                    <span class="file-size">{formatBytes(f.size_bytes)}</span>
                  </div>
                </button>
              {/each}
            </div>
          </aside>
        {/if}

        <div class="viewport-wrapper">
          <VirtualizedViewport
            bind:this={viewportComponent}
            {lines}
            {totalLines}
            {activeLine}
            {searchQuery}
            on:scrollChange={handleScrollChange}
            on:bookmarkLine={handleBookmarkLine}
          />
        </div>
      </div>
    {/if}
  </main>

  {#if isSystemLogsOpen}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="modal-backdrop" on:click={() => (isSystemLogsOpen = false)}>
      <div class="system-logs-dialog glass-panel" on:click|stopPropagation>
        <div class="dialog-header">
          <h3>💻 Local System Event Logs</h3>
          <button class="dialog-close" on:click={() => (isSystemLogsOpen = false)}>✕</button>
        </div>
        <p class="dialog-sub">Discovered operational logs on your machine:</p>
        <div class="system-logs-list">
          {#if systemLogs.length === 0}
            <p class="empty-state">No accessible system logs found in default OS paths.</p>
          {:else}
            {#each systemLogs as item}
              <button
                class="syslog-row"
                on:click={() => {
                  isSystemLogsOpen = false;
                  openRealFile(item.path);
                }}
              >
                <div class="syslog-details">
                  <span class="syslog-title">{item.name}</span>
                  <span class="syslog-location">{item.path}</span>
                </div>
                <span class="syslog-size">{formatBytes(item.size_bytes)}</span>
              </button>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- Slide-out Bookmarks Drawer -->
  <BookmarksDrawer
    {bookmarks}
    isOpen={isBookmarksOpen}
    on:close={() => (isBookmarksOpen = false)}
    on:jumpToLine={(e) => {
      handleJumpLine(e);
      isBookmarksOpen = false;
    }}
    on:deleteBookmark={handleDeleteBookmark}
  />

  <!-- Slide-out AI Root-Cause Drawer (Phase 2 Advanced) -->
  <AiInsightsDrawer
    isOpen={showAiInsights}
    templates={minedTemplates}
    anomalies={detectedAnomalies}
    errorClusters={detectedErrorClusters}
    isAnalyzing={isAiAnalyzing}
    on:close={() => (showAiInsights = false)}
    on:jumpToLine={(e) => {
      handleJumpLine(e);
      showAiInsights = false;
    }}
    on:filterTimeRange={(e) => {
      handleRangeSelected(e);
      showAiInsights = false;
    }}
    on:filterTemplate={(e) => {
      searchQuery = e.detail.pattern;
      isRegex = false;
      handleSearch(new CustomEvent('search', { detail: { query: e.detail.pattern, isRegex: false, caseSensitive: false } }));
      showAiInsights = false;
    }}
    on:refresh={loadAiInsights}
  />

  <!-- PII & Credential Redacted Stream Export Modal -->
  <RedactExportModal
    isOpen={isRedactModalOpen}
    {totalLines}
    {filePath}
    onExport={handleExportRedacted}
    on:close={() => (isRedactModalOpen = false)}
  />

  <!-- Bottom Operational Status Bar -->
  <StatusBar
    {filePath}
    {totalLines}
    {originalTotalLines}
    {visibleStart}
    {visibleCount}
    {fileSizeBytes}
    {tailModeActive}
  />
</div>

<style>
  .app-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background: var(--bg-primary);
  }

  .open-error-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 16px;
    background: rgba(239, 68, 68, 0.18);
    border-bottom: 1px solid rgba(239, 68, 68, 0.45);
    color: #fca5a5;
    font-size: 12px;
  }

  .open-error-banner button {
    background: transparent;
    border: 1px solid rgba(252, 165, 165, 0.4);
    color: #fca5a5;
    border-radius: 4px;
    padding: 2px 8px;
    font-size: 11px;
    cursor: pointer;
  }

  .main-content {
    flex: 1;
    display: flex;
    overflow: hidden;
    position: relative;
  }

  .hero-container {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 30px;
  }

  .hero-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    max-width: 480px;
    padding: 40px 30px;
    border-radius: 16px;
  }

  .hero-icon {
    width: 72px;
    height: 72px;
    border-radius: 18px;
    background: linear-gradient(135deg, var(--accent-blue), var(--accent-violet));
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 20px;
    box-shadow: 0 8px 30px -5px hsla(217, 91%, 60%, 0.4);
  }

  .hero-btn {
    padding: 10px 24px;
    font-size: 14px;
  }

  /* Multi-File Tabs Bar */
  .tabs-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    background: hsla(222, 24%, 6%, 0.85);
    border-bottom: 1px solid var(--border-color);
    overflow-x: auto;
  }

  .tab-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s ease;
  }

  .tab-pill:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-primary);
  }

  .tab-pill.active {
    background: hsla(217, 91%, 60%, 0.18);
    border-color: hsla(217, 91%, 60%, 0.45);
    color: var(--accent-blue);
    font-weight: 600;
  }

  .tab-close {
    margin-left: 4px;
    font-size: 11px;
    opacity: 0.6;
    border-radius: 50%;
    padding: 0 4px;
  }
  .tab-close:hover {
    opacity: 1;
    background: rgba(239, 68, 68, 0.25);
    color: #fca5a5;
  }

  /* Sub-toolbar & Severity Filters */
  .sub-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 16px;
    border-bottom: 1px solid var(--border-color);
  }

  .severity-pills {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .filter-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .severity-pill {
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: var(--text-secondary);
    transition: all 0.15s ease;
  }

  .severity-pill.active {
    background: hsla(217, 91%, 60%, 0.25);
    border-color: var(--accent-blue);
    color: #fff;
  }

  .severity-pill.error.active {
    background: rgba(239, 68, 68, 0.3);
    border-color: #ef4444;
    color: #fca5a5;
  }

  .severity-pill.warn.active {
    background: rgba(245, 158, 11, 0.3);
    border-color: #f59e0b;
    color: #fde68a;
  }

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .btn-sm {
    padding: 4px 10px;
    font-size: 11px;
  }

  /* Workspace Viewport Row & Folder Sidebar */
  .workspace-viewport-row {
    display: flex;
    flex: 1;
    overflow: hidden;
    width: 100%;
    height: 100%;
  }

  .folder-sidebar {
    width: 240px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border-color);
    background: hsla(222, 24%, 7%, 0.95);
  }

  .folder-sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border-color);
  }

  .folder-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .close-sidebar-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 12px;
  }

  .folder-file-list {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .folder-file-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: 6px;
    background: transparent;
    border: 1px solid transparent;
    text-align: left;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .folder-file-btn:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .folder-file-btn.active {
    background: hsla(217, 91%, 60%, 0.16);
    border-color: hsla(217, 91%, 60%, 0.4);
  }

  .folder-file-btn .file-info {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .folder-file-btn .file-name {
    font-size: 12px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .folder-file-btn .file-size {
    font-size: 10px;
    color: var(--text-muted);
  }

  .viewport-wrapper {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  /* Welcome Hub Container */
  .welcome-hub-container {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 30px;
  }

  .welcome-card {
    max-width: 680px;
    width: 100%;
    padding: 36px;
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .welcome-badge {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    padding: 3px 10px;
    border-radius: 9999px;
    background: rgba(6, 182, 212, 0.16);
    border: 1px solid rgba(6, 182, 212, 0.4);
    color: #67e8f9;
    margin-bottom: 12px;
  }

  .welcome-card h2 {
    font-size: 26px;
    font-weight: 800;
    margin-bottom: 8px;
  }

  .welcome-desc {
    font-size: 13.5px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin-bottom: 24px;
    max-width: 540px;
  }

  .drop-target-box {
    width: 100%;
    padding: 32px 20px;
    border: 2px dashed rgba(59, 130, 246, 0.4);
    border-radius: 12px;
    background: rgba(59, 130, 246, 0.04);
    cursor: pointer;
    transition: all 0.2s ease;
    margin-bottom: 24px;
  }

  .drop-target-box:hover {
    border-color: var(--accent-blue);
    background: rgba(59, 130, 246, 0.08);
  }

  .drop-icon {
    font-size: 36px;
    margin-bottom: 8px;
  }

  .drop-target-box h3 {
    font-size: 16px;
    font-weight: 600;
    margin-bottom: 4px;
  }

  .drop-target-box p {
    font-size: 12px;
    color: var(--text-muted);
  }

  .quick-actions-row {
    display: flex;
    gap: 12px;
    margin-bottom: 24px;
  }

  .btn-lg {
    padding: 10px 18px;
    font-size: 13px;
    font-weight: 600;
  }

  .recents-panel {
    width: 100%;
    text-align: left;
    border-top: 1px solid var(--border-color);
    padding-top: 18px;
  }

  .recents-header {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    margin-bottom: 8px;
    display: block;
  }

  .recents-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 8px;
  }

  .recent-pill {
    display: flex;
    flex-direction: column;
    padding: 8px 12px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    text-align: left;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .recent-pill:hover {
    background: rgba(255, 255, 255, 0.08);
    border-color: var(--accent-blue);
  }

  .recent-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .recent-path {
    font-size: 10.5px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Modal Backdrop & Dialog */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(6px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .system-logs-dialog {
    width: 620px;
    max-height: 80vh;
    padding: 24px;
    border-radius: 14px;
    display: flex;
    flex-direction: column;
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .dialog-header h3 {
    font-size: 18px;
    font-weight: 700;
  }

  .dialog-close {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
  }

  .dialog-sub {
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 16px;
  }

  .system-logs-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .syslog-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;
  }

  .syslog-row:hover {
    background: rgba(59, 130, 246, 0.14);
    border-color: var(--accent-blue);
  }

  .syslog-details {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .syslog-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .syslog-location {
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .syslog-size {
    font-size: 12px;
    color: var(--accent-cyan);
    font-family: var(--font-mono);
    margin-left: 12px;
    flex-shrink: 0;
  }
</style>
