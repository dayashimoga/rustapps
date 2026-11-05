<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export interface KeyValuePair {
    key: string;
    value: string;
    enabled: boolean;
    description: string | null;
  }

  export interface ApiRequestItem {
    id: string;
    collection_id: string;
    name: string;
    protocol: string;
    method: string;
    url: string;
    headers: KeyValuePair[];
    query_params: KeyValuePair[];
    auth: Record<string, any>;
    body: {
      body_type: string;
      raw_content: string;
      graphql_query: string | null;
      graphql_variables: string | null;
    };
    timeout_ms: number;
  }

  export let request: ApiRequestItem;
  export let isExecuting: boolean = false;

  const dispatch = createEventDispatcher<{
    execute: ApiRequestItem;
    save: ApiRequestItem;
  }>();

  let activeTab: 'params' | 'headers' | 'auth' | 'body' | 'settings' = 'params';

  const methods = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'];
  const protocols = [
    { id: 'HttpRest', label: 'HTTP / REST' },
    { id: 'WebSocket', label: 'WebSocket' },
    { id: 'Grpc', label: 'gRPC' },
    { id: 'Mqtt', label: 'MQTT' },
  ];

  function addParam() {
    request.query_params = [
      ...request.query_params,
      { key: '', value: '', enabled: true, description: null },
    ];
  }

  function removeParam(index: number) {
    request.query_params = request.query_params.filter((_, i) => i !== index);
  }

  function addHeader() {
    request.headers = [
      ...request.headers,
      { key: '', value: '', enabled: true, description: null },
    ];
  }

  function removeHeader(index: number) {
    request.headers = request.headers.filter((_, i) => i !== index);
  }

  function handleSend() {
    dispatch('execute', request);
  }

  function handleSave() {
    dispatch('save', request);
  }

  function getMethodColor(m: string): string {
    switch (m.toUpperCase()) {
      case 'GET': return 'var(--method-get)';
      case 'POST': return 'var(--method-post)';
      case 'PUT': return 'var(--method-put)';
      case 'DELETE': return 'var(--method-delete)';
      case 'PATCH': return 'var(--method-patch)';
      default: return 'var(--method-head)';
    }
  }
</script>

