<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { CompilationArtifact, DocumentTemplate, PublishingProject, TargetFormat } from '../lib/types';

  export let project: PublishingProject | null = null;
  export let artifact: CompilationArtifact | null = null;
  export let compiling = false;

  const dispatch = createEventDispatcher();

  function triggerCompile() {
    dispatch('compile');
  }

  function handleTemplateChange(e: Event) {
    const val = (e.target as HTMLSelectElement).value as DocumentTemplate;
    dispatch('changeTemplate', val);
  }

  function handleFormatChange(e: Event) {
    const val = (e.target as HTMLSelectElement).value as TargetFormat;
    dispatch('changeFormat', val);
  }
</script>

<div class="flex flex-col h-full bg-slate-950 overflow-hidden">
  <!-- Controls Bar -->
  <div class="p-3 border-b border-slate-800 flex items-center justify-between bg-slate-900/60 font-mono text-xs shrink-0">
    <div class="flex items-center gap-3">
      <span class="text-slate-400 uppercase text-[10px] font-bold">Template:</span>
      <select
        value={project?.template || 'ieee_two_column'}
        on:change={handleTemplateChange}
        class="px-2.5 py-1 bg-slate-950 border border-slate-700 rounded text-slate-200 text-xs focus:border-amber-500 focus:outline-none"
      >
        <option value="ieee_two_column">IEEE Two-Column</option>
        <option value="acm_sigconf">ACM SIGCONF</option>
        <option value="springer_lncs">Springer LNCS</option>
        <option value="arxiv_preprint">arXiv Preprint</option>
        <option value="enterprise_sop">Enterprise SOP</option>
        <option value="book_chapter">Book Chapter</option>
      </select>

      <span class="text-slate-400 uppercase text-[10px] font-bold">Format:</span>
      <select
        value={project?.target_format || 'print_pdf'}
        on:change={handleFormatChange}
        class="px-2.5 py-1 bg-slate-950 border border-slate-700 rounded text-slate-200 text-xs focus:border-amber-500 focus:outline-none"
      >
        <option value="print_pdf">Vector PDF</option>
        <option value="standalone_html5">HTML5 Bundle</option>
        <option value="epub3">EPUB3 eBook</option>
        <option value="typst_source">Typst Source</option>
      </select>
    </div>

    <div class="flex items-center gap-3">
      {#if artifact}
        <span class="text-[11px] px-2 py-0.5 rounded bg-slate-800 text-slate-300">
          {artifact.page_count} {artifact.page_count === 1 ? 'Page' : 'Pages'} • {artifact.compile_duration_ms}ms
        </span>
      {/if}
      <button
        class="px-3 py-1 bg-amber-600 hover:bg-amber-500 text-slate-950 font-bold rounded text-xs transition-colors disabled:opacity-50"
        on:click={triggerCompile}
        disabled={compiling}
      >
        {compiling ? 'Compiling...' : '⚡ Typeset'}
      </button>
    </div>
  </div>

  <!-- Unresolved References Warning -->
  {#if artifact && artifact.unresolved_references.length > 0}
    <div class="p-2.5 bg-amber-950/40 border-b border-amber-800/60 text-amber-300 text-xs font-mono flex items-center gap-2 shrink-0">
      <span>⚠️</span>
      <span>{artifact.unresolved_references.length} Unresolved Reference Warning(s): {artifact.unresolved_references.join(', ')}</span>
    </div>
  {/if}

  <!-- Live Rendered Document Area -->
  <div class="flex-1 overflow-y-auto p-6 flex justify-center bg-slate-950">
    {#if artifact}
      <div class="w-full max-w-4xl h-full paper-sheet overflow-hidden border border-slate-300">
        <iframe
          title="Manuscript Live Preview"
          srcdoc={artifact.rendered_document}
          class="w-full h-full border-none"
        ></iframe>
      </div>
    {:else}
      <div class="flex flex-col items-center justify-center text-center text-slate-500 font-mono text-xs">
        <span class="text-4xl mb-3 opacity-40">📰</span>
        <div>Click "Typeset" to run offline multi-template compilation.</div>
      </div>
    {/if}
  </div>

  <!-- Footer Info -->
  {#if artifact}
    <div class="p-2 border-t border-slate-800/80 bg-slate-950 flex items-center justify-between text-[10px] font-mono text-slate-500 shrink-0 px-4">
      <div class="truncate max-w-md">BLAKE3: <span class="text-slate-400">{artifact.document_blake3_hash}</span></div>
      <div>100% Offline Typesetting • Zero Network Egress</div>
    </div>
  {/if}
</div>
