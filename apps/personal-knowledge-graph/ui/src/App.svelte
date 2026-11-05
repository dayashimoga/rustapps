<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import GraphCanvas from './components/GraphCanvas.svelte';
  import LicenseModal from './components/LicenseModal.svelte';

  // State
  let nodes: any[] = [];
  let edges: any[] = [];
  let stats: any = { total_nodes: 0, total_edges: 0, density: 0 };
  let selectedNode: any = null;
  let searchQuery = '';
  let isSemanticSearch = false;
  let isEditing = false;
  let isLicenseModalOpen = false;
  let currentTier = 'FREE';

  // Editor fields
  let editTitle = '';
  let editContent = '';
  let editType = 'note';
  let editTags = '';

  // Shortest path finder
  let pathTargetId = '';
  let foundPath: any[] = [];

  async function loadGraph() {
    try {
      const data: any = await invokeIpc('get_graph_visualization');
      nodes = data.nodes || [];
      edges = data.edges || [];

      const s: any = await invokeIpc('get_graph_stats');
      stats = s || { total_nodes: 0, total_edges: 0, density: 0 };
    } catch (e) {
      console.error('Failed to load graph:', e);
    }
  }

  async function loadLicenseStatus() {
    try {
      const res: any = await invokeIpc('get_license_status');
      if (res && res.tier) {
        currentTier = res.tier.toUpperCase();
      }
    } catch (e) {
      console.error('Failed to load license status:', e);
    }
  }

  function handleNodeSelect(event: CustomEvent<{ node: any }>) {
    const node = event.detail.node;
    selectNode(node.id);
  }

  async function selectNode(nodeId: string) {
    try {
      const node: any = await invokeIpc('get_node', { id: nodeId });
      if (node) {
        selectedNode = node;
        editTitle = node.title;
        editContent = node.content;
        editType = node.node_type;
        editTags = (node.tags || []).join(', ');
        isEditing = false;
        foundPath = [];
      }
    } catch (e) {
      console.error('Failed to get node:', e);
    }
  }

  function createNewNote() {
    selectedNode = {
      id: null,
      title: 'Untitled Note',
      content: '',
      node_type: 'note',
      tags: [],
    };
    editTitle = 'Untitled Note';
    editContent = '';
    editType = 'note';
    editTags = '';
    isEditing = true;
    foundPath = [];
  }

  async function saveCurrentNote() {
    try {
      const tagsArray = editTags
        .split(',')
        .map((t) => t.trim())
        .filter((t) => t.length > 0);

      const saved: any = await invokeIpc('save_note', {
        id: selectedNode?.id || undefined,
        title: editTitle,
        content: editContent,
        node_type: editType,
        tags: tagsArray,
      });

      selectedNode = saved;
      isEditing = false;
      await loadGraph();
    } catch (e: any) {
      alert(`Save error: ${e.message}`);
    }
  }

  async function deleteCurrentNode() {
    if (!selectedNode?.id) return;
    if (!confirm(`Delete "${selectedNode.title}" and its edges?`)) return;

    try {
      await invokeIpc('delete_node', { id: selectedNode.id });
      selectedNode = null;
      await loadGraph();
    } catch (e: any) {
      alert(`Delete error: ${e.message}`);
    }
  }

  async function handleSearch() {
    try {
      const results: any = await invokeIpc('search_knowledge', {
        query: searchQuery,
        semantic: isSemanticSearch,
        limit: 25,
      });
      if (results && results.length > 0) {
        selectNode(results[0].node.id);
      }
    } catch (e) {
      console.error('Search error:', e);
    }
  }

  async function computePath() {
    if (!selectedNode?.id || !pathTargetId) return;
    try {
      const path: any = await invokeIpc('find_path', {
        source_id: selectedNode.id,
        target_id: pathTargetId,
      });
      foundPath = path || [];
    } catch (e) {
      console.error('Path error:', e);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'l') {
      e.preventDefault();
      isLicenseModalOpen = !isLicenseModalOpen;
    }
  }

  onMount(() => {
    loadGraph();
    loadLicenseStatus();
    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });
</script>

