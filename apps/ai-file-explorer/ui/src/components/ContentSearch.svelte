<script lang="ts">
  import type { ContentSearchResult, ContentSearchQuery, SearchMode, SemanticSearchResult } from '../types';
  import { invokeIpc } from '../lib/ipc';

  export let onInspectDoc: (path: string) => void = () => {};

  let searchQuery = '';
  let selectedMode: SearchMode = 'Combined';
  let selectedExtension: string | null = null;
  let isSearching = false;
  let results: ContentSearchResult = {
    items: [],
    total_matches: 0,
    execution_time_ms: 0,
  };

  const extensionFilters = [
    { label: 'All Formats', value: null },
    { label: 'PDF', value: 'pdf' },
    { label: 'Word (DOCX)', value: 'docx' },
    { label: 'Markdown', value: 'md' },
    { label: 'Rust Source', value: 'rs' },
    { label: 'Logs', value: 'log' },
  ];

  async function handleSearch() {
    const effectiveQuery = searchQuery.trim() || (selectedExtension ? '*' : '');
    if (!effectiveQuery) {
      results = { items: [], total_matches: 0, execution_time_ms: 0 };
      return;
    }

    isSearching = true;
    try {
      const queryPayload: ContentSearchQuery = {
        query: effectiveQuery,
        mode: selectedMode,
        limit: 50,
        extension_filter: selectedExtension,
      };
      results = await invokeIpc<ContentSearchResult>('search_content_cmd', { query: queryPayload });
    } catch (e) {
      console.error('Content search failed:', e);
      results = { items: [], total_matches: 0, execution_time_ms: 0 };
    } finally {
      isSearching = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      handleSearch();
    }
  }
</script>

