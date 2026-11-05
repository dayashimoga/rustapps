<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import {
    formatTimecode,
    tauriInvoke,
    type AudioSyncResult,
    type Clip,
    type TimelineProject,
    type Track,
  } from '../types';

  export let project: TimelineProject | null = null;
  export let playheadMs: number = 0;
  export let durationMs: number = 6000;
  export let selectedClipId: string | null = null;

  let zoomPxPerSec: number = 80; // 80px per second default zoom
  let snappingEnabled: boolean = true;
  let timelineNotice: string = '';

  // Drag / Trim state
  let dragMode: 'move' | 'trim-left' | 'trim-right' | null = null;
  let dragClipId: string | null = null;
  let dragTrackId: string | null = null;
  let dragStartX: number = 0;
  let dragOrigInMs: number = 0;
  let dragOrigOutMs: number = 0;
  let dragCurrentInMs: number = 0;
  let dragCurrentOutMs: number = 0;

  const dispatch = createEventDispatcher<{
    seek: number;
    selectClip: Clip;
    refreshProject: void;
  }>();

  $: timelineWidthPx = Math.max(800, Math.round((durationMs / 1000) * zoomPxPerSec));
  $: hasAnyClips = Boolean(
    project?.tracks?.some((t) => t.clips && t.clips.length > 0)
  );

  function showNotice(msg: string) {
    timelineNotice = msg;
    setTimeout(() => {
      if (timelineNotice === msg) {
        timelineNotice = '';
      }
    }, 3000);
  }

  onMount(() => {
    window.addEventListener('keydown', handleTimelineKeys);
    window.addEventListener('mousemove', handleGlobalMouseMove);
    window.addEventListener('mouseup', handleGlobalMouseUp);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleTimelineKeys);
    window.removeEventListener('mousemove', handleGlobalMouseMove);
    window.removeEventListener('mouseup', handleGlobalMouseUp);
  });

  function handleTimelineKeys(e: KeyboardEvent) {
    const active = document.activeElement;
    if (active && (active.tagName === 'INPUT' || active.tagName === 'TEXTAREA' || active.tagName === 'SELECT')) {
      return;
    }
    if (e.code === 'KeyC' && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      splitSelectedClip();
    } else if ((e.code === 'Delete' || e.code === 'Backspace') && e.shiftKey) {
      e.preventDefault();
      rippleDeleteSelectedClip();
    } else if (e.code === 'Delete' || e.code === 'Backspace') {
      e.preventDefault();
      removeSelectedClip();
    }
  }

  function handleRulerClick(e: MouseEvent) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    let targetMs = Math.round((clickX / zoomPxPerSec) * 1000);

    if (snappingEnabled && project) {
      targetMs = calculateSnap(targetMs);
    }

    dispatch('seek', Math.max(0, Math.min(targetMs, durationMs)));
  }

  function calculateSnap(candidateMs: number): number {
    if (!project) return candidateMs;
    const toleranceMs = 80;
    let closest = candidateMs;
    let minDiff = toleranceMs + 1;

    for (const track of project.tracks) {
      for (const clip of track.clips) {
        const dIn = Math.abs(candidateMs - clip.timeline_in_ms);
        if (dIn <= toleranceMs && dIn < minDiff) {
          minDiff = dIn;
          closest = clip.timeline_in_ms;
        }
        const dOut = Math.abs(candidateMs - clip.timeline_out_ms);
        if (dOut <= toleranceMs && dOut < minDiff) {
          minDiff = dOut;
          closest = clip.timeline_out_ms;
        }
      }
    }
    return closest;
  }

  function selectClip(clip: Clip, e: MouseEvent) {
    e.stopPropagation();
    selectedClipId = clip.id;
    dispatch('selectClip', clip);
  }

  function resolveTargetClip(requirePlayheadInside: boolean = false): Clip | null {
    if (!project) return null;
    if (selectedClipId) {
      for (const tr of project.tracks) {
        if (tr.locked) continue;
        const found = tr.clips.find((c) => c.id === selectedClipId);
        if (found) {
          if (!requirePlayheadInside || (playheadMs > found.timeline_in_ms && playheadMs < found.timeline_out_ms)) {
            return found;
          }
        }
      }
    }
    // Fallback: find the first unlocked clip under the playhead
    for (const tr of project.tracks) {
      if (tr.locked) continue;
      for (const cl of tr.clips) {
        if (playheadMs > cl.timeline_in_ms && playheadMs < cl.timeline_out_ms) {
          selectedClipId = cl.id;
          dispatch('selectClip', cl);
          return cl;
        }
      }
    }
    if (!requirePlayheadInside) {
      for (const tr of project.tracks) {
        if (tr.locked) continue;
        if (tr.clips.length > 0) {
          selectedClipId = tr.clips[0].id;
          dispatch('selectClip', tr.clips[0]);
          return tr.clips[0];
        }
      }
    }
    return null;
  }

  async function splitSelectedClip() {
    const target = resolveTargetClip(true) || resolveTargetClip(false);
    if (!target) {
      showNotice('No clip on timeline to split.');
      return;
    }
    let splitAtMs = playheadMs;
    if (splitAtMs <= target.timeline_in_ms || splitAtMs >= target.timeline_out_ms) {
      // Split at clip midpoint if playhead is outside the selected clip
      splitAtMs = Math.round((target.timeline_in_ms + target.timeline_out_ms) / 2);
    }
    try {
      await tauriInvoke('split_clip_cmd', {
        clipId: target.id,
        timestampMs: splitAtMs,
      });
      showNotice(`Split "${target.name}" at ${formatTimecode(splitAtMs)}`);
      dispatch('refreshProject');
    } catch (e) {
      console.error('Split failed:', e);
      showNotice(`Split error: ${e}`);
    }
  }

  async function rippleDeleteSelectedClip() {
    const target = resolveTargetClip(false);
    if (!target) {
      showNotice('Select a clip to ripple delete.');
      return;
    }
    try {
      await tauriInvoke('ripple_delete_cmd', {
        clipId: target.id,
      });
      selectedClipId = null;
      showNotice(`Ripple deleted "${target.name}"`);
      dispatch('refreshProject');
    } catch (e) {
      console.error('Ripple delete failed:', e);
      showNotice(`Ripple delete error: ${e}`);
    }
  }

  async function removeSelectedClip() {
    const target = resolveTargetClip(false);
    if (!target) {
      showNotice('Select a clip to delete.');
      return;
    }
    try {
      await tauriInvoke('remove_clip_cmd', {
        clipId: target.id,
      });
      selectedClipId = null;
      showNotice(`Deleted "${target.name}"`);
      dispatch('refreshProject');
    } catch (e) {
      console.error('Delete failed:', e);
      showNotice(`Delete error: ${e}`);
    }
  }

  async function autoSyncAudioTracks() {
    if (!project) return;
    const allClips: Clip[] = [];
    for (const tr of project.tracks) {
      for (const cl of tr.clips) {
        allClips.push(cl);
      }
    }
    if (allClips.length < 2) {
      showNotice('Need at least 2 clips on timeline to auto-sync audio.');
      return;
    }

    let refClip = selectedClipId ? allClips.find((c) => c.id === selectedClipId) : allClips[0];
    if (!refClip) refClip = allClips[0];
    const tgtClip = allClips.find((c) => c.id !== refClip?.id);
    if (!tgtClip) {
      showNotice('Need a second clip on timeline to sync.');
      return;
    }

    try {
      showNotice('Analyzing audio waveforms...');
      const res = await tauriInvoke<AudioSyncResult>('sync_audio_tracks_cmd', {
        referenceClipId: refClip.id,
        targetClipId: tgtClip.id,
      });
      if (res && res.aligned) {
        showNotice(`Synced "${tgtClip.name}" with "${refClip.name}" (${res.offset_ms >= 0 ? '+' : ''}${res.offset_ms}ms)`);
        dispatch('refreshProject');
      } else {
        showNotice(`Audio sync: Low correlation (${((res?.correlation_score || 0) * 100).toFixed(0)}%).`);
      }
    } catch (e) {
      console.error('Audio sync failed:', e);
      showNotice(`Audio sync error: ${e}`);
    }
  }

  async function addNewTrack(trackType: 'Video' | 'Audio') {
    try {
      const existing = project?.tracks.filter((t) => t.track_type === trackType).length || 0;
      const trackName = `${trackType} ${existing + 1}`;
      const updated = await tauriInvoke<TimelineProject>('add_track_cmd', {
        name: trackName,
        trackType,
      });
      if (updated && updated.tracks) {
        project = updated;
      }
      showNotice(`Added ${trackName}`);
      dispatch('refreshProject');
    } catch (e) {
      console.error('Failed to add track:', e);
      showNotice(`Failed to add track: ${e}`);
    }
  }

  async function toggleTrackState(
    track: Track,
    field: 'muted' | 'solo' | 'locked'
  ) {
    const nextMuted = field === 'muted' ? !track.muted : track.muted;
    const nextSolo = field === 'solo' ? !track.solo : track.solo;
    const nextLocked = field === 'locked' ? !track.locked : track.locked;
    try {
      await tauriInvoke('update_track_state_cmd', {
        trackId: track.id,
        muted: nextMuted,
        solo: nextSolo,
        locked: nextLocked,
      });
      dispatch('refreshProject');
    } catch (e) {
      console.error('Failed to update track state:', e);
    }
  }

  function startClipDrag(
    clip: Clip,
    track: Track,
    mode: 'move' | 'trim-left' | 'trim-right',
    e: MouseEvent
  ) {
    if (track.locked) {
      showNotice(`Track "${track.name}" is locked.`);
      return;
    }
    e.stopPropagation();
    selectedClipId = clip.id;
    dispatch('selectClip', clip);

    dragMode = mode;
    dragClipId = clip.id;
    dragTrackId = track.id;
    dragStartX = e.clientX;
    dragOrigInMs = clip.timeline_in_ms;
    dragOrigOutMs = clip.timeline_out_ms;
    dragCurrentInMs = clip.timeline_in_ms;
    dragCurrentOutMs = clip.timeline_out_ms;
  }

  function handleGlobalMouseMove(e: MouseEvent) {
    if (!dragMode || !dragClipId) return;
    const deltaPx = e.clientX - dragStartX;
    const deltaMs = Math.round((deltaPx / zoomPxPerSec) * 1000);

    if (dragMode === 'move') {
      const dur = dragOrigOutMs - dragOrigInMs;
      let nextIn = Math.max(0, dragOrigInMs + deltaMs);
      if (snappingEnabled) {
        nextIn = calculateSnap(nextIn);
      }
      dragCurrentInMs = nextIn;
      dragCurrentOutMs = nextIn + dur;
    } else if (dragMode === 'trim-left') {
      let nextIn = Math.max(0, Math.min(dragOrigOutMs - 100, dragOrigInMs + deltaMs));
      if (snappingEnabled) {
        nextIn = calculateSnap(nextIn);
      }
      dragCurrentInMs = nextIn;
    } else if (dragMode === 'trim-right') {
      let nextOut = Math.max(dragOrigInMs + 100, dragOrigOutMs + deltaMs);
      if (snappingEnabled) {
        nextOut = calculateSnap(nextOut);
      }
      dragCurrentOutMs = nextOut;
    }
  }

  async function handleGlobalMouseUp() {
    if (!dragMode || !dragClipId) return;
    const mode = dragMode;
    const clipId = dragClipId;
    const trackId = dragTrackId;
    const finalIn = dragCurrentInMs;
    const finalOut = dragCurrentOutMs;
    const changed = finalIn !== dragOrigInMs || finalOut !== dragOrigOutMs;

    dragMode = null;
    dragClipId = null;
    dragTrackId = null;

    if (!changed) return;

    try {
      if (mode === 'move') {
        await tauriInvoke('move_clip_cmd', {
          clipId,
          trackId,
          timelineInMs: finalIn,
        });
      } else {
        await tauriInvoke('trim_clip_cmd', {
          clipId,
          newInMs: finalIn,
          newOutMs: finalOut,
        });
      }
      dispatch('refreshProject');
    } catch (e) {
      console.error('Clip drag/trim failed:', e);
    }
  }
