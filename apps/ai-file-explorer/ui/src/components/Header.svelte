<script lang="ts">
  import type { ActiveTab } from '../types';

  export let currentPath: string;
  export let searchQuery: string;
  export let activeTab: ActiveTab;
  export let showStatsDrawer: boolean;
  export let isDualPaneMode: boolean = false;
  export let onToggleDualPane: () => void = () => {};
  export let onNavigate: (path: string) => void;
  export let onSearch: (query: string) => void;
  export let onRefresh: () => void;
  export let onNavigateUp: () => void;
  export let onBrowseFolder: () => void = () => {};

  let pathInputValue = currentPath;

  $: pathInputValue = currentPath;

  function handlePathSubmit() {
    if (pathInputValue.trim()) {
      onNavigate(pathInputValue.trim());
    }
  }

  function handleSearchInput(e: Event) {
    const target = e.target as HTMLInputElement;
    onSearch(target.value);
  }
</script>

<header class="header glass-panel">
  <!-- Brand and Status -->
  <div class="brand-section">
    <div class="logo-box">
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
      </svg>
    </div>
    <div class="brand-info">
      <h1>AI File Explorer</h1>
      <span class="badge badge-cyan">100% Offline</span>
    </div>
  </div>

  <!-- Path and Search Navigation -->
  <div class="nav-controls">
    <button class="icon-btn" on:click={onNavigateUp} title="Navigate Up (Alt + Up)">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="18 15 12 9 6 15"></polyline>
      </svg>
    </button>

    <form class="path-bar" on:submit|preventDefault={handlePathSubmit}>
      <input
        type="text"
        class="path-input"
        bind:value={pathInputValue}
        placeholder="Enter folder path..."
      />
      <button type="submit" class="go-btn" title="Go to Path">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <polyline points="9 18 15 12 9 6"></polyline>
        </svg>
      </button>
    </form>

    <button class="icon-btn" on:click={onBrowseFolder} title="Browse Folder (Native OS Picker)">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
        <line x1="12" y1="11" x2="12" y2="17"></line>
        <line x1="9" y1="14" x2="15" y2="14"></line>
      </svg>
    </button>

    <div class="search-box">
      <svg class="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8"></circle>
        <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
      </svg>
      <input
        type="text"
        class="search-input"
        value={searchQuery}
        on:input={handleSearchInput}
        placeholder="Instant search (<50ms)..."
      />
    </div>

    <button class="icon-btn" on:click={onRefresh} title="Refresh Directory">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="23 4 23 10 17 10"></polyline>
        <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
      </svg>
    </button>
  </div>

  <!-- Tabs & Actions -->
  <div class="tabs-section">
    <div class="tab-group">
      <button
        class="tab-btn {activeTab === 'browser' ? 'active' : ''}"
        on:click={() => (activeTab = 'browser')}
      >
        Explorer
      </button>
      <button
        class="tab-btn {activeTab === 'content_search' ? 'active' : ''}"
        on:click={() => (activeTab = 'content_search')}
      >
        Content Search
      </button>
      <button
        class="tab-btn {activeTab === 'screenshots' ? 'active' : ''}"
        on:click={() => (activeTab = 'screenshots')}
      >
        Screenshots
      </button>
      <button
        class="tab-btn {activeTab === 'duplicates' ? 'active' : ''}"
        on:click={() => (activeTab = 'duplicates')}
      >
        Duplicates
      </button>
      <button
        class="tab-btn {activeTab === 'treemap' ? 'active' : ''}"
        on:click={() => (activeTab = 'treemap')}
      >
        Treemap
      </button>
      <button
        class="tab-btn {activeTab === 'batch_rename' ? 'active' : ''}"
        on:click={() => (activeTab = 'batch_rename')}
      >
        Batch Rename
      </button>
      <button
        class="tab-btn {activeTab === 'compliance' ? 'active' : ''}"
        on:click={() => (activeTab = 'compliance')}
      >
        Compliance
      </button>
    </div>


    <button
      class="btn btn-secondary commander-toggle-btn {isDualPaneMode ? 'active' : ''}"
      on:click={onToggleDualPane}
      title="Toggle Dual-Pane Commander Mode (Ctrl+D)"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect x="3" y="3" width="8" height="18" rx="1"></rect>
        <rect x="13" y="3" width="8" height="18" rx="1"></rect>
      </svg>
      Commander
    </button>

    <button
      class="btn btn-secondary stats-btn"
      on:click={() => (showStatsDrawer = !showStatsDrawer)}
      title="Toggle Storage Analytics"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M18 20V10"></path>
        <path d="M12 20V4"></path>
        <path d="M6 20v-6"></path>
      </svg>
      Stats
    </button>
  </div>
</header>

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 20px;
    margin: 12px 16px 8px 16px;
    border-radius: var(--radius-lg);
  }

  .brand-section {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 200px;
  }

  .logo-box {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 38px;
    height: 38px;
    background: linear-gradient(135deg, rgba(6, 182, 212, 0.2), rgba(59, 130, 246, 0.2));
    border: 1px solid rgba(6, 182, 212, 0.4);
    border-radius: var(--radius-md);
    color: var(--accent-cyan);
  }

  .brand-info h1 {
    font-size: 15px;
    font-weight: 700;
    letter-spacing: -0.01em;
    line-height: 1.2;
  }

  .nav-controls {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
    max-width: 800px;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.2s ease;
  }
  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-main);
  }

  .path-bar {
    display: flex;
    align-items: center;
    flex: 1;
    background: rgba(11, 17, 30, 0.85);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 2px 6px 2px 12px;
    transition: border-color 0.2s ease;
  }
  .path-bar:focus-within {
    border-color: var(--border-focus);
    box-shadow: var(--border-glow);
  }

  .path-input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text-main);
    font-family: var(--font-mono);
    font-size: 12px;
    outline: none;
  }

  .go-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 6px;
    border-radius: 4px;
  }
  .go-btn:hover {
    color: var(--accent-cyan);
  }

  .search-box {
    position: relative;
    width: 240px;
  }

  .search-icon {
    position: absolute;
    left: 10px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    background: rgba(11, 17, 30, 0.85);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 8px 12px 8px 32px;
    color: var(--text-main);
    font-size: 12px;
    outline: none;
    transition: all 0.2s ease;
  }
  .search-input:focus {
    border-color: var(--border-focus);
    box-shadow: var(--border-glow);
    width: 280px;
  }

  .tabs-section {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .tab-group {
    display: flex;
    align-items: center;
    background: rgba(11, 17, 30, 0.6);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 3px;
    gap: 2px;
  }

  .tab-btn {
    padding: 6px 12px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
  }
  .tab-btn:hover {
    color: var(--text-main);
  }
  .tab-btn.active {
    background: linear-gradient(135deg, rgba(6, 182, 212, 0.25), rgba(59, 130, 246, 0.25));
    border: 1px solid rgba(6, 182, 212, 0.4);
    color: var(--accent-cyan);
    font-weight: 600;
  }

  .stats-btn, .commander-toggle-btn {
    padding: 6px 12px;
    font-size: 12px;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .commander-toggle-btn.active {
    background: rgba(6, 182, 212, 0.22);
    border-color: rgba(6, 182, 212, 0.6);
    color: var(--accent-cyan);
    font-weight: 700;
    box-shadow: 0 0 10px rgba(6, 182, 212, 0.2);
  }
</style>