<div class="content-search-container">
  <div class="search-header-panel">
    <div class="search-title-row">
      <div class="title-with-badge">
        <h2>Content & Semantic Search</h2>
        <span class="ai-badge">8-Bit Quantized AI</span>
      </div>
      <p class="search-subtitle">
        Search inside PDF, DOCX, Markdown, code, and OCR-scanned text using natural language or keywords in &lt;50ms.
      </p>
    </div>

    <!-- Mode Selector Tabs -->
    <div class="mode-selector-bar">
      <button
        class="mode-btn {selectedMode === 'Combined' ? 'active' : ''}"
        on:click={() => { selectedMode = 'Combined'; handleSearch(); }}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10" />
          <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
        </svg>
        Hybrid (Keyword + AI Vector)
      </button>

      <button
        class="mode-btn {selectedMode === 'Semantic' ? 'active' : ''}"
        on:click={() => { selectedMode = 'Semantic'; handleSearch(); }}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
          <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
          <line x1="12" y1="22.08" x2="12" y2="12" />
        </svg>
        Semantic Vector Only
      </button>

      <button
        class="mode-btn {selectedMode === 'Keyword' ? 'active' : ''}"
        on:click={() => { selectedMode = 'Keyword'; handleSearch(); }}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        Tantivy Exact Keyword
      </button>
    </div>

    <!-- Query Input Bar -->
    <div class="search-input-box">
      <svg class="search-icon" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8" />
        <line x1="21" y1="21" x2="16.65" y2="16.65" />
      </svg>
      <input
        type="text"
        placeholder="Search documents by concept (e.g. 'unhandled database exception' or 'quarterly audit report')..."
        bind:value={searchQuery}
        on:keydown={handleKeydown}
      />
      <button class="search-submit-btn" on:click={handleSearch} disabled={isSearching}>
        {#if isSearching}
          Scanning...
        {:else}
          Search
        {/if}
      </button>
    </div>

    <!-- Extension Filters -->
    <div class="filters-row">
      <span class="filters-label">Format:</span>
      {#each extensionFilters as filter}
        <button
          class="filter-pill {selectedExtension === filter.value ? 'active' : ''}"
          on:click={() => { selectedExtension = filter.value; handleSearch(); }}
        >
          {filter.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- Results Section -->
  <div class="search-results-panel">
    {#if results.items.length > 0}
      <div class="results-stats-bar">
        <span>Found <strong>{results.total_matches}</strong> matching document(s)</span>
        <span class="latency-badge">{results.execution_time_ms.toFixed(2)} ms</span>
      </div>

      <div class="results-grid">
        {#each results.items as hit}
          <div class="result-card">
            <div class="card-top-row">
              <span class="format-badge {hit.format.toLowerCase()}">{hit.format}</span>
              <div class="score-indicator" title="Relevance match">
                <span class="score-val">{(hit.score * 100).toFixed(0)}% Match</span>
                <div class="score-bar">
                  <div class="score-fill" style="width: {hit.score * 100}%"></div>
                </div>
              </div>
            </div>

            <h3 class="doc-title">{hit.file_name}</h3>
            <p class="doc-path" title={hit.path}>{hit.path}</p>

            <div class="snippet-box">
              <p>{hit.snippet}</p>
            </div>

            <div class="card-footer">
              <button class="inspect-btn" on:click={() => onInspectDoc(hit.path)}>
                <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                  <circle cx="12" cy="12" r="3" />
                </svg>
                Inspect Document
              </button>
            </div>
          </div>
        {/each}
      </div>
    {:else if searchQuery && !isSearching}
      <div class="empty-state">
        <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
          <line x1="8" y1="11" x2="14" y2="11" />
        </svg>
        <p>No document content matched your query.</p>
        <span>Try adjusting your phrasing or changing the search mode above.</span>
      </div>
    {:else}
      <div class="prompt-state">
        <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
          <polyline points="14 2 14 8 20 8" />
          <line x1="16" y1="13" x2="8" y2="13" />
          <line x1="16" y1="17" x2="8" y2="17" />
          <polyline points="10 9 9 9 8 9" />
        </svg>
        <p>Type a conceptual question or keywords to search across all local files.</p>
        <span>All OCR processing, document parsing, and vector embeddings operate strictly on-device.</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .content-search-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
    height: 100%;
    overflow-y: auto;
  }

  .search-header-panel {
    background: rgba(15, 23, 42, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .title-with-badge {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .title-with-badge h2 {
    font-size: 1.25rem;
    font-weight: 600;
    color: #f8fafc;
    margin: 0;
  }

  .ai-badge {
    font-size: 0.72rem;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 12px;
    background: rgba(14, 165, 233, 0.15);
    color: #38bdf8;
    border: 1px solid rgba(14, 165, 233, 0.3);
  }

  .search-subtitle {
    font-size: 0.85rem;
    color: #94a3b8;
    margin: 4px 0 0 0;
  }

  .mode-selector-bar {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .mode-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: #94a3b8;
    font-size: 0.82rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .mode-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #e2e8f0;
  }

  .mode-btn.active {
    background: rgba(14, 165, 233, 0.15);
    border-color: rgba(14, 165, 233, 0.4);
    color: #38bdf8;
  }

  .search-input-box {
    display: flex;
    align-items: center;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;
    padding: 4px 6px 4px 12px;
    gap: 10px;
  }

  .search-icon {
    color: #64748b;
  }

  .search-input-box input {
    flex: 1;
    background: transparent;
    border: none;
    color: #f8fafc;
    font-size: 0.95rem;
    outline: none;
  }

  .search-input-box input::placeholder {
    color: #64748b;
  }

  .search-submit-btn {
    padding: 8px 18px;
    border-radius: 6px;
    background: #0ea5e9;
    color: #ffffff;
    border: none;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .search-submit-btn:hover:not(:disabled) {
    background: #0284c7;
  }

  .search-submit-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .filters-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .filters-label {
    font-size: 0.78rem;
    color: #64748b;
    font-weight: 500;
  }

  .filter-pill {
    padding: 4px 10px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    color: #94a3b8;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .filter-pill:hover {
    background: rgba(255, 255, 255, 0.06);
    color: #e2e8f0;
  }

  .filter-pill.active {
    background: rgba(56, 189, 248, 0.15);
    border-color: rgba(56, 189, 248, 0.3);
    color: #38bdf8;
  }

  .search-results-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .results-stats-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.82rem;
    color: #94a3b8;
    padding: 0 4px;
  }

  .latency-badge {
    background: rgba(34, 197, 94, 0.15);
    color: #4ade80;
    border: 1px solid rgba(34, 197, 94, 0.3);
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 0.75rem;
    font-family: monospace;
  }

  .results-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 14px;
  }

  .result-card {
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: all 0.15s ease;
  }

  .result-card:hover {
    border-color: rgba(14, 165, 233, 0.3);
    transform: translateY(-2px);
  }

  .card-top-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .format-badge {
    font-size: 0.7rem;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
    font-family: monospace;
  }

  .format-badge.pdf { background: rgba(239, 68, 68, 0.15); color: #f87171; border: 1px solid rgba(239, 68, 68, 0.3); }
  .format-badge.docx { background: rgba(59, 130, 246, 0.15); color: #60a5fa; border: 1px solid rgba(59, 130, 246, 0.3); }
  .format-badge.md { background: rgba(168, 85, 247, 0.15); color: #c084fc; border: 1px solid rgba(168, 85, 247, 0.3); }
  .format-badge.rs { background: rgba(249, 115, 22, 0.15); color: #fb923c; border: 1px solid rgba(249, 115, 22, 0.3); }

  .score-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .score-val {
    font-size: 0.75rem;
    color: #94a3b8;
    font-family: monospace;
  }

  .score-bar {
    width: 60px;
    height: 4px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 2px;
    overflow: hidden;
  }

  .score-fill {
    height: 100%;
    background: #0ea5e9;
    border-radius: 2px;
  }

  .doc-title {
    font-size: 0.95rem;
    font-weight: 600;
    color: #f1f5f9;
    margin: 0;
  }

  .doc-path {
    font-size: 0.78rem;
    color: #64748b;
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: monospace;
  }

  .snippet-box {
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    padding: 10px;
    font-size: 0.82rem;
    color: #cbd5e1;
    line-height: 1.4;
  }

  .snippet-box p {
    margin: 0;
  }

  .card-footer {
    display: flex;
    justify-content: flex-end;
    margin-top: auto;
  }

  .inspect-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #cbd5e1;
    font-size: 0.78rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .inspect-btn:hover {
    background: rgba(14, 165, 233, 0.15);
    border-color: rgba(14, 165, 233, 0.4);
    color: #38bdf8;
  }

  .empty-state, .prompt-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    color: #64748b;
    gap: 8px;
    text-align: center;
  }

  .empty-state p, .prompt-state p {
    font-size: 1rem;
    color: #94a3b8;
    margin: 8px 0 0 0;
  }

  .empty-state span, .prompt-state span {
    font-size: 0.82rem;
  }
</style>