</script>

<div class="timeline-container">
  <!-- Editing Tools Bar -->
  <div class="timeline-toolbar">
    <div class="tool-group">
      <button
        class="action-btn"
        disabled={!hasAnyClips}
        on:click={splitSelectedClip}
        title="Razor / Split at Playhead (C)"
      >
        ✂ Split (Razor)
      </button>
      <button
        class="action-btn"
        disabled={!hasAnyClips}
        on:click={rippleDeleteSelectedClip}
        title="Ripple Delete Clip (Shift+Del)"
      >
        ⏪ Ripple Delete
      </button>
      <button
        class="action-btn"
        disabled={!hasAnyClips}
        on:click={removeSelectedClip}
        title="Delete Clip (Del)"
      >
        🗑 Delete
      </button>
      <button
        class="action-btn"
        disabled={!hasAnyClips}
        on:click={autoSyncAudioTracks}
        title="1-Click Auto-Sync Audio Waveforms across Tracks"
      >
        🎵 Auto-Sync Audio
      </button>
      <button
        class="action-btn"
        on:click={() => addNewTrack('Video')}
        title="Add New Video Track"
      >
        + Video Track
      </button>
      <button
        class="action-btn"
        on:click={() => addNewTrack('Audio')}
        title="Add New Audio Track"
      >
        + Audio Track
      </button>
      {#if timelineNotice}
        <span class="timeline-notice">{timelineNotice}</span>
      {/if}
    </div>

    <div class="snap-zoom-group">
      <label class="snap-label">
        <input type="checkbox" bind:checked={snappingEnabled} />
        <span>🧲 Magnetic Snapping</span>
      </label>

      <div class="zoom-wrap">
        <span class="zoom-label">Zoom:</span>
        <input
          type="range"
          min="30"
          max="250"
          bind:value={zoomPxPerSec}
          class="zoom-slider"
        />
      </div>
    </div>
  </div>

  <!-- Tracks & Timeline Canvas -->
  <div class="tracks-scroll-wrap">
    <!-- Fixed Track Headers Column -->
    <div class="track-headers-col">
      <div class="ruler-header-spacer">Tracks</div>
      {#if project}
        {#each project.tracks as track}
          <div class="track-header" class:video-header={track.track_type === 'Video'} class:audio-header={track.track_type === 'Audio'}>
            <span class="track-name">{track.name}</span>
            <div class="track-controls">
              <button
                class="state-btn"
                class:active={track.muted}
                on:click={() => toggleTrackState(track, 'muted')}
                title="Mute Track"
              >
                M
              </button>
              <button
                class="state-btn"
                class:active={track.solo}
                on:click={() => toggleTrackState(track, 'solo')}
                title="Solo Track"
              >
                S
              </button>
              <button
                class="state-btn"
                class:active={track.locked}
                on:click={() => toggleTrackState(track, 'locked')}
                title="Lock Track"
              >
                🔒
              </button>
            </div>
          </div>
        {/each}
      {/if}
    </div>

    <!-- Scrollable Timeline Tracks Area -->
    <div class="timeline-body-scroll">
      <div class="timeline-content" style={`width: ${timelineWidthPx}px;`}>
        <!-- Timecode Ruler -->
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
        <div
          class="ruler"
          role="slider"
          aria-label="Timeline Ruler"
          aria-valuemin="0"
          aria-valuemax={durationMs}
          aria-valuenow={playheadMs}
          tabindex="0"
          on:click={handleRulerClick}
        >
          {#each Array(Math.ceil(durationMs / 1000) + 1) as _, sec}
            <div
              class="ruler-tick"
              style={`left: ${sec * zoomPxPerSec}px;`}
            >
              <span class="tick-label">{formatTimecode(sec * 1000)}</span>
            </div>
          {/each}
        </div>

        <!-- Track Lanes -->
        {#if project}
          <div class="track-lanes">
            {#each project.tracks as track}
              <div
                class="track-lane"
                class:video-lane={track.track_type === 'Video'}
                class:audio-lane={track.track_type === 'Audio'}
              >
                {#each track.clips as clip}
                  {@const dispInMs = dragClipId === clip.id ? dragCurrentInMs : clip.timeline_in_ms}
                  {@const dispOutMs = dragClipId === clip.id ? dragCurrentOutMs : clip.timeline_out_ms}
                  <!-- svelte-ignore a11y-click-events-have-key-events -->
                  <div
                    class="clip-box"
                    role="button"
                    tabindex="0"
                    class:selected={selectedClipId === clip.id}
                    class:video-clip={track.track_type === 'Video'}
                    class:audio-clip={track.track_type === 'Audio'}
                    class:sub-clip={track.track_type === 'Subtitle'}
                    style={`left: ${(dispInMs / 1000) * zoomPxPerSec}px; width: ${Math.max(20, ((dispOutMs - dispInMs) / 1000) * zoomPxPerSec)}px;`}
                    on:mousedown={(e) => startClipDrag(clip, track, 'move', e)}
                    on:click={(e) => selectClip(clip, e)}
                  >
                    <!-- svelte-ignore a11y-no-static-element-interactions -->
                    <div
                      class="trim-handle trim-left"
                      title="Drag to Trim In-Point"
                      on:mousedown={(e) => startClipDrag(clip, track, 'trim-left', e)}
                    ></div>
                    <span class="clip-name">{clip.name}</span>
                    <span class="clip-dur">{((dispOutMs - dispInMs) / 1000).toFixed(1)}s</span>
                    <!-- svelte-ignore a11y-no-static-element-interactions -->
                    <div
                      class="trim-handle trim-right"
                      title="Drag to Trim Out-Point"
                      on:mousedown={(e) => startClipDrag(clip, track, 'trim-right', e)}
                    ></div>
                  </div>
                {/each}
              </div>
            {/each}
          </div>
        {/if}

        <!-- Playhead Cursor Line -->
        <div
          class="playhead-line"
          style={`left: ${(playheadMs / 1000) * zoomPxPerSec}px;`}
        >
          <div class="playhead-handle">▼</div>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .timeline-container {
    display: flex;
    flex-direction: column;
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    overflow: hidden;
    height: 100%;
  }

  .timeline-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.45rem 1rem;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-subtle);
  }

  .tool-group {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .timeline-notice {
    font-size: 0.72rem;
    color: #38bdf8;
    background: rgba(56, 189, 248, 0.12);
    border: 1px solid rgba(56, 189, 248, 0.3);
    padding: 2px 8px;
    border-radius: 4px;
    margin-left: 6px;
  }

  .trim-handle {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 6px;
    cursor: ew-resize;
    z-index: 5;
  }

  .trim-left {
    left: 0;
    border-left: 2px solid rgba(255, 255, 255, 0.55);
  }

  .trim-right {
    right: 0;
    border-right: 2px solid rgba(255, 255, 255, 0.55);
  }

  .trim-handle:hover {
    background: rgba(255, 255, 255, 0.35);
  }

  .action-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    padding: 0.35rem 0.65rem;
    border-radius: var(--radius-sm);
    font-size: 0.75rem;
    font-weight: 500;
  }

  .action-btn:hover:not(:disabled) {
    background: var(--bg-panel-hover);
    border-color: var(--accent-primary);
  }

  .action-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .snap-zoom-group {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .snap-label {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.75rem;
    color: var(--text-muted);
    cursor: pointer;
  }

  .zoom-wrap {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .zoom-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .zoom-slider {
    width: 90px;
    cursor: pointer;
  }

  .tracks-scroll-wrap {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .track-headers-col {
    width: 140px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    z-index: 10;
  }

  .ruler-header-spacer {
    height: 28px;
    display: flex;
    align-items: center;
    padding-left: 0.75rem;
    font-size: 0.7rem;
    color: var(--text-faint);
    text-transform: uppercase;
    border-bottom: 1px solid var(--border-subtle);
  }

  .track-header {
    height: 52px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0.5rem;
    border-bottom: 1px solid var(--border-subtle);
    border-left: 3px solid transparent;
  }

  .video-header { border-left-color: var(--track-video); }
  .audio-header { border-left-color: var(--track-audio); }

  .track-name {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-main);
  }

  .track-controls {
    display: flex;
    gap: 2px;
  }

  .state-btn {
    background: var(--bg-tertiary);
    border: none;
    color: var(--text-muted);
    font-size: 0.65rem;
    padding: 2px 5px;
    border-radius: 2px;
  }

  .state-btn.active {
    background: var(--accent-primary);
    color: white;
  }

  .timeline-body-scroll {
    flex: 1;
    overflow-x: auto;
    overflow-y: hidden;
    position: relative;
  }

  .timeline-content {
    position: relative;
    height: 100%;
    min-height: 200px;
  }

  .ruler {
    height: 28px;
    background: var(--bg-primary);
    border-bottom: 1px solid var(--border-subtle);
    position: relative;
    cursor: pointer;
  }

  .ruler-tick {
    position: absolute;
    top: 0;
    bottom: 0;
    border-left: 1px solid rgba(255, 255, 255, 0.15);
    padding-left: 3px;
  }

  .tick-label {
    font-family: var(--font-mono);
    font-size: 0.65rem;
    color: var(--text-faint);
  }

  .track-lanes {
    display: flex;
    flex-direction: column;
  }

  .track-lane {
    height: 52px;
    position: relative;
    border-bottom: 1px solid var(--border-subtle);
    background: rgba(255, 255, 255, 0.01);
  }

  .clip-box {
    position: absolute;
    top: 4px;
    bottom: 4px;
    border-radius: var(--radius-sm);
    padding: 0.25rem 0.5rem;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    cursor: pointer;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    border: 1px solid rgba(255, 255, 255, 0.2);
  }

  .video-clip {
    background: linear-gradient(135deg, rgba(37, 99, 235, 0.75), rgba(30, 64, 175, 0.85));
    border-color: #3b82f6;
  }

  .audio-clip {
    background: linear-gradient(135deg, rgba(5, 150, 105, 0.75), rgba(4, 120, 87, 0.85));
    border-color: #10b981;
  }

  .sub-clip {
    background: linear-gradient(135deg, rgba(217, 119, 6, 0.75), rgba(180, 83, 9, 0.85));
    border-color: #f59e0b;
  }

  .clip-box.selected {
    outline: 2px solid #ffffff;
    box-shadow: 0 0 10px rgba(255, 255, 255, 0.5);
  }

  .clip-name {
    font-size: 0.75rem;
    font-weight: 500;
    color: white;
  }

  .clip-dur {
    font-size: 0.65rem;
    font-family: var(--font-mono);
    color: rgba(255, 255, 255, 0.75);
  }

  .playhead-line {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--accent-rose);
    pointer-events: none;
    z-index: 50;
  }

  .playhead-handle {
    position: absolute;
    top: -2px;
    left: -5px;
    color: var(--accent-rose);
    font-size: 0.75rem;
  }
</style>

