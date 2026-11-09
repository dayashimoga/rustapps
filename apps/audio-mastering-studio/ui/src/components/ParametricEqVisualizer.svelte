<script lang="ts">
  import { onMount, afterUpdate, createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  export interface EqBand {
    band_index: number;
    name: string;
    filter_type: string;
    frequency_hz: number;
    gain_db: number;
    q_factor: number;
  }

  export let bands: EqBand[] = [];

  let canvas: HTMLCanvasElement;
  let selectedBand: number = 2; // Default to mid band
  let isDragging: boolean = false;
  let draggedBandIndex: number = -1;

  function drawEqCurve() {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;
    const centerY = height / 2;

    // Background
    ctx.fillStyle = '#0a0d14';
    ctx.fillRect(0, 0, width, height);

    // Grid lines
    ctx.strokeStyle = '#18202d';
    ctx.lineWidth = 1;

    // Horizontal dB lines (+12, +6, 0, -6, -12 dB)
    for (let db = -12; db <= 12; db += 6) {
      const y = centerY - (db / 12) * (centerY - 10);
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();

      ctx.fillStyle = '#64748b';
      ctx.font = '9px monospace';
      ctx.fillText(`${db > 0 ? '+' : ''}${db}dB`, 6, y - 2);
    }

    // Vertical Frequency lines: 50, 100, 500, 1k, 5k, 10k, 20k Hz
    const freqMarkers = [50, 100, 250, 500, 1000, 2500, 5000, 10000, 20000];
    for (const f of freqMarkers) {
      const x = freqToX(f, width);
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();

      ctx.fillStyle = '#475569';
      ctx.font = '8px monospace';
      ctx.fillText(f >= 1000 ? `${f / 1000}k` : `${f}`, x + 2, height - 4);
    }

    if (!bands || bands.length === 0) return;

    // Draw Composite EQ Curve
    ctx.strokeStyle = '#e5a93c';
    ctx.lineWidth = 2.0;
    ctx.beginPath();

    const points = 240;
    for (let i = 0; i <= points; i++) {
      const x = (i / points) * width;
      const f = xToFreq(x, width);

      let totalGainDb = 0.0;
      for (const b of bands) {
        // Approximate bell / shelf curve contribution
        const delta = Math.log10(f / b.frequency_hz);
        const bandwidth = 0.5 / (b.q_factor || 1.0);
        const response = Math.exp(-0.5 * (delta / bandwidth) * (delta / bandwidth));
        totalGainDb += b.gain_db * response;
      }

      const y = centerY - (totalGainDb / 12) * (centerY - 10);
      if (i === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    }
    ctx.stroke();

    // Fill subtle glow under curve
    ctx.lineTo(width, centerY);
    ctx.lineTo(0, centerY);
    ctx.closePath();
    ctx.fillStyle = '#e5a93c15';
    ctx.fill();

    // Draw interactive nodes for each band
    for (let i = 0; i < bands.length; i++) {
      const b = bands[i];
      const x = freqToX(b.frequency_hz, width);
      const y = centerY - (b.gain_db / 12) * (centerY - 10);

      ctx.beginPath();
      ctx.arc(x, y, i === selectedBand ? 8 : 5, 0, 2 * Math.PI);
      ctx.fillStyle = i === selectedBand ? '#ffffff' : '#e5a93c';
      ctx.fill();
      ctx.strokeStyle = i === selectedBand ? '#f59e0b' : '#0e1117';
      ctx.lineWidth = i === selectedBand ? 2.5 : 1.5;
      ctx.stroke();

      ctx.fillStyle = '#ffffff';
      ctx.font = 'bold 9px monospace';
      ctx.fillText(`${i + 1}`, x - 3, y + 3);
    }
  }

  function freqToX(freq: number, width: number): number {
    const minF = 20;
    const maxF = 20000;
    return (Math.log10(freq / minF) / Math.log10(maxF / minF)) * width;
  }

  function xToFreq(x: number, width: number): number {
    const minF = 20;
    const maxF = 20000;
    const fraction = Math.max(0, Math.min(1, x / width));
    return minF * Math.pow(maxF / minF, fraction);
  }

  function yToGain(y: number, height: number): number {
    const centerY = height / 2;
    const scale = centerY - 10;
    const gain = -((y - centerY) / scale) * 12;
    return Math.max(-12, Math.min(12, gain));
  }

  function handleSelectBand(index: number) {
    selectedBand = index;
    drawEqCurve();
  }

  function handleParamChange() {
    dispatch('change', { bands });
    if (bands[selectedBand]) {
      dispatch('band_updated', bands[selectedBand]);
    }
    drawEqCurve();
  }

  function getCanvasPos(e: MouseEvent) {
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: (e.clientY - rect.top) * scaleY,
    };
  }

  function handleMouseDown(e: MouseEvent) {
    if (!canvas || !bands || bands.length === 0) return;
    const { x, y } = getCanvasPos(e);
    const width = canvas.width;
    const height = canvas.height;
    const centerY = height / 2;

    let closestIdx = -1;
    let minDistance = 20; // 20px hit radius

    bands.forEach((b, idx) => {
      const bx = freqToX(b.frequency_hz, width);
      const by = centerY - (b.gain_db / 12) * (centerY - 10);
      const dist = Math.hypot(x - bx, y - by);
      if (dist < minDistance) {
        minDistance = dist;
        closestIdx = idx;
      }
    });

    if (closestIdx !== -1) {
      isDragging = true;
      draggedBandIndex = closestIdx;
      selectedBand = closestIdx;
      drawEqCurve();
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging || draggedBandIndex === -1 || !bands[draggedBandIndex]) return;
    const { x, y } = getCanvasPos(e);
    const newFreq = Math.round(xToFreq(x, canvas.width));
    const newGain = parseFloat(yToGain(y, canvas.height).toFixed(1));

    bands[draggedBandIndex].frequency_hz = Math.max(20, Math.min(20000, newFreq));
    bands[draggedBandIndex].gain_db = newGain;

    drawEqCurve();
    dispatch('change', { bands });
  }

  function handleMouseUp() {
    if (isDragging && draggedBandIndex !== -1 && bands[draggedBandIndex]) {
      dispatch('band_updated', bands[draggedBandIndex]);
    }
    isDragging = false;
    draggedBandIndex = -1;
  }

  onMount(() => {
    drawEqCurve();
  });

  afterUpdate(() => {
    drawEqCurve();
  });
</script>

<div class="eq-visualizer-container">
  <div class="eq-header">
    <span class="title">5-BAND PARAMETRIC EQUALIZER (DRAGGABLE BIQUAD FILTER)</span>
    <div class="band-pills">
      {#each bands as b, idx}
        <button
          class="band-pill"
          class:active={idx === selectedBand}
          on:click={() => handleSelectBand(idx)}
        >
          {idx + 1}: {b.frequency_hz >= 1000 ? `${(b.frequency_hz / 1000).toFixed(1)}k` : `${b.frequency_hz.toFixed(0)}Hz`} ({b.gain_db > 0 ? '+' : ''}{b.gain_db.toFixed(1)}dB)
        </button>
      {/each}
    </div>
  </div>

  <div class="canvas-wrap">
    <canvas
      bind:this={canvas}
      width={700}
      height={180}
      on:mousedown={handleMouseDown}
      on:mousemove={handleMouseMove}
      on:mouseup={handleMouseUp}
      on:mouseleave={handleMouseUp}
    ></canvas>
  </div>

  {#if bands[selectedBand]}
    <div class="eq-controls-row">
      <div class="param">
        <label for="eq-freq">Freq</label>
        <input
          id="eq-freq"
          type="range"
          min="20"
          max="20000"
          step="10"
          bind:value={bands[selectedBand].frequency_hz}
          on:input={handleParamChange}
        />
        <span>{bands[selectedBand].frequency_hz >= 1000 ? `${(bands[selectedBand].frequency_hz / 1000).toFixed(2)} kHz` : `${bands[selectedBand].frequency_hz.toFixed(0)} Hz`}</span>
      </div>

      <div class="param">
        <label for="eq-gain">Gain</label>
        <input
          id="eq-gain"
          type="range"
          min="-12"
          max="12"
          step="0.5"
          bind:value={bands[selectedBand].gain_db}
          on:input={handleParamChange}
        />
        <span>{bands[selectedBand].gain_db > 0 ? '+' : ''}{bands[selectedBand].gain_db.toFixed(1)} dB</span>
      </div>

      <div class="param">
        <label for="eq-q">Q Factor</label>
        <input
          id="eq-q"
          type="range"
          min="0.3"
          max="5.0"
          step="0.1"
          bind:value={bands[selectedBand].q_factor}
          on:input={handleParamChange}
        />
        <span>{bands[selectedBand].q_factor.toFixed(2)}</span>
      </div>
    </div>
  {/if}
</div>

<style>
  .eq-visualizer-container {
    display: flex;
    flex-direction: column;
    background: #11151c;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    overflow: hidden;
  }

  .eq-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 12px;
    background: #161b24;
    border-bottom: 1px solid var(--border-color);
    font-family: var(--font-mono);
  }

  .title {
    font-size: 11px;
    font-weight: 700;
    color: var(--accent-gold);
  }

  .band-pills {
    display: flex;
    gap: 4px;
  }

  .band-pill {
    background: #1e2838;
    border: 1px solid #2e3e56;
    color: #94a3b8;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 9px;
    font-family: var(--font-mono);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .band-pill:hover {
    border-color: var(--accent-gold);
    color: #f8fafc;
  }

  .band-pill.active {
    background: var(--accent-gold);
    color: #0e1117;
    font-weight: 700;
    border-color: var(--accent-gold);
  }

  .canvas-wrap {
    padding: 6px;
    background: #0a0d14;
    cursor: crosshair;
  }

  canvas {
    width: 100%;
    height: 160px;
    display: block;
    border-radius: 4px;
  }

  .eq-controls-row {
    display: flex;
    gap: 16px;
    padding: 8px 12px;
    background: #131822;
    border-top: 1px solid var(--border-color);
    align-items: center;
    font-family: var(--font-mono);
    font-size: 10px;
  }

  .param {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
  }

  label {
    color: #94a3b8;
  }

  input[type="range"] {
    flex: 1;
    accent-color: var(--accent-gold);
    cursor: pointer;
  }

  span {
    width: 55px;
    text-align: right;
    color: #f1f5f9;
    font-weight: 600;
  }
</style>
