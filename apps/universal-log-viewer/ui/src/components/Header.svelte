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

  const dispatch = createEventDispatcher<{
    openFile: void;
    toggleTail: void;
    search: { query: string; isRegex: boolean; caseSensitive: boolean };
    jumpLine: number;
    toggleBookmarks: void;
  }>();

  let jumpLineInput: string = '';

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

    <button class="btn btn-primary" on:click={() => dispatch('openFile')} id="btn-open-file">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
      </svg>
      Open Log
    </button>
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

    <!-- Bookmarks Toggle -->
    <button class="btn" on:click={() => dispatch('toggleBookmarks')} id="btn-toggle-bookmarks">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z"></path>
      </svg>
      Bookmarks ({bookmarksCount})
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
    gap: 12px;
    min-width: 260px;
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
</style>
