<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import {
    formatTimecode,
    getMediaFileUrl,
    tauriInvoke,
    type Clip,
    type MediaSource,
    type TimelineProject,
    type VideoScopeData,
  } from '../types';

  export let project: TimelineProject | null = null;
  export let playheadMs: number = 0;
  export let durationMs: number = 5000;
  export let fps: number = 60.0;

  const dispatch = createEventDispatcher<{
    timeupdate: number;
  }>();

  let isPlaying: boolean = false;
  let playbackSpeed: number = 1.0; // 1x, 2x, 4x, -1x, etc. (J-K-L shuttle)
  let animFrameId: number | null = null;
  let lastFrameTime: number = 0;
  let isRenderingFrame: boolean = false;
  let pendingRenderMs: number | null = null;

  let previewBmpBase64: string | null = null;
  let showMiniScopes: boolean = false;
  let showSafeAreas: boolean = false;
  let showCenterCross: boolean = false;
  let zoomLevel: 'fit' | '100%' | '200%' = 'fit';
  let scopesData: VideoScopeData | null = null;
  let activeSourceLabel: string = '';

  let viewportContainerRef: HTMLDivElement | null = null;
  let canvasRef: HTMLCanvasElement | null = null;
  let videoEl: HTMLVideoElement | null = null;
  let audioEl: HTMLAudioElement | null = null;
  let loadedVideoSrcUrl: string = '';
  let loadedAudioSrcUrl: string = '';
  let loadedImageCache: Record<string, HTMLImageElement> = {};

  $: timecodeStr = formatTimecode(playheadMs, fps);

  function resolveMediaSource(sourceId?: string | null): MediaSource | null {
    if (!sourceId || !project?.media_pool) return null;
    return (
      project.media_pool.find(
        (m) => m.id === sourceId || m.file_path === sourceId
      ) || null
    );
  }

  function findActiveClip(
    trackType: 'Video' | 'Audio'
  ): { clip: Clip; source: MediaSource } | null {
    if (!project) return null;
    for (const track of project.tracks) {
      if (track.track_type !== trackType || track.muted) continue;
      for (const clip of track.clips) {
        if (
          playheadMs >= clip.timeline_in_ms &&
          playheadMs < clip.timeline_out_ms
        ) {
          const source = resolveMediaSource(clip.source_id);
          if (source && source.file_path) {
            return { clip, source };
          }
        }
      }
    }
    return null;
  }

  function isHtmlVideoOrImage(filePath: string): 'video' | 'image' | null {
    const ext = (filePath.split('.').pop() || '').toLowerCase();
    if (['mp4', 'webm', 'mov', 'mkv', 'avi', 'm4v'].includes(ext)) {
      return 'video';
    }
    if (['jpg', 'jpeg', 'webp', 'gif', 'png', 'bmp'].includes(ext)) {
      return 'image';
    }
    return null;
  }

  function applyGradeFilterString(clip: Clip): string {
    const cg = clip.color_grading;
    if (!cg) return 'none';
    const brightness = Math.max(0.2, Math.min(3.0, Math.pow(2.0, cg.exposure || 0)));
    const contrast = Math.max(0.2, Math.min(3.0, cg.contrast ?? 1.0));
    const saturate = Math.max(0.0, Math.min(3.0, cg.saturation ?? 1.0));
    const hueRotate = Math.round(((cg.temperature || 0) * -0.25) + ((cg.tint || 0) * 0.25));
    return `brightness(${brightness.toFixed(2)}) contrast(${contrast.toFixed(2)}) saturate(${saturate.toFixed(2)}) hue-rotate(${hueRotate}deg)`;
  }

  function syncAudioPlayback() {
    if (!audioEl) return;
    const activeAudio = findActiveClip('Audio') || findActiveClip('Video');
    if (!activeAudio) {
      if (!audioEl.paused) {
        audioEl.pause();
      }
      return;
    }
    const mediaUrl = getMediaFileUrl(activeAudio.source.file_path);
    if (loadedAudioSrcUrl !== mediaUrl) {
      loadedAudioSrcUrl = mediaUrl;
      audioEl.src = mediaUrl;
    }
    const clipOffsetSec =
      ((playheadMs - activeAudio.clip.timeline_in_ms) *
        (activeAudio.clip.speed_multiplier || 1.0) +
        activeAudio.clip.source_in_ms) /
      1000.0;
    const vol = activeAudio.clip.volume?.static_value ?? 1.0;
    audioEl.volume = Math.max(0, Math.min(1, vol));
    audioEl.playbackRate = Math.max(0.25, Math.min(4.0, Math.abs(playbackSpeed)));

    if (isPlaying && playbackSpeed > 0) {
      if (Math.abs(audioEl.currentTime - clipOffsetSec) > 0.25) {
        audioEl.currentTime = Math.max(0, clipOffsetSec);
      }
      if (audioEl.paused) {
        audioEl.play().catch(() => {});
      }
    } else {
      if (!audioEl.paused) {
        audioEl.pause();
      }
    }
  }

  onMount(async () => {
    window.addEventListener('keydown', handleKeydown);
    await updateFrame();
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
    stopPlayback();
  });

  function handleKeydown(e: KeyboardEvent) {
    // Ignore keystrokes when typing inside text inputs / textareas
    const active = document.activeElement;
    if (active && (active.tagName === 'INPUT' || active.tagName === 'TEXTAREA')) {
      return;
    }

    if (e.code === 'Space') {
      e.preventDefault();
      togglePlayback();
    } else if (e.code === 'KeyK') {
      e.preventDefault();
      stopPlayback();
    } else if (e.code === 'KeyL') {
      e.preventDefault();
      cycleShuttleForward();
    } else if (e.code === 'KeyJ') {
      e.preventDefault();
      cycleShuttleReverse();
    } else if (e.code === 'ArrowLeft') {
      e.preventDefault();
      stepFrames(-1);
    } else if (e.code === 'ArrowRight') {
      e.preventDefault();
      stepFrames(1);
    } else if (e.code === 'Home') {
      e.preventDefault();
      seekTo(0);
    } else if (e.code === 'End') {
      e.preventDefault();
      seekTo(durationMs);
    }
  }

  function cycleShuttleForward() {
    if (!isPlaying || playbackSpeed < 1.0) {
      startPlayback(1.0);
    } else if (playbackSpeed === 1.0) {
      playbackSpeed = 2.0;
    } else if (playbackSpeed === 2.0) {
      playbackSpeed = 4.0;
    } else if (playbackSpeed === 4.0) {
      playbackSpeed = 8.0;
    } else {
      playbackSpeed = 1.0;
    }
  }

  function cycleShuttleReverse() {
    if (!isPlaying || playbackSpeed > -1.0) {
      startPlayback(-1.0);
    } else if (playbackSpeed === -1.0) {
      playbackSpeed = -2.0;
    } else if (playbackSpeed === -2.0) {
      playbackSpeed = -4.0;
    } else if (playbackSpeed === -4.0) {
      playbackSpeed = -8.0;
    } else {
      playbackSpeed = -1.0;
    }
  }

  export async function seekTo(ms: number) {
    playheadMs = Math.max(0, Math.min(ms, durationMs));
    dispatch('timeupdate', playheadMs);
    syncAudioPlayback();
    await updateFrame();
  }

  function drawHudOverlays(ctx: CanvasRenderingContext2D, w: number, h: number) {
    // Safe Areas Overlay
    if (showSafeAreas) {
      const ax = w * 0.05;
      const ay = h * 0.05;
      const aw = w * 0.9;
      const ah = h * 0.9;
      ctx.strokeStyle = 'rgba(56, 189, 248, 0.7)';
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 4]);
      ctx.strokeRect(ax, ay, aw, ah);

      const tx = w * 0.1;
      const ty = h * 0.1;
      const tw = w * 0.8;
      const th = h * 0.8;
      ctx.strokeStyle = 'rgba(251, 191, 36, 0.7)';
      ctx.strokeRect(tx, ty, tw, th);
      ctx.setLineDash([]);
    }

    // Center Crosshair
    if (showCenterCross) {
      const cx = w / 2;
      const cy = h / 2;
      const len = 16;
      ctx.strokeStyle = 'rgba(239, 68, 68, 0.8)';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(cx - len, cy);
      ctx.lineTo(cx + len, cy);
      ctx.moveTo(cx, cy - len);
      ctx.lineTo(cx, cy + len);
      ctx.stroke();
    }
  }

  async function tryRenderLiveHtmlMedia(): Promise<boolean> {
    const activeVideo = findActiveClip('Video');
    if (!activeVideo || !canvasRef) {
      activeSourceLabel = '';
      if (videoEl && !videoEl.paused) {
        videoEl.pause();
      }
      return false;
    }

    const kind = isHtmlVideoOrImage(activeVideo.source.file_path);
    if (!kind) return false;

    const mediaUrl = getMediaFileUrl(activeVideo.source.file_path);
    activeSourceLabel = activeVideo.source.name;
    const ctx = canvasRef.getContext('2d');
    if (!ctx) return false;

    if (kind === 'video' && videoEl) {
      if (loadedVideoSrcUrl !== mediaUrl) {
        loadedVideoSrcUrl = mediaUrl;
        videoEl.src = mediaUrl;
      }
      const clipOffsetSec =
        ((playheadMs - activeVideo.clip.timeline_in_ms) *
          (activeVideo.clip.speed_multiplier || 1.0) +
          activeVideo.clip.source_in_ms) /
        1000.0;

      if (isPlaying && playbackSpeed > 0) {
        videoEl.playbackRate = Math.max(0.25, Math.min(4.0, playbackSpeed));
        if (Math.abs(videoEl.currentTime - clipOffsetSec) > 0.25) {
          videoEl.currentTime = Math.max(0, clipOffsetSec);
        }
        if (videoEl.paused) {
          videoEl.play().catch(() => {});
        }
      } else {
        if (!videoEl.paused) {
          videoEl.pause();
        }
        if (Math.abs(videoEl.currentTime - clipOffsetSec) > 0.04) {
          videoEl.currentTime = Math.max(0, clipOffsetSec);
        }
      }

      if (videoEl.readyState >= 2) {
        canvasRef.width = 640;
        canvasRef.height = 360;
        ctx.save();
        ctx.fillStyle = '#000';
        ctx.fillRect(0, 0, 640, 360);
        ctx.filter = applyGradeFilterString(activeVideo.clip);
        ctx.globalAlpha = activeVideo.clip.opacity?.static_value ?? 1.0;
        ctx.drawImage(videoEl, 0, 0, 640, 360);
        ctx.restore();
        drawHudOverlays(ctx, 640, 360);
        previewBmpBase64 = 'LIVE_VIDEO';
        return true;
      }
    } else if (kind === 'image') {
      // For JPG/WEBP/GIF (or fast cached PNG/BMP), draw via HTMLImageElement if cached
      const ext = (activeVideo.source.file_path.split('.').pop() || '').toLowerCase();
      if (['jpg', 'jpeg', 'webp', 'gif'].includes(ext)) {
        let img = loadedImageCache[mediaUrl];
        if (!img) {
          img = new Image();
          img.crossOrigin = 'anonymous';
          img.src = mediaUrl;
          loadedImageCache[mediaUrl] = img;
        }
        if (img.complete && img.naturalWidth > 0) {
          canvasRef.width = 640;
          canvasRef.height = 360;
          ctx.save();
          ctx.fillStyle = '#000';
          ctx.fillRect(0, 0, 640, 360);
          ctx.filter = applyGradeFilterString(activeVideo.clip);
          ctx.globalAlpha = activeVideo.clip.opacity?.static_value ?? 1.0;
          ctx.drawImage(img, 0, 0, 640, 360);
          ctx.restore();
          drawHudOverlays(ctx, 640, 360);
          previewBmpBase64 = 'LIVE_IMAGE';
          return true;
        }
      }
    }

    return false;
  }

  export async function updateFrame() {
    if (isRenderingFrame) {
      pendingRenderMs = playheadMs;
      return;
    }
    isRenderingFrame = true;

    try {
      syncAudioPlayback();
      const handledByHtmlMedia = await tryRenderLiveHtmlMedia();

      if (!handledByHtmlMedia) {
        const res = await tauriInvoke<string>('render_preview_frame_cmd', {
          timestampMs: playheadMs,
          width: 640,
          height: 360,
        });

        if (res) {
          previewBmpBase64 = res;
          renderCanvasFrame(res);
        }
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
    } finally {
      isRenderingFrame = false;
      if (pendingRenderMs !== null) {
        const nextMs = pendingRenderMs;
        pendingRenderMs = null;
        if (nextMs !== playheadMs) {
          playheadMs = nextMs;
          await updateFrame();
        }
      }
    }
  }

  function renderCanvasFrame(base64Bmp: string) {
    if (!canvasRef || base64Bmp === 'LIVE_VIDEO' || base64Bmp === 'LIVE_IMAGE') return;
    const img = new Image();
    img.onload = () => {
      if (!canvasRef) return;
      const ctx = canvasRef.getContext('2d');
      if (!ctx) return;

      canvasRef.width = img.width || 640;
      canvasRef.height = img.height || 360;
      ctx.drawImage(img, 0, 0);

      drawHudOverlays(ctx, canvasRef.width, canvasRef.height);
    };
    img.src = `data:image/bmp;base64,${base64Bmp}`;
  }

  function togglePlayback() {
    if (isPlaying) {
      stopPlayback();
    } else {
      startPlayback(1.0);
    }
  }

  function startPlayback(speed: number = 1.0) {
    playbackSpeed = speed;
    if (!isPlaying) {
      isPlaying = true;
      lastFrameTime = performance.now();
      animFrameId = requestAnimationFrame(animationLoop);
    }
  }

  function stopPlayback() {
    isPlaying = false;
    playbackSpeed = 1.0;
    if (videoEl && !videoEl.paused) {
      videoEl.pause();
    }
    if (audioEl && !audioEl.paused) {
      audioEl.pause();
    }
    if (animFrameId !== null) {
      cancelAnimationFrame(animFrameId);
      animFrameId = null;
    }
  }

  async function animationLoop(now: number) {
    if (!isPlaying) return;

    const elapsed = now - lastFrameTime;
    const targetInterval = 1000 / fps;

    if (elapsed >= targetInterval) {
      lastFrameTime = now;
      const stepMs = (targetInterval * playbackSpeed);
      let nextMs = playheadMs + stepMs;

      if (nextMs >= durationMs) {
        nextMs = 0;
      } else if (nextMs < 0) {
        nextMs = durationMs;
      }

      playheadMs = Math.round(nextMs);
      dispatch('timeupdate', playheadMs);
      await updateFrame();
    }

    if (isPlaying) {
      animFrameId = requestAnimationFrame(animationLoop);
    }
  }

  function stepFrames(deltaFrames: number) {
    stopPlayback();
    const frameMs = 1000 / fps;
    seekTo(playheadMs + Math.round(deltaFrames * frameMs));
  }

  function toggleFullscreen() {
    if (!viewportContainerRef) return;
    if (document.fullscreenElement) {
      document.exitFullscreen().catch(console.error);
    } else {
      viewportContainerRef.requestFullscreen().catch(console.error);
    }
  }
</script>

<div class="monitor-container" bind:this={viewportContainerRef}>
  <!-- Top Monitor HUD Bar -->
  <div class="monitor-top-bar">
    <div class="hud-left">
      <span class="program-title">Program Monitor</span>
      <span class="res-tag">1920×1080 @ {Math.round(fps)}fps</span>
      {#if activeSourceLabel}
        <span class="res-tag" style="color: #38bdf8; border: 1px solid rgba(56,189,248,0.3);">
          🎬 {activeSourceLabel}
        </span>
      {/if}
      {#if isPlaying && playbackSpeed !== 1.0}
        <span class="speed-pill">
          {playbackSpeed > 0 ? `▶▶ ${playbackSpeed}x` : `◀◀ ${playbackSpeed}x`}
        </span>
      {/if}
    </div>

    <div class="hud-right">
      <div class="zoom-wrap">
        <label class="hud-label" for="zoom-select">Zoom:</label>
        <select id="zoom-select" bind:value={zoomLevel} class="hud-select">
          <option value="fit">Fit</option>
          <option value="100%">100%</option>
          <option value="200%">200%</option>
        </select>
      </div>

      <button
        type="button"
        class="hud-btn"
        class:active={showSafeAreas}
        on:click={() => { showSafeAreas = !showSafeAreas; if (previewBmpBase64) renderCanvasFrame(previewBmpBase64); }}
        title="Toggle Action & Title Safe Guides"
      >
        ⊞ Safe Guides
      </button>

      <button
        type="button"
        class="hud-btn"
        class:active={showCenterCross}
        on:click={() => { showCenterCross = !showCenterCross; if (previewBmpBase64) renderCanvasFrame(previewBmpBase64); }}
        title="Toggle Center Crosshair"
      >
        ✛ Center
      </button>

      <button
        type="button"
        class="hud-btn"
        class:active={showMiniScopes}
        on:click={() => { showMiniScopes = !showMiniScopes; updateFrame(); }}
        title="Toggle Video Scopes HUD"
      >
        📊 Scopes HUD
      </button>

      <button
        type="button"
        class="hud-btn"
        on:click={toggleFullscreen}
        title="Toggle Fullscreen Monitor (F)"
      >
        ⛶
      </button>
    </div>
  </div>

  <!-- Viewport Canvas Area -->
  <div class="viewport" class:zoom-fit={zoomLevel === 'fit'} class:zoom-100={zoomLevel === '100%'} class:zoom-200={zoomLevel === '200%'}>
    <!-- Hidden HTML5 Video & Audio Elements for Real Media Decoding -->
    <!-- svelte-ignore a11y-media-has-caption -->
    <video
      bind:this={videoEl}
      style="display: none;"
      preload="auto"
      playsinline
      muted
    ></video>
    <audio
      bind:this={audioEl}
      style="display: none;"
      preload="auto"
    ></audio>

    <canvas
      bind:this={canvasRef}
      class="program-canvas"
      width="640"
      height="360"
    ></canvas>

    {#if !previewBmpBase64}
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
          {#each scopesData.histogram_luma.slice(0, 32) as val}
            <div class="hist-bar" style={`height: ${Math.min(val * 4, 30)}px;`}></div>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <!-- Transport Controls Bar -->
  <div class="transport-bar">
    <div class="transport-left">
      <button type="button" class="transport-btn" on:click={() => seekTo(0)} title="Go to Start (Home)">⏮</button>
      <button type="button" class="transport-btn" on:click={() => stepFrames(-1)} title="Previous Frame (Left Arrow)">◀</button>
      <button
        type="button"
        class="transport-btn play-btn"
        on:click={togglePlayback}
        title="Play/Pause (Space / K)"
      >
        {isPlaying ? '⏸' : '▶'}
      </button>
      <button type="button" class="transport-btn" on:click={() => stepFrames(1)} title="Next Frame (Right Arrow)">▶</button>
      <button type="button" class="transport-btn" on:click={() => seekTo(durationMs)} title="Go to End (End)">⏭</button>
    </div>

    <div class="transport-center">
      <span class="smpte-display">{timecodeStr}</span>
      <span class="duration-display">/ {formatTimecode(durationMs, fps)}</span>
    </div>

    <div class="transport-right">
      <div class="shuttle-shortcuts" title="J-K-L Shuttle Keyboard Shortcuts">
        <span class="shortcut-pill">J: Rev</span>
        <span class="shortcut-pill">K: Pause</span>
        <span class="shortcut-pill">L: Fwd</span>
        <span class="shortcut-pill">Space: Play</span>
      </div>
    </div>
  </div>
</div>

<style>
  .monitor-container {
    display: flex;
    flex-direction: column;
    background: var(--bg-panel, #0f131a);
    border: 1px solid var(--border-subtle, #1f2937);
    border-radius: var(--radius-md, 6px);
    overflow: hidden;
    height: 100%;
  }

  .monitor-top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.35rem 0.75rem;
    background: #161b22;
    border-bottom: 1px solid #21262d;
    font-size: 0.75rem;
  }

  .hud-left {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .program-title {
    font-weight: 600;
    color: #e6edf3;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.7rem;
  }

  .res-tag {
    font-family: var(--font-mono, monospace);
    color: #8b949e;
    font-size: 0.7rem;
    background: rgba(255, 255, 255, 0.04);
    padding: 1px 5px;
    border-radius: 3px;
  }

  .speed-pill {
    font-family: var(--font-mono, monospace);
    background: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.3);
    padding: 1px 6px;
    border-radius: 4px;
    font-weight: 600;
  }

  .hud-right {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .zoom-wrap {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    margin-right: 0.25rem;
  }

  .hud-label {
    color: #8b949e;
    font-size: 0.7rem;
  }

  .hud-select {
    background: #21262d;
    border: 1px solid #30363d;
    color: #c9d1d9;
    padding: 2px 4px;
    border-radius: 3px;
    font-size: 0.7rem;
  }

  .hud-btn {
    background: #21262d;
    border: 1px solid #30363d;
    color: #8b949e;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 0.7rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .hud-btn:hover {
    color: #e6edf3;
    border-color: #58a6ff;
  }

  .hud-btn.active {
    background: rgba(56, 189, 248, 0.15);
    border-color: rgba(56, 189, 248, 0.4);
    color: #38bdf8;
    font-weight: 600;
  }

  .viewport {
    position: relative;
    width: 100%;
    flex: 1;
    background: #000;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .program-canvas {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    display: block;
  }

  .zoom-100 .program-canvas {
    max-width: none;
    max-height: none;
    width: 640px;
    height: 360px;
  }

  .zoom-200 .program-canvas {
    max-width: none;
    max-height: none;
    width: 1280px;
    height: 720px;
  }

  .black-canvas {
    position: absolute;
    color: #8b949e;
    font-size: 0.8rem;
  }

  .timecode-hud {
    position: absolute;
    top: 8px;
    right: 8px;
    background: rgba(0, 0, 0, 0.75);
    padding: 2px 8px;
    border-radius: 4px;
    font-family: var(--font-mono, monospace);
    font-size: 0.85rem;
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.3);
    pointer-events: none;
  }

  .mini-scopes-overlay {
    position: absolute;
    bottom: 8px;
    left: 8px;
    background: rgba(0, 0, 0, 0.85);
    padding: 6px 8px;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    pointer-events: none;
  }

  .scope-title {
    font-size: 0.65rem;
    color: #8b949e;
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
    background: #10b981;
    border-radius: 1px;
  }

  .transport-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.4rem 1rem;
    background: #161b22;
    border-top: 1px solid #21262d;
  }

  .transport-left {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .transport-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #21262d;
    border: 1px solid #30363d;
    color: #c9d1d9;
    border-radius: 4px;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .transport-btn:hover {
    background: #30363d;
    border-color: #58a6ff;
  }

  .play-btn {
    background: #238636;
    border-color: #2ea043;
    color: white;
  }

  .play-btn:hover {
    background: #2ea043;
  }

  .transport-center {
    display: flex;
    align-items: baseline;
    gap: 0.35rem;
  }

  .smpte-display {
    font-family: var(--font-mono, monospace);
    font-size: 0.95rem;
    font-weight: 600;
    color: #e6edf3;
  }

  .duration-display {
    font-family: var(--font-mono, monospace);
    font-size: 0.75rem;
    color: #8b949e;
  }

  .transport-right {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .shuttle-shortcuts {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .shortcut-pill {
    font-size: 0.65rem;
    font-family: var(--font-mono, monospace);
    color: #8b949e;
    background: rgba(255, 255, 255, 0.05);
    padding: 1px 5px;
    border-radius: 3px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
</style>
