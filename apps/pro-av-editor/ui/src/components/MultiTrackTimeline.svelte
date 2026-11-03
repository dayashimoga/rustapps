<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import {
    formatTimecode,
    tauriInvoke,
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

  const dispatch = createEventDispatcher<{
    seek: number;
    selectClip: Clip;
    refreshProject: void;
  }>();

  $: timelineWidthPx = Math.max(800, Math.round((durationMs / 1000) * zoomPxPerSec));

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

  async function splitSelectedClip() {
    if (!selectedClipId) return;
    try {
      await tauriInvoke('split_clip_cmd', {
        clipId: selectedClipId,
        timestampMs: playheadMs,
      });
      dispatch('refreshProject');
    } catch (e) {
      console.error('Split failed:', e);
    }
  }

  async function rippleDeleteSelectedClip() {
    if (!selectedClipId) return;
    try {
      await tauriInvoke('ripple_delete_cmd', {
        clipId: selectedClipId,
      });
      selectedClipId = null;
      dispatch('refreshProject');
    } catch (e) {
      console.error('Ripple delete failed:', e);
    }
  }

  async function removeSelectedClip() {
    if (!selectedClipId) return;
    try {
      await tauriInvoke('remove_clip_cmd', {
        clipId: selectedClipId,
      });
      selectedClipId = null;
      dispatch('refreshProject');
    } catch (e) {
      console.error('Delete failed:', e);
    }
  }
</script>

<div class="timeline-container">
  <!-- Editing Tools Bar -->
  <div class="timeline-toolbar">
    <div class="tool-group">
      <button
        class="action-btn"
        disabled={!selectedClipId}
        on:click={splitSelectedClip}
        title="Razor / Split at Playhead (C)"
      >
        ✂ Split (Razor)
      </button>
      <button
        class="action-btn"
        disabled={!selectedClipId}
        on:click={rippleDeleteSelectedClip}
        title="Ripple Delete Clip (Shift+Del)"
      >
        ⏪ Ripple Delete
      </button>
      <button
        class="action-btn"
        disabled={!selectedClipId}
        on:click={removeSelectedClip}
        title="Delete Clip (Del)"
      >
        🗑 Delete
      </button>
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
                on:click={() => (track.muted = !track.muted)}
                title="Mute Track"
              >
                M
              </button>
              <button
                class="state-btn"
                class:active={track.solo}
                on:click={() => (track.solo = !track.solo)}
                title="Solo Track"
              >
                S
              </button>
              <button
                class="state-btn"
                class:active={track.locked}
                on:click={() => (track.locked = !track.locked)}
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
        <div class="ruler" on:click={handleRulerClick} role="region" aria-label="Timeline Ruler">
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
                  <!-- svelte-ignore a11y-click-events-have-key-events -->
                  <div
                    class="clip-box"
                    class:selected={selectedClipId === clip.id}
                    class:video-clip={track.track_type === 'Video'}
                    class:audio-clip={track.track_type === 'Audio'}
                    class:sub-clip={track.track_type === 'Subtitle'}
                    style={`left: ${(clip.timeline_in_ms / 1000) * zoomPxPerSec}px; width: ${Math.max(20, ((clip.timeline_out_ms - clip.timeline_in_ms) / 1000) * zoomPxPerSec)}px;`}
                    on:click={(e) => selectClip(clip, e)}
                  >
                    <span class="clip-name">{clip.name}</span>
                    <span class="clip-dur">{((clip.timeline_out_ms - clip.timeline_in_ms) / 1000).toFixed(1)}s</span>
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
    gap: 0.4rem;
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

