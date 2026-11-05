<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import LicenseModal, { type LicenseStatusReport } from './components/LicenseModal.svelte';
  import RequestComposer, { type ApiRequestItem, type KeyValuePair } from './components/RequestComposer.svelte';
  import ResponseInspector, { type ApiResponseSnapshot } from './components/ResponseInspector.svelte';
  import FuzzerRunner from './components/FuzzerRunner.svelte';

  interface ApiCollection {
    id: string;
    name: string;
    description: string | null;
    variables: KeyValuePair[];
    created_at: number;
    updated_at: number;
  }

  interface StudioStats {
    total_collections: number;
    total_requests: number;
    total_responses: number;
  }

  let collections: ApiCollection[] = [];
  let activeCollectionId: string = '';
  let requests: ApiRequestItem[] = [];
  let activeRequestId: string = '';
  let activeRequest: ApiRequestItem | null = null;
  let activeResponse: ApiResponseSnapshot | null = null;

  let activeView: 'composer' | 'fuzzer' = 'composer';
  let isLicenseModalOpen: boolean = false;
  let licenseTier: string = 'FREE';
  let isExecuting: boolean = false;

  let stats: StudioStats = {
    total_collections: 0,
    total_requests: 0,
    total_responses: 0,
  };

  let newCollName: string = '';
  let showNewCollInput: boolean = false;

  onMount(async () => {
    await loadInitialData();
  });

  async function loadInitialData() {
    await refreshStats();
    await loadCollections();
    try {
      const lic = await invokeIpc<LicenseStatusReport>('get_license_status');
      if (lic) licenseTier = lic.tier.toUpperCase();
    } catch (e) {
      console.warn('License status fetch error:', e);
    }
  }

  async function refreshStats() {
    try {
      const res = await invokeIpc<StudioStats>('get_studio_stats');
      if (res) stats = res;
    } catch (e) {
      console.error('Failed to get stats:', e);
    }
  }

  async function loadCollections() {
    try {
      const colls = await invokeIpc<ApiCollection[]>('list_collections');
      collections = colls || [];
      if (collections.length > 0 && !activeCollectionId) {
        selectCollection(collections[0].id);
      } else if (collections.length === 0) {
        // Auto-seed a default collection
        await createDefaultCollection();
      }
    } catch (e) {
      console.error('Failed to list collections:', e);
    }
  }

  async function createDefaultCollection() {
    const coll: ApiCollection = {
      id: `coll-${Date.now()}`,
      name: 'Default Local API',
      description: 'Default collection for local and offline endpoints',
      variables: [
        { key: 'baseUrl', value: 'http://127.0.0.1:8080', enabled: true, description: 'Default local server' },
      ],
      created_at: Math.floor(Date.now() / 1000),
      updated_at: Math.floor(Date.now() / 1000),
    };
    await invokeIpc('save_collection', { collection: coll });
    await loadCollections();
    if (collections.length > 0) {
      selectCollection(collections[0].id);
    }
  }

  async function selectCollection(id: string) {
    activeCollectionId = id;
    await loadRequests(id);
  }

  async function loadRequests(collectionId: string) {
    try {
      const reqs = await invokeIpc<ApiRequestItem[]>('list_requests', { collection_id: collectionId });
      requests = reqs || [];
      if (requests.length > 0) {
        selectRequest(requests[0]);
      } else {
        // Auto create a sample request in this collection
        await createNewRequest();
      }
    } catch (e) {
      console.error('Failed to load requests:', e);
    }
  }

  function selectRequest(req: ApiRequestItem) {
    activeRequestId = req.id;
    activeRequest = { ...req };
    loadLatestResponse(req.id);
  }

  async function loadLatestResponse(requestId: string) {
    try {
      const resp = await invokeIpc<ApiResponseSnapshot | null>('get_latest_response', { request_id: requestId });
      activeResponse = resp;
    } catch {
      activeResponse = null;
    }
  }

  async function handleAddCollection() {
    if (!newCollName.trim()) return;
    const coll: ApiCollection = {
      id: `coll-${Date.now()}`,
      name: newCollName.trim(),
      description: 'Air-Gapped API collection',
      variables: [
        { key: 'baseUrl', value: 'http://127.0.0.1:8080', enabled: true, description: null },
      ],
      created_at: Math.floor(Date.now() / 1000),
      updated_at: Math.floor(Date.now() / 1000),
    };
    await invokeIpc('save_collection', { collection: coll });
    newCollName = '';
    showNewCollInput = false;
    await loadCollections();
    selectCollection(coll.id);
    await refreshStats();
  }

  async function handleDeleteCollection(id: string, e: MouseEvent) {
    e.stopPropagation();
    if (!confirm('Are you sure you want to delete this collection and all its requests?')) return;
    await invokeIpc('delete_collection', { id });
    activeCollectionId = '';
    await loadCollections();
    await refreshStats();
  }

  async function createNewRequest() {
    if (!activeCollectionId) return;
    const req: ApiRequestItem = {
      id: `req-${Date.now()}`,
      collection_id: activeCollectionId,
      name: 'New HTTP Request',
      protocol: 'HttpRest',
      method: 'GET',
      url: '{{baseUrl}}/api/v1/health',
      headers: [
        { key: 'Accept', value: 'application/json', enabled: true, description: null },
      ],
      query_params: [],
      auth: { type: 'None' },
      body: {
        body_type: 'none',
        raw_content: '',
        graphql_query: null,
        graphql_variables: null,
      },
      timeout_ms: 5000,
    };
    await invokeIpc('save_request', { request: req });
    await loadRequests(activeCollectionId);
    selectRequest(req);
    await refreshStats();
  }

  async function handleSaveRequest(e: CustomEvent<ApiRequestItem>) {
    const updated = e.detail;
    await invokeIpc('save_request', { request: updated });
    activeRequest = updated;
    const idx = requests.findIndex((r) => r.id === updated.id);
    if (idx >= 0) {
      requests[idx] = updated;
    }
  }

  async function handleDeleteRequest(id: string, e: MouseEvent) {
    e.stopPropagation();
    await invokeIpc('delete_request', { id });
    await loadRequests(activeCollectionId);
    await refreshStats();
  }

  async function handleExecuteRequest(e: CustomEvent<ApiRequestItem>) {
    const req = e.detail;
    isExecuting = true;
    try {
      // First save any unsaved changes
      await invokeIpc('save_request', { request: req });
      const resp = await invokeIpc<ApiResponseSnapshot>('execute_request', {
        request_id: req.id,
      });
      activeResponse = resp;
      await refreshStats();
    } catch (err: any) {
      console.error('Request execution error:', err);
    } finally {
      isExecuting = false;
    }
  }

  function handleLicenseStatusChange(e: CustomEvent<LicenseStatusReport>) {
    licenseTier = e.detail.tier.toUpperCase();
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

<div class="app-layout">
  <!-- Left Sidebar -->
  <aside class="sidebar">
    <!-- Top Branding -->
    <div class="sidebar-header">
      <div class="brand">
        <span class="brand-icon">⚡</span>
        <div class="brand-text">
          <span class="brand-title">API STUDIO</span>
          <span class="airgap-badge">100% AIR-GAPPED</span>
        </div>
      </div>
      <button class="tier-badge" on:click={() => (isLicenseModalOpen = true)}>
        [{licenseTier}]
      </button>
    </div>

    <!-- Quick Stats -->
    <div class="sidebar-stats">
      <div class="s-stat">
        <span class="s-val">{stats.total_collections}</span>
        <span class="s-lbl">Colls</span>
      </div>
      <div class="s-stat">
        <span class="s-val">{stats.total_requests}</span>
        <span class="s-lbl">Reqs</span>
      </div>
      <div class="s-stat">
        <span class="s-val">{stats.total_responses}</span>
        <span class="s-lbl">Runs</span>
      </div>
    </div>

    <!-- Collections List -->
    <div class="sidebar-section-header">
      <span>COLLECTIONS</span>
      <button class="btn-icon" on:click={() => (showNewCollInput = !showNewCollInput)} title="Add Collection">
        +
      </button>
    </div>

    {#if showNewCollInput}
      <div class="new-coll-row">
        <input
          type="text"
          bind:value={newCollName}
          placeholder="Collection name..."
          class="new-coll-input"
          on:keydown={(e) => e.key === 'Enter' && handleAddCollection()}
        />
        <button class="btn-confirm" on:click={handleAddCollection}>✓</button>
      </div>
    {/if}

    <div class="collections-list">
      {#each collections as coll}
        <div
          class="coll-item"
          class:active={coll.id === activeCollectionId}
          on:click={() => selectCollection(coll.id)}
          on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && selectCollection(coll.id)}
          role="button"
          tabindex="0"
        >
          <div class="coll-name-row">
            <span class="coll-icon">📁</span>
            <span class="coll-title">{coll.name}</span>
            <button
              class="btn-trash"
              on:click={(e) => handleDeleteCollection(coll.id, e)}
              title="Delete Collection"
            >
              🗑️
            </button>
          </div>
        </div>
      {/each}
    </div>

    <!-- Requests in Active Collection -->
    {#if activeCollectionId}
      <div class="sidebar-section-header">
        <span>REQUESTS</span>
        <button class="btn-icon" on:click={createNewRequest} title="Add Request">
          +
        </button>
      </div>

      <div class="requests-list">
        {#each requests as req}
          <div
            class="req-item"
            class:active={req.id === activeRequestId}
            on:click={() => selectRequest(req)}
            on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && selectRequest(req)}
            role="button"
            tabindex="0"
          >
            <span class="req-method" style="color: {getMethodColor(req.method)}">
              {req.method}
            </span>
            <span class="req-title">{req.name}</span>
            <button
              class="btn-trash"
              on:click={(e) => handleDeleteRequest(req.id, e)}
              title="Delete Request"
            >
              ✕
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </aside>

  <!-- Main Content Workspace -->
  <main class="main-workspace">
    {#if activeRequest}
      <!-- Workspace Top Bar -->
      <header class="workspace-header">
        <div class="req-info-row">
          <input
            type="text"
            bind:value={activeRequest.name}
            class="req-name-input"
            on:blur={() => handleSaveRequest(new CustomEvent('save', { detail: activeRequest }))}
          />
        </div>

        <div class="view-switch-tabs">
          <button
            class="vtab-btn"
            class:active={activeView === 'composer'}
            on:click={() => (activeView = 'composer')}
          >
            📡 Composer
          </button>
          <button
            class="vtab-btn"
            class:active={activeView === 'fuzzer'}
            on:click={() => (activeView = 'fuzzer')}
          >
            ⚡ Security Fuzzer
          </button>
        </div>
      </header>

      <!-- View Area -->
      <div class="workspace-body">
        {#if activeView === 'composer'}
          <div class="composer-split-view">
            <div class="left-pane">
              <RequestComposer
                bind:request={activeRequest}
                {isExecuting}
                on:execute={handleExecuteRequest}
                on:save={handleSaveRequest}
              />
            </div>
            <div class="right-pane">
              <ResponseInspector response={activeResponse} {isExecuting} />
            </div>
          </div>
        {:else if activeView === 'fuzzer'}
          <FuzzerRunner requestId={activeRequest.id} />
        {/if}
      </div>
    {:else}
      <div class="no-req-view">
        <div class="no-req-icon">⚡</div>
        <h2>Air-Gapped API & Protocol Studio</h2>
        <p>Select a request from the sidebar or click + to compose a new offline endpoint test.</p>
      </div>
    {/if}
  </main>
</div>

<!-- Offline Ed25519 Licensing Modal -->
<LicenseModal
  isOpen={isLicenseModalOpen}
  appName="Air-Gapped API & Protocol Studio"
  on:close={() => (isLicenseModalOpen = false)}
  on:statusChange={handleLicenseStatusChange}
/>

<style>
  .app-layout {
    display: flex;
    width: 100vw;
    height: 100vh;
    background: #0b0f19;
    overflow: hidden;
  }
  .sidebar {
    width: 280px;
    background: #111827;
    border-right: 1px solid rgba(75, 85, 99, 0.4);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }
  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px;
    border-bottom: 1px solid rgba(75, 85, 99, 0.3);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .brand-icon {
    font-size: 1.4rem;
  }
  .brand-text {
    display: flex;
    flex-direction: column;
  }
  .brand-title {
    font-weight: 800;
    font-size: 0.95rem;
    letter-spacing: 0.05em;
    color: #f8fafc;
  }
  .airgap-badge {
    font-size: 0.65rem;
    color: #10b981;
    font-weight: 600;
  }
  .tier-badge {
    background: #1e293b;
    border: 1px solid #334155;
    color: #38bdf8;
    font-size: 0.75rem;
    font-weight: 700;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
  }
  .tier-badge:hover {
    background: #334155;
  }
  .sidebar-stats {
    display: flex;
    padding: 10px 16px;
    background: rgba(15, 23, 42, 0.5);
    border-bottom: 1px solid rgba(75, 85, 99, 0.2);
  }
  .s-stat {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }
  .s-val {
    font-size: 0.95rem;
    font-weight: 700;
    color: #e2e8f0;
  }
  .s-lbl {
    font-size: 0.65rem;
    color: #64748b;
    text-transform: uppercase;
  }
  .sidebar-section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px 6px;
    font-size: 0.7rem;
    font-weight: 700;
    color: #64748b;
    letter-spacing: 0.05em;
  }
  .btn-icon {
    background: none;
    border: none;
    color: #94a3b8;
    font-size: 1.1rem;
    cursor: pointer;
    line-height: 1;
  }
  .btn-icon:hover {
    color: #38bdf8;
  }
  .new-coll-row {
    display: flex;
    gap: 6px;
    padding: 6px 16px;
  }
  .new-coll-input {
    flex: 1;
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 4px;
    padding: 4px 8px;
    color: #f8fafc;
    font-size: 0.8rem;
  }
  .btn-confirm {
    background: #10b981;
    border: none;
    color: white;
    padding: 4px 8px;
    border-radius: 4px;
    cursor: pointer;
  }
  .collections-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px 8px;
    max-height: 180px;
    overflow-y: auto;
    border-bottom: 1px solid rgba(75, 85, 99, 0.2);
  }
  .coll-item {
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
    color: #cbd5e1;
    transition: background 0.1s;
  }
  .coll-item:hover {
    background: #1e293b;
  }
  .coll-item.active {
    background: #1e293b;
    color: #38bdf8;
    font-weight: 600;
  }
  .coll-name-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .coll-title {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .btn-trash {
    background: none;
    border: none;
    color: #64748b;
    cursor: pointer;
    font-size: 0.8rem;
    opacity: 0.5;
  }
  .btn-trash:hover {
    opacity: 1;
    color: #ef4444;
  }
  .requests-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .req-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.8rem;
    color: #cbd5e1;
  }
  .req-item:hover {
    background: #1e293b;
  }
  .req-item.active {
    background: #1e293b;
    border-left: 3px solid #38bdf8;
    color: #ffffff;
  }
  .req-method {
    font-weight: 700;
    font-size: 0.75rem;
    width: 48px;
    flex-shrink: 0;
  }
  .req-title {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .main-workspace {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
  .workspace-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 20px;
    background: #111827;
    border-bottom: 1px solid rgba(75, 85, 99, 0.4);
  }
  .req-info-row {
    flex: 1;
  }
  .req-name-input {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    color: #f8fafc;
    font-size: 1.1rem;
    font-weight: 700;
    padding: 4px 8px;
    width: 60%;
  }
  .req-name-input:hover {
    border-color: #334155;
  }
  .req-name-input:focus {
    background: #0f172a;
    border-color: #38bdf8;
    outline: none;
  }
  .view-switch-tabs {
    display: flex;
    gap: 6px;
  }
  .vtab-btn {
    background: #1e293b;
    border: 1px solid #334155;
    color: #94a3b8;
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }
  .vtab-btn:hover {
    color: #e2e8f0;
  }
  .vtab-btn.active {
    background: #38bdf8;
    color: #0b0f19;
    border-color: #38bdf8;
  }
  .workspace-body {
    flex: 1;
    height: calc(100vh - 65px);
    overflow: hidden;
  }
  .composer-split-view {
    display: grid;
    grid-template-columns: 1fr 1fr;
    height: 100%;
  }
  .left-pane, .right-pane {
    height: 100%;
    overflow: hidden;
  }
  .no-req-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: #64748b;
  }
  .no-req-icon {
    font-size: 3rem;
  }
  .no-req-view h2 {
    color: #e2e8f0;
  }
</style>
