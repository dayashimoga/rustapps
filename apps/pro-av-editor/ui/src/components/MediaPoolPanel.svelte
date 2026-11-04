<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import {
    getMediaFileUrl,
    tauriInvoke,
    type BlendMode,
    type Clip,
    type MediaSource,
    type TimelineProject,
  } from '../types';
  import { isDesktopApp } from '../lib/ipc';

  export let project: TimelineProject | null = null;
  export let selectedClipId: string | null = null;
  export let playheadMs: number = 0;

  const dispatch = createEventDispatcher<{
    refreshProject: void;
    importMedia: void;
    importMediaDialog: void;
    toast: string;
  }>();

  let activeSubTab: 'pool' | 'inspector' = 'pool';
  let customFilePath = '';
  let isImporting = false;
  let fileInputRef: HTMLInputElement;

  function triggerBrowseMedia() {
    if (isDesktopApp()) {
      dispatch('importMedia');
    } else if (fileInputRef) {
      fileInputRef.value = '';
      fileInputRef.click();
    }
  }

  async function handleLocalFilesSelected(e: Event) {
    const input = e.target as HTMLInputElement;
    const files = input?.files;
    if (!files || files.length === 0) return;
    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      const filePath = (file as any).path || file.name;
      await importPath(filePath, true);
    }
  }

  // Editable clip inspector state
  let editName = '';
  let editInSec = 0;
  let editOutSec = 5;
  let editSpeed = 1.0;
  let editOpacity = 1.0;
  let editVolume = 1.0;
  let editPan = 0.0;
  let editBlendMode: BlendMode = 'Normal';
  let editScaleX = 1.0;
  let editScaleY = 1.0;
  let editPosX = 0.0;
  let editPosY = 0.0;
  let editRotation = 0.0;
  let lastSyncedClipId: string | null = null;

  $: mediaPool = project?.media_pool || [];

  $: selectedClip = (() => {
    if (!project || !selectedClipId) return null;
    for (const tr of project.tracks) {
      const found = tr.clips.find((c) => c.id === selectedClipId);
      if (found) return found;
    }
    return null;
  })();

  $: selectedClipSource = (() => {
    if (!selectedClip?.source_id || !project?.media_pool) return null;
    return project.media_pool.find((m) => m.id === selectedClip.source_id) || null;
  })();

  $: if (selectedClip && selectedClip.id !== lastSyncedClipId) {
    lastSyncedClipId = selectedClip.id;
    syncInspectorFromClip(selectedClip);
    activeSubTab = 'inspector';
  } else if (!selectedClip) {
    lastSyncedClipId = null;
  }

  function syncInspectorFromClip(clip: Clip) {
    editName = clip.name;
    editInSec = Number((clip.timeline_in_ms / 1000).toFixed(2));
    editOutSec = Number((clip.timeline_out_ms / 1000).toFixed(2));
    editSpeed = clip.speed_multiplier || 1.0;
    editOpacity = clip.opacity?.default_value ?? 1.0;
    editVolume = clip.volume?.default_value ?? 1.0;
    editPan = clip.pan ?? 0.0;
    editBlendMode = clip.blend_mode || 'Normal';
    editScaleX = clip.transform?.scale_x ?? 1.0;
    editScaleY = clip.transform?.scale_y ?? 1.0;
    editPosX = clip.transform?.translate_x ?? 0.0;
    editPosY = clip.transform?.translate_y ?? 0.0;
    editRotation = clip.transform?.rotation_degrees ?? 0.0;
  }

  async function importPath(filePath: string, autoAdd: boolean = true) {
    const trimmed = filePath.trim();
    if (!trimmed) return;
    isImporting = true;
    try {
      const updated = await tauriInvoke<TimelineProject>('import_media_file_cmd', {
        filePath: trimmed,
        autoAddToTimeline: autoAdd,
      });
      customFilePath = '';
      // Probe HTML5 video/audio metadata if applicable
      const addedSource = updated?.media_pool?.find(
        (m) => m.file_path.toLowerCase() === trimmed.toLowerCase() || trimmed.endsWith(m.file_path.split(/[\\/]/).pop() || '')
      );
      if (addedSource) {
        probeHtml5Duration(addedSource);
      }
      dispatch('refreshProject');
      dispatch('toast', `Imported media: ${trimmed.split(/[\\/]/).pop()}`);
    } catch (e: any) {
      dispatch('toast', `Import error: ${e?.message || e}`);
    } finally {
      isImporting = false;
    }
  }

  function probeHtml5Duration(source: MediaSource) {
    const ext = source.file_path.split('.').pop()?.toLowerCase() || '';
    const url = getMediaFileUrl(source.file_path);
    if (['mp4', 'webm', 'mov', 'mkv', 'avi'].includes(ext)) {
      const v = document.createElement('video');
      v.preload = 'metadata';
      v.src = url;
      v.onloadedmetadata = async () => {
        if (v.duration && isFinite(v.duration) && v.duration > 0) {
          const durMs = Math.round(v.duration * 1000);
          await tauriInvoke('update_media_duration_cmd', {
            sourceId: source.id,
            durationMs: durMs,
            width: v.videoWidth || source.width,
            height: v.videoHeight || source.height,
          });
          dispatch('refreshProject');
        }
      };
    } else if (['mp3', 'flac', 'ogg', 'aac', 'm4a', 'wav'].includes(ext)) {
      const a = document.createElement('audio');
      a.preload = 'metadata';
      a.src = url;
      a.onloadedmetadata = async () => {
        if (a.duration && isFinite(a.duration) && a.duration > 0) {
          const durMs = Math.round(a.duration * 1000);
          await tauriInvoke('update_media_duration_cmd', {
            sourceId: source.id,
            durationMs: durMs,
          });
          dispatch('refreshProject');
        }
      };
    }
  }

  async function handleAddSourceToTimeline(source: MediaSource) {
    try {
      await tauriInvoke('add_media_to_timeline_cmd', {
        sourceId: source.id,
        timelineInMs: playheadMs > 0 ? playheadMs : undefined,
      });
      dispatch('refreshProject');
      dispatch('toast', `Added "${source.file_path.split(/[\\/]/).pop()}" to timeline.`);
    } catch (e: any) {
      dispatch('toast', `Failed to add clip: ${e?.message || e}`);
    }
  }

  async function handleAddGeneratorClip(clipType: 'Title' | 'ColorSolid') {
    if (!project) return;
    const videoTrack = project.tracks.find((t) => t.track_type === 'Video' && !t.locked);
    const trackId = videoTrack ? videoTrack.id : 'v1';
    const trackEnd = videoTrack
      ? Math.max(0, ...videoTrack.clips.map((c) => c.timeline_out_ms))
      : playheadMs;

    const newClip: Clip = {
      id: `gen_${clipType.toLowerCase()}_${Date.now()}`,
      name: clipType === 'Title' ? 'Title Overlay Banner' : 'Color Solid Matte',
      clip_type: clipType,
      track_id: trackId,
      source_id: null,
      source_in_ms: 0,
      source_out_ms: 3000,
      timeline_in_ms: trackEnd,
      timeline_out_ms: trackEnd + 3000,
      speed_multiplier: 1.0,
      volume: { default_value: 1.0, keyframes: [] },
      pan: 0.0,
      opacity: { default_value: 1.0, keyframes: [] },
      blend_mode: 'Normal',
      transform: {
        scale_x: 1.0,
        scale_y: 1.0,
        translate_x: 0.0,
        translate_y: 0.0,
        rotation_degrees: 0.0,
        anchor_x: 0.5,
        anchor_y: 0.5,
      },
      color_grading: {
        lift: { r: 0, g: 0, b: 0, luma: 0 },
        gamma: { r: 0, g: 0, b: 0, luma: 0 },
        gain: { r: 0, g: 0, b: 0, luma: 0 },
        exposure: 0,
        contrast: 1,
        saturation: 1,
        temperature: 0,
        tint: 0,
      },
      eq_settings: { enabled: true, bands: [] },
      compressor: {
        enabled: false,
        threshold_db: -18,
        ratio: 4,
        attack_ms: 10,
        release_ms: 100,
        makeup_gain_db: 0,
      },
    };

    try {
      await tauriInvoke('add_clip_cmd', { clip: newClip });
      dispatch('refreshProject');
      dispatch('toast', `Added ${newClip.name} to ${trackId.toUpperCase()}.`);
    } catch (e: any) {
      dispatch('toast', `Error adding generator clip: ${e?.message || e}`);
    }
  }

  async function handleAddTrack(trackType: 'Video' | 'Audio') {
    try {
      const existing = project?.tracks.filter((t) => t.track_type === trackType).length || 0;
      const trackName = `${trackType} ${existing + 1}`;
      const updated = await tauriInvoke<TimelineProject>('add_track_cmd', {
        name: trackName,
        trackType,
      });
      if (updated && updated.tracks && project) {
        project = updated;
      }
      dispatch('refreshProject');
      dispatch('toast', `Added new ${trackName} track.`);
    } catch (e: any) {
      dispatch('toast', `Error adding track: ${e?.message || e}`);
    }
  }

  async function handleInsertDemoAsset(type: 'smpte' | 'tone' | 'countdown' | 'title' | 'ambient') {
    if (!project) return;
    try {
      if (type === 'smpte') {
        await handleAddGeneratorClip('ColorSolid');
        dispatch('toast', 'Added SMPTE Test Solid to timeline.');
      } else if (type === 'title') {
        await handleAddGeneratorClip('Title');
        dispatch('toast', 'Added Studio Lower-Third Title.');
      } else {
        const demoPath = `demo_assets/${type}_48khz.wav`;
        const updated = await tauriInvoke<TimelineProject>('import_media_file_cmd', {
          filePath: demoPath,
          autoAddToTimeline: true,
        });
        if (updated) project = updated;
        dispatch('refreshProject');
        dispatch('toast', `Added ${type.toUpperCase()} demo asset to timeline.`);
      }
    } catch (e: any) {
      dispatch('toast', `Demo asset notice: ${e?.message || e}`);
    }
  }

  async function applyClipProperties() {
    if (!selectedClip) return;
    const inMs = Math.max(0, Math.round(editInSec * 1000));
    const outMs = Math.max(inMs + 100, Math.round(editOutSec * 1000));
    try {
      await tauriInvoke('update_clip_properties_cmd', {
        clipId: selectedClip.id,
        name: editName,
        timelineInMs: inMs,
        timelineOutMs: outMs,
        speedMultiplier: editSpeed,
        opacity: editOpacity,
        volume: editVolume,
        pan: editPan,
        blendMode: editBlendMode,
        transform: {
          scale_x: editScaleX,
          scale_y: editScaleY,
          translate_x: editPosX,
          translate_y: editPosY,
          rotation_degrees: editRotation,
          anchor_x: 0.5,
          anchor_y: 0.5,
        },
      });
      dispatch('refreshProject');
      dispatch('toast', `Updated properties for "${editName}".`);
    } catch (e: any) {
      dispatch('toast', `Update error: ${e?.message || e}`);
    }
  }

  async function handleSplitSelected() {
    if (!selectedClip) return;
    const splitPt =
      playheadMs > selectedClip.timeline_in_ms && playheadMs < selectedClip.timeline_out_ms
        ? playheadMs
        : Math.floor((selectedClip.timeline_in_ms + selectedClip.timeline_out_ms) / 2);
    try {
      await tauriInvoke('split_clip_cmd', {
        clipId: selectedClip.id,
        timestampMs: splitPt,
      });
      dispatch('refreshProject');
      dispatch('toast', `Split "${selectedClip.name}" at ${(splitPt / 1000).toFixed(2)}s.`);
    } catch (e: any) {
      dispatch('toast', `Split error: ${e?.message || e}`);
    }
  }

  async function handleRippleDeleteSelected() {
    if (!selectedClip) return;
    try {
      await tauriInvoke('ripple_delete_cmd', { clipId: selectedClip.id });
      dispatch('refreshProject');
      dispatch('toast', `Ripple deleted "${selectedClip.name}".`);
    } catch (e: any) {
      dispatch('toast', `Ripple delete error: ${e?.message || e}`);
    }
  }

  async function handleDeleteSelected() {
    if (!selectedClip) return;
    try {
      await tauriInvoke('remove_clip_cmd', { clipId: selectedClip.id });
      dispatch('refreshProject');
      dispatch('toast', `Removed "${selectedClip.name}".`);
    } catch (e: any) {
      dispatch('toast', `Delete error: ${e?.message || e}`);
    }
  }

  function getSourceBadge(src: MediaSource): string {
    const ext = src.file_path.split('.').pop()?.toLowerCase() || '';
    if (['png', 'bmp', 'jpg', 'jpeg', 'webp', 'gif'].includes(ext)) return 'IMAGE';
    if (src.has_video) return 'VIDEO';
    return 'AUDIO';
  }
