<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { CursorEvent, SubtitleCue, TutorialStep, ZoomKeyframe } from '../App.svelte';

  export let durationMs: number = 10_000;
  export let currentTimeMs: number = 0;
  export let cursorEvents: CursorEvent[] = [];
  export let zoomKeyframes: ZoomKeyframe[] = [];
  export let subtitles: SubtitleCue[] = [];
  export let silenceTrimmedMs: number = 0;

  const dispatch = createEventDispatcher<{
    seek: number;
  }>();

  function handleTimelineClick(e: MouseEvent) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    dispatch('seek', Math.floor(pct * durationMs));
  }
</script>

<div class="timeline-container">
  <div class="timeline-header">
    <div class="track-labels">
      <span class="label">Ruler</span>
      <span class="label">Clicks</span>
      <span class="label">Auto-Zoom</span>
      <span class="label">Subtitles</span>
    </div>

    <!-- Main timeline canvas area -->
    <div
      class="tracks-area"
      on:click={handleTimelineClick}
      role="slider"
      tabindex="0"
      aria-valuenow={currentTimeMs}
      aria-valuemin={0}
      aria-valuemax={durationMs}
    >
      <!-- Playhead bar -->
      <div
        class="playhead"
        style="left: {(currentTimeMs / (durationMs || 1)) * 100}%;"
      >
        <div class="playhead-tag"></div>
        <div class="playhead-line"></div>
      </div>

      <!-- Track 1: Time Ruler -->
      <div class="track ruler-track">
        {#each Array(Math.ceil((durationMs || 1) / 2000)) as _, i}
          <div class="ruler-tick" style="left: {((i * 2000) / (durationMs || 1)) * 100}%;">
            <span>{i * 2}s</span>
          </div>
        {/each}
      </div>

      <!-- Track 2: Mouse Clicks -->
      <div class="track click-track">
        {#each cursorEvents.filter((e) => e.click !== 'None') as event}
          <div
            class="click-dot click-{event.click.toLowerCase()}"
            style="left: {(event.timestamp_ms / (durationMs || 1)) * 100}%;"
            title="{event.click} click at {event.timestamp_ms}ms"
          ></div>
        {/each}
      </div>

      <!-- Track 3: Camera Auto-Zoom Intervals -->
      <div class="track zoom-track">
        {#each zoomKeyframes as zoom}
          <div
            class="zoom-block"
            style="
              left: {(zoom.start_ms / (durationMs || 1)) * 100}%;
              width: {((zoom.end_ms - zoom.start_ms) / (durationMs || 1)) * 100}%;
            "
            title="{zoom.label} ({zoom.scale}x)"
          >
            <span>🔍 {zoom.label}</span>
          </div>
        {/each}
      </div>

      <!-- Track 4: Subtitles & Audio -->
      <div class="track subs-track">
        {#each subtitles as sub}
          <div
            class="sub-block"
            style="
              left: {(sub.start_ms / (durationMs || 1)) * 100}%;
              width: {((sub.end_ms - sub.start_ms) / (durationMs || 1)) * 100}%;
            "
            title="{sub.text}"
          >
            <span>{sub.text}</span>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <div class="timeline-footer">
    <div class="legend">
      <div class="legend-item"><span class="dot left"></span> Left Click</div>
      <div class="legend-item"><span class="dot right"></span> Right Click</div>
      <div class="legend-item"><span class="dot double"></span> Double Click</div>
      <div class="legend-item"><span class="box zoom"></span> Camera Zoom</div>
      <div class="legend-item"><span class="box sub"></span> Subtitle Cue</div>
    </div>

    <div class="deadair-stat">
      <span>🔇 Dead-Air Detected: <strong>{(silenceTrimmedMs / 1000).toFixed(1)}s trimmed</strong></span>
    </div>
  </div>
</div>

<style>
  .timeline-container {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .timeline-header {
    display: flex;
    position: relative;
    background: #0d1322;
  }

  .track-labels {
    width: 100px;
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    flex-shrink: 0;
  }

  .track-labels .label {
    height: 32px;
    display: flex;
    align-items: center;
    padding-left: 12px;
    font-size: 0.72rem;
    color: var(--text-sub);
    text-transform: uppercase;
    font-weight: 600;
    border-bottom: 1px solid var(--border);
  }

  .tracks-area {
    flex: 1;
    position: relative;
    display: flex;
    flex-direction: column;
    cursor: pointer;
    overflow: hidden;
  }

  .playhead {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--accent-cyan);
    z-index: 50;
    pointer-events: none;
    transform: translateX(-50%);
  }

  .playhead-tag {
    position: absolute;
    top: 0;
    left: -5px;
    width: 12px;
    height: 10px;
    background: var(--accent-cyan);
    clip-path: polygon(0 0, 100% 0, 50% 100%);
  }

  .playhead-line {
    width: 2px;
    height: 100%;
    background: var(--accent-cyan);
    box-shadow: 0 0 6px rgba(56, 189, 248, 0.8);
  }

  .track {
    height: 32px;
    position: relative;
    border-bottom: 1px solid rgba(35, 49, 77, 0.6);
  }

  .ruler-track {
    background: #090e1a;
  }

  .ruler-tick {
    position: absolute;
    top: 0;
    bottom: 0;
    border-left: 1px solid var(--border);
    padding-left: 4px;
  }

  .ruler-tick span {
    font-size: 0.68rem;
    color: var(--text-sub);
    font-family: var(--font-mono);
  }

  .click-track {
    background: rgba(15, 23, 42, 0.4);
  }

  .click-dot {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }

  .click-left {
    background: var(--accent-cyan);
    box-shadow: 0 0 6px var(--accent-cyan);
  }

  .click-right {
    background: var(--accent-orange);
    box-shadow: 0 0 6px var(--accent-orange);
  }

  .click-double {
    background: var(--accent-purple);
    box-shadow: 0 0 6px var(--accent-purple);
  }

  .zoom-track {
    background: rgba(15, 23, 42, 0.6);
  }

  .zoom-block {
    position: absolute;
    top: 4px;
    bottom: 4px;
    background: rgba(56, 189, 248, 0.25);
    border: 1px solid var(--accent-cyan);
    border-radius: 4px;
    display: flex;
    align-items: center;
    padding: 0 6px;
    overflow: hidden;
    white-space: nowrap;
  }

  .zoom-block span {
    font-size: 0.68rem;
    color: #e0f2fe;
    font-weight: 600;
  }

  .subs-track {
    background: rgba(15, 23, 42, 0.8);
  }

  .sub-block {
    position: absolute;
    top: 4px;
    bottom: 4px;
    background: rgba(99, 102, 241, 0.25);
    border: 1px solid var(--accent-indigo);
    border-radius: 4px;
    display: flex;
    align-items: center;
    padding: 0 6px;
    overflow: hidden;
    white-space: nowrap;
  }

  .sub-block span {
    font-size: 0.68rem;
    color: #e0e7ff;
    text-overflow: ellipsis;
    overflow: hidden;
  }

  .timeline-footer {
    padding: 8px 16px;
    background: var(--bg-secondary);
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.75rem;
  }

  .legend {
    display: flex;
    gap: 14px;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .dot.left { background: var(--accent-cyan); }
  .dot.right { background: var(--accent-orange); }
  .dot.double { background: var(--accent-purple); }

  .box {
    width: 12px;
    height: 8px;
    border-radius: 2px;
  }

  .box.zoom { background: var(--accent-cyan); }
  .box.sub { background: var(--accent-indigo); }

  .deadair-stat {
    color: var(--accent-orange);
  }
</style>
