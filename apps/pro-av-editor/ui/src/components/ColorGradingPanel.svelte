<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { tauriInvoke, type ColorGradingSettings, type VideoScopeData } from '../types';

  export let playheadMs: number = 0;
  export let selectedClipId: string | null = null;

  const dispatch = createEventDispatcher<{
    gradeUpdated: void;
  }>();

  // Primary Color Sliders
  let exposure: number = 0.0;
  let contrast: number = 1.0;
  let saturation: number = 1.0;
  let temperature: number = 0.0;
  let tint: number = 0.0;

  // 3-Way Lift / Gamma / Gain (R, G, B)
  let liftR: number = 0.0;
  let liftG: number = 0.0;
  let liftB: number = 0.0;

  let gammaR: number = 0.0;
  let gammaG: number = 0.0;
  let gammaB: number = 0.0;

  let gainR: number = 0.0;
  let gainG: number = 0.0;
  let gainB: number = 0.0;

  // Active Scope View Tab
  let activeScopeTab: 'all' | 'histogram' | 'waveform' | 'vectorscope' = 'all';

  // Canvases
  let histCanvasRef: HTMLCanvasElement | null = null;
  let wfCanvasRef: HTMLCanvasElement | null = null;
  let vecCanvasRef: HTMLCanvasElement | null = null;

  let scopes: VideoScopeData | null = null;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  $: if (playheadMs !== undefined) {
    scheduleScopeUpdate();
  }

  onMount(async () => {
    await loadScopes();
  });

  function scheduleScopeUpdate() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      await loadScopes();
    }, 40);
  }

  async function loadScopes() {
    try {
      scopes = await tauriInvoke<VideoScopeData>('get_video_scopes_cmd', {
        timestampMs: playheadMs,
        width: 160,
        height: 90,
      });
      if (scopes) {
        renderAllActiveScopes(scopes);
      }
    } catch (e) {
      console.error('Failed to load scopes:', e);
    }
  }

  function renderAllActiveScopes(data: VideoScopeData) {
    if (histCanvasRef) drawHistogram(histCanvasRef, data);
    if (wfCanvasRef) drawWaveform(wfCanvasRef, data);
    if (vecCanvasRef) drawVectorscope(vecCanvasRef, data);
  }

  function drawHistogram(canvas: HTMLCanvasElement, data: VideoScopeData) {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const w = canvas.width;
    const h = canvas.height;

    ctx.fillStyle = '#090d16';
    ctx.fillRect(0, 0, w, h);

    // Draw IRE reference lines
    ctx.strokeStyle = '#1e293b';
    ctx.lineWidth = 1;
    for (let pct = 0.25; pct <= 0.75; pct += 0.25) {
      const y = h * (1 - pct);
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(w, y);
      ctx.stroke();
    }

    // Determine peak
    let maxVal = 1;
    for (let i = 0; i < 256; i++) {
      if (data.histogram_r[i] > maxVal) maxVal = data.histogram_r[i];
      if (data.histogram_g[i] > maxVal) maxVal = data.histogram_g[i];
      if (data.histogram_b[i] > maxVal) maxVal = data.histogram_b[i];
      if (data.histogram_luma[i] > maxVal) maxVal = data.histogram_luma[i];
    }

    // Draw channels
    ctx.globalCompositeOperation = 'screen';

    // Red Channel
    drawHistogramCurve(ctx, data.histogram_r, maxVal, w, h, 'rgba(239, 68, 68, 0.7)');
    // Green Channel
    drawHistogramCurve(ctx, data.histogram_g, maxVal, w, h, 'rgba(16, 185, 129, 0.7)');
    // Blue Channel
    drawHistogramCurve(ctx, data.histogram_b, maxVal, w, h, 'rgba(56, 189, 248, 0.7)');
    // Luma Channel
    drawHistogramCurve(ctx, data.histogram_luma, maxVal, w, h, 'rgba(241, 245, 249, 0.45)');

    ctx.globalCompositeOperation = 'source-over';

    // IRE labels
    ctx.fillStyle = '#64748b';
    ctx.font = '9px monospace';
    ctx.fillText('100 IRE', 4, 11);
    ctx.fillText('0 IRE', 4, h - 3);
  }

  function drawHistogramCurve(
    ctx: CanvasRenderingContext2D,
    bins: number[],
    maxVal: number,
    w: number,
    h: number,
    fillStyle: string
  ) {
    if (!bins || bins.length === 0) return;
    ctx.fillStyle = fillStyle;
    ctx.beginPath();
    ctx.moveTo(0, h);

    const stepX = w / bins.length;
    for (let i = 0; i < bins.length; i++) {
      const x = i * stepX;
      const barH = (bins[i] / maxVal) * (h - 14);
      const y = h - barH;
      ctx.lineTo(x, y);
    }
    ctx.lineTo(w, h);
    ctx.closePath();
    ctx.fill();
  }

  function drawWaveform(canvas: HTMLCanvasElement, data: VideoScopeData) {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const w = canvas.width;
    const h = canvas.height;

    ctx.fillStyle = '#05080f';
    ctx.fillRect(0, 0, w, h);

    const cols = data.waveform_cols || 64;
    const rows = data.waveform_rows || 32;
    const matrix = data.waveform_matrix;

    if (matrix && matrix.length === cols * rows) {
      // Offscreen buffer for raster
      const offscreen = document.createElement('canvas');
      offscreen.width = cols;
      offscreen.height = rows;
      const offCtx = offscreen.getContext('2d');
      if (offCtx) {
        const imgData = offCtx.createImageData(cols, rows);
        for (let i = 0; i < matrix.length; i++) {
          const v = matrix[i];
          const idx = i * 4;
          // Emerald phosphor glow
          imgData.data[idx] = Math.min(255, v * 2);
          imgData.data[idx + 1] = Math.min(255, v * 5);
          imgData.data[idx + 2] = Math.min(255, v * 3);
          imgData.data[idx + 3] = v > 0 ? Math.min(255, 30 + v * 3) : 0;
        }
        offCtx.putImageData(imgData, 0, 0);

        ctx.imageSmoothingEnabled = true;
        ctx.drawImage(offscreen, 0, 0, w, h);
      }
    }

    // Draw IRE Graticules (0, 20, 40, 60, 80, 100 IRE)
    ctx.strokeStyle = 'rgba(56, 189, 248, 0.25)';
    ctx.lineWidth = 1;
    ctx.setLineDash([2, 2]);

    const ireLevels = [0, 20, 40, 60, 80, 100];
    ctx.fillStyle = '#64748b';
    ctx.font = '8px monospace';

    for (const ire of ireLevels) {
      const y = h - (ire / 100) * (h - 12) - 6;
      ctx.beginPath();
      ctx.moveTo(28, y);
      ctx.lineTo(w, y);
      ctx.stroke();
      ctx.fillText(`${ire}`, 4, y + 3);
    }
    ctx.setLineDash([]);
  }

  function drawVectorscope(canvas: HTMLCanvasElement, data: VideoScopeData) {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const w = canvas.width;
    const h = canvas.height;
    const cx = w / 2;
    const cy = h / 2;
    const rOuter = Math.min(w, h) * 0.44;
    const r75 = rOuter * 0.75;

    ctx.fillStyle = '#05080f';
    ctx.fillRect(0, 0, w, h);

    // Draw Polar Rings (100% and 75% saturation targets)
    ctx.strokeStyle = 'rgba(100, 116, 139, 0.35)';
    ctx.lineWidth = 1;

    ctx.beginPath();
    ctx.arc(cx, cy, rOuter, 0, Math.PI * 2);
    ctx.stroke();

    ctx.beginPath();
    ctx.arc(cx, cy, r75, 0, Math.PI * 2);
    ctx.stroke();

    // Center Crosshair
    ctx.beginPath();
    ctx.moveTo(cx - 8, cy);
    ctx.lineTo(cx + 8, cy);
    ctx.moveTo(cx, cy - 8);
    ctx.lineTo(cx, cy + 8);
    ctx.stroke();

    // I & Q Axes (Skin tone line ~33 degrees)
    ctx.strokeStyle = 'rgba(251, 146, 60, 0.35)';
    ctx.setLineDash([3, 3]);
    const iRad = (33 * Math.PI) / 180;
    ctx.beginPath();
    ctx.moveTo(cx - Math.cos(iRad) * rOuter, cy + Math.sin(iRad) * rOuter);
    ctx.lineTo(cx + Math.cos(iRad) * rOuter, cy - Math.sin(iRad) * rOuter);
    ctx.stroke();
    ctx.setLineDash([]);

    // SMPTE Target Boxes (R, Mg, B, Cy, G, Yl)
    const targets = [
      { name: 'R', angleDeg: 104, color: '#ef4444' },
      { name: 'Mg', angleDeg: 61, color: '#ec4899' },
      { name: 'B', angleDeg: 347, color: '#38bdf8' },
      { name: 'Cy', angleDeg: 284, color: '#06b6d4' },
      { name: 'G', angleDeg: 241, color: '#10b981' },
      { name: 'Yl', angleDeg: 167, color: '#eab308' },
    ];

    ctx.font = '8px monospace';
    for (const t of targets) {
      const rad = ((t.angleDeg - 90) * Math.PI) / 180;
      const tx = cx + Math.cos(rad) * r75;
      const ty = cy + Math.sin(rad) * r75;

      ctx.strokeStyle = t.color;
      ctx.strokeRect(tx - 4, ty - 4, 8, 8);
      ctx.fillStyle = t.color;
      ctx.fillText(t.name, tx + 6, ty + 3);
    }

    // Draw Vectorscope phosphor trace
    const dim = data.vectorscope_dim || 64;
    const grid = data.vectorscope_grid;
    if (grid && grid.length === dim * dim) {
      const offscreen = document.createElement('canvas');
      offscreen.width = dim;
      offscreen.height = dim;
      const offCtx = offscreen.getContext('2d');
      if (offCtx) {
        const imgData = offCtx.createImageData(dim, dim);
        for (let i = 0; i < grid.length; i++) {
          const v = grid[i];
          const idx = i * 4;
          imgData.data[idx] = Math.min(255, v * 4);
          imgData.data[idx + 1] = Math.min(255, v * 5);
          imgData.data[idx + 2] = Math.min(255, v * 2);
          imgData.data[idx + 3] = v > 0 ? Math.min(255, 40 + v * 3) : 0;
        }
        offCtx.putImageData(imgData, 0, 0);

        ctx.imageSmoothingEnabled = true;
        const scale = (rOuter * 1.8) / dim;
        const drawW = dim * scale;
        const drawH = dim * scale;
        ctx.drawImage(offscreen, cx - drawW / 2, cy - drawH / 2, drawW, drawH);
      }
    }
  }

  async function applyGrading() {
    const grading: ColorGradingSettings = {
      lift: { r: liftR, g: liftG, b: liftB, luma: 0.0 },
      gamma: { r: gammaR, g: gammaG, b: gammaB, luma: 0.0 },
      gain: { r: gainR, g: gainG, b: gainB, luma: 0.0 },
      exposure,
      contrast,
      saturation,
      temperature,
      tint,
      lut_file_path: null,
    };
    try {
      await tauriInvoke('update_clip_color_grading_cmd', {
        clipId: selectedClipId,
        grading,
      });
      await loadScopes();
      dispatch('gradeUpdated');
    } catch (e) {
      console.error('Failed to update color grading:', e);
    }
  }

  type ColorPreset =
    | 'teal_orange'
    | 'golden'
    | 'bleach'
    | 'noir'
    | 'neutral'
    | 'cyberpunk'
    | 'portra400'
    | 'velvia50'
    | 'nordic'
    | 'polaroid'
    | 'cross_process'
    | 'vibrant_hdr';

  function applyPreset(preset: ColorPreset) {
    if (preset === 'teal_orange') {
      exposure = 0.2;
      contrast = 1.25;
      saturation = 1.35;
      temperature = 18;
      tint = -6;
      liftR = -0.08; liftG = 0.04; liftB = 0.12; // Cyan shadows
      gainR = 0.15; gainG = 0.06; gainB = -0.10; // Warm amber highlights
    } else if (preset === 'golden') {
      exposure = 0.15;
      contrast = 1.1;
      saturation = 1.2;
      temperature = 32;
      tint = 8;
      liftR = 0.04; liftG = 0.02; liftB = -0.06;
      gainR = 0.18; gainG = 0.12; gainB = -0.08;
    } else if (preset === 'bleach') {
      exposure = 0.0;
      contrast = 1.5;
      saturation = 0.35;
      temperature = -4;
      tint = 2;
      liftR = -0.05; liftG = -0.05; liftB = -0.05;
      gainR = 0.08; gainG = 0.08; gainB = 0.08;
    } else if (preset === 'noir') {
      exposure = -0.1;
      contrast = 1.6;
      saturation = 0.0;
      temperature = 0;
      tint = 0;
      liftR = -0.1; liftG = -0.1; liftB = -0.1;
      gainR = 0.12; gainG = 0.12; gainB = 0.12;
    } else if (preset === 'cyberpunk') {
      exposure = 0.1;
      contrast = 1.4;
      saturation = 1.55;
      temperature = -12;
      tint = 22;
      liftR = -0.12; liftG = 0.08; liftB = 0.20;
      gainR = 0.22; gainG = -0.05; gainB = 0.18;
    } else if (preset === 'portra400') {
      exposure = 0.1;
      contrast = 0.95;
      saturation = 1.08;
      temperature = 12;
      tint = 4;
      liftR = 0.03; liftG = 0.01; liftB = -0.02;
      gainR = 0.08; gainG = 0.06; gainB = 0.02;
    } else if (preset === 'velvia50') {
      exposure = 0.05;
      contrast = 1.35;
      saturation = 1.6;
      temperature = -6;
      tint = -8;
      liftR = -0.06; liftG = 0.08; liftB = -0.02;
      gainR = 0.12; gainG = 0.14; gainB = 0.04;
    } else if (preset === 'nordic') {
      exposure = -0.08;
      contrast = 1.25;
      saturation = 0.75;
      temperature = -28;
      tint = -4;
      liftR = -0.04; liftG = 0.02; liftB = 0.15;
      gainR = -0.02; gainG = 0.02; gainB = 0.08;
    } else if (preset === 'polaroid') {
      exposure = 0.18;
      contrast = 0.88;
      saturation = 1.15;
      temperature = 16;
      tint = 10;
      liftR = 0.08; liftG = 0.06; liftB = 0.02;
      gainR = 0.06; gainG = 0.08; gainB = -0.04;
    } else if (preset === 'cross_process') {
      exposure = 0.08;
      contrast = 1.45;
      saturation = 1.25;
      temperature = -10;
      tint = 18;
      liftR = -0.08; liftG = 0.12; liftB = -0.04;
      gainR = 0.18; gainG = -0.04; gainB = 0.14;
    } else if (preset === 'vibrant_hdr') {
      exposure = 0.22;
      contrast = 1.3;
      saturation = 1.45;
      temperature = 6;
      tint = 0;
      liftR = -0.02; liftG = -0.02; liftB = -0.02;
      gainR = 0.10; gainG = 0.10; gainB = 0.10;
    } else {
      resetGrading();
      return;
    }
    applyGrading();
  }

  async function resetGrading() {
    exposure = 0.0;
    contrast = 1.0;
    saturation = 1.0;
    temperature = 0.0;
    tint = 0.0;
    liftR = 0; liftG = 0; liftB = 0;
    gammaR = 0; gammaG = 0; gammaB = 0;
    gainR = 0; gainG = 0; gainB = 0;
    await applyGrading();
  }
