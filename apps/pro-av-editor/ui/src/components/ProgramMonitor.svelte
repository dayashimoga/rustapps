<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { formatTimecode, tauriInvoke, type VideoScopeData } from '../types';

  export let playheadMs: number = 0;
  export let durationMs: number = 5000;
  export let fps: number = 60.0;

  let isPlaying: boolean = false;
  let playInterval: ReturnType<typeof setInterval> | null = null;
  let previewBmpBase64: string | null = null;
  let showMiniScopes: boolean = false;
  let scopesData: VideoScopeData | null = null;

  $: timecodeStr = formatTimecode(playheadMs, fps);

  onMount(async () => {
    await updateFrame();
  });

  onDestroy(() => {
    stopPlayback();
  });

  export async function seekTo(ms: number) {
    playheadMs = Math.max(0, Math.min(ms, durationMs));
    await updateFrame();
  }

  export async function updateFrame() {
    try {
      const res = await tauriInvoke<string>('render_preview_frame_cmd', {
        timestampMs: playheadMs,
        width: 640,
        height: 360,
      });
      if (res) {
        previewBmpBase64 = res;
      }
      if (showMiniScopes) {
        scopesData = await tauriInvoke<VideoScopeData>('get_video_scopes_cmd', {
          timestampMs: playheadMs,
          width: 160,
          height: 90,
        });
      }
    } catch (e) {
      console.error('Frame render failed:', e);
    }
  }

  function togglePlayback() {
    if (isPlaying) {
      stopPlayback();
    } else {
      startPlayback();
    }
  }

  function startPlayback() {
    isPlaying = true;
    const intervalMs = Math.round(1000 / fps);
    playInterval = setInterval(async () => {
      playheadMs += intervalMs;
      if (playheadMs >= durationMs) {
        playheadMs = 0;
      }
      await updateFrame();
    }, intervalMs);
  }

  function stopPlayback() {
    isPlaying = false;
    if (playInterval) {
      clearInterval(playInterval);
      playInterval = null;
    }
  }

  function stepFrames(deltaFrames: number) {
    stopPlayback();
    const frameMs = 1000 / fps;
    seekTo(playheadMs + Math.round(deltaFrames * frameMs));
  }
</script>

<div class="monitor-container">
  <!-- Viewport Canvas Area -->
  <div class="viewport">
    {#if previewBmpBase64}
      <img
        src={`data:image/bmp;base64,${previewBmpBase64}`}
        alt="Program Canvas"
        class="frame-image"
      />
    {:else}
      <div class="black-canvas">
        <span class="loading-tag">Awaiting Frame...</span>
      </div>
    {/if}

    <!-- Overlay Timecode HUD -->
    <div class="timecode-hud">
      <span class="tc-val">{timecodeStr}</span>
    </div>

    <!-- Mini RGB Scopes Overlay -->
    {#if showMiniScopes && scopesData}
      <div class="mini-scopes-overlay">
        <span class="scope-title">RGB Waveform</span>
        <div class="mini-histogram">
          {#each scopesData.histogram_luma.slice(0, 32) as val, i}
            <div class="hist-bar" style={`height: ${Math.min(val * 4, 30)}px;`}></div>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <!-- Transport Controls Bar -->
  <div class="transport-bar">
    <div class="transport-left">
      <button class="transport-btn" on:click={() => seekTo(0)} title="Go to Start">⏮</button>
      <button class="transport-btn" on:click={() => stepFrames(-1)} title="Previous Frame (Left Arrow)">◀</button>
      <button class="transport-btn play-btn" on:click={togglePlayback} title="Play/Pause (Space)">
        {isPlaying ? '⏸' : '▶'}
      </button>
      <button class="transport-btn" on:click={() => stepFrames(1)} title="Next Frame (Right Arrow)">▶</button>
      <button class="transport-btn" on:click={() => seekTo(durationMs)} title="Go to End">⏭</button>
    </div>

    <div class="transport-center">
      <span class="smpte-display">{timecodeStr}</span>
      <span class="duration-display">/ {formatTimecode(durationMs, fps)}</span>
    </div>

    <div class="transport-right">
      <button
        class="tool-btn"
        class:active={showMiniScopes}
        on:click={() => { showMiniScopes = !showMiniScopes; updateFrame(); }}
      >
        📊 Scopes HUD
      </button>
    </div>
  </div>
</div>

<style>
  .monitor-container {
    display: flex;
    flex-direction: column;
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .viewport {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    background: #000;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .frame-image {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }

  .black-canvas {
    color: var(--text-faint);
    font-size: 0.8rem;
  }

  .timecode-hud {
    position: absolute;
    top: 8px;
    right: 8px;
    background: rgba(0, 0, 0, 0.7);
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 0.85rem;
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.3);
  }

  .mini-scopes-overlay {
    position: absolute;
    bottom: 8px;
    left: 8px;
    background: rgba(0, 0, 0, 0.8);
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .scope-title {
    font-size: 0.65rem;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .mini-histogram {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 30px;
    margin-top: 4px;
  }

  .hist-bar {
    width: 3px;
    background: var(--accent-emerald);
    border-radius: 1px;
  }

  .transport-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 1rem;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-subtle);
  }

  .transport-left {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .transport-btn {
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    transition: all 0.15s;
  }

  .transport-btn:hover {
    background: var(--bg-panel-hover);
    border-color: var(--accent-primary);
  }

  .play-btn {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
    color: white;
  }

  .transport-center {
    display: flex;
    align-items: baseline;
    gap: 0.35rem;
  }

  .smpte-display {
    font-family: var(--font-mono);
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-main);
  }

  .duration-display {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--text-faint);
  }

  .tool-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    padding: 0.3rem 0.65rem;
    border-radius: var(--radius-sm);
    font-size: 0.75rem;
  }

  .tool-btn.active {
    background: rgba(99, 102, 241, 0.2);
    color: #a5b4fc;
    border-color: var(--accent-primary);
  }
</style>

