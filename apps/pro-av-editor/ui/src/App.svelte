<script lang="ts">
  import { onMount } from 'svelte';
  import Header from './components/Header.svelte';
  import ProgramMonitor from './components/ProgramMonitor.svelte';
  import MultiTrackTimeline from './components/MultiTrackTimeline.svelte';
  import ColorGradingPanel from './components/ColorGradingPanel.svelte';
  import AudioMixerPanel from './components/AudioMixerPanel.svelte';
  import AiToolsModal from './components/AiToolsModal.svelte';
  import {
    tauriInvoke,
    type Clip,
    type TimelineProject,
  } from './types';

  let project: TimelineProject | null = null;
  let activePanel: 'edit' | 'color' | 'audio' = 'edit';
  let playheadMs: number = 0;
  let durationMs: number = 6000;
  let selectedClipId: string | null = null;
  let showAiModal: boolean = false;
  let canUndo: boolean = true;
  let canRedo: boolean = true;

  let monitorRef: ProgramMonitor;

  onMount(async () => {
    await refreshProject();
  });

  async function refreshProject() {
    try {
      project = await tauriInvoke<TimelineProject>('get_project_state_cmd');
      if (project) {
        let maxOut = 5000;
        for (const tr of project.tracks) {
          for (const cl of tr.clips) {
            if (cl.timeline_out_ms > maxOut) {
              maxOut = cl.timeline_out_ms;
            }
          }
        }
        durationMs = Math.max(maxOut + 1000, 6000);
      }
      if (monitorRef && monitorRef.updateFrame) {
        await monitorRef.updateFrame();
      }
    } catch (e) {
      console.error('Failed to load project state:', e);
    }
  }

  async function handleNewProject() {
    try {
      project = await tauriInvoke<TimelineProject>('create_project_cmd', {
        name: `Studio Project ${Date.now()}`,
        width: 1920,
        height: 1080,
      });
      playheadMs = 0;
      canUndo = false;
      canRedo = false;
      await refreshProject();
    } catch (e) {
      console.error('Failed to create new project:', e);
    }
  }

  async function handleSaveProject() {
    try {
      await tauriInvoke<string>('save_project_cmd');
      alert('Project saved successfully to disk.');
    } catch (e) {
      console.error('Failed to save project:', e);
    }
  }

  async function handleUndo() {
    try {
      const res = await tauriInvoke<TimelineProject | null>('undo_cmd');
      if (res) {
        project = res;
      }
      await refreshProject();
    } catch (e) {
      console.error('Undo failed:', e);
    }
  }

  async function handleRedo() {
    try {
      const res = await tauriInvoke<TimelineProject | null>('redo_cmd');
      if (res) {
        project = res;
      }
      await refreshProject();
    } catch (e) {
      console.error('Redo failed:', e);
    }
  }

  function handleSeek(e: CustomEvent<number>) {
    playheadMs = e.detail;
    if (monitorRef && monitorRef.seekTo) {
      monitorRef.seekTo(playheadMs);
    }
  }

  function handleSelectClip(e: CustomEvent<Clip>) {
    selectedClipId = e.detail.id;
  }
</script>

<div class="editor-shell">
  <Header
    {project}
    {activePanel}
    {canUndo}
    {canRedo}
    on:newProject={handleNewProject}
    on:saveProject={handleSaveProject}
    on:undo={handleUndo}
    on:redo={handleRedo}
    on:openAiTools={() => (showAiModal = true)}
    on:switchPanel={(e) => (activePanel = e.detail)}
  />

  <main class="workspace-area">
    <!-- Top Stage (Monitor + Specialized Panel) -->
    <div class="top-stage">
      <div class="monitor-wrapper" class:full-width={activePanel === 'edit'}>
        <ProgramMonitor
          bind:this={monitorRef}
          {playheadMs}
          {durationMs}
          fps={60.0}
        />
      </div>

      {#if activePanel === 'color'}
        <div class="side-panel-wrapper">
          <ColorGradingPanel
            {playheadMs}
            {selectedClipId}
            on:gradeUpdated={refreshProject}
          />
        </div>
      {:else if activePanel === 'audio'}
        <div class="side-panel-wrapper">
          <AudioMixerPanel {project} />
        </div>
      {/if}
    </div>

    <!-- Bottom Stage (Multi-Track Timeline) -->
    <div class="bottom-stage">
      <MultiTrackTimeline
        {project}
        {playheadMs}
        {durationMs}
        {selectedClipId}
        on:seek={handleSeek}
        on:selectClip={handleSelectClip}
        on:refreshProject={refreshProject}
      />
    </div>
  </main>

  <!-- AI Tools & Interchange Modal -->
  <AiToolsModal
    show={showAiModal}
    {project}
    on:close={() => (showAiModal = false)}
    on:refreshProject={refreshProject}
  />
</div>

<style>
  .editor-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background: #090c10;
    color: #e6edf3;
    overflow: hidden;
  }

  .workspace-area {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .top-stage {
    display: flex;
    height: 52%;
    border-bottom: 2px solid #1f2937;
    background: #0f131a;
    overflow: hidden;
  }

  .monitor-wrapper {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 480px;
    height: 100%;
    overflow: hidden;
    border-right: 1px solid #1f2937;
  }

  .monitor-wrapper.full-width {
    border-right: none;
  }

  .side-panel-wrapper {
    flex: 1;
    height: 100%;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .bottom-stage {
    flex: 1;
    height: 48%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
</style>