</script>

<div class="color-panel">
  <!-- Controls Area -->
  <div class="wheels-and-sliders">
    <!-- Presets Toolbar -->
    <div class="presets-toolbar">
      <span class="toolbar-title">Creative Looks &amp; Film Stocks:</span>
      <div class="preset-btns">
        <button type="button" class="preset-btn" on:click={() => applyPreset('teal_orange')}>
          🎬 Teal &amp; Orange
        </button>
        <button type="button" class="preset-btn" on:click={() => applyPreset('golden')}>
          ☀️ Golden Hour
        </button>
        <button type="button" class="preset-btn" on:click={() => applyPreset('cyberpunk')}>
          🌆 Cyberpunk
        </button>
        <button type="button" class="preset-btn" on:click={() => applyPreset('portra400')}>
          📷 Portra 400
        </button>
        <button type="button" class="preset-btn" on:click={() => applyPreset('velvia50')}>
          🌄 Velvia 50
        </button>
        <button type="button" class="preset-btn" on:click={() => applyPreset('nordic')}>
          🧊 Nordic Cold
        </button>
        <button type="button" class="preset-btn" on:click={() => applyPreset('polaroid')}>
          📸 Polaroid 70s
        </button>
        <button type="button" class="preset-btn" on:click={() => applyPreset('cross_process')}>
          🌈 Cross-Process
        </button>
        <button type="button" class="preset-btn" on:click={() => applyPreset('vibrant_hdr')}>
          💥 Vibrant HDR
        </button>
        <button type="button" class="preset-btn" on:click={() => applyPreset('bleach')}>
          ⚔️ Bleach Bypass
        </button>
        <button type="button" class="preset-btn" on:click={() => applyPreset('noir')}>
          🎞️ Film Noir
        </button>
        <button type="button" class="preset-btn" on:click={() => applyPreset('neutral')}>
          ⚖️ Rec.709 Flat
        </button>
      </div>
    </div>

    <!-- 3-Way Color Wheels -->
    <div class="wheels-row">
      <!-- Lift -->
      <div class="wheel-box">
        <span class="wheel-title">Lift (Shadows)</span>
        <div class="wheel-circle">
          <div class="wheel-reticle" style={`left: ${50 + liftR * 40}%; top: ${50 - liftB * 40}%;`}></div>
        </div>
        <div class="rgb-sliders">
          <label>R <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={liftR} on:input={applyGrading} /></label>
          <label>G <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={liftG} on:input={applyGrading} /></label>
          <label>B <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={liftB} on:input={applyGrading} /></label>
        </div>
      </div>

      <!-- Gamma -->
      <div class="wheel-box">
        <span class="wheel-title">Gamma (Midtones)</span>
        <div class="wheel-circle">
          <div class="wheel-reticle" style={`left: ${50 + gammaR * 40}%; top: ${50 - gammaB * 40}%;`}></div>
        </div>
        <div class="rgb-sliders">
          <label>R <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={gammaR} on:input={applyGrading} /></label>
          <label>G <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={gammaG} on:input={applyGrading} /></label>
          <label>B <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={gammaB} on:input={applyGrading} /></label>
        </div>
      </div>

      <!-- Gain -->
      <div class="wheel-box">
        <span class="wheel-title">Gain (Highlights)</span>
        <div class="wheel-circle">
          <div class="wheel-reticle" style={`left: ${50 + gainR * 40}%; top: ${50 - gainB * 40}%;`}></div>
        </div>
        <div class="rgb-sliders">
          <label>R <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={gainR} on:input={applyGrading} /></label>
          <label>G <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={gainG} on:input={applyGrading} /></label>
          <label>B <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={gainB} on:input={applyGrading} /></label>
        </div>
      </div>
    </div>

    <!-- Sliders row -->
    <div class="primary-sliders">
      <div class="slider-item">
        <div class="slider-header">
          <span>Exposure</span>
          <span class="slider-val">{exposure > 0 ? `+${exposure.toFixed(1)}` : exposure.toFixed(1)} EV</span>
        </div>
        <input type="range" min="-3.0" max="3.0" step="0.1" bind:value={exposure} on:input={applyGrading} />
      </div>

      <div class="slider-item">
        <div class="slider-header">
          <span>Contrast</span>
          <span class="slider-val">{contrast.toFixed(2)}</span>
        </div>
        <input type="range" min="0.5" max="1.8" step="0.02" bind:value={contrast} on:input={applyGrading} />
      </div>

      <div class="slider-item">
        <div class="slider-header">
          <span>Saturation</span>
          <span class="slider-val">{saturation.toFixed(2)}</span>
        </div>
        <input type="range" min="0.0" max="2.0" step="0.05" bind:value={saturation} on:input={applyGrading} />
      </div>

      <div class="slider-item">
        <div class="slider-header">
          <span>Temperature</span>
          <span class="slider-val">{temperature.toFixed(0)}</span>
        </div>
        <input type="range" min="-50" max="50" step="1" bind:value={temperature} on:input={applyGrading} />
      </div>

      <div class="slider-item">
        <div class="slider-header">
          <span>Tint</span>
          <span class="slider-val">{tint.toFixed(0)}</span>
        </div>
        <input type="range" min="-50" max="50" step="1" bind:value={tint} on:input={applyGrading} />
      </div>

      <button type="button" class="reset-btn" on:click={resetGrading}>Reset Grade</button>
    </div>
  </div>

  <!-- Real-Time Scopes -->
  <div class="scopes-panel">
    <!-- Scope Selector Tabs -->
    <div class="scopes-tab-bar">
      <button
        type="button"
        class="scope-tab"
        class:active={activeScopeTab === 'all'}
        on:click={() => { activeScopeTab = 'all'; if (scopes) setTimeout(() => renderAllActiveScopes(scopes), 30); }}
      >
        All (3-Up)
      </button>
      <button
        type="button"
        class="scope-tab"
        class:active={activeScopeTab === 'histogram'}
        on:click={() => { activeScopeTab = 'histogram'; if (scopes) setTimeout(() => renderAllActiveScopes(scopes), 30); }}
      >
        Histogram
      </button>
      <button
        type="button"
        class="scope-tab"
        class:active={activeScopeTab === 'waveform'}
        on:click={() => { activeScopeTab = 'waveform'; if (scopes) setTimeout(() => renderAllActiveScopes(scopes), 30); }}
      >
        Waveform
      </button>
      <button
        type="button"
        class="scope-tab"
        class:active={activeScopeTab === 'vectorscope'}
        on:click={() => { activeScopeTab = 'vectorscope'; if (scopes) setTimeout(() => renderAllActiveScopes(scopes), 30); }}
      >
        Vectorscope
      </button>
    </div>

    <!-- Histogram Card -->
    {#if activeScopeTab === 'all' || activeScopeTab === 'histogram'}
      <div class="scope-card">
        <div class="scope-title-row">
          <span class="scope-header">RGB &amp; Luma Histogram</span>
          <span class="scope-badge">256 Bins • Screen Overlay</span>
        </div>
        <div class="canvas-wrap-scope">
          <canvas bind:this={histCanvasRef} width="280" height="90" class="scope-canvas"></canvas>
        </div>
      </div>
    {/if}

    <!-- Waveform Card -->
    {#if activeScopeTab === 'all' || activeScopeTab === 'waveform'}
      <div class="scope-card">
        <div class="scope-title-row">
          <span class="scope-header">Broadcast Luma Waveform</span>
          <span class="scope-badge">0–100 IRE Graticule</span>
        </div>
        <div class="canvas-wrap-scope">
          <canvas bind:this={wfCanvasRef} width="280" height="100" class="scope-canvas"></canvas>
        </div>
      </div>
    {/if}

    <!-- Vectorscope Card -->
    {#if activeScopeTab === 'all' || activeScopeTab === 'vectorscope'}
      <div class="scope-card">
        <div class="scope-title-row">
          <span class="scope-header">Polar Vectorscope (Cb / Cr)</span>
          <span class="scope-badge">75% &amp; 100% SMPTE Boxes</span>
        </div>
        <div class="canvas-wrap-scope center-wrap">
          <canvas bind:this={vecCanvasRef} width="160" height="160" class="scope-canvas vec-canvas"></canvas>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .color-panel {
    display: grid;
    grid-template-columns: 1.6fr 1.4fr;
    gap: 1rem;
    padding: 1rem;
    background: var(--bg-panel, #0f131a);
    border: 1px solid var(--border-subtle, #1f2937);
    border-radius: var(--radius-md, 6px);
    height: 100%;
    overflow-y: auto;
  }

  .wheels-and-sliders {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .presets-toolbar {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    background: #161b22;
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
    border: 1px solid #21262d;
  }

  .toolbar-title {
    font-size: 0.7rem;
    font-weight: 600;
    color: #8b949e;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .preset-btns {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  .preset-btn {
    background: #21262d;
    border: 1px solid #30363d;
    color: #c9d1d9;
    padding: 3px 8px;
    border-radius: 3px;
    font-size: 0.72rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .preset-btn:hover {
    border-color: #58a6ff;
    color: #e6edf3;
  }

  .wheels-row {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 0.6rem;
  }

  .wheel-box {
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 4px;
    padding: 0.6rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4rem;
  }

  .wheel-title {
    font-size: 0.7rem;
    font-weight: 600;
    color: #8b949e;
  }

  .wheel-circle {
    width: 76px;
    height: 76px;
    border-radius: 50%;
    background: radial-gradient(circle, #21262d 0%, #0d1117 100%);
    border: 2px solid rgba(255, 255, 255, 0.12);
    position: relative;
  }

  .wheel-reticle {
    position: absolute;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #38bdf8;
    box-shadow: 0 0 6px #38bdf8;
    transform: translate(-50%, -50%);
  }

  .rgb-sliders {
    display: flex;
    flex-direction: column;
    width: 100%;
    gap: 0.15rem;
    font-size: 0.62rem;
    color: #8b949e;
  }

  .rgb-sliders label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.25rem;
  }

  .rgb-sliders input {
    flex: 1;
    height: 3px;
  }

  .primary-sliders {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 0.6rem;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 4px;
    padding: 0.65rem;
  }

  .slider-item {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .slider-header {
    display: flex;
    justify-content: space-between;
    font-size: 0.7rem;
    color: #8b949e;
  }

  .slider-val {
    font-family: var(--font-mono, monospace);
    color: #58a6ff;
  }

  .slider-item input {
    width: 100%;
    cursor: pointer;
  }

  .reset-btn {
    grid-column: span 3;
    background: #21262d;
    border: 1px solid #30363d;
    color: #8b949e;
    padding: 0.3rem;
    border-radius: 3px;
    font-size: 0.72rem;
    cursor: pointer;
    margin-top: 0.2rem;
    transition: all 0.15s;
  }

  .reset-btn:hover {
    color: #f87171;
    border-color: #f87171;
  }

  .scopes-panel {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .scopes-tab-bar {
    display: flex;
    gap: 4px;
    background: #161b22;
    padding: 2px;
    border-radius: 4px;
    border: 1px solid #21262d;
  }

  .scope-tab {
    flex: 1;
    background: transparent;
    border: none;
    color: #8b949e;
    padding: 3px 6px;
    border-radius: 3px;
    font-size: 0.7rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .scope-tab.active {
    background: #21262d;
    color: #38bdf8;
    font-weight: 600;
  }

  .scope-card {
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 4px;
    padding: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .scope-title-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .scope-header {
    font-size: 0.7rem;
    font-weight: 600;
    color: #8b949e;
    text-transform: uppercase;
  }

  .scope-badge {
    font-size: 0.62rem;
    color: #64748b;
    font-family: var(--font-mono, monospace);
  }

  .canvas-wrap-scope {
    background: #05080f;
    border-radius: 3px;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid #1e293b;
  }

  .canvas-wrap-scope.center-wrap {
    padding: 6px 0;
  }

  .scope-canvas {
    width: 100%;
    height: auto;
    display: block;
  }

  .vec-canvas {
    width: 150px;
    height: 150px;
  }
</style>
