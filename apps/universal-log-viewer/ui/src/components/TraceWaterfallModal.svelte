<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invokeIpc } from '../lib/ipc';

  export let isOpen = false;
  export let initialTraceId = '';

  const dispatch = createEventDispatcher<{
    close: void;
    jumpToLine: number;
  }>();

  let traceQuery = '';
  let isLoading = false;
  let errorMessage = '';
  let report: {
    trace_id: string;
    total_spans: number;
    start_time_epoch_ms: number;
    end_time_epoch_ms: number;
    total_duration_ms: number;
    root_service: string;
    spans: Array<{
      span_id: string | null;
      parent_id: string | null;
      service_name: string;
      line_number: number;
      timestamp_epoch_ms: number;
      offset_ms: number;
      duration_ms: number;
      severity: string;
      message: string;
      source_file: string;
    }>;
  } | null = null;

  $: if (isOpen && initialTraceId && initialTraceId !== traceQuery) {
    traceQuery = initialTraceId;
    loadTraceWaterfall(traceQuery);
  }

  async function loadTraceWaterfall(tid: string) {
    const clean = tid.trim();
    if (!clean) return;
    isLoading = true;
    errorMessage = '';
    report = null;
    try {
      const res = await invokeIpc<any>('correlate_traces', { traceId: clean });
      report = res;
    } catch (e: any) {
      errorMessage = e?.message || String(e);
    } finally {
      isLoading = false;
    }
  }

  function formatTime(epochMs: number): string {
    if (!epochMs) return '00:00:00.000';
    const d = new Date(epochMs);
    return d.toTimeString().split(' ')[0] + '.' + d.getMilliseconds().toString().padStart(3, '0');
  }

  function getBarWidth(durationMs: number, totalDurationMs: number): string {
    const total = Math.max(totalDurationMs, 10);
    const pct = Math.max((durationMs / total) * 100, 3);
    return `${Math.min(pct, 100).toFixed(1)}%`;
  }

  function getBarLeft(offsetMs: number, totalDurationMs: number): string {
    const total = Math.max(totalDurationMs, 10);
    const pct = (offsetMs / total) * 100;
    return `${Math.min(pct, 95).toFixed(1)}%`;
  }

  function getServiceColor(service: string): string {
    const colors = [
      '#3b82f6', '#10b981', '#f59e0b', '#ec4899', '#8b5cf6',
      '#06b6d4', '#14b8a6', '#f97316', '#6366f1'
    ];
    let hash = 0;
    for (let i = 0; i < service.length; i++) {
      hash = service.charCodeAt(i) + ((hash << 5) - hash);
    }
    return colors[Math.abs(hash) % colors.length];
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
  <div
    class="modal-backdrop"
    role="dialog"
    aria-modal="true"
    aria-label="OpenTelemetry Distributed Trace Waterfall"
    tabindex="-1"
    on:click|self={() => dispatch('close')}
    on:keydown={(e) => e.key === 'Escape' && dispatch('close')}
  >
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="waterfall-modal glass-panel" on:click|stopPropagation role="document">
      <div class="modal-header">
        <div class="header-title">
          <span class="icon">⚡</span>
          <div>
            <h3>OpenTelemetry Distributed Trace Waterfall</h3>
            <p class="subtitle">Correlate spans, services, and execution timings across microservice logs</p>
          </div>
        </div>
        <button class="btn-close" on:click={() => dispatch('close')}>✕</button>
      </div>

      <div class="search-bar">
        <input
          type="text"
          class="input-trace"
          placeholder="Enter Trace ID / Request ID (e.g. trace-101, req_8842, 4bf92f...)..."
          bind:value={traceQuery}
          on:keydown={(e) => e.key === 'Enter' && loadTraceWaterfall(traceQuery)}
        />
        <button
          class="btn btn-primary"
          on:click={() => loadTraceWaterfall(traceQuery)}
          disabled={isLoading || !traceQuery.trim()}
        >
          {isLoading ? 'Correlating...' : '🔍 Trace Spans'}
        </button>
      </div>

      {#if errorMessage}
        <div class="error-banner">⚠️ {errorMessage}</div>
      {/if}

      {#if report}
        <div class="trace-overview">
          <div class="stat-card">
            <span class="label">TRACE ID</span>
            <span class="value mono">{report.trace_id}</span>
          </div>
          <div class="stat-card">
            <span class="label">TOTAL SPANS</span>
            <span class="value">{report.total_spans}</span>
          </div>
          <div class="stat-card">
            <span class="label">LATENCY</span>
            <span class="value highlight">{report.total_duration_ms} ms</span>
          </div>
          <div class="stat-card">
            <span class="label">ROOT SERVICE</span>
            <span class="value">{report.root_service}</span>
          </div>
        </div>

        <div class="waterfall-container">
          <div class="waterfall-header-row">
            <span class="col-service">Service / Span</span>
            <span class="col-timeline">Timing Waterfall (0ms → {report.total_duration_ms}ms)</span>
            <span class="col-delta">Offset / Duration</span>
            <span class="col-line">Line</span>
          </div>

          <div class="spans-list">
            {#each report.spans as span}
              <div
                class="span-row"
                on:click={() => dispatch('jumpToLine', span.line_number)}
                title="Click to jump to line #{span.line_number} in viewer"
              >
                <div class="col-service">
                  <span
                    class="service-tag"
                    style="background: {getServiceColor(span.service_name)}22; color: {getServiceColor(span.service_name)}; border: 1px solid {getServiceColor(span.service_name)}55;"
                  >
                    {span.service_name}
                  </span>
                  {#if span.span_id}
                    <code class="span-id">{span.span_id}</code>
                  {/if}
                </div>

                <div class="col-timeline">
                  <div class="timeline-track">
                    <div
                      class="timeline-bar"
                      style="left: {getBarLeft(span.offset_ms, report.total_duration_ms)}; width: {getBarWidth(span.duration_ms, report.total_duration_ms)}; background: {getServiceColor(span.service_name)};"
                    >
                      <span class="bar-label">{span.duration_ms}ms</span>
                    </div>
                  </div>
                </div>

                <div class="col-delta">
                  <span class="offset-badge">+{span.offset_ms}ms</span>
                  <span class="duration-badge">{span.duration_ms}ms</span>
                </div>

                <div class="col-line">
                  <button class="line-btn">#{span.line_number}</button>
                </div>
              </div>
              <div class="span-snippet">
                <span class="severity-badge sev-{span.severity.toLowerCase()}">{span.severity}</span>
                <span class="snippet-text">{span.message}</span>
              </div>
            {/each}
          </div>
        </div>
      {:else if !isLoading && !errorMessage}
        <div class="empty-state">
          <p>Enter a <code>trace_id</code>, <code>request_id</code>, or correlation token above to visualize distributed latency.</p>
        </div>
      {/if}

      <div class="modal-footer">
        <span class="footer-note">100% Offline Multi-Service Correlation • Zero Network Egress</span>
        <button class="btn btn-secondary" on:click={() => dispatch('close')}>Close</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .waterfall-modal {
    width: 960px;
    max-height: 85vh;
    border-radius: 14px;
    display: flex;
    flex-direction: column;
    padding: 24px;
    background: hsla(222, 24%, 7%, 0.98);
    border: 1px solid var(--border-color);
  }

  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .icon {
    font-size: 28px;
  }

  .header-title h3 {
    font-size: 18px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .subtitle {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .btn-close {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 18px;
    cursor: pointer;
  }

  .search-bar {
    display: flex;
    gap: 10px;
    margin-bottom: 16px;
  }

  .input-trace {
    flex: 1;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 8px 12px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
    outline: none;
  }

  .trace-overview {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    margin-bottom: 16px;
  }

  .stat-card {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .stat-card .label {
    font-size: 10px;
    font-weight: 700;
    color: var(--text-muted);
  }

  .stat-card .value {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .value.highlight {
    color: #4ade80;
  }

  .waterfall-container {
    flex: 1;
    overflow-y: auto;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.25);
    padding: 8px;
  }

  .waterfall-header-row {
    display: grid;
    grid-template-columns: 200px 1fr 140px 60px;
    font-size: 11px;
    font-weight: 700;
    color: var(--text-muted);
    padding: 6px 10px;
    border-bottom: 1px solid var(--border-color);
  }

  .spans-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 6px;
  }

  .span-row {
    display: grid;
    grid-template-columns: 200px 1fr 140px 60px;
    align-items: center;
    padding: 6px 10px;
    border-radius: 6px;
    cursor: pointer;
    background: rgba(255, 255, 255, 0.02);
    transition: background 0.15s ease;
  }

  .span-row:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .service-tag {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
  }

  .span-id {
    font-size: 10px;
    color: var(--text-muted);
    margin-left: 6px;
  }

  .timeline-track {
    position: relative;
    height: 18px;
    background: rgba(255, 255, 255, 0.04);
    border-radius: 4px;
    width: 100%;
  }

  .timeline-bar {
    position: absolute;
    top: 2px;
    bottom: 2px;
    border-radius: 3px;
    min-width: 8px;
    display: flex;
    align-items: center;
    padding-left: 4px;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
  }

  .bar-label {
    font-size: 9px;
    font-weight: 700;
    color: #ffffff;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.8);
    white-space: nowrap;
  }

  .col-delta {
    display: flex;
    gap: 6px;
    font-size: 11px;
    font-family: var(--font-mono);
  }

  .offset-badge {
    color: var(--text-secondary);
  }

  .duration-badge {
    color: #4ade80;
    font-weight: 600;
  }

  .line-btn {
    background: transparent;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    color: var(--accent-blue);
    font-size: 11px;
    font-family: var(--font-mono);
    cursor: pointer;
    padding: 2px 6px;
  }

  .span-snippet {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 10px 6px 20px;
    font-size: 11px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  }

  .severity-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 1px 4px;
    border-radius: 3px;
  }

  .sev-error, .sev-fatal {
    background: rgba(239, 68, 68, 0.2);
    color: #f87171;
  }

  .sev-warn {
    background: rgba(245, 158, 11, 0.2);
    color: #fbbf24;
  }

  .sev-info {
    background: rgba(59, 130, 246, 0.2);
    color: #60a5fa;
  }

  .snippet-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty-state {
    padding: 32px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }

  .error-banner {
    padding: 8px 12px;
    border-radius: 6px;
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #f87171;
    font-size: 12px;
    margin-bottom: 12px;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 16px;
  }

  .footer-note {
    font-size: 11px;
    color: var(--text-muted);
  }
</style>