<div class="composer-container">
  <!-- Top Address Bar -->
  <div class="address-bar-card">
    <div class="method-select-wrap">
      <select
        bind:value={request.method}
        class="method-select"
        style="color: {getMethodColor(request.method)}"
      >
        {#each methods as m}
          <option value={m}>{m}</option>
        {/each}
      </select>
    </div>

    <div class="url-input-wrap">
      <input
        type="text"
        bind:value={request.url}
        placeholder="Enter request URL (e.g., http://127.0.0.1:8080/api/v1/health or &#123;&#123;baseUrl&#125;&#125;/items)"
        class="url-input"
        on:keydown={(e) => e.key === 'Enter' && handleSend()}
      />
    </div>

    <div class="actions-wrap">
      <button class="btn btn-primary send-btn" on:click={handleSend} disabled={isExecuting}>
        {#if isExecuting}
          <span class="spinner">⏳</span> Sending...
        {:else}
          <span class="icon">🚀</span> Send
        {/if}
      </button>
      <button class="btn btn-secondary save-btn" on:click={handleSave} title="Save Request (Ctrl+S)">
        💾 Save
      </button>
    </div>
  </div>

  <!-- Sub Navigation Tabs -->
  <div class="subtabs-bar">
    <button
      class="subtab-btn"
      class:active={activeTab === 'params'}
      on:click={() => (activeTab = 'params')}
    >
      Params {#if request.query_params.length > 0}<span class="badge">{request.query_params.length}</span>{/if}
    </button>
    <button
      class="subtab-btn"
      class:active={activeTab === 'headers'}
      on:click={() => (activeTab = 'headers')}
    >
      Headers {#if request.headers.length > 0}<span class="badge">{request.headers.length}</span>{/if}
    </button>
    <button
      class="subtab-btn"
      class:active={activeTab === 'auth'}
      on:click={() => (activeTab = 'auth')}
    >
      Auth <span class="badge-sub">{request.auth?.type || 'None'}</span>
    </button>
    <button
      class="subtab-btn"
      class:active={activeTab === 'body'}
      on:click={() => (activeTab = 'body')}
    >
      Body <span class="badge-sub">{request.body?.body_type || 'none'}</span>
    </button>
    <button
      class="subtab-btn"
      class:active={activeTab === 'settings'}
      on:click={() => (activeTab = 'settings')}
    >
      Settings
    </button>
  </div>

  <!-- Tab Content Pane -->
  <div class="tab-content-area">
    {#if activeTab === 'params'}
      <div class="kv-editor">
        <div class="kv-header-row">
          <span class="col-check"></span>
          <span class="col-key">Parameter Key</span>
          <span class="col-val">Value</span>
          <span class="col-desc">Description</span>
          <span class="col-act"></span>
        </div>
        {#each request.query_params as param, index}
          <div class="kv-row">
            <input type="checkbox" bind:checked={param.enabled} class="col-check" />
            <input type="text" bind:value={param.key} placeholder="Key" class="kv-input col-key" />
            <input type="text" bind:value={param.value} placeholder="Value" class="kv-input col-val" />
            <input type="text" bind:value={param.description} placeholder="Description (optional)" class="kv-input col-desc" />
            <button class="btn-del col-act" on:click={() => removeParam(index)} title="Delete parameter">✕</button>
          </div>
        {/each}
        <button class="btn btn-outline-add" on:click={addParam}>+ Add Query Parameter</button>
      </div>

    {:else if activeTab === 'headers'}
      <div class="kv-editor">
        <div class="kv-header-row">
          <span class="col-check"></span>
          <span class="col-key">Header Name</span>
          <span class="col-val">Header Value</span>
          <span class="col-desc">Description</span>
          <span class="col-act"></span>
        </div>
        {#each request.headers as header, index}
          <div class="kv-row">
            <input type="checkbox" bind:checked={header.enabled} class="col-check" />
            <input type="text" bind:value={header.key} placeholder="Header (e.g. Content-Type)" class="kv-input col-key" />
            <input type="text" bind:value={header.value} placeholder="Value (e.g. application/json)" class="kv-input col-val" />
            <input type="text" bind:value={header.description} placeholder="Description (optional)" class="kv-input col-desc" />
            <button class="btn-del col-act" on:click={() => removeHeader(index)} title="Delete header">✕</button>
          </div>
        {/each}
        <button class="btn btn-outline-add" on:click={addHeader}>+ Add Custom Header</button>
      </div>

    {:else if activeTab === 'auth'}
      <div class="auth-editor">
        <div class="auth-type-row">
          <label for="auth-select">Authentication Type</label>
          <select id="auth-select" bind:value={request.auth.type} class="styled-select">
            <option value="None">No Auth</option>
            <option value="Bearer">Bearer Token</option>
            <option value="ApiKey">API Key</option>
            <option value="Basic">Basic Auth (Base64)</option>
          </select>
        </div>

        {#if request.auth.type === 'Bearer'}
          <div class="auth-fields">
            <label for="bearer-tok">Token</label>
            <input
              id="bearer-tok"
              type="password"
              bind:value={request.auth.token}
              placeholder="Paste bearer token or &#123;&#123;apiKey&#125;&#125; variable..."
              class="styled-input"
            />
          </div>
        {:else if request.auth.type === 'ApiKey'}
          <div class="auth-fields grid-2">
            <div>
              <label for="apikey-k">Key Header/Query</label>
              <input id="apikey-k" type="text" bind:value={request.auth.key} placeholder="X-API-Key" class="styled-input" />
            </div>
            <div>
              <label for="apikey-v">Value</label>
              <input id="apikey-v" type="password" bind:value={request.auth.value} placeholder="Value or &#123;&#123;secretKey&#125;&#125;" class="styled-input" />
            </div>
            <div class="full-span">
              <label>
                <input type="checkbox" bind:checked={request.auth.in_header} /> Send in HTTP Header (otherwise Query Parameter)
              </label>
            </div>
          </div>
        {:else if request.auth.type === 'Basic'}
          <div class="auth-fields grid-2">
            <div>
              <label for="basic-u">Username</label>
              <input id="basic-u" type="text" bind:value={request.auth.username} placeholder="Username" class="styled-input" />
            </div>
            <div>
              <label for="basic-p">Password</label>
              <input id="basic-p" type="password" bind:value={request.auth.password} placeholder="Password" class="styled-input" />
            </div>
          </div>
        {:else}
          <div class="empty-hint">This request does not use any authentication credentials.</div>
        {/if}
      </div>

    {:else if activeTab === 'body'}
      <div class="body-editor">
        <div class="body-type-selector">
          {#each ['none', 'json', 'raw', 'graphql'] as btype}
            <label class="radio-label">
              <input
                type="radio"
                name="bodyType"
                value={btype}
                bind:group={request.body.body_type}
              />
              {btype.toUpperCase()}
            </label>
          {/each}
        </div>

        {#if request.body.body_type === 'none'}
          <div class="empty-hint">No body payload will be transmitted with this request.</div>
        {:else if request.body.body_type === 'graphql'}
          <div class="graphql-editor-grid">
            <div class="gql-col">
              <label for="gql-q">GraphQL Query / Mutation</label>
              <textarea
                id="gql-q"
                bind:value={request.body.graphql_query}
                placeholder="query GetUsers &#123; users &#123; id name email &#125; &#125;"
                class="code-textarea"
                rows="10"
              ></textarea>
            </div>
            <div class="gql-col">
              <label for="gql-v">Variables (JSON)</label>
              <textarea
                id="gql-v"
                bind:value={request.body.graphql_variables}
                placeholder="&#123;&#10;  &quot;limit&quot;: 10&#10;&#125;"
                class="code-textarea"
                rows="10"
              ></textarea>
            </div>
          </div>
        {:else}
          <div class="raw-body-wrap">
            <textarea
              bind:value={request.body.raw_content}
              placeholder={request.body.body_type === 'json' ? "{\n  \"key\": \"value\"\n}" : "Raw request body content..."}
              class="code-textarea"
              rows="12"
            ></textarea>
          </div>
        {/if}
      </div>

    {:else if activeTab === 'settings'}
      <div class="settings-editor">
        <div class="setting-item">
          <label for="proto-sel">Protocol Type</label>
          <select id="proto-sel" bind:value={request.protocol} class="styled-select">
            {#each protocols as p}
              <option value={p.id}>{p.label}</option>
            {/each}
          </select>
        </div>
        <div class="setting-item">
          <label for="timeout-input">Connection Timeout (ms)</label>
          <input
            id="timeout-input"
            type="number"
            bind:value={request.timeout_ms}
            min="500"
            max="60000"
            step="500"
            class="styled-input"
          />
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .composer-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #0f172a;
    border-right: 1px solid rgba(75, 85, 99, 0.3);
  }
  .address-bar-card {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: #1e293b;
    border-bottom: 1px solid rgba(75, 85, 99, 0.4);
  }
  .method-select-wrap {
    flex-shrink: 0;
  }
  .method-select {
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 6px;
    padding: 8px 12px;
    font-weight: 700;
    font-size: 0.9rem;
    cursor: pointer;
  }
  .method-select:focus {
    outline: none;
    border-color: #6366f1;
  }
  .url-input-wrap {
    flex: 1;
  }
  .url-input {
    width: 100%;
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 6px;
    padding: 8px 12px;
    color: #f8fafc;
    font-family: var(--font-mono);
    font-size: 0.9rem;
  }
  .url-input:focus {
    outline: none;
    border-color: #6366f1;
  }
  .actions-wrap {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: all 0.15s ease;
  }
  .send-btn {
    background: #10b981;
    color: #ffffff;
  }
  .send-btn:hover:not(:disabled) {
    background: #059669;
  }
  .send-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .save-btn {
    background: #334155;
    color: #cbd5e1;
  }
  .save-btn:hover {
    background: #475569;
    color: #ffffff;
  }
  .subtabs-bar {
    display: flex;
    gap: 4px;
    padding: 0 16px;
    background: #1e293b;
    border-bottom: 1px solid rgba(75, 85, 99, 0.3);
  }
  .subtab-btn {
    background: none;
    border: none;
    padding: 10px 14px;
    color: #94a3b8;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .subtab-btn:hover {
    color: #e2e8f0;
  }
  .subtab-btn.active {
    color: #38bdf8;
    border-bottom-color: #38bdf8;
  }
  .badge {
    background: rgba(56, 189, 248, 0.2);
    color: #38bdf8;
    font-size: 0.7rem;
    padding: 1px 6px;
    border-radius: 9999px;
  }
  .badge-sub {
    font-size: 0.7rem;
    color: #64748b;
  }
  .tab-content-area {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }
  .kv-editor {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .kv-header-row {
    display: flex;
    gap: 8px;
    padding: 4px 8px;
    font-size: 0.75rem;
    font-weight: 600;
    color: #64748b;
    text-transform: uppercase;
  }
  .kv-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .col-check {
    width: 24px;
    cursor: pointer;
  }
  .col-key {
    flex: 1.5;
  }
  .col-val {
    flex: 2;
  }
  .col-desc {
    flex: 1.5;
  }
  .col-act {
    width: 28px;
  }
  .kv-input {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 4px;
    padding: 6px 10px;
    color: #f1f5f9;
    font-size: 0.85rem;
    font-family: var(--font-mono);
  }
  .kv-input:focus {
    outline: none;
    border-color: #38bdf8;
  }
  .btn-del {
    background: none;
    border: none;
    color: #ef4444;
    cursor: pointer;
    font-size: 0.9rem;
    padding: 4px;
  }
  .btn-del:hover {
    color: #f87171;
  }
  .btn-outline-add {
    align-self: flex-start;
    margin-top: 10px;
    background: transparent;
    border: 1px dashed #475569;
    color: #94a3b8;
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .btn-outline-add:hover {
    border-color: #38bdf8;
    color: #38bdf8;
  }
  .auth-editor {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 600px;
  }
  .auth-type-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .auth-fields {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .full-span {
    grid-column: span 2;
  }
  .styled-select, .styled-input {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 6px;
    padding: 8px 12px;
    color: #f8fafc;
    font-size: 0.85rem;
  }
  .styled-select:focus, .styled-input:focus {
    outline: none;
    border-color: #38bdf8;
  }
  .body-editor {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
  }
  .body-type-selector {
    display: flex;
    gap: 16px;
    font-size: 0.85rem;
    color: #cbd5e1;
  }
  .radio-label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
  }
  .code-textarea {
    width: 100%;
    background: #0b0f19;
    border: 1px solid #334155;
    border-radius: 6px;
    padding: 12px;
    color: #a5f3fc;
    font-family: var(--font-mono);
    font-size: 0.85rem;
    line-height: 1.5;
    resize: vertical;
  }
  .code-textarea:focus {
    outline: none;
    border-color: #38bdf8;
  }
  .graphql-editor-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .gql-col label {
    display: block;
    font-size: 0.8rem;
    color: #94a3b8;
    margin-bottom: 6px;
  }
  .empty-hint {
    padding: 24px;
    text-align: center;
    color: #64748b;
    font-size: 0.85rem;
  }
  .settings-editor {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 400px;
  }
  .setting-item {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .setting-item label {
    font-size: 0.8rem;
    color: #94a3b8;
  }
</style>
