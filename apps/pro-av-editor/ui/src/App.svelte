<script lang="ts">
  import { onMount } from 'svelte';
  import Header from './components/Header.svelte';
  import ProgramMonitor from './components/ProgramMonitor.svelte';
  import MultiTrackTimeline from './components/MultiTrackTimeline.svelte';
  import ColorGradingPanel from './components/ColorGradingPanel.svelte';
  import AudioMixerPanel from './components/AudioMixerPanel.svelte';
  import MediaPoolPanel from './components/MediaPoolPanel.svelte';
  import AiToolsModal from './components/AiToolsModal.svelte';
  import NewProjectModal from './components/NewProjectModal.svelte';
  import {
    tauriInvoke,
    type Clip,
    type TimelineProject,
  } from './types';
  import { openNativeDialog, saveNativeDialog, isDesktopApp } from './lib/ipc';

  let project: TimelineProject | null = null;
  let activePanel: 'edit' | 'color' | 'audio' = 'edit';
  let playheadMs: number = 0;
  let durationMs: number = 6000;
  let selectedClipId: string | null = null;
  let showAiModal: boolean = false;
  let showNewProjectModal: boolean = false;
  let activeAiModalTab: 'silence' | 'beats' | 'scenecut' | 'captions' | 'export' = 'silence';
  let canUndo: boolean = true;
  let canRedo: boolean = true;
  let statusMessage: string = '';

  let monitorRef: ProgramMonitor;
  let openProjectFileInput: HTMLInputElement;
  let importMediaFileInput: HTMLInputElement;

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

  function handleNewProject() {
    showNewProjectModal = true;
  }

  async function handleCreateProjectFromModal(
    e: CustomEvent<{
      name: string;
      templateId: string;
      width: number;
      height: number;
      fps: number;
      includeDemoClips: boolean;
    }>
  ) {
    const { name, templateId, width, height, fps, includeDemoClips } = e.detail;
    try {
      project = await tauriInvoke<TimelineProject>('create_project_cmd', {
        name,
        templateId,
        width,
        height,
        fps,
        includeDemoClips,
      });
      playheadMs = 0;
      selectedClipId = null;
      canUndo = false;
      canRedo = false;
      await refreshProject();
      showToast(`Created project: "${name}" (${width}×${height} @ ${fps}fps)`);
    } catch (err) {
      console.error('Failed to create new project:', err);
      showToast(`Failed to create project: ${err}`);
    }
  }

  async function handleOpenProject() {
    try {
      if (isDesktopApp()) {
        const chosen = await openNativeDialog({
          title: 'Open Pro AV Studio Project (.avedproj)',
          filter: '*.avedproj;*.json',
          filters: [{ name: 'Project Files', extensions: ['avedproj', 'json'] }],
        });
        if (chosen) {
          const chosenPath = Array.isArray(chosen) ? chosen[0] : chosen;
          project = await tauriInvoke<TimelineProject>('load_project_cmd', {
            path: chosenPath,
          });
          playheadMs = 0;
          selectedClipId = null;
          await refreshProject();
          showToast(`Loaded project: ${chosenPath}`);
        }
      } else if (openProjectFileInput) {
        openProjectFileInput.value = '';
        openProjectFileInput.click();
      }
    } catch (e) {
      console.error('Failed to open project:', e);
      showToast(`Failed to open project: ${e}`);
    }
  }

  async function handleProjectFileSelected(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input?.files?.[0];
    if (!file) return;
    try {
      const localPath = (file as any).path;
      if (localPath) {
        project = await tauriInvoke<TimelineProject>('load_project_cmd', { path: localPath });
        playheadMs = 0;
        selectedClipId = null;
        await refreshProject();
        showToast(`Loaded project: ${file.name}`);
        return;
      }
      const text = await file.text();
      const parsed = JSON.parse(text);
      if (parsed && parsed.project_id) {
        project = parsed;
        await refreshProject();
        showToast(`Loaded project: ${file.name}`);
      }
    } catch (err) {
      console.error('Failed to parse project file:', err);
      showToast(`Error opening project: ${err}`);
    }
  }

  async function handleImportMedia() {
    try {
      if (isDesktopApp()) {
        const chosen = await openNativeDialog({
          title: 'Import Video, Audio, or Image Media',
          filter: '*.mp4;*.webm;*.mov;*.mkv;*.avi;*.wav;*.mp3;*.flac;*.ogg;*.png;*.jpg;*.jpeg;*.bmp;*.webp',
          filters: [
            {
              name: 'Media Files',
              extensions: [
                'mp4', 'webm', 'mov', 'mkv', 'avi', 'wav', 'mp3',
                'flac', 'ogg', 'png', 'jpg', 'jpeg', 'bmp', 'webp',
              ],
            },
            { name: 'All Files', extensions: ['*'] },
          ],
        });
        if (chosen) {
          const chosenPath = Array.isArray(chosen) ? chosen[0] : chosen;
          project = await tauriInvoke<TimelineProject>('import_media_file_cmd', {
            filePath: chosenPath,
            autoAddToTimeline: true,
          });
          await refreshProject();
          showToast(`Imported media: ${chosenPath.split(/[\\/]/).pop()}`);
        }
      } else if (importMediaFileInput) {
        importMediaFileInput.value = '';
        importMediaFileInput.click();
      }
    } catch (e) {
      console.error('Failed to import media:', e);
      showToast(`Import error: ${e}`);
    }
  }

  async function handleMediaFilesSelected(e: Event) {
    const input = e.target as HTMLInputElement;
    const files = input?.files;
    if (!files || files.length === 0) return;
    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      const filePath = (file as any).path || file.name;
      try {
        project = await tauriInvoke<TimelineProject>('import_media_file_cmd', {
          filePath,
          autoAddToTimeline: true,
        });
        showToast(`Imported media: ${file.name}`);
      } catch (err) {
        console.warn(`Could not import ${file.name} by path, continuing:`, err);
        showToast(`Added file: ${file.name}`);
      }
    }
    await refreshProject();
  }

  async function handleSaveProject() {
    try {
      let targetPath: string | undefined = undefined;
      if (isDesktopApp()) {
        const defaultName = `${(project?.name || 'studio_project').replace(/[^a-zA-Z0-9_-]/g, '_')}.avedproj`;
        const picked = await saveNativeDialog({
          title: 'Save Pro AV Studio Project (.avedproj)',
          defaultPath: defaultName,
          filter: '*.avedproj',
          filters: [{ name: 'Studio Project File', extensions: ['avedproj'] }],
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

  $: projectFps = (() => {
    if (!project?.settings?.frame_rate) return 60.0;
    const fr = project.settings.frame_rate;
    if (fr === 'Fps23_976') return 23.976;
    if (fr === 'Fps24') return 24.0;
    if (fr === 'Fps25') return 25.0;
    if (fr === 'Fps29_97') return 29.97;
    if (fr === 'Fps30') return 30.0;
    if (fr === 'Fps50') return 50.0;
    if (fr === 'Fps59_94') return 59.94;
    if (fr === 'Fps60') return 60.0;
    if (fr === 'Fps120') return 120.0;
    return 60.0;
  })();

  async function handleGradeUpdated(e: CustomEvent<{ clipId: string; grading: any }>) {
    if (project && e.detail?.clipId && e.detail?.grading) {
      for (const track of project.tracks) {
        for (const clip of track.clips) {
          if (clip.id === e.detail.clipId) {
            clip.color_grading = e.detail.grading;
          }
        }
      }
    }
    if (monitorRef && monitorRef.updateFrame) {
      await monitorRef.updateFrame();
    }
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
    on:refreshProject={refreshProject}
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
          fps={projectFps}
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
            on:toast={(e) => showToast(e.detail)}
          />
        {:else if activePanel === 'color'}
          <ColorGradingPanel
            {project}
            {playheadMs}
            {selectedClipId}
            on:gradeUpdated={handleGradeUpdated}
            on:toast={(e) => showToast(e.detail)}
          />
        {:else if activePanel === 'audio'}
          <AudioMixerPanel
            {project}
            on:toast={(e) => showToast(e.detail)}
          />
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
        on:toast={(e) => showToast(e.detail)}
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

  <!-- New Project Modal -->
  <NewProjectModal
    show={showNewProjectModal}
    on:close={() => (showNewProjectModal = false)}
    on:create={handleCreateProjectFromModal}
  />

  <!-- Hidden Native File Inputs for 100% Reliable Fallback -->
  <input
    type="file"
    accept=".avedproj,.json"
    style="display: none;"
    bind:this={openProjectFileInput}
    on:change={handleProjectFileSelected}
  />
  <input
    type="file"
    multiple
    accept="video/*,audio/*,image/*,.mp4,.webm,.mov,.mkv,.avi,.wav,.mp3,.flac,.ogg,.png,.jpg,.jpeg,.bmp,.webp"
    style="display: none;"
    bind:this={importMediaFileInput}
    on:change={handleMediaFilesSelected}
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
