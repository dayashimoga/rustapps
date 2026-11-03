<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { TemplateClusterUi, AnomalySpikeUi, ErrorClusterUi } from '../types';

  export let isOpen = false;
  export let templates: TemplateClusterUi[] = [];
  export let anomalies: AnomalySpikeUi[] = [];
  export let errorClusters: ErrorClusterUi[] = [];
  export let isAnalyzing = false;

  const dispatch = createEventDispatcher<{
    close: void;
    jumpToLine: number;
    filterTimeRange: { startMs: number; endMs: number };
    filterTemplate: { pattern: string };
    refresh: void;
  }>();

  let activeTab: 'templates' | 'anomalies' | 'errors' = 'templates';

  function formatTime(epochMs: number): string {
    const d = new Date(epochMs);
    return `${d.getHours().toString().padStart(2, '0')}:${d
      .getMinutes()
      .toString()
      .padStart(2, '0')}:${d.getSeconds().toString().padStart(2, '0')}`;
  }

  function cleanTemplatePattern(template: string): string {
    // Strip <...> wildcard tokens to get clean keyword search string
    return template.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim();
  }
</script>

{#if isOpen}
  <div class="drawer-backdrop" on:click={() => dispatch('close')} role="presentation"></div>

  <aside class="ai-drawer glass-panel" aria-label="AI Log Analysis Drawer">
    <div class="drawer-header">
      <div class="header-title">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="3"></circle>
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
        </svg>
        <h3>LOCAL ROOT-CAUSE INTELLIGENCE</h3>
      </div>
      <div class="header-actions">
        <button
          type="button"
          class="btn-refresh"
          disabled={isAnalyzing}
          on:click={() => dispatch('refresh')}
          title="Re-run AI Mining & Anomaly Detection"
        >
          {#if isAnalyzing}
            <span class="spinner-ai"></span> Analyzing...
          {:else}
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M23 4v6h-6"></path>
              <path d="M1 20v-6h6"></path>
              <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
            </svg>
            Re-run
          {/if}
        </button>
        <button class="close-btn" on:click={() => dispatch('close')} title="Close Drawer">
          ✕
        </button>
      </div>
    </div>

    <!-- Mode Selector Tabs -->
    <div class="tabs-bar">
      <button
        class="tab-btn"
        class:active={activeTab === 'templates'}
        on:click={() => (activeTab = 'templates')}
      >
        Templates ({templates.length})
      </button>
      <button
        class="tab-btn"
        class:active={activeTab === 'anomalies'}
        on:click={() => (activeTab = 'anomalies')}
      >
        Anomalies ({anomalies.length})
      </button>
      <button
        class="tab-btn"
        class:active={activeTab === 'errors'}
        on:click={() => (activeTab = 'errors')}
      >
        Error Causes ({errorClusters.length})
      </button>
    </div>

    <div class="drawer-body">
      {#if activeTab === 'templates'}
        <div class="tab-content">
          <div class="tab-desc">
            Algorithmic Drain template miner extracted <strong>{templates.length}</strong> structural patterns completely offline.
          </div>
          {#each templates as t}
            <div class="insight-card">
              <div class="card-meta">
                <span class="badge freq">{t.frequency} occurrences</span>
                <span class="badge tokens">{t.token_count} tokens</span>
              </div>
              <div class="template-str">
                {t.template}
              </div>
              <div class="sample-actions">
                <button
                  type="button"
                  class="btn-action-sm"
                  on:click={() => dispatch('filterTemplate', { pattern: cleanTemplatePattern(t.template) })}
                  title="Search viewer for this pattern"
                >
                  🔍 Filter
                </button>
                {#if t.sample_lines.length > 0}
                  <span class="sample-lbl">Jump:</span>
                  {#each t.sample_lines.slice(0, 4) as line}
                    <button
                      class="btn-line-link"
                      on:click={() => dispatch('jumpToLine', line)}
                    >
                      L{line + 1}
                    </button>
                  {/each}
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {:else if activeTab === 'anomalies'}
        <div class="tab-content">
          <div class="tab-desc">
            Rolling z-score engine detected <strong>{anomalies.length}</strong> volume or error surges (&gt; 2.5 std-dev).
          </div>
          {#if anomalies.length === 0}
            <div class="empty-notice">No statistical anomalies detected in active log window.</div>
          {/if}
          {#each anomalies as a}
            <div class="insight-card anomaly" class:crit={a.severity === 'ERROR'}>
              <div class="card-meta">
                <span class="badge zscore">z-score: {a.z_score}</span>
                <span class="badge time">{formatTime(a.start_epoch_ms)} — {formatTime(a.end_epoch_ms)}</span>
              </div>
              <div class="anomaly-desc">
                {a.description}
              </div>
              <div class="metric-detail">
                Observed: <strong>{a.observed_value}</strong> | Expected: <strong>{a.expected_value}</strong>
              </div>
              <div class="anomaly-actions">
                <button
                  type="button"
                  class="btn-action-sm focus-btn"
                  on:click={() => dispatch('filterTimeRange', { startMs: a.start_epoch_ms, endMs: a.end_epoch_ms })}
                >
                  ⚡ Focus Anomaly Window ({formatTime(a.start_epoch_ms)} — {formatTime(a.end_epoch_ms)})
                </button>
              </div>
            </div>
          {/each}
        </div>
      {:else if activeTab === 'errors'}
        <div class="tab-content">
          <div class="tab-desc">
            Ranked root-cause error clusters correlated across containers and pods.
          </div>
          {#if errorClusters.length === 0}
            <div class="empty-notice">No error events detected in current log stream.</div>
          {/if}
          {#each errorClusters as e}
            <div class="insight-card error-card">
              <div class="card-meta">
                <span class="badge err-count">{e.error_count} errors</span>
                {#each e.affected_sources as src}
                  <span class="badge src">{src}</span>
                {/each}
              </div>
              <div class="template-str">
                {e.template}
              </div>
              <div class="sample-actions">
                <button
                  type="button"
                  class="btn-action-sm"
                  on:click={() => dispatch('filterTemplate', { pattern: cleanTemplatePattern(e.template) })}
                  title="Search viewer for this error cause"
                >
                  🔍 Filter Cause
                </button>
                {#if e.sample_lines.length > 0}
                  <span class="sample-lbl">Inspect:</span>
                  {#each e.sample_lines.slice(0, 4) as line}
                    <button
                      class="btn-line-link"
                      on:click={() => dispatch('jumpToLine', line)}
                    >
                      Line {line + 1}
                    </button>
                  {/each}
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </aside>
{/if}

<style>
  .drawer-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.5);
    z-index: 50;
    backdrop-filter: blur(2px);
  }

  .ai-drawer {
    position: fixed;
    top: 0;
    right: 0;
    width: 460px;
    height: 100vh;
    background: hsla(220, 24%, 10%, 0.95);
    border-left: 1px solid var(--border-subtle);
    z-index: 60;
    display: flex;
    flex-direction: column;
    box-shadow: -10px 0 30px rgba(0, 0, 0, 0.5);
  }

  .drawer-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border-subtle);
    gap: 8px;
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--accent-violet);
  }

  .header-title h3 {
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.5px;
    color: var(--text-primary);
    margin: 0;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .btn-refresh {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    background: hsla(220, 20%, 20%, 0.6);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-refresh:hover:not(:disabled) {
    background: hsla(217, 91%, 60%, 0.15);
    border-color: hsla(217, 91%, 60%, 0.35);
    color: var(--accent-blue);
  }

  .btn-refresh:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .spinner-ai {
    width: 10px;
    height: 10px;
    border: 2px solid rgba(255, 255, 255, 0.2);
    border-top-color: var(--accent-blue);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
    padding: 4px 6px;
  }

  .close-btn:hover {
    color: var(--text-primary);
  }

  .tabs-bar {
    display: flex;
    background: hsla(220, 20%, 8%, 0.8);
    border-bottom: 1px solid var(--border-subtle);
    padding: 0 12px;
  }

  .tab-btn {
    flex: 1;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 10px 4px;
    color: var(--text-muted);
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tab-btn:hover {
    color: var(--text-secondary);
  }

  .tab-btn.active {
    color: var(--accent-blue);
    border-bottom-color: var(--accent-blue);
  }

  .drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  .tab-desc {
    font-size: 11.5px;
    color: var(--text-muted);
    margin-bottom: 14px;
    line-height: 1.4;
  }

  .tab-desc strong {
    color: var(--text-primary);
  }

  .insight-card {
    background: hsla(220, 22%, 14%, 0.7);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 12px;
  }

  .insight-card.anomaly.crit {
    border-color: hsla(0, 85%, 60%, 0.4);
    background: hsla(0, 85%, 20%, 0.15);
  }

  .card-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 8px;
  }

  .badge {
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: var(--font-mono);
  }

  .badge.freq {
    background: hsla(217, 91%, 60%, 0.2);
    color: var(--accent-blue);
  }

  .badge.tokens {
    background: hsla(220, 20%, 25%, 0.5);
    color: var(--text-secondary);
  }

  .badge.zscore {
    background: hsla(38, 92%, 55%, 0.2);
    color: hsl(38, 92%, 60%);
    font-weight: 700;
  }

  .badge.time {
    background: hsla(220, 20%, 20%, 0.5);
    color: var(--text-muted);
  }

  .badge.err-count {
    background: hsla(0, 85%, 60%, 0.2);
    color: hsl(0, 85%, 65%);
    font-weight: 700;
  }

  .badge.src {
    background: hsla(260, 70%, 65%, 0.2);
    color: hsl(260, 70%, 75%);
  }

  .template-str {
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--text-primary);
    word-break: break-all;
    line-height: 1.4;
    margin-bottom: 8px;
  }

  .anomaly-desc {
    font-size: 12px;
    color: var(--text-primary);
    margin-bottom: 6px;
    line-height: 1.4;
  }

  .metric-detail {
    font-size: 10.5px;
    font-family: var(--font-mono);
    color: var(--text-muted);
    margin-bottom: 8px;
  }

  .sample-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .anomaly-actions {
    display: flex;
    align-items: center;
    margin-top: 6px;
  }

  .sample-lbl {
    font-size: 10.5px;
    color: var(--text-muted);
  }

  .btn-action-sm {
    background: hsla(217, 91%, 60%, 0.15);
    border: 1px solid hsla(217, 91%, 60%, 0.3);
    color: var(--accent-blue);
    font-size: 11px;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-action-sm:hover {
    background: var(--accent-blue);
    color: #fff;
  }

  .focus-btn {
    width: 100%;
    text-align: center;
    background: hsla(38, 92%, 55%, 0.15);
    border-color: hsla(38, 92%, 55%, 0.35);
    color: hsl(38, 92%, 60%);
    padding: 5px 10px;
  }

  .focus-btn:hover {
    background: hsl(38, 92%, 55%);
    color: #000;
  }

  .btn-line-link {
    background: hsla(220, 20%, 20%, 0.6);
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    font-size: 10.5px;
    font-family: var(--font-mono);
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
  }

  .btn-line-link:hover {
    background: var(--accent-blue);
    color: #fff;
    border-color: var(--accent-blue);
  }

  .empty-notice {
    padding: 24px;
    text-align: center;
    color: var(--text-muted);
    font-size: 12px;
  }
</style>