<div class="h-screen w-screen flex flex-col bg-slate-950 text-slate-100 font-sans select-none">
  <!-- Top Application Bar -->
  <header class="h-14 border-b border-slate-800 bg-slate-900/90 px-4 flex items-center justify-between backdrop-blur-md z-10">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-indigo-600 flex items-center justify-center text-lg shadow-md">
        🧠
      </div>
      <div>
        <h1 class="text-sm font-semibold tracking-wide text-slate-100 flex items-center gap-2">
          Research Studio
          <span class="text-[10px] uppercase font-bold px-1.5 py-0.5 rounded bg-indigo-950 text-indigo-400 border border-indigo-800/60">
            Offline Graph
          </span>
        </h1>
      </div>
    </div>

    <!-- Search Bar -->
    <div class="flex items-center gap-2 max-w-md w-full mx-4">
      <div class="relative w-full">
        <input
          type="text"
          placeholder="Search notes, citations, concepts... (Enter)"
          bind:value={searchQuery}
          on:keydown={(e) => e.key === 'Enter' && handleSearch()}
          class="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-indigo-500"
        />
      </div>
      <button
        class="text-xs px-2.5 py-1.5 rounded border {isSemanticSearch ? 'bg-indigo-600 border-indigo-500 text-white font-medium' : 'bg-slate-800 border-slate-700 text-slate-400'}"
        on:click={() => { isSemanticSearch = !isSemanticSearch; handleSearch(); }}
        title="Toggle Concept-Aware Dense Semantic Vector Search"
      >
        Vector
      </button>
    </div>

    <!-- Right Controls -->
    <div class="flex items-center gap-3">
      <div class="text-xs text-slate-400 flex items-center gap-2 pr-2 border-r border-slate-800">
        <span>Nodes: <strong class="text-slate-200">{stats.total_nodes}</strong></span>
        <span>Edges: <strong class="text-slate-200">{stats.total_edges}</strong></span>
      </div>

      <button
        class="text-xs px-2.5 py-1 rounded font-bold border border-emerald-500/30 bg-emerald-500/10 text-emerald-400 hover:bg-emerald-500/20"
        on:click={() => (isLicenseModalOpen = true)}
        title="Click to manage offline licensing (Ctrl+L)"
      >
        [{currentTier}]
      </button>

      <button
        class="bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium px-3 py-1.5 rounded-lg flex items-center gap-1.5 shadow"
        on:click={createNewNote}
      >
        <span>+</span> New Note
      </button>
    </div>
  </header>

  <!-- Main Workspace Layout -->
  <main class="flex-1 flex overflow-hidden">
    <!-- Left Pane: Interactive Graph Visualizer -->
    <div class="flex-1 h-full relative border-r border-slate-800">
      <GraphCanvas
        {nodes}
        {edges}
        selectedNodeId={selectedNode ? selectedNode.id : null}
        on:select={handleNodeSelect}
      />
    </div>

    <!-- Right Pane: Note Inspector / Editor -->
    <aside class="w-[480px] h-full bg-slate-900 flex flex-col border-l border-slate-800/80">
      {#if selectedNode}
        <div class="p-4 border-b border-slate-800 flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="text-xs font-semibold px-2 py-0.5 rounded capitalize bg-slate-800 text-slate-300">
              {selectedNode.node_type}
            </span>
            <span class="text-xs text-slate-500">ID: {selectedNode.id ? selectedNode.id.substring(0, 8) : 'New'}</span>
          </div>

          <div class="flex items-center gap-2">
            {#if !isEditing}
              <button
                class="text-xs bg-slate-800 hover:bg-slate-700 text-slate-200 px-2.5 py-1 rounded"
                on:click={() => (isEditing = true)}
              >
                Edit
              </button>
            {:else}
              <button
                class="text-xs bg-emerald-600 hover:bg-emerald-500 text-white px-3 py-1 rounded font-medium shadow"
                on:click={saveCurrentNote}
              >
                Save
              </button>
            {/if}
            {#if selectedNode.id}
              <button
                class="text-xs bg-rose-950 hover:bg-rose-900 text-rose-300 px-2.5 py-1 rounded border border-rose-800/40"
                on:click={deleteCurrentNode}
              >
                Delete
              </button>
            {/if}
          </div>
        </div>

        <div class="flex-1 overflow-y-auto p-4 flex flex-col gap-3">
          {#if isEditing}
            <!-- Editing Controls -->
            <div class="flex flex-col gap-1">
              <label for="note-title-input" class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">Title</label>
              <input
                id="note-title-input"
                type="text"
                bind:value={editTitle}
                class="bg-slate-950 border border-slate-700 rounded px-2.5 py-1.5 text-sm text-slate-100 focus:outline-none focus:border-indigo-500"
              />
            </div>

            <div class="flex gap-2">
              <div class="flex-1 flex flex-col gap-1">
                <label for="note-type-select" class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">Type</label>
                <select
                  id="note-type-select"
                  bind:value={editType}
                  class="bg-slate-950 border border-slate-700 rounded px-2 py-1.5 text-xs text-slate-200 focus:outline-none"
                >
                  <option value="note">Note</option>
                  <option value="document">Document</option>
                  <option value="concept">Concept</option>
                  <option value="citation">Citation</option>
                  <option value="excerpt">Excerpt</option>
                </select>
              </div>

              <div class="flex-[2] flex flex-col gap-1">
                <label for="note-tags-input" class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">Tags (comma-separated)</label>
                <input
                  id="note-tags-input"
                  type="text"
                  bind:value={editTags}
                  placeholder="e.g. ai, math, crypto"
                  class="bg-slate-950 border border-slate-700 rounded px-2.5 py-1.5 text-xs text-slate-100 focus:outline-none"
                />
              </div>
            </div>

            <div class="flex-1 flex flex-col gap-1 mt-1">
              <label for="note-content-textarea" class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
                Content (supports [[Wiki-Links]] and [@citations])
              </label>
              <textarea
                id="note-content-textarea"
                bind:value={editContent}
                rows="14"
                class="w-full flex-1 bg-slate-950 border border-slate-700 rounded p-2.5 text-xs font-mono text-slate-200 focus:outline-none focus:border-indigo-500"
              ></textarea>
            </div>
          {:else}
            <!-- Read-only View -->
            <div>
              <h2 class="text-xl font-bold text-slate-100">{selectedNode.title}</h2>
              <div class="flex flex-wrap gap-1.5 mt-2">
                {#each selectedNode.tags || [] as tag}
                  <span class="text-[10px] bg-indigo-950 text-indigo-300 border border-indigo-800/40 px-2 py-0.5 rounded-full">
                    #{tag}
                  </span>
                {/each}
              </div>
            </div>

            <div class="mt-2 text-xs leading-relaxed text-slate-300 font-mono whitespace-pre-wrap bg-slate-950 p-3 rounded-lg border border-slate-800/60 min-h-[160px]">
              {selectedNode.content || '(No content)'}
            </div>

            <!-- Shortest Path Finder -->
            <div class="mt-4 p-3 bg-slate-950 rounded-lg border border-slate-800/80 flex flex-col gap-2">
              <span class="text-xs font-semibold text-slate-300 flex items-center gap-1.5">
                <span>🧭</span> Shortest Citation Path Finder
              </span>
              <div class="flex gap-2">
                <select
                  bind:value={pathTargetId}
                  class="flex-1 bg-slate-900 border border-slate-700 rounded px-2 py-1 text-xs text-slate-200 focus:outline-none"
                >
                  <option value="">Select target concept...</option>
                  {#each nodes as n}
                    {#if n.id !== selectedNode.id}
                      <option value={n.id}>{n.title} ({n.node_type})</option>
                    {/if}
                  {/each}
                </select>
                <button
                  class="bg-indigo-600 hover:bg-indigo-500 text-white text-xs px-3 py-1 rounded"
                  on:click={computePath}
                  disabled={!pathTargetId}
                >
                  Trace
                </button>
              </div>

              {#if foundPath.length > 0}
                <div class="mt-1 flex flex-wrap items-center gap-1 text-xs text-slate-400">
                  {#each foundPath as p, i}
                    <span class="font-medium text-slate-200">{p.title}</span>
                    {#if i < foundPath.length - 1}
                      <span class="text-indigo-400 font-bold">→</span>
                    {/if}
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {:else}
        <!-- No Node Selected Placeholder -->
        <div class="h-full flex flex-col items-center justify-center p-8 text-center text-slate-500">
          <div class="text-4xl mb-3 opacity-60">🕸️</div>
          <h3 class="text-sm font-semibold text-slate-300">Select or Create a Node</h3>
          <p class="text-xs text-slate-500 mt-1 max-w-xs">
            Click any concept in the interactive graph canvas to inspect references, or create a new research note.
          </p>
          <button
            class="mt-4 bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium px-4 py-2 rounded-lg shadow"
            on:click={createNewNote}
          >
            + Create Research Note
          </button>
        </div>
      {/if}
    </aside>
  </main>

  <!-- Offline Licensing Modal -->
  <LicenseModal
    isOpen={isLicenseModalOpen}
    appName="Personal Knowledge Graph"
    on:close={() => (isLicenseModalOpen = false)}
    on:statusChange={(e) => (currentTier = e.detail.tier.toUpperCase())}
  />
</div>

