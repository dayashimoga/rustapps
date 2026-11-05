<script lang="ts">
  import { onMount, afterUpdate } from 'svelte';

  export let binFrequencies: number[] = [];
  export let magnitudesDb: number[] = [];
  export let dominantFreqHz: number = 0;
  export let snrDb: number = 0;
  export let thdPercent: number = 0;

  let canvas: HTMLCanvasElement;

  function drawSpectrum() {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    // Background
    ctx.fillStyle = '#080c10';
    ctx.fillRect(0, 0, width, height);

    // Spectrum Reticle
    ctx.strokeStyle = '#182433';
    ctx.lineWidth = 1;

    for (let x = 0; x < width; x += width / 8) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();
    }

    for (let y = 0; y < height; y += height / 6) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }

    if (!magnitudesDb || magnitudesDb.length === 0) return;

    // dB range from 0 dBFS down to -90 dBFS
    const minDb = -90.0;
    const maxDb = 0.0;

    const stepX = width / (magnitudesDb.length - 1);

    // Fill gradient under spectrum
    const gradient = ctx.createLinearGradient(0, 0, 0, height);
    gradient.addColorStop(0, '#9d4edd99');
    gradient.addColorStop(0.5, '#7b2cbf44');
    gradient.addColorStop(1, '#10002b05');

    ctx.fillStyle = gradient;
    ctx.beginPath();
    ctx.moveTo(0, height);

    for (let i = 0; i < magnitudesDb.length; i++) {
      const db = Math.max(minDb, Math.min(maxDb, magnitudesDb[i]));
      const normalizedY = 1.0 - ((db - minDb) / (maxDb - minDb));
      const y = normalizedY * height;
      const x = i * stepX;
      ctx.lineTo(x, y);
    }

    ctx.lineTo(width, height);
    ctx.closePath();
    ctx.fill();

    // Outline line
    ctx.strokeStyle = '#c77dff';
    ctx.lineWidth = 1.8;
    ctx.beginPath();

    for (let i = 0; i < magnitudesDb.length; i++) {
      const db = Math.max(minDb, Math.min(maxDb, magnitudesDb[i]));
      const normalizedY = 1.0 - ((db - minDb) / (maxDb - minDb));
      const y = normalizedY * height;
      const x = i * stepX;

      if (i === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    }
    ctx.stroke();

    // Dominant Frequency Peak Indicator
    if (dominantFreqHz > 0 && binFrequencies.length > 0) {
      const maxFreq = binFrequencies[binFrequencies.length - 1];
      if (maxFreq > 0) {
        const peakX = (dominantFreqHz / maxFreq) * width;
        ctx.fillStyle = '#ff007f';
        ctx.beginPath();
        ctx.arc(peakX, 18, 4, 0, 2 * Math.PI);
        ctx.fill();

        ctx.fillStyle = '#ff70a6';
        ctx.font = '10px monospace';
        ctx.fillText(`Peak: ${dominantFreqHz.toFixed(1)} Hz`, Math.min(width - 90, Math.max(8, peakX - 25)), 14);
      }
    }
  }

  onMount(() => {
    drawSpectrum();
  });

  afterUpdate(() => {
    drawSpectrum();
  });
</script>

<div class="spectrum-container">
  <div class="spectrum-hud">
    <div class="hud-tag">
      <span class="label">FFT SPECTRUM</span>
      <span class="val">HANN WINDOW (dBFS)</span>
    </div>
    <div class="hud-tag">
      <span class="label">DOMINANT FREQ</span>
      <span class="val highlight">{dominantFreqHz >= 1000 ? (dominantFreqHz / 1000).toFixed(2) + ' kHz' : dominantFreqHz.toFixed(1) + ' Hz'}</span>
    </div>
    <div class="hud-tag">
      <span class="label">SNR</span>
      <span class="val">{snrDb.toFixed(1)} dB</span>
    </div>
    <div class="hud-tag">
      <span class="label">THD</span>
      <span class="val">{thdPercent.toFixed(2)} %</span>
    </div>
  </div>

  <div class="canvas-wrap">
    <canvas bind:this={canvas} width={1000} height={200}></canvas>
  </div>
</div>

<style>
  .spectrum-container {
    display: flex;
    flex-direction: column;
    background: #0d131a;
    border: 1px solid #1e2c3d;
    border-radius: 8px;
    overflow: hidden;
  }

  .spectrum-hud {
    display: flex;
    gap: 16px;
    padding: 6px 14px;
    background: #141c26;
    border-bottom: 1px solid #1e2c3d;
    align-items: center;
    font-size: 11px;
    font-family: var(--font-mono);
  }

  .hud-tag {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .label {
    color: #8b949e;
  }

  .val {
    color: #c77dff;
    font-weight: 600;
  }

  .highlight {
    color: #ff70a6;
  }

  .canvas-wrap {
    padding: 4px;
    background: #080c10;
  }

  canvas {
    width: 100%;
    height: 180px;
    display: block;
    border-radius: 4px;
  }
</style>

