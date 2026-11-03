<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { tauriInvoke, type ColorGradingSettings, type VideoScopeData } from '../types';

  export let playheadMs: number = 0;
  export let selectedClipId: string | null = null;

  const dispatch = createEventDispatcher<{
    gradeUpdated: void;
  }>();

  let exposure: number = 0.0;
  let contrast: number = 1.0;
  let saturation: number = 1.0;
  let temperature: number = 0.0;
  let tint: number = 0.0;

  let liftR: number = 0.0;
  let liftG: number = 0.0;
  let liftB: number = 0.0;

  let gammaR: number = 0.0;
  let gammaG: number = 0.0;
  let gammaB: number = 0.0;

  let gainR: number = 0.0;
  let gainG: number = 0.0;
  let gainB: number = 0.0;

  let scopes: VideoScopeData | null = null;

  onMount(async () => {
    await loadScopes();
  });

  async function loadScopes() {
    try {
      scopes = await tauriInvoke<VideoScopeData>('get_video_scopes_cmd', {
        timestampMs: playheadMs,
        width: 160,
        height: 90,
      });
    } catch (e) {
      console.error('Failed to load scopes:', e);
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
    <!-- 3-Way Color Wheels -->
    <div class="wheels-row">
      <!-- Lift -->
      <div class="wheel-box">
        <span class="wheel-title">Lift (Shadows)</span>
        <div class="wheel-circle">
          <div class="wheel-reticle" style={`left: ${50 + liftR * 40}%; top: ${50 - liftB * 40}%;`}></div>
        </div>
        <div class="rgb-sliders">
          <label>R <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={liftR} on:change={applyGrading} /></label>
          <label>G <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={liftG} on:change={applyGrading} /></label>
          <label>B <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={liftB} on:change={applyGrading} /></label>
        </div>
      </div>

      <!-- Gamma -->
      <div class="wheel-box">
        <span class="wheel-title">Gamma (Midtones)</span>
        <div class="wheel-circle">
          <div class="wheel-reticle" style={`left: ${50 + gammaR * 40}%; top: ${50 - gammaB * 40}%;`}></div>
        </div>
        <div class="rgb-sliders">
          <label>R <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={gammaR} on:change={applyGrading} /></label>
          <label>G <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={gammaG} on:change={applyGrading} /></label>
          <label>B <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={gammaB} on:change={applyGrading} /></label>
        </div>
      </div>

      <!-- Gain -->
      <div class="wheel-box">
        <span class="wheel-title">Gain (Highlights)</span>
        <div class="wheel-circle">
          <div class="wheel-reticle" style={`left: ${50 + gainR * 40}%; top: ${50 - gainB * 40}%;`}></div>
        </div>
        <div class="rgb-sliders">
          <label>R <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={gainR} on:change={applyGrading} /></label>
          <label>G <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={gainG} on:change={applyGrading} /></label>
          <label>B <input type="range" min="-0.5" max="0.5" step="0.01" bind:value={gainB} on:change={applyGrading} /></label>
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
        <input type="range" min="-3.0" max="3.0" step="0.1" bind:value={exposure} on:change={applyGrading} />
      </div>

      <div class="slider-item">
        <div class="slider-header">
          <span>Contrast</span>
          <span class="slider-val">{contrast.toFixed(2)}</span>
        </div>
        <input type="range" min="0.5" max="1.8" step="0.02" bind:value={contrast} on:change={applyGrading} />
      </div>

      <div class="slider-item">
        <div class="slider-header">
          <span>Saturation</span>
          <span class="slider-val">{saturation.toFixed(2)}</span>
        </div>
        <input type="range" min="0.0" max="2.0" step="0.05" bind:value={saturation} on:change={applyGrading} />
      </div>

      <div class="slider-item">
        <div class="slider-header">
          <span>Temperature</span>
          <span class="slider-val">{temperature.toFixed(0)}</span>
        </div>
        <input type="range" min="-50" max="50" step="1" bind:value={temperature} on:change={applyGrading} />
      </div>

      <div class="slider-item">
        <div class="slider-header">
          <span>Tint</span>
          <span class="slider-val">{tint.toFixed(0)}</span>
        </div>
        <input type="range" min="-50" max="50" step="1" bind:value={tint} on:change={applyGrading} />
      </div>

      <button class="reset-btn" on:click={resetGrading}>Reset Grade</button>
    </div>
  </div>

  <!-- Real-Time Scopes -->
  <div class="scopes-panel">
    <div class="scope-card">
      <span class="scope-header">RGB Histogram</span>
      <div class="hist-canvas">
        {#if scopes}
          <div class="hist-group">
            {#each scopes.histogram_r.slice(0, 48) as v}
              <div class="r-bar" style={`height: ${Math.min(v * 2, 45)}px;`}></div>
            {/each}
          </div>
          <div class="hist-group">
            {#each scopes.histogram_g.slice(0, 48) as v}
              <div class="g-bar" style={`height: ${Math.min(v * 2, 45)}px;`}></div>
            {/each}
          </div>
          <div class="hist-group">
            {#each scopes.histogram_b.slice(0, 48) as v}
              <div class="b-bar" style={`height: ${Math.min(v * 2, 45)}px;`}></div>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <div class="scope-card">
      <span class="scope-header">Luma Waveform &amp; Vectorscope</span>
      <div class="scope-mini-grid">
        <div class="waveform-box">
          <span class="scope-label">IRE 100</span>
          {#if scopes && scopes.histogram_luma}
            <div class="hist-group">
              {#each scopes.histogram_luma.slice(0, 32) as v}
                <div class="g-bar" style={`height: ${Math.max(2, Math.min(v * 2, 36))}px;`}></div>
              {/each}
            </div>
          {:else}
            <div class="wf-visual"></div>
          {/if}
          <span class="scope-label">IRE 0</span>
        </div>
        <div class="vectorscope-box">
          <div class="polar-crosshair" style={`transform: translate(${temperature * 0.25}px, ${-tint * 0.25}px);`}></div>
          <span class="cb-cr-label">Cb / Cr</span>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .color-panel {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 1rem;
    padding: 1rem;
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    height: 100%;
    overflow-y: auto;
  }

  .wheels-and-sliders {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .wheels-row {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 0.75rem;
  }

  .wheel-box {
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
  }

  .wheel-title {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
  }

  .wheel-circle {
    width: 80px;
    height: 80px;
    border-radius: 50%;
    background: radial-gradient(circle, #334155 0%, #0f172a 100%);
    border: 2px solid rgba(255, 255, 255, 0.15);
    position: relative;
  }

  .wheel-reticle {
    position: absolute;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-cyan);
    box-shadow: 0 0 6px var(--accent-cyan);
    transform: translate(-50%, -50%);
  }

  .rgb-sliders {
    display: flex;
    flex-direction: column;
    width: 100%;
    gap: 0.2rem;
    font-size: 0.65rem;
    color: var(--text-faint);
  }

  .rgb-sliders label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.3rem;
  }

  .rgb-sliders input {
    flex: 1;
    height: 4px;
  }

  .primary-sliders {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 0.75rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.75rem;
  }

  .slider-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .slider-header {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .slider-val {
    font-family: var(--font-mono);
    color: var(--accent-primary);
  }

  .slider-item input {
    width: 100%;
    cursor: pointer;
  }

  .reset-btn {
    grid-column: span 3;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    padding: 0.35rem;
    border-radius: var(--radius-sm);
    font-size: 0.75rem;
    margin-top: 0.25rem;
  }

  .reset-btn:hover {
    color: var(--accent-rose);
  }

  .scopes-panel {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .scope-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .scope-header {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .hist-canvas {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    height: 60px;
    background: #000;
    border-radius: var(--radius-sm);
    padding: 4px;
  }

  .hist-group {
    flex: 1;
    display: flex;
    align-items: flex-end;
    gap: 1px;
    height: 100%;
  }

  .r-bar { width: 2px; background: #ef4444; }
  .g-bar { width: 2px; background: #10b981; }
  .b-bar { width: 2px; background: #3b82f6; }

  .scope-mini-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }

  .waveform-box {
    height: 70px;
    background: #000;
    border-radius: var(--radius-sm);
    padding: 4px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }

  .scope-label {
    font-size: 0.55rem;
    font-family: var(--font-mono);
    color: var(--text-faint);
  }

  .wf-visual {
    height: 2px;
    background: #22c55e;
    opacity: 0.8;
  }

  .vectorscope-box {
    height: 70px;
    background: #000;
    border-radius: var(--radius-sm);
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .polar-crosshair {
    width: 40px;
    height: 40px;
    border: 1px dashed rgba(255, 255, 255, 0.2);
    border-radius: 50%;
  }

  .cb-cr-label {
    position: absolute;
    bottom: 4px;
    font-size: 0.55rem;
    font-family: var(--font-mono);
    color: var(--text-faint);
  }
</style>

