<script lang="ts">
  import { onMount } from 'svelte';
  import Header from './components/Header.svelte';
  import ProgramMonitor from './components/ProgramMonitor.svelte';
  import MultiTrackTimeline from './components/MultiTrackTimeline.svelte';
  import ColorGradingPanel from './components/ColorGradingPanel.svelte';
  import AudioMixerPanel from './components/AudioMixerPanel.svelte';
  import MediaPoolPanel from './components/MediaPoolPanel.svelte';
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
  let activeAiModalTab: 'silence' | 'beats' | 'scenecut' | 'captions' | 'export' = 'silence';
  let canUndo: boolean = true;
  let canRedo: boolean = true;
  let statusMessage: string = '';

  let monitorRef: ProgramMonitor;

  function showToast(msg: string) {
    statusMessage = msg;
    setTimeout(() => {
      if (statusMessage === msg) {
        statusMessage = '';
      }
    }, 3500);
  }

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
        name: `Studio Project ${new Date().toLocaleTimeString()}`,
        width: 1920,
        height: 1080,
      });
      playheadMs = 0;
      selectedClipId = null;
      canUndo = false;
      canRedo = false;
      await refreshProject();
      showToast('Created new empty Studio Project.');
    } catch (e) {
      console.error('Failed to create new project:', e);
    }
  }

  async function handleOpenProject() {
    try {
      let chosenPath: string | null = null;
      if (typeof window !== 'undefined' && window.__TAURI__?.dialog?.open) {
        chosenPath = await window.__TAURI__.dialog.open({
          title: 'Open Pro AV Studio Project (.avedproj)',
          filter: '*.avedproj;*.json',
        });
      }
      if (!chosenPath) return;
      project = await tauriInvoke<TimelineProject>('load_project_cmd', {
        path: chosenPath,
      });
      playheadMs = 0;
      selectedClipId = null;
      await refreshProject();
      showToast(`Loaded project: ${chosenPath}`);
    } catch (e) {
      console.error('Failed to open project:', e);
      showToast(`Failed to open project: ${e}`);
    }
  }

  async function handleImportMedia() {
    try {
      let chosenPath: string | null = null;
      if (typeof window !== 'undefined' && window.__TAURI__?.dialog?.open) {
        chosenPath = await window.__TAURI__.dialog.open({
          title: 'Import Video, Audio, or Image Media',
          filter: '*.mp4;*.webm;*.mov;*.mkv;*.avi;*.wav;*.mp3;*.flac;*.ogg;*.png;*.jpg;*.jpeg;*.bmp;*.webp',
        });
      }
      if (!chosenPath) return;
      project = await tauriInvoke<TimelineProject>('import_media_file_cmd', {
        filePath: chosenPath,
        autoAddToTimeline: true,
      });
      await refreshProject();
      showToast(`Imported media: ${chosenPath.split(/[\\/]/).pop()}`);
    } catch (e) {
      console.error('Failed to import media:', e);
      showToast(`Import error: ${e}`);
    }
  }

  async function handleSaveProject() {
    try {
      let targetPath: string | undefined = undefined;
      if (typeof window !== 'undefined' && window.__TAURI__?.dialog?.save) {
        const picked = await window.__TAURI__.dialog.save({
          title: 'Save Pro AV Studio Project (.avedproj)',
          defaultName: `${(project?.name || 'studio_project').replace(/[^a-zA-Z0-9_-]/g, '_')}.avedproj`,
          filter: '*.avedproj',
        });
        if (picked) {
          targetPath = picked;
        }
      }
      const savedPath = await tauriInvoke<string>('save_project_cmd', {
        path: targetPath,
      });
      showToast(`Project saved to ${savedPath}`);
    } catch (e) {
      console.error('Failed to save project:', e);
      showToast(`Save failed: ${e}`);
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
    on:openProject={handleOpenProject}
    on:importMedia={handleImportMedia}
    on:saveProject={handleSaveProject}
    on:undo={handleUndo}
    on:redo={handleRedo}
    on:openAiTools={() => { activeAiModalTab = 'silence'; showAiModal = true; }}
    on:openExport={() => { activeAiModalTab = 'export'; showAiModal = true; }}
    on:switchPanel={(e) => (activePanel = e.detail)}
  />

  {#if statusMessage}
    <div class="toast-banner" role="status">{statusMessage}</div>
  {/if}

  <main class="workspace-area">
    <!-- Top Stage (Monitor + Specialized Panel) -->
    <div class="top-stage">
      <div class="monitor-wrapper">
        <ProgramMonitor
          bind:this={monitorRef}
          {project}
          {playheadMs}
          {durationMs}
          fps={60.0}
          on:timeupdate={(e) => (playheadMs = e.detail)}
        />
      </div>

      <div class="side-panel-wrapper">
        {#if activePanel === 'edit'}
          <MediaPoolPanel
            {project}
            {selectedClipId}
            {playheadMs}
            on:importMedia={handleImportMedia}
            on:refreshProject={refreshProject}
          />
        {:else if activePanel === 'color'}
          <ColorGradingPanel
            {playheadMs}
            {selectedClipId}
            on:gradeUpdated={refreshProject}
          />
        {:else if activePanel === 'audio'}
          <AudioMixerPanel {project} />
        {/if}
      </div>
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
    activeTab={activeAiModalTab}
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
    position: relative;
  }

  .toast-banner {
    position: fixed;
    bottom: 18px;
    right: 22px;
    z-index: 200;
    background: rgba(17, 24, 39, 0.95);
    border: 1px solid #3b82f6;
    color: #e2e8f0;
    padding: 8px 14px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
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
