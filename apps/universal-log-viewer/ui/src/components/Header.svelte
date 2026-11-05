<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { ProcessResourceUsage } from '../types';

  export let filePath: string = '';
  export let tailModeActive: boolean = false;
  export let searchQuery: string = '';
  export let isRegex: boolean = false;
  export let caseSensitive: boolean = false;
  export let searchMatchCount: number | null = null;
  export let searchLatencyMicros: number | null = null;
  export let systemMetrics: ProcessResourceUsage | null = null;
  export let bookmarksCount: number = 0;
  export let showTimeline: boolean = true;
  export let showAiInsights: boolean = false;
  export let tier: string = 'Free';

  const dispatch = createEventDispatcher<{
    openFile: void;
    openFolder: void;
    openSystemLogs: void;
    openPath: string;
    toggleTail: void;
    search: { query: string; isRegex: boolean; caseSensitive: boolean };
    jumpLine: number;
    toggleBookmarks: void;
    toggleTimeline: void;
    toggleAiInsights: void;
    openRedactModal: void;
    openLicenseModal: void;
  }>();

  let jumpLineInput: string = '';
  let directPathInput: string = '';
  $: if (filePath && !directPathInput) {
    directPathInput = filePath;
  }

  function handleDirectPathSubmit() {
    if (directPathInput.trim()) {
      dispatch('openPath', directPathInput.trim());
    }
  }

  function handleSearchInput() {
    dispatch('search', { query: searchQuery, isRegex, caseSensitive });
  }

  function toggleRegex() {
    isRegex = !isRegex;
    handleSearchInput();
  }

  function toggleCase() {
    caseSensitive = !caseSensitive;
    handleSearchInput();
  }

  function handleJumpSubmit(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      const line = parseInt(jumpLineInput, 10);
      if (!isNaN(line) && line > 0) {
        dispatch('jumpLine', line - 1);
        jumpLineInput = '';
      }
    }
  }
</script>

