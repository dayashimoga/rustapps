<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import type {
    CompilationArtifact,
    CopyrightCertificate,
    DocumentTemplate,
    LicenseStatus,
    PressStats,
    ProjectSummary,
    PublishingProject,
    TargetFormat,
  } from './lib/types';

  import CommandPalette from './components/CommandPalette.svelte';
  import LicenseModal from './components/LicenseModal.svelte';
  import NewProjectModal from './components/NewProjectModal.svelte';
  import CertificateModal from './components/CertificateModal.svelte';
  import EditorPane from './components/EditorPane.svelte';
  import PreviewPane from './components/PreviewPane.svelte';

  let stats: PressStats = {
    total_projects: 0,
    total_compilations: 0,
    total_pages_rendered: 0,
    total_citations_indexed: 0,
  };

  let projectSummaries: ProjectSummary[] = [];
  let currentProjectId = '';
  let activeProject: PublishingProject | null = null;
  let activeArtifact: CompilationArtifact | null = null;
  let licenseStatus: LicenseStatus | null = null;
  let copyrightCert: CopyrightCertificate | null = null;

  let compiling = false;
  let showCommandPalette = false;
  let showLicenseModal = false;
  let showNewProjectModal = false;
  let showCertModal = false;

  onMount(async () => {
    await reloadData();
  });

  async function reloadData() {
    try {
      stats = await invokeIpc<PressStats>('get_stats');
      projectSummaries = await invokeIpc<ProjectSummary[]>('list_projects');
      licenseStatus = await invokeIpc<LicenseStatus>('get_license_status');

      if (projectSummaries.length > 0) {
        if (!currentProjectId || !projectSummaries.find(p => p.project_id === currentProjectId)) {
          currentProjectId = projectSummaries[0].project_id;
        }
        await loadProject(currentProjectId);
      }
    } catch (e) {
      console.error('Failed to load press data', e);
    }
  }

  async function loadProject(projectId: string) {
    try {
      currentProjectId = projectId;
      activeProject = await invokeIpc<PublishingProject>('get_project', { project_id: projectId });
      activeArtifact = await invokeIpc<CompilationArtifact>('get_artifact', { project_id: projectId });
    } catch (e) {
      console.error('Failed to load project', e);
    }
  }

  async function handleCompile() {
    if (!activeProject) return;
    compiling = true;
    try {
      // First save current markdown/bibtex state
      await invokeIpc<PublishingProject>('save_project', {
        project_id: activeProject.project_id,
        title: activeProject.title,
        authors: activeProject.authors,
        abstract_text: activeProject.abstract_text,
        template: activeProject.template,
        target_format: activeProject.target_format,
        source_markdown: activeProject.source_markdown,
        bibtex_raw: activeProject.bibtex_raw,
      });

      // Then compile
      activeArtifact = await invokeIpc<CompilationArtifact>('compile_project', {
        project_id: activeProject.project_id,
        template: activeProject.template,
        format: activeProject.target_format,
      });

      stats = await invokeIpc<PressStats>('get_stats');
    } catch (e) {
      console.error('Compilation failed', e);
    } finally {
      compiling = false;
    }
  }

  async function handleTemplateChange(template: DocumentTemplate) {
    if (!activeProject) return;
    activeProject.template = template;
    await handleCompile();
  }

  async function handleFormatChange(format: TargetFormat) {
    if (!activeProject) return;
    activeProject.target_format = format;
    await handleCompile();
  }

  async function openCopyrightCertificate() {
    if (!activeProject) return;
    try {
      copyrightCert = await invokeIpc<CopyrightCertificate>('sign_copyright_certificate', {
        project_id: activeProject.project_id,
      });
      showCertModal = true;
    } catch (e) {
      console.error('Failed to sign copyright certificate', e);
    }
  }

  function handleCommandSelect(e: CustomEvent<{ id: string }>) {
    switch (e.detail.id) {
      case 'new_project':
        showNewProjectModal = true;
        break;
      case 'recompile':
        handleCompile();
        break;
      case 'sign_copyright':
        openCopyrightCertificate();
        break;
      case 'manage_license':
        showLicenseModal = true;
        break;
      case 'reload_projects':
        reloadData();
        break;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
      e.preventDefault();
      showCommandPalette = !showCommandPalette;
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="flex flex-col h-screen w-screen bg-slate-950 text-slate-100 font-sans overflow-hidden select-none">
  <!-- Top Navigation & Metrics Header -->
  <header class="px-5 py-3 bg-slate-900 border-b border-slate-800 flex items-center justify-between shrink-0">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-amber-500/10 border border-amber-500/30 flex items-center justify-center text-amber-400 font-bold text-base">
        📰
      </div>
      <div>
        <div class="flex items-center gap-2">
          <h1 class="text-sm font-bold text-slate-100 tracking-wide">
            Markdown Publishing Press
          </h1>
          <span class="px-2 py-0.5 rounded text-[10px] font-mono font-medium bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
            AIR-GAPPED 100%
          </span>
          <span class="px-2 py-0.5 rounded text-[10px] font-mono font-medium bg-amber-500/20 text-amber-300 border border-amber-500/30">
            TYPST / LATEX
          </span>
        </div>
        <p class="text-[11px] text-slate-400">
          Academic Multi-Template Typesetter (IEEE, ACM, Springer, SOP) & Copyright Issuer
        </p>
      </div>
    </div>

    <!-- Live Executive Metrics -->
    <div class="hidden lg:flex items-center gap-6 font-mono text-xs border-x border-slate-800 px-6">
      <div>
        <div class="text-[10px] text-slate-500 uppercase">Projects</div>
        <div class="text-slate-200 font-bold">{stats.total_projects}</div>
      </div>
      <div>
        <div class="text-[10px] text-slate-500 uppercase">Compilations</div>
        <div class="text-slate-200 font-bold">{stats.total_compilations}</div>
      </div>
      <div>
        <div class="text-[10px] text-slate-500 uppercase">Pages Rendered</div>
        <div class="text-amber-400 font-bold">{stats.total_pages_rendered}</div>
      </div>
      <div>
        <div class="text-[10px] text-slate-500 uppercase">Indexed Citations</div>
        <div class="text-blue-400 font-bold">{stats.total_citations_indexed}</div>
      </div>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-2">
      <button
        class="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg text-xs font-mono font-medium transition-colors border border-slate-700"
        on:click={() => showNewProjectModal = true}
      >
        + New Manuscript
      </button>

      <button
        class="px-3 py-1.5 bg-emerald-950/80 hover:bg-emerald-900 text-emerald-300 border border-emerald-700/60 rounded-lg text-xs font-mono font-medium transition-colors"
        on:click={openCopyrightCertificate}
      >
        Copyright Certificate
      </button>

      <button
        class="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-amber-300 rounded-lg text-xs font-mono font-medium transition-colors border border-slate-700"
        on:click={() => showLicenseModal = true}
      >
        {licenseStatus?.tier?.toUpperCase() || 'LICENSE'}
      </button>

      <button
        class="px-2.5 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-slate-200 rounded-lg text-xs font-mono border border-slate-700"
        on:click={() => showCommandPalette = true}
        title="Command Palette (Ctrl+K)"
      >
        ⌘K
      </button>
    </div>
  </header>

  <!-- Project Selector Sub-header -->
  <div class="px-5 py-2 bg-slate-900/60 border-b border-slate-800 flex items-center justify-between shrink-0 font-mono text-xs">
    <div class="flex items-center gap-3">
      <span class="text-slate-400 uppercase text-[10px] font-bold">Active Manuscript:</span>
      <select
        bind:value={currentProjectId}
        on:change={() => loadProject(currentProjectId)}
        class="px-3 py-1 bg-slate-950 border border-slate-700 rounded text-slate-200 font-semibold focus:border-amber-500 focus:outline-none"
      >
        {#each projectSummaries as p}
          <option value={p.project_id}>
            {p.title} ({p.template.toUpperCase().replace('_', ' ')} • {p.word_count} words • {p.citations_count} cites)
          </option>
        {/each}
      </select>
    </div>

    {#if activeProject}
      <div class="flex items-center gap-4 text-[11px] text-slate-400">
        <div>Authors: <span class="text-slate-200">{activeProject.authors}</span></div>
      </div>
    {/if}
  </div>

  <!-- Main Split Layout -->
  <main class="flex-1 flex overflow-hidden">
    <!-- Left Pane: Editor (50%) -->
    <section class="w-1/2 h-full">
      <EditorPane
        project={activeProject}
        on:change={(e) => activeProject = e.detail}
      />
    </section>

    <!-- Right Pane: Live Preview & Typesetter (50%) -->
    <section class="w-1/2 h-full">
      <PreviewPane
        project={activeProject}
        artifact={activeArtifact}
        {compiling}
        on:compile={handleCompile}
        on:changeTemplate={(e) => handleTemplateChange(e.detail)}
        on:changeFormat={(e) => handleFormatChange(e.detail)}
      />
    </section>
  </main>

  <!-- Modals -->
  <CommandPalette
    bind:show={showCommandPalette}
    on:select={handleCommandSelect}
  />

  <LicenseModal
    bind:show={showLicenseModal}
    status={licenseStatus}
    on:activated={reloadData}
  />

  <NewProjectModal
    bind:show={showNewProjectModal}
    on:created={async (e) => {
      await reloadData();
      await loadProject(e.detail.project_id);
    }}
  />

  <CertificateModal
    bind:show={showCertModal}
    certificate={copyrightCert}
  />
</div>
