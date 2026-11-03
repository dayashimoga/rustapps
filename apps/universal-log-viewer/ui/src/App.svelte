<script lang="ts">
  import { onMount } from 'svelte';
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

  // In-memory mock database for standalone browser preview / testing
  let mockFileLines: string[] = [];

  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI__;

  async function tauriInvoke<T>(cmd: string, args?: Record<string, any>): Promise<T | null> {
    if (isTauri && (window as any).__TAURI__?.invoke) {
      try {
        return await (window as any).__TAURI__.invoke(cmd, args);
      } catch (err) {
        console.error(`Tauri IPC error for ${cmd}:`, err);
        throw err;
      }
    }
    return null;
  }

  async function openRealFile(path: string) {
    if (isTauri) {
      try {
        const info = await tauriInvoke<OpenedFileInfo>('open_file', { path });
        if (info) {
          filePath = info.path;
          originalTotalLines = info.total_lines;
          totalLines = info.total_lines;
          filteredLineIndices = null;
          selectedRange = null;
          fileSizeBytes = info.file_size_bytes;
          await fetchSlice(0, 60);
          await loadAiInsights();
          return;
        }
      } catch (e) {
        console.error('Failed to open file via Tauri:', e);
      }
    }
    generateSampleLogs(100000);
  }

  async function handleOpenFilePrompt() {
    if (isTauri && (window as any).__TAURI__?.dialog?.open) {
      try {
        const selected = await (window as any).__TAURI__.dialog.open({
          multiple: false,
          filters: [{ name: 'Log Files', extensions: ['log', 'txt', 'jsonl', 'csv', 'tsv'] }],
        });
        if (typeof selected === 'string') {
          await openRealFile(selected);
          return;
        }
      } catch (err) {
        console.warn('Native open dialog failed:', err);
      }
    }
    // Browser fallback
    generateSampleLogs(100000);
  }

  async function loadAiInsights() {
    isAiAnalyzing = true;
    try {
      if (isTauri && filePath) {
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
      } else {
        await new Promise((r) => setTimeout(r, 350));
      }
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
    if (isTauri) {
      try {
        const sampled = await tauriInvoke<ProcessResourceUsage>('get_system_metrics');
        if (sampled) {
          systemMetrics = sampled;
        }
      } catch {
        // ignore
      }
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown);

    if (isTauri) {
      openRealFile('default');
      updateMetrics();
    } else {
      // Auto-load sample logs on initial browser preview
      generateSampleLogs(50000);
    }

    metricsInterval = window.setInterval(updateMetrics, 2500);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      if (tailInterval) clearInterval(tailInterval);
      if (metricsInterval) clearInterval(metricsInterval);
    };
  });
</script>

<div class="app-layout">
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
    on:toggleTail={toggleTailMode}
    on:search={handleSearch}
    on:jumpLine={handleJumpLine}
    on:toggleBookmarks={() => (isBookmarksOpen = !isBookmarksOpen)}
    on:toggleTimeline={() => (showTimeline = !showTimeline)}
    on:toggleAiInsights={() => (showAiInsights = !showAiInsights)}
    on:openRedactModal={() => (isRedactModalOpen = true)}
  />

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
    {#if totalLines === 0}
      <!-- Empty State Hero -->
      <div class="hero-container">
        <div class="hero-card glass-panel">
          <div class="hero-icon">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
              <polyline points="14 2 14 8 20 8"></polyline>
            </svg>
          </div>
          <h2>Open Massive Logs Instantaneously</h2>
          <p>Zero-copy memory mapping processes 500GB+ logs within a 300MB RAM budget.</p>

          <button class="btn btn-primary hero-btn" on:click={handleOpenFilePrompt}>
            Open Log File
          </button>
        </div>
      </div>
    {:else}
      <!-- Virtualized 60FPS Log Viewport -->
      <VirtualizedViewport
        bind:this={viewportComponent}
        {lines}
        {totalLines}
        {activeLine}
        {searchQuery}
        on:scrollChange={handleScrollChange}
        on:bookmarkLine={handleBookmarkLine}
      />
    {/if}
  </main>

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

  .hero-card h2 {
    font-size: 20px;
    font-weight: 700;
    margin-bottom: 10px;
    letter-spacing: -0.3px;
  }

  .hero-card p {
    font-size: 13.5px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin-bottom: 24px;
  }

  .hero-btn {
    padding: 10px 24px;
    font-size: 14px;
  }
</style>