<header class="header glass-panel">
  <!-- Brand and Open Action -->
  <div class="brand-group">
    <div class="app-icon">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
        <polyline points="14 2 14 8 20 8"></polyline>
        <line x1="16" y1="13" x2="8" y2="13"></line>
        <line x1="16" y1="17" x2="8" y2="17"></line>
        <polyline points="10 9 9 9 8 9"></polyline>
      </svg>
    </div>
    <div class="title-meta">
      <span class="app-title">LogViewer</span>
      {#if filePath}
        <span class="file-name" title={filePath}>{filePath.split(/[/\\]/).pop()}</span>
      {/if}
    </div>

    <button type="button" class="tier-badge-btn" on:click={() => dispatch('openLicenseModal')} title="Manage Offline License">
      <span class="tier-badge tier-{tier.toLowerCase()}">[{tier.toUpperCase()}]</span>
    </button>

    <button class="btn btn-primary" on:click={() => dispatch('openFile')} id="btn-open-file" title="Open Log File (Ctrl+O)">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
        <polyline points="14 2 14 8 20 8"></polyline>
      </svg>
      File
    </button>

    <button class="btn btn-secondary header-btn" on:click={() => dispatch('openFolder')} id="btn-open-folder" title="Open Folder with Multiple Logs">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
      </svg>
      Folder
    </button>

    <button class="btn btn-secondary header-btn" on:click={() => dispatch('openSystemLogs')} id="btn-system-logs" title="Explore Local System Logs">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
        <line x1="8" y1="21" x2="16" y2="21"></line>
        <line x1="12" y1="17" x2="12" y2="21"></line>
      </svg>
      System
    </button>

    <form class="direct-path-form" on:submit|preventDefault={handleDirectPathSubmit}>
      <input
        type="text"
        class="direct-path-input"
        placeholder="C:\logs\app.log or drop file..."
        bind:value={directPathInput}
        title="Enter absolute log file path and press Enter"
      />
      <button type="submit" class="direct-path-btn" title="Load Log Path">Load</button>
    </form>
  </div>

  <!-- Central Search Omnibar -->
  <div class="search-group">
    <div class="search-box">
      <svg class="search-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8"></circle>
        <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
      </svg>
      <input
        type="text"
        class="search-input"
        placeholder="Filter logs or query (Ctrl+F)..."
        bind:value={searchQuery}
        on:input={handleSearchInput}
        id="input-search"
      />

      <div class="search-toggles">
        <button
          class="toggle-btn {isRegex ? 'active' : ''}"
          on:click={toggleRegex}
          title="Toggle Regular Expression"
        >
          .*
        </button>
        <button
          class="toggle-btn {caseSensitive ? 'active' : ''}"
          on:click={toggleCase}
          title="Toggle Case Sensitivity"
        >
          Aa
        </button>
      </div>

      {#if searchMatchCount !== null}
        <span class="badge search-stat">
          {searchMatchCount} hits
          {#if searchLatencyMicros !== null}
            <span class="latency">({(searchLatencyMicros / 1000).toFixed(1)}ms)</span>
          {/if}
        </span>
      {/if}
    </div>
  </div>

  <!-- Operational Actions & Metrics -->
  <div class="controls-group">
    <!-- Live Tail Mode -->
    <button
      class="btn {tailModeActive ? 'btn-active' : ''}"
      on:click={() => dispatch('toggleTail')}
      title="Toggle real-time streaming tail mode (Ctrl+T)"
      id="btn-toggle-tail"
    >
      {#if tailModeActive}
        <span class="live-indicator"></span>
      {:else}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polygon points="5 3 19 12 5 21 5 3"></polygon>
        </svg>
      {/if}
      Tail Mode
    </button>

    <!-- Jump to Line -->
    <div class="jump-box">
      <input
        type="text"
        class="input-text jump-input"
        placeholder="Jump line #"
        bind:value={jumpLineInput}
        on:keydown={handleJumpSubmit}
        id="input-jump-line"
      />
    </div>

    <!-- Timeline Toggle -->
    <button
      class="btn {showTimeline ? 'btn-active' : ''}"
      on:click={() => dispatch('toggleTimeline')}
      title="Toggle Chronological Timeline Histogram"
      id="btn-toggle-timeline"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M3 3v18h18"></path>
        <rect x="7" y="10" width="3" height="8"></rect>
        <rect x="12" y="6" width="3" height="12"></rect>
      </svg>
      Timeline
    </button>

    <!-- AI Insights Toggle -->
    <button
      class="btn {showAiInsights ? 'btn-active' : ''}"
      on:click={() => dispatch('toggleAiInsights')}
      title="Toggle AI Root-Cause Intelligence"
      id="btn-toggle-ai-insights"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3"></circle>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
      </svg>
      AI Insights
    </button>

    <!-- Bookmarks Toggle -->
    <button class="btn" on:click={() => dispatch('toggleBookmarks')} id="btn-toggle-bookmarks">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z"></path>
      </svg>
      Bookmarks ({bookmarksCount})
    </button>

    <!-- PII Redaction Export Trigger -->
    <button
      class="btn btn-redact"
      on:click={() => dispatch('openRedactModal')}
      title="Mask PII & Export Cleaned Logs (Business Tier)"
      id="btn-redact-export"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path>
      </svg>
      Redact
    </button>

    <!-- Live Telemetry SLA Badge -->
    {#if systemMetrics}
      <div class="metric-badge" title="Physical RAM and CPU SLA Monitor">
        <span class="metric-val">{((systemMetrics.resident_set_size_bytes) / (1024 * 1024)).toFixed(0)}MB</span>
        <span class="metric-sub">{systemMetrics.cpu_percentage.toFixed(0)}% CPU</span>
      </div>
    {/if}
  </div>
</header>

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 18px;
    height: 56px;
    border-top: none;
    border-left: none;
    border-right: none;
    z-index: 100;
  }

  .brand-group {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 380px;
  }

  .direct-path-form {
    display: flex;
    align-items: center;
    background: hsla(222, 24%, 6%, 0.9);
    border: 1px solid var(--border-color);
    border-radius: 7px;
    padding: 2px 4px 2px 8px;
  }

  .direct-path-input {
    width: 155px;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 11px;
    outline: none;
  }

  .direct-path-btn {
    background: hsla(217, 91%, 60%, 0.18);
    border: 1px solid hsla(217, 91%, 60%, 0.4);
    color: var(--accent-blue);
    border-radius: 5px;
    padding: 3px 7px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }

  .app-icon {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: linear-gradient(135deg, var(--accent-blue), var(--accent-violet));
    display: flex;
    align-items: center;
    justify-content: center;
    color: #fff;
    box-shadow: var(--shadow-glow);
  }

  .title-meta {
    display: flex;
    flex-direction: column;
  }

  .app-title {
    font-size: 14px;
    font-weight: 700;
    letter-spacing: -0.2px;
    background: linear-gradient(135deg, var(--text-primary), var(--accent-cyan));
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
  }

  .file-name {
    font-size: 11px;
    color: var(--text-muted);
    max-width: 130px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .search-group {
    flex: 1;
    max-width: 620px;
    margin: 0 16px;
  }

  .search-box {
    display: flex;
    align-items: center;
    background: hsla(222, 24%, 6%, 0.9);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 3px 10px;
    transition: all var(--transition-fast);
  }

  .search-box:focus-within {
    border-color: var(--accent-blue);
    box-shadow: 0 0 15px -3px hsla(217, 91%, 60%, 0.3);
  }

  .search-icon {
    color: var(--text-muted);
    margin-right: 8px;
  }

  .search-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: 13px;
  }

  .search-toggles {
    display: flex;
    gap: 4px;
    margin-left: 8px;
  }

  .toggle-btn {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    padding: 2px 6px;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .toggle-btn:hover {
    color: var(--text-primary);
    background: hsla(220, 20%, 30%, 0.3);
  }

  .toggle-btn.active {
    color: var(--accent-blue);
    background: hsla(217, 91%, 60%, 0.15);
    border-color: var(--accent-blue);
  }

  .search-stat {
    margin-left: 8px;
    background: hsla(217, 91%, 60%, 0.15);
    color: var(--accent-blue);
  }

  .latency {
    opacity: 0.7;
    margin-left: 4px;
  }

  .controls-group {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .jump-input {
    width: 95px;
    padding: 5px 8px;
    font-size: 12px;
    font-family: var(--font-mono);
  }

  .metric-badge {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    background: hsla(220, 20%, 15%, 0.6);
    padding: 3px 8px;
    border-radius: 6px;
    border: 1px solid var(--border-color);
  }

  .metric-val {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    color: var(--accent-cyan);
  }

  .metric-sub {
    font-size: 9px;
    color: var(--text-muted);
  }

  .btn-redact {
    background: hsla(145, 80%, 45%, 0.12);
    border-color: hsla(145, 80%, 45%, 0.3);
    color: hsl(145, 80%, 60%);
  }

  .btn-redact:hover {
    background: hsla(145, 80%, 45%, 0.25);
    border-color: hsla(145, 80%, 45%, 0.5);
    color: #fff;
  }

  .tier-badge-btn {
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
  }
  .tier-badge {
    cursor: pointer;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 10px;
    text-transform: uppercase;
    transition: transform 0.15s ease, opacity 0.15s ease;
  }
  .tier-badge:hover {
    transform: scale(1.05);
    opacity: 0.9;
  }
  .tier-badge.tier-free {
    background: rgba(148, 163, 184, 0.2);
    color: #cbd5e1;
    border: 1px solid rgba(148, 163, 184, 0.3);
  }
  .tier-badge.tier-premium {
    background: rgba(59, 130, 246, 0.25);
    color: #93c5fd;
    border: 1px solid rgba(59, 130, 246, 0.45);
  }
  .tier-badge.tier-business {
    background: rgba(168, 85, 247, 0.25);
    color: #e9d5ff;
    border: 1px solid rgba(168, 85, 247, 0.45);
  }
</style>
