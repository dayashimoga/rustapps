<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import type { CursorEvent, ZoomKeyframe } from '../App.svelte';

  export let currentTimeMs: number = 0;
  export let durationMs: number = 10_000;
  export let isPlaying: boolean = false;
  export let zoomKeyframes: ZoomKeyframe[] = [];
  export let cursorEvents: CursorEvent[] = [];
  export let currentZoomScale: number = 1.45;

  const dispatch = createEventDispatcher<{
    seek: number;
    togglePlay: void;
    scaleChange: number;
  }>();

  let canvas: HTMLCanvasElement;

  function formatTime(ms: number): string {
    const totalSecs = Math.floor(ms / 1000);
    const mins = Math.floor(totalSecs / 60);
    const secs = totalSecs % 60;
    const millis = Math.floor((ms % 1000) / 100);
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}.${millis}`;
  }

  // Active zoom keyframe at currentTimeMs
  $: activeZoom = zoomKeyframes.find(
    (z) => currentTimeMs >= z.start_ms && currentTimeMs <= z.end_ms
  );

  // Active cursor event nearest to currentTimeMs
  $: activeCursor = cursorEvents.reduce((prev, curr) => {
    return Math.abs(curr.timestamp_ms - currentTimeMs) < Math.abs(prev.timestamp_ms - currentTimeMs)
      ? curr
      : prev;
  }, cursorEvents[0] || { timestamp_ms: 0, x: 0.5, y: 0.5, click: 'None' });

  onMount(() => {
    drawFrame();
  });

  $: if (canvas && (currentTimeMs || activeZoom || activeCursor)) {
    drawFrame();
  }

  function drawFrame() {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;

    // Background simulated desktop
    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, w, h);

    // Simulated UI application window
    ctx.fillStyle = '#1e293b';
    ctx.strokeStyle = '#334155';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.roundRect(40, 30, w - 80, h - 60, 8);
    ctx.fill();
    ctx.stroke();

    // Top window header
    ctx.fillStyle = '#0f172a';
    ctx.fillRect(40, 30, w - 80, 30);
    ctx.fillStyle = '#ef4444';
    ctx.beginPath();
    ctx.arc(58, 45, 5, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = '#f59e0b';
    ctx.beginPath();
    ctx.arc(74, 45, 5, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = '#10b981';
    ctx.beginPath();
    ctx.arc(90, 45, 5, 0, Math.PI * 2);
    ctx.fill();

    // Simulated UI Cards inside window
    ctx.fillStyle = '#334155';
    ctx.fillRect(60, 80, 180, 120);
    ctx.fillRect(260, 80, 240, 120);
    ctx.fillRect(520, 80, 180, 120);
    ctx.fillRect(60, 220, 440, 140);
    ctx.fillRect(520, 220, 180, 140);

    // If active zoom, draw dynamic zoom boundary box
    if (activeZoom) {
      const zx = activeZoom.focus_x * w;
      const zy = activeZoom.focus_y * h;
      const boxW = w / activeZoom.scale;
      const boxH = h / activeZoom.scale;

      ctx.strokeStyle = '#38bdf8';
      ctx.lineWidth = 2;
      ctx.setLineDash([6, 6]);
      ctx.strokeRect(zx - boxW / 2, zy - boxH / 2, boxW, boxH);
      ctx.setLineDash([]);

      ctx.fillStyle = 'rgba(56, 189, 248, 0.9)';
      ctx.font = 'bold 11px sans-serif';
      ctx.fillText(`CAM AUTO-ZOOM (${activeZoom.scale.toFixed(2)}x)`, zx - boxW / 2 + 6, zy - boxH / 2 - 6);
    }

    // Draw mouse cursor
    if (activeCursor) {
      const cx = activeCursor.x * w;
      const cy = activeCursor.y * h;

      // Cursor arrow
      ctx.fillStyle = '#ffffff';
      ctx.strokeStyle = '#000000';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.lineTo(cx, cy + 18);
      ctx.lineTo(cx + 4, cy + 14);
      ctx.lineTo(cx + 8, cy + 22);
      ctx.lineTo(cx + 12, cy + 20);
      ctx.lineTo(cx + 7, cy + 12);
      ctx.lineTo(cx + 13, cy + 12);
      ctx.closePath();
      ctx.fill();
      ctx.stroke();

      // Click ripple effect
      if (activeCursor.click && activeCursor.click !== 'None') {
        ctx.strokeStyle = activeCursor.click === 'Double' ? '#a855f7' : '#38bdf8';
        ctx.lineWidth = 2.5;
        ctx.beginPath();
        ctx.arc(cx, cy, 18, 0, Math.PI * 2);
        ctx.stroke();
      }
    }
  }

  function handleSeek(e: MouseEvent) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    dispatch('seek', Math.floor(pct * durationMs));
  }
</script>

<div class="player-container">
  <div class="viewport-wrap">
    <canvas bind:this={canvas} width="760" height="428"></canvas>

    {#if activeZoom}
      <div class="zoom-active-pill">
        <span>🔍 {activeZoom.label} ({activeZoom.scale}x)</span>
      </div>
    {/if}
  </div>

  <div class="player-controls">
    <div class="transport-left">
      <button class="play-btn" on:click={() => dispatch('togglePlay')}>
        {#if isPlaying}
          ⏸ Pause
        {:else}
          ▶ Play Walkthrough
        {/if}
      </button>

      <span class="timecode">
        {formatTime(currentTimeMs)} / {formatTime(durationMs)}
      </span>
    </div>

    <!-- Scrubber bar -->
    <div
      class="scrubber"
      on:click={handleSeek}
      role="slider"
      tabindex="0"
      aria-valuenow={currentTimeMs}
      aria-valuemin={0}
      aria-valuemax={durationMs}
    >
      <div class="scrubber-track">
        <div class="scrubber-fill" style="width: {(currentTimeMs / (durationMs || 1)) * 100}%;"></div>
        <div class="scrubber-head" style="left: {(currentTimeMs / (durationMs || 1)) * 100}%;"></div>
      </div>
    </div>

    <div class="transport-right">
      <label class="scale-label">
        <span>Zoom Scale:</span>
        <input
          type="range"
          min="1.2"
          max="2.0"
          step="0.05"
          value={currentZoomScale}
          on:input={(e) => dispatch('scaleChange', parseFloat(e.currentTarget.value))}
        />
        <span class="scale-val">{currentZoomScale.toFixed(2)}x</span>
      </label>
    </div>
  </div>
</div>

<style>
  .player-container {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .viewport-wrap {
    position: relative;
    background: #000;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  canvas {
    width: 100%;
    max-height: 440px;
    object-fit: contain;
    display: block;
  }

  .zoom-active-pill {
    position: absolute;
    top: 14px;
    right: 14px;
    background: rgba(56, 189, 248, 0.2);
    border: 1px solid var(--accent-cyan);
    color: var(--accent-cyan);
    padding: 4px 10px;
    border-radius: 20px;
    font-size: 0.75rem;
    font-weight: 700;
    backdrop-filter: blur(4px);
  }

  .player-controls {
    padding: 10px 16px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .transport-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .play-btn {
    background: var(--accent-blue);
    color: #fff;
    border: none;
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 600;
  }

  .play-btn:hover {
    background: #1d4ed8;
  }

  .timecode {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .scrubber {
    flex: 1;
    height: 24px;
    display: flex;
    align-items: center;
    cursor: pointer;
  }

  .scrubber-track {
    position: relative;
    width: 100%;
    height: 6px;
    background: var(--border);
    border-radius: 3px;
  }

  .scrubber-fill {
    height: 100%;
    background: var(--accent-cyan);
    border-radius: 3px;
  }

  .scrubber-head {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 14px;
    height: 14px;
    background: #fff;
    border: 2px solid var(--accent-cyan);
    border-radius: 50%;
  }

  .transport-right {
    display: flex;
    align-items: center;
  }

  .scale-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.78rem;
    color: var(--text-sub);
  }

  .scale-label input {
    width: 80px;
  }

  .scale-val {
    font-family: var(--font-mono);
    color: var(--text-main);
    font-weight: 600;
  }
</style>
