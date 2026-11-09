<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { PublishingProject } from '../lib/types';

  export let project: PublishingProject | null = null;
  const dispatch = createEventDispatcher();

  let activeTab: 'markdown' | 'bibtex' = 'markdown';

  $: wordCount = project?.source_markdown.split(/\s+/).filter(Boolean).length || 0;
  $: charCount = project?.source_markdown.length || 0;
  $: lineCount = project?.source_markdown.split('\n').length || 0;
  $: bibCount = project?.bibliography.length || 0;

  function updateMarkdown(e: Event) {
    const val = (e.target as HTMLTextAreaElement).value;
    if (project) {
      project.source_markdown = val;
      dispatch('change', project);
    }
  }

  function updateBibtex(e: Event) {
    const val = (e.target as HTMLTextAreaElement).value;
    if (project) {
      project.bibtex_raw = val;
      dispatch('change', project);
    }
  }
</script>

<div class="flex flex-col h-full bg-slate-900 border-r border-slate-800">
  <!-- Tabs & Stats Header -->
  <div class="p-3 border-b border-slate-800 flex items-center justify-between bg-slate-950/40">
    <div class="flex gap-2">
      <button
        class="px-3 py-1 rounded text-xs font-mono font-medium transition-colors {activeTab === 'markdown' ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40' : 'text-slate-400 hover:text-slate-200'}"
        on:click={() => activeTab = 'markdown'}
      >
        Markdown Source
      </button>
      <button
        class="px-3 py-1 rounded text-xs font-mono font-medium transition-colors {activeTab === 'bibtex' ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40' : 'text-slate-400 hover:text-slate-200'}"
        on:click={() => activeTab = 'bibtex'}
      >
        BibTeX ({bibCount})
      </button>
    </div>

    <div class="flex items-center gap-3 text-[11px] font-mono text-slate-400">
      {#if activeTab === 'markdown'}
        <span>{lineCount} lines</span>
        <span>•</span>
        <span>{wordCount} words</span>
        <span>•</span>
        <span>{charCount} chars</span>
      {:else}
        <span>{bibCount} indexed citations</span>
      {/if}
    </div>
  </div>

  <!-- Editor Textarea Body -->
  <div class="flex-1 flex overflow-hidden p-3 bg-slate-950">
    {#if project}
      {#if activeTab === 'markdown'}
        <textarea
          class="w-full h-full bg-transparent text-slate-100 font-mono text-xs leading-relaxed resize-none focus:outline-none p-2"
          value={project.source_markdown}
          on:input={updateMarkdown}
          placeholder="# Type your academic or technical manuscript here..."
          spellcheck="false"
        ></textarea>
      {:else}
        <textarea
          class="w-full h-full bg-transparent text-amber-200/90 font-mono text-xs leading-relaxed resize-none focus:outline-none p-2"
          value={project.bibtex_raw}
          on:input={updateBibtex}
          placeholder="Enter BibTeX bibliography records here..."
          spellcheck="false"
        ></textarea>
      {/if}
    {:else}
      <div class="flex-1 flex items-center justify-center text-slate-500 text-xs font-mono">
        No project loaded.
      </div>
    {/if}
  </div>
</div>
