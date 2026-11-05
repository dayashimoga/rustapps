<script lang="ts">
  export interface KeyValuePair {
    key: string;
    value: string;
    enabled: boolean;
    description: string | null;
  }

  export interface ApiResponseSnapshot {
    id: string;
    request_id: string;
    status: number;
    status_text: string;
    headers: KeyValuePair[];
    body: string;
    latency_ms: number;
    size_bytes: number;
    timestamp: number;
    is_fuzzing_run: boolean;
  }

  export let response: ApiResponseSnapshot | null = null;
  export let isExecuting: boolean = false;

  let activeTab: 'body' | 'headers' | 'raw' = 'body';
  let copied: boolean = false;

  function copyBody() {
    if (!response) return;
    navigator.clipboard.writeText(response.body);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }

  function getStatusClass(status: number): string {
    if (status >= 200 && status < 300) return 'status-2xx';
    if (status >= 300 && status < 400) return 'status-3xx';
    if (status >= 400 && status < 500) return 'status-4xx';
    return 'status-5xx';
  }

  function tryFormatJson(body: string): string {
    try {
      return JSON.stringify(JSON.parse(body), null, 2);
    } catch {
      return body;
    }
  }
</script>

<div class="inspector-container">
  {#if isExecuting}
    <div class="empty-state">
      <div class="spinner-large">⏳</div>
      <p>Sending request over air-gapped transport...</p>
    </div>
  {:else if !response}
    <div class="empty-state">
      <div class="empty-icon">📡</div>
      <p>Send a request or execute security fuzzer to inspect offline response data.</p>
    </div>
  {:else}
    <!-- Status & Metrics Header -->
    <div class="response-header-bar">
      <div class="status-indicator {getStatusClass(response.status)}">
        <span class="status-code">{response.status}</span>
        <span class="status-text">{response.status_text}</span>
      </div>

      <div class="metric-pill">
        <span class="metric-label">TIME</span>
        <span class="metric-val">{response.latency_ms} ms</span>
      </div>

      <div class="metric-pill">
        <span class="metric-label">SIZE</span>
        <span class="metric-val">{formatBytes(response.size_bytes)}</span>
      </div>

      {#if response.is_fuzzing_run}
        <div class="fuzz-pill">⚡ FUZZ RUN</div>
      {/if}

      <div class="header-actions">
        <button class="btn-copy" on:click={copyBody}>
          {copied ? '✓ Copied' : '📋 Copy Body'}
        </button>
      </div>
    </div>

    <!-- Subtabs -->
    <div class="resp-subtabs">
      <button
        class="resp-tab-btn"
        class:active={activeTab === 'body'}
        on:click={() => (activeTab = 'body')}
      >
        Body
      </button>
      <button
        class="resp-tab-btn"
        class:active={activeTab === 'headers'}
        on:click={() => (activeTab = 'headers')}
      >
        Headers ({response.headers.length})
      </button>
      <button
        class="resp-tab-btn"
        class:active={activeTab === 'raw'}
        on:click={() => (activeTab = 'raw')}
      >
        Raw
      </button>
    </div>

    <!-- Tab View -->
    <div class="resp-tab-content">
      {#if activeTab === 'body'}
        <div class="code-view">
          <pre><code>{tryFormatJson(response.body)}</code></pre>
        </div>
      {:else if activeTab === 'headers'}
        <div class="headers-table">
          <div class="headers-head">
            <span>Header</span>
            <span>Value</span>
          </div>
          {#each response.headers as h}
            <div class="headers-row">
              <span class="h-key">{h.key}</span>
              <span class="h-val">{h.value}</span>
            </div>
          {/each}
        </div>
      {:else if activeTab === 'raw'}
        <div class="code-view">
          <pre><code>HTTP/1.1 {response.status} {response.status_text}&#10;{#each response.headers as h}{h.key}: {h.value}&#10;{/each}&#10;{response.body}</code></pre>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .inspector-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #0b0f19;
  }
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: #64748b;
    font-size: 0.9rem;
    padding: 32px;
    text-align: center;
  }
  .empty-icon {
    font-size: 2.5rem;
  }
  .spinner-large {
    font-size: 2rem;
    animation: pulse 1.5s infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 1; }
  }
  .response-header-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    background: #111827;
    border-bottom: 1px solid rgba(75, 85, 99, 0.4);
  }
  .status-indicator {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 700;
  }
  .status-2xx {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.3);
  }
  .status-3xx {
    background: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.3);
  }
  .status-4xx {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
    border: 1px solid rgba(245, 158, 11, 0.3);
  }
  .status-5xx {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.3);
  }
  .metric-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: #1e293b;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.75rem;
    color: #cbd5e1;
  }
  .metric-label {
    color: #64748b;
    font-weight: 600;
  }
  .fuzz-pill {
    background: rgba(139, 92, 246, 0.2);
    color: #c084fc;
    font-size: 0.75rem;
    font-weight: 700;
    padding: 4px 8px;
    border-radius: 6px;
  }
  .header-actions {
    margin-left: auto;
  }
  .btn-copy {
    background: #1e293b;
    border: 1px solid #334155;
    color: #cbd5e1;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.75rem;
    cursor: pointer;
  }
  .btn-copy:hover {
    background: #334155;
    color: #ffffff;
  }
  .resp-subtabs {
    display: flex;
    gap: 4px;
    padding: 0 16px;
    background: #111827;
    border-bottom: 1px solid rgba(75, 85, 99, 0.3);
  }
  .resp-tab-btn {
    background: none;
    border: none;
    padding: 8px 12px;
    color: #94a3b8;
    font-size: 0.8rem;
    cursor: pointer;
    border-bottom: 2px solid transparent;
  }
  .resp-tab-btn:hover {
    color: #e2e8f0;
  }
  .resp-tab-btn.active {
    color: #38bdf8;
    border-bottom-color: #38bdf8;
  }
  .resp-tab-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }
  .code-view {
    height: 100%;
    overflow: auto;
  }
  pre {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 0.85rem;
    line-height: 1.5;
    color: #e2e8f0;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .headers-table {
    display: flex;
    flex-direction: column;
    border: 1px solid #1e293b;
    border-radius: 6px;
    overflow: hidden;
  }
  .headers-head {
    display: grid;
    grid-template-columns: 200px 1fr;
    background: #1e293b;
    padding: 8px 12px;
    font-size: 0.75rem;
    font-weight: 600;
    color: #64748b;
    text-transform: uppercase;
  }
  .headers-row {
    display: grid;
    grid-template-columns: 200px 1fr;
    padding: 8px 12px;
    font-size: 0.8rem;
    border-top: 1px solid #1e293b;
  }
  .headers-row:hover {
    background: rgba(30, 41, 59, 0.4);
  }
  .h-key {
    color: #38bdf8;
    font-family: var(--font-mono);
  }
  .h-val {
    color: #cbd5e1;
    font-family: var(--font-mono);
    word-break: break-all;
  }
</style>
