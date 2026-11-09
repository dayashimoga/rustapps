<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invokeIpc } from '../lib/ipc';
  import type { DocumentTemplate, PublishingProject, TargetFormat } from '../lib/types';

  export let show = false;
  const dispatch = createEventDispatcher();

  let title = '';
  let authors = '';
  let abstractText = '';
  let template: DocumentTemplate = 'ieee_two_column';
  let targetFormat: TargetFormat = 'print_pdf';
  let creating = false;
  let errorMsg = '';

  function close() {
    show = false;
    title = '';
    authors = '';
    abstractText = '';
    errorMsg = '';
    dispatch('close');
  }

  async function create() {
    if (!title.trim() || !authors.trim()) return;
    creating = true;
    errorMsg = '';

    const starterMd = `# Introduction\nThis manuscript investigates novel algorithms and empirical validations.\n\n## Methodology\nAll experiments were conducted offline in air-gapped environments.\n\n## Conclusion\nResults demonstrate superior convergence and zero data egress.\n`;

    try {
      const res = await invokeIpc<PublishingProject>('save_project', {
        title: title.trim(),
        authors: authors.trim(),
        abstract_text: abstractText.trim(),
        template,
        target_format: targetFormat,
        source_markdown: starterMd,
        bibtex_raw: '',
      });
      dispatch('created', res);
      close();
    } catch (err: any) {
      errorMsg = err.message || 'Project creation failed';
    } finally {
      creating = false;
    }
  }
</script>

{#if show}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4" on:click={close}>
    <div class="w-full max-w-xl bg-slate-900 border border-slate-700 rounded-xl shadow-2xl p-6" on:click|stopPropagation>
      <div class="flex items-center justify-between pb-4 border-b border-slate-800">
        <div class="flex items-center gap-2">
          <span class="text-amber-400 font-mono text-base">📄</span>
          <h2 class="text-base font-semibold text-slate-100">Create New Manuscript Project</h2>
        </div>
        <button class="text-slate-400 hover:text-slate-200" on:click={close}>✕</button>
      </div>

      <div class="py-4 space-y-4">
        <div>
          <label for="proj-title" class="block text-xs font-mono text-slate-400 uppercase mb-1">Manuscript Title</label>
          <input
            id="proj-title"
            type="text"
            bind:value={title}
            placeholder="e.g. Provably Secure Zero-Knowledge Auditing in SCADA"
            class="w-full px-3 py-2 bg-slate-950 border border-slate-700 rounded-lg text-slate-100 text-sm focus:border-amber-500 focus:outline-none"
          />
        </div>

        <div>
          <label for="proj-authors" class="block text-xs font-mono text-slate-400 uppercase mb-1">Authors & Affiliations</label>
          <input
            id="proj-authors"
            type="text"
            bind:value={authors}
            placeholder="e.g. Dr. Jane Doe, IEEE Member • Stanford Security Lab"
            class="w-full px-3 py-2 bg-slate-950 border border-slate-700 rounded-lg text-slate-100 text-sm focus:border-amber-500 focus:outline-none"
          />
        </div>

        <div>
          <label for="proj-abstract" class="block text-xs font-mono text-slate-400 uppercase mb-1">Abstract</label>
          <textarea
            id="proj-abstract"
            rows="3"
            bind:value={abstractText}
            placeholder="Executive summary of the research methodology, findings, and contributions..."
            class="w-full px-3 py-2 bg-slate-950 border border-slate-700 rounded-lg text-slate-100 text-xs focus:border-amber-500 focus:outline-none"
          ></textarea>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div>
            <label for="template-sel" class="block text-xs font-mono text-slate-400 uppercase mb-1">Layout Template</label>
            <select
              id="template-sel"
              bind:value={template}
              class="w-full px-3 py-2 bg-slate-950 border border-slate-700 rounded-lg text-slate-100 text-sm focus:border-amber-500 focus:outline-none"
            >
              <option value="ieee_two_column">IEEE Two-Column Transactions</option>
              <option value="acm_sigconf">ACM SIGCONF Proceedings</option>
              <option value="springer_lncs">Springer LNCS Monograph</option>
              <option value="arxiv_preprint">arXiv.org Research Preprint</option>
              <option value="enterprise_sop">ISO 9001 Enterprise SOP</option>
              <option value="book_chapter">Academic Book Chapter</option>
            </select>
          </div>
          <div>
            <label for="format-sel" class="block text-xs font-mono text-slate-400 uppercase mb-1">Distribution Format</label>
            <select
              id="format-sel"
              bind:value={targetFormat}
              class="w-full px-3 py-2 bg-slate-950 border border-slate-700 rounded-lg text-slate-100 text-sm focus:border-amber-500 focus:outline-none"
            >
              <option value="print_pdf">Vector Print-Ready PDF</option>
              <option value="standalone_html5">Standalone Offline HTML5</option>
              <option value="epub3">EPUB3 eBook</option>
              <option value="typst_source">Native Typst Source</option>
            </select>
          </div>
        </div>

        {#if errorMsg}
          <div class="p-3 bg-red-950/40 border border-red-800/60 rounded-lg text-xs text-red-300">{errorMsg}</div>
        {/if}
      </div>

      <div class="flex justify-end gap-3 pt-3 border-t border-slate-800">
        <button class="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-lg text-xs font-medium" on:click={close}>
          Cancel
        </button>
        <button
          class="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-slate-950 font-semibold rounded-lg text-xs transition-colors disabled:opacity-50"
          on:click={create}
          disabled={creating || !title.trim() || !authors.trim()}
        >
          {creating ? 'Initializing...' : 'Create Project'}
        </button>
      </div>
    </div>
  </div>
{/if}