</script>

<div class="media-pool-panel">
  <div class="panel-tabs">
    <button
      type="button"
      class="ptab"
      class:active={activeSubTab === 'pool'}
      on:click={() => (activeSubTab = 'pool')}
    >
      📁 Media Pool &amp; Bin ({mediaPool.length})
    </button>
    <button
      type="button"
      class="ptab"
      class:active={activeSubTab === 'inspector'}
      on:click={() => (activeSubTab = 'inspector')}
    >
      🛠 Clip Inspector {selectedClip ? `• ${selectedClip.name}` : ''}
    </button>
  </div>

  {#if activeSubTab === 'pool'}
    <div class="pool-body">
      <div class="import-bar">
        <button
          type="button"
          class="btn-browse-media"
          disabled={isImporting}
          on:click={triggerBrowseMedia}
        >
          📥 Browse Video / Audio / Image...
        </button>

        <form
          class="path-import-form"
          on:submit|preventDefault={() => importPath(customFilePath, true)}
        >
          <input
            type="text"
            class="path-input"
            placeholder="Or paste local media file path (e.g. C:\media\clip.mp4, .wav, .png)..."
            bind:value={customFilePath}
          />
          <button type="submit" class="btn-path-go" disabled={isImporting || !customFilePath.trim()}>
            Import
          </button>
        </form>
      </div>

      <div class="generators-bar">
        <span class="gen-label">QUICK INSERT:</span>
        <button type="button" class="gen-chip" on:click={() => handleAddGeneratorClip('Title')}>
          + Title Overlay
        </button>
        <button type="button" class="gen-chip" on:click={() => handleAddGeneratorClip('ColorSolid')}>
          + Color Solid
        </button>
        <button type="button" class="gen-chip" on:click={() => handleAddTrack('Video')}>
          + Video Track
        </button>
        <button type="button" class="gen-chip" on:click={() => handleAddTrack('Audio')}>
          + Audio Track
        </button>
      </div>

      <div class="demo-assets-bar">
        <span class="gen-label">PRE-BUNDLED DEMO MEDIA:</span>
        <button type="button" class="demo-chip" on:click={() => handleInsertDemoAsset('smpte')}>
          📺 SMPTE Bars
        </button>
        <button type="button" class="demo-chip" on:click={() => handleInsertDemoAsset('countdown')}>
          ⏱️ 8s Countdown
        </button>
        <button type="button" class="demo-chip" on:click={() => handleInsertDemoAsset('tone')}>
          🔊 1kHz Tone
        </button>
        <button type="button" class="demo-chip" on:click={() => handleInsertDemoAsset('ambient')}>
          🎵 Audio Bed
        </button>
      </div>

      <!-- Hidden Local File Input for 100% Reliable File Browser Trigger -->
      <input
        type="file"
        multiple
        accept="video/*,audio/*,image/*,.mp4,.webm,.mov,.mkv,.avi,.wav,.mp3,.flac,.ogg,.png,.jpg,.jpeg,.bmp,.webp"
        style="display: none;"
        bind:this={fileInputRef}
        on:change={handleLocalFilesSelected}
      />

      <div class="pool-list">
        {#if mediaPool.length === 0}
          <div class="empty-pool">
            <p class="empty-title">No External Media Imported Yet</p>
            <p class="empty-desc">
              Click <strong>📥 Browse Video / Audio / Image...</strong> to import real <code>.mp4</code>, <code>.webm</code>, <code>.mov</code>, <code>.wav</code>, <code>.mp3</code>, <code>.png</code>, <code>.jpg</code>, or <code>.bmp</code> files from disk directly onto the multi-track timeline.
            </p>
          </div>
        {:else}
          {#each mediaPool as src (src.id)}
            <div class="media-card">
              <div class="media-info">
                <div class="media-top">
                  <span class="badge badge-{getSourceBadge(src).toLowerCase()}">{getSourceBadge(src)}</span>
                  <span class="media-name" title={src.file_path}>
                    {src.file_path.split(/[\\/]/).pop()}
                  </span>
                </div>
                <div class="media-meta">
                  {#if src.width > 0 && src.height > 0}
                    <span>{src.width}×{src.height}</span>
                    <span>•</span>
                  {/if}
                  {#if src.sample_rate > 0}
                    <span>{(src.sample_rate / 1000).toFixed(1)}kHz</span>
                    <span>•</span>
                  {/if}
                  <span>{(src.duration_ms / 1000).toFixed(1)}s</span>
                  <span class="media-path-trunc" title={src.file_path}>{src.file_path}</span>
                </div>
              </div>
              <button
                type="button"
                class="btn-insert-clip"
                on:click={() => handleAddSourceToTimeline(src)}
                title="Insert onto timeline"
              >
                ➕ Add to Timeline
              </button>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {:else}
    <div class="inspector-body">
      {#if !selectedClip}
        <div class="empty-pool">
          <p class="empty-title">No Timeline Clip Selected</p>
          <p class="empty-desc">
            Click any clip on the multi-track timeline below to inspect or modify its timing, playback speed, blend mode, 2D spatial transform, or audio gain.
          </p>
        </div>
      {:else}
        <div class="inspector-grid">
          <div class="field-row full">
            <label for="clip-name-in">Clip Name</label>
            <input id="clip-name-in" type="text" bind:value={editName} on:change={applyClipProperties} />
          </div>

          {#if selectedClipSource}
            <div class="field-row full source-banner">
              <span class="src-lbl">SOURCE FILE:</span>
              <code class="src-val">{selectedClipSource.file_path}</code>
            </div>
          {/if}

          <div class="field-row">
            <label for="clip-in-sec">Timeline In (s)</label>
            <input id="clip-in-sec" type="number" step="0.1" min="0" bind:value={editInSec} on:change={applyClipProperties} />
          </div>

          <div class="field-row">
            <label for="clip-out-sec">Timeline Out (s)</label>
            <input id="clip-out-sec" type="number" step="0.1" min="0.1" bind:value={editOutSec} on:change={applyClipProperties} />
          </div>

          <div class="field-row">
            <label for="clip-speed">Speed ({editSpeed.toFixed(2)}x)</label>
            <input id="clip-speed" type="range" min="0.25" max="4.0" step="0.05" bind:value={editSpeed} on:input={applyClipProperties} on:change={applyClipProperties} />
          </div>

          <div class="field-row">
            <label for="clip-opacity">Opacity ({Math.round(editOpacity * 100)}%)</label>
            <input id="clip-opacity" type="range" min="0" max="1" step="0.02" bind:value={editOpacity} on:input={applyClipProperties} on:change={applyClipProperties} />
          </div>

          <div class="field-row">
            <label for="clip-blend">Blend Mode</label>
            <select id="clip-blend" bind:value={editBlendMode} on:change={applyClipProperties}>
              <option value="Normal">Normal</option>
              <option value="Multiply">Multiply</option>
              <option value="Screen">Screen</option>
              <option value="Overlay">Overlay</option>
              <option value="Add">Add (Linear Dodge)</option>
              <option value="Darken">Darken</option>
              <option value="Lighten">Lighten</option>
              <option value="Difference">Difference</option>
            </select>
          </div>

          <div class="field-row">
            <label for="clip-vol">Audio Gain ({Math.round(editVolume * 100)}%)</label>
            <input id="clip-vol" type="range" min="0" max="2" step="0.05" bind:value={editVolume} on:input={applyClipProperties} on:change={applyClipProperties} />
          </div>

          <div class="field-row">
            <label for="clip-scale-x">Scale X ({editScaleX.toFixed(2)})</label>
            <input id="clip-scale-x" type="range" min="0.1" max="3.0" step="0.05" bind:value={editScaleX} on:input={applyClipProperties} on:change={applyClipProperties} />
          </div>

          <div class="field-row">
            <label for="clip-scale-y">Scale Y ({editScaleY.toFixed(2)})</label>
            <input id="clip-scale-y" type="range" min="0.1" max="3.0" step="0.05" bind:value={editScaleY} on:input={applyClipProperties} on:change={applyClipProperties} />
          </div>

          <div class="field-row">
            <label for="clip-rot">Rotation ({editRotation.toFixed(0)}°)</label>
            <input id="clip-rot" type="range" min="-180" max="180" step="1" bind:value={editRotation} on:input={applyClipProperties} on:change={applyClipProperties} />
          </div>

          <div class="field-row">
            <label for="clip-pan">Stereo Pan ({editPan.toFixed(2)})</label>
            <input id="clip-pan" type="range" min="-1" max="1" step="0.05" bind:value={editPan} on:input={applyClipProperties} on:change={applyClipProperties} />
          </div>
        </div>

        <div class="inspector-actions">
          <button type="button" class="act-btn primary" on:click={applyClipProperties}>
            ✓ Apply Changes
          </button>
          <button type="button" class="act-btn" on:click={handleSplitSelected}>
            ✂ Split Clip
          </button>
          <button type="button" class="act-btn" on:click={handleRippleDeleteSelected}>
            ⏭ Ripple Delete
          </button>
          <button type="button" class="act-btn danger" on:click={handleDeleteSelected}>
            🗑 Delete
          </button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .media-pool-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #0d1117;
    color: #e6edf3;
    overflow: hidden;
  }

  .panel-tabs {
    display: flex;
    background: #161b22;
    border-bottom: 1px solid #21262d;
  }

  .ptab {
    flex: 1;
    padding: 0.55rem 0.75rem;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: #8b949e;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ptab:hover {
    color: #e6edf3;
  }

  .ptab.active {
    color: #38bdf8;
    border-bottom-color: #38bdf8;
    background: rgba(56, 189, 248, 0.06);
  }

  .pool-body,
  .inspector-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 0.75rem;
    gap: 0.65rem;
    overflow-y: auto;
  }

  .import-bar {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .btn-browse-media {
    width: 100%;
    padding: 0.5rem 0.85rem;
    background: linear-gradient(135deg, rgba(56, 189, 248, 0.2), rgba(99, 102, 241, 0.2));
    border: 1px solid rgba(56, 189, 248, 0.45);
    border-radius: 6px;
    color: #7dd3fc;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-browse-media:hover:not(:disabled) {
    background: rgba(56, 189, 248, 0.3);
    border-color: #38bdf8;
    color: #ffffff;
  }

  .path-import-form {
    display: flex;
    gap: 0.35rem;
  }

  .path-input {
    flex: 1;
    background: #090c10;
    border: 1px solid #30363d;
    border-radius: 4px;
    padding: 0.35rem 0.6rem;
    color: #e6edf3;
    font-size: 0.75rem;
    font-family: var(--font-mono, monospace);
  }

  .btn-path-go {
    background: #21262d;
    border: 1px solid #30363d;
    border-radius: 4px;
    color: #e6edf3;
    padding: 0.35rem 0.75rem;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-path-go:hover:not(:disabled) {
    background: #30363d;
  }

  .generators-bar {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-wrap: wrap;
    padding-bottom: 0.4rem;
    border-bottom: 1px solid #21262d;
  }

  .gen-label {
    font-size: 0.65rem;
    font-weight: 700;
    color: #8b949e;
    margin-right: 0.2rem;
  }

  .gen-chip {
    background: #161b22;
    border: 1px solid #30363d;
    color: #c9d1d9;
    padding: 0.22rem 0.55rem;
    border-radius: 4px;
    font-size: 0.7rem;
    cursor: pointer;
  }

  .gen-chip:hover {
    border-color: #58a6ff;
    color: #58a6ff;
  }

  .demo-assets-bar {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-wrap: wrap;
    padding-bottom: 0.4rem;
    border-bottom: 1px solid #21262d;
  }

  .demo-chip {
    background: rgba(168, 85, 247, 0.12);
    border: 1px solid rgba(168, 85, 247, 0.35);
    color: #d8b4fe;
    padding: 0.22rem 0.55rem;
    border-radius: 4px;
    font-size: 0.7rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .demo-chip:hover {
    background: rgba(168, 85, 247, 0.25);
    border-color: #c084fc;
    color: #ffffff;
  }

  .pool-list {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    flex: 1;
  }

  .empty-pool {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 1.5rem 1rem;
    background: rgba(22, 27, 34, 0.5);
    border: 1px dashed #30363d;
    border-radius: 8px;
    margin-top: 0.25rem;
  }

  .empty-title {
    font-size: 0.85rem;
    font-weight: 600;
    color: #c9d1d9;
    margin-bottom: 0.35rem;
  }

  .empty-desc {
    font-size: 0.74rem;
    color: #8b949e;
    line-height: 1.45;
    max-width: 360px;
  }

  .media-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
    padding: 0.5rem 0.65rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 6px;
  }

  .media-info {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
    flex: 1;
  }

  .media-top {
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }

  .badge {
    font-size: 0.62rem;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: 3px;
    font-family: var(--font-mono, monospace);
  }

  .badge-video {
    background: rgba(59, 130, 246, 0.2);
    color: #60a5fa;
    border: 1px solid rgba(59, 130, 246, 0.4);
  }

  .badge-audio {
    background: rgba(16, 185, 129, 0.2);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.4);
  }

  .badge-image {
    background: rgba(245, 158, 11, 0.2);
    color: #fbbf24;
    border: 1px solid rgba(245, 158, 11, 0.4);
  }

  .media-name {
    font-size: 0.8rem;
    font-weight: 600;
    color: #f0f6fc;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .media-meta {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.68rem;
    color: #8b949e;
    font-family: var(--font-mono, monospace);
  }

  .media-path-trunc {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 180px;
    opacity: 0.75;
  }

  .btn-insert-clip {
    background: rgba(56, 189, 248, 0.14);
    border: 1px solid rgba(56, 189, 248, 0.4);
    color: #38bdf8;
    padding: 0.3rem 0.6rem;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 600;
    cursor: pointer;
    flex-shrink: 0;
  }

  .btn-insert-clip:hover {
    background: rgba(56, 189, 248, 0.25);
  }

  .inspector-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.55rem;
  }

  .field-row {
    display: flex;
    flex-direction: column;
    gap: 0.22rem;
  }

  .field-row.full {
    grid-column: span 2;
  }

  .source-banner {
    background: #161b22;
    border: 1px solid #21262d;
    padding: 0.35rem 0.55rem;
    border-radius: 4px;
    flex-direction: row;
    align-items: center;
    gap: 0.45rem;
  }

  .src-lbl {
    font-size: 0.65rem;
    font-weight: 700;
    color: #8b949e;
  }

  .src-val {
    font-size: 0.7rem;
    color: #7dd3fc;
    word-break: break-all;
  }

  .field-row label {
    font-size: 0.68rem;
    font-weight: 600;
    color: #8b949e;
    text-transform: uppercase;
  }

  .field-row input[type='text'],
  .field-row input[type='number'],
  .field-row select {
    background: #090c10;
    border: 1px solid #30363d;
    border-radius: 4px;
    padding: 0.3rem 0.5rem;
    color: #e6edf3;
    font-size: 0.76rem;
  }

  .inspector-actions {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
    margin-top: 0.35rem;
    padding-top: 0.55rem;
    border-top: 1px solid #21262d;
  }

  .act-btn {
    background: #21262d;
    border: 1px solid #30363d;
    color: #e6edf3;
    padding: 0.35rem 0.65rem;
    border-radius: 4px;
    font-size: 0.74rem;
    font-weight: 600;
    cursor: pointer;
  }

  .act-btn.primary {
    background: #0284c7;
    border-color: #38bdf8;
    color: white;
  }

  .act-btn.danger {
    background: rgba(239, 68, 68, 0.15);
    border-color: rgba(239, 68, 68, 0.4);
    color: #f87171;
  }
</style>

