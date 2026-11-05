<script lang="ts">
  import { onMount, afterUpdate } from 'svelte';

  export interface StemTrack {
    stem_type: string;
    title: string;
    samples: number[];
    volume_gain_db: number;
    is_muted: boolean;
    is_solo: boolean;
    peak_db: number;
    rms_db: number;
  }

  export let stems: StemTrack[] = [];
  export let durationSeconds: number = 2.0;

  let canvases: HTMLCanvasElement[] = [];

  const stemColors: Record<string, string> = {
    Vocals: '#ff70a6',
    Drums: '#ffaa00',
    Bass: '#38bdf8',
    Other: '#4ade80',
  };

  function drawWaveform(canvas: HTMLCanvasElement, samples: number[], color: string) {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;
    const centerY = height / 2;

    // Background
    ctx.fillStyle = '#10141c';
    ctx.fillRect(0, 0, width, height);

    // Center baseline
    ctx.strokeStyle = '#1e2838';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, centerY);
    ctx.lineTo(width, centerY);
    ctx.stroke();

    if (!samples || samples.length === 0) return;

    const step = Math.max(1, Math.floor(samples.length / width));
    ctx.fillStyle = color;

    for (let x = 0; x < width; x++) {
      const idx = x * step;
      if (idx >= samples.length) break;

      let min = 1.0;
      let max = -1.0;
      for (let k = 0; k < step && idx + k < samples.length; k++) {
        const val = samples[idx + k];
        if (val < min) min = val;
        if (val > max) max = val;
      }

      const yTop = centerY - (max * (height * 0.45));
      const yBottom = centerY - (min * (height * 0.45));
      const barHeight = Math.max(1, yBottom - yTop);

      ctx.fillRect(x, yTop, 1, barHeight);
    }
  }

  function renderAllWaveforms() {
    for (let i = 0; i < stems.length; i++) {
      if (canvases[i]) {
        const color = stemColors[stems[i].stem_type] || '#38bdf8';
        drawWaveform(canvases[i], stems[i].samples, color);
      }
    }
  }

  onMount(() => {
    renderAllWaveforms();
  });

  afterUpdate(() => {
    renderAllWaveforms();
  });

  function toggleMute(index: number) {
    stems[index].is_muted = !stems[index].is_muted;
    stems = [...stems];
  }

  function toggleSolo(index: number) {
    stems[index].is_solo = !stems[index].is_solo;
    stems = [...stems];
  }
</script>

<div class="stem-viewer-container">
  <div class="viewer-header">
    <div class="title-block">
      <span class="icon">🎛️</span>
      <span class="title">AI 4-STEM DEMIXING SEPARATION TIMELINE</span>
    </div>
    <span class="duration-badge">{durationSeconds.toFixed(2)}s Duration</span>
  </div>

  <div class="tracks-list">
    {#each stems as stem, i}
      <div class="track-row" class:muted={stem.is_muted} class:solo={stem.is_solo}>
        <!-- Channel Strip Controls -->
        <div class="track-channel-strip">
          <div class="strip-header">
            <span class="stem-dot" style="background-color: {stemColors[stem.stem_type] || '#38bdf8'};"></span>
            <span class="stem-title">{stem.title}</span>
          </div>

          <div class="strip-actions">
            <button
              class="btn-track mute"
              class:active={stem.is_muted}
              on:click={() => toggleMute(i)}
            >
              M
            </button>
            <button
              class="btn-track solo"
              class:active={stem.is_solo}
              on:click={() => toggleSolo(i)}
            >
              S
            </button>
            <span class="meter-readout">
              PK: {stem.peak_db.toFixed(1)} dB
            </span>
          </div>

          <div class="volume-slider-row">
            <input
              type="range"
              min="-24"
              max="6"
              step="0.5"
              bind:value={stem.volume_gain_db}
            />
            <span class="gain-val">{stem.volume_gain_db > 0 ? '+' : ''}{stem.volume_gain_db.toFixed(1)} dB</span>
          </div>
        </div>

        <!-- Waveform Canvas -->
        <div class="waveform-box">
          <canvas bind:this={canvases[i]} width={800} height={70}></canvas>
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .stem-viewer-container {
    display: flex;
    flex-direction: column;
    background: #11151c;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    overflow: hidden;
  }

  .viewer-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 14px;
    background: #161b24;
    border-bottom: 1px solid var(--border-color);
    font-family: var(--font-mono);
  }

  .title-block {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    font-weight: 700;
    color: var(--accent-gold);
  }

  .duration-badge {
    background: #243042;
    color: #94a3b8;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
  }

  .tracks-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 6px;
    background: #0d1016;
  }

  .track-row {
    display: flex;
    gap: 8px;
    background: #151a23;
    border: 1px solid #202836;
    border-radius: 6px;
    padding: 6px 10px;
    align-items: center;
    transition: opacity 0.2s;
  }

  .track-row.muted {
    opacity: 0.45;
  }

  .track-channel-strip {
    width: 220px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .strip-header {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .stem-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .stem-title {
    font-weight: 600;
    color: #e2e8f0;
  }

  .strip-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn-track {
    width: 20px;
    height: 20px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 700;
    border: 1px solid #334155;
    background: #1e293b;
    color: #94a3b8;
  }

  .btn-track.mute.active {
    background: #ef4444;
    border-color: #dc2626;
    color: #fff;
  }

  .btn-track.solo.active {
    background: #f59e0b;
    border-color: #d97706;
    color: #fff;
  }

  .meter-readout {
    font-size: 9px;
    color: #94a3b8;
    margin-left: auto;
  }

  .volume-slider-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .volume-slider-row input[type="range"] {
    flex: 1;
    accent-color: var(--accent-gold);
    height: 4px;
  }

  .gain-val {
    font-size: 9px;
    width: 44px;
    text-align: right;
    color: #cbd5e1;
  }

  .waveform-box {
    flex: 1;
    height: 60px;
    background: #10141c;
    border-radius: 4px;
    overflow: hidden;
  }

  canvas {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>

