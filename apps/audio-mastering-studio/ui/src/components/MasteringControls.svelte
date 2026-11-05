<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  export let compressorThresholdDb: number = -18.0;
  export let compressorRatio: number = 2.5;
  export let compressorAttackMs: number = 25.0;
  export let compressorReleaseMs: number = 120.0;
  export let deReverbAmount: number = 0.15;
  export let targetLufs: number = -14.0;
  export let limiterCeilingDb: number = -0.5;
  export let isProcessing: boolean = false;

  const targetLufsPresets = [
    { label: 'Streaming (-14 LUFS)', val: -14.0 },
    { label: 'Broadcast EBU R128 (-23 LUFS)', val: -23.0 },
    { label: 'Podcast (-16 LUFS)', val: -16.0 },
    { label: 'Club / CD (-9 LUFS)', val: -9.0 },
  ];

  function applyMastering() {
    dispatch('remaster', {
      compressorThresholdDb,
      compressorRatio,
      compressorAttackMs,
      compressorReleaseMs,
      deReverbAmount,
      targetLufs,
      limiterCeilingDb,
    });
  }
</script>

<div class="mastering-controls-container">
  <div class="controls-header">
    <span class="title">DYNAMICS & TARGET LOUDNESS</span>
  </div>

  <div class="controls-body">
    <!-- Compressor Section -->
    <div class="control-box">
      <div class="box-title">DYNAMICS COMPRESSOR</div>
      <div class="slider-item">
        <div class="slider-head">
          <label for="comp-thresh">Threshold</label>
          <span class="val">{compressorThresholdDb.toFixed(1)} dB</span>
        </div>
        <input id="comp-thresh" type="range" min="-36" max="0" step="0.5" bind:value={compressorThresholdDb} />
      </div>

      <div class="slider-item">
        <div class="slider-head">
          <label for="comp-ratio">Ratio</label>
          <span class="val">{compressorRatio.toFixed(1)}:1</span>
        </div>
        <input id="comp-ratio" type="range" min="1.2" max="10.0" step="0.1" bind:value={compressorRatio} />
      </div>

      <div class="row-params">
        <div class="micro-param">
          <label for="comp-att">Attack</label>
          <input id="comp-att" type="number" min="1" max="100" bind:value={compressorAttackMs} />
          <span>ms</span>
        </div>
        <div class="micro-param">
          <label for="comp-rel">Release</label>
          <input id="comp-rel" type="number" min="20" max="500" bind:value={compressorReleaseMs} />
          <span>ms</span>
        </div>
      </div>
    </div>

    <!-- De-Reverb & Gating Section -->
    <div class="control-box">
      <div class="box-title">SPECTRAL DE-REVERB</div>
      <div class="slider-item">
        <div class="slider-head">
          <label for="de-rev">Tail Suppression</label>
          <span class="val">{(deReverbAmount * 100).toFixed(0)}%</span>
        </div>
        <input id="de-rev" type="range" min="0" max="0.8" step="0.05" bind:value={deReverbAmount} />
      </div>

      <div class="box-title mt-4">TARGET LOUDNESS PRESET</div>
      <div class="preset-select">
        <select bind:value={targetLufs}>
          {#each targetLufsPresets as p}
            <option value={p.val}>{p.label}</option>
          {/each}
        </select>
      </div>

      <div class="slider-item mt-2">
        <div class="slider-head">
          <label for="limit-ceil">Limiter Ceiling</label>
          <span class="val">{limiterCeilingDb.toFixed(1)} dBFS</span>
        </div>
        <input id="limit-ceil" type="range" min="-2.0" max="0.0" step="0.1" bind:value={limiterCeilingDb} />
      </div>
    </div>
  </div>

  <div class="actions-bar">
    <button class="btn-master" on:click={applyMastering} disabled={isProcessing}>
      {isProcessing ? 'CALCULATING EBU R128 MASTER...' : 'APPLY AI MASTERING'}
    </button>
  </div>
</div>

<style>
  .mastering-controls-container {
    display: flex;
    flex-direction: column;
    background: #11151c;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    overflow: hidden;
    height: 100%;
  }

  .controls-header {
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

  .controls-body {
    padding: 10px;
    display: flex;
    gap: 10px;
    flex: 1;
    background: #0d1016;
  }

  .control-box {
    flex: 1;
    background: #141a24;
    border: 1px solid #1e2838;
    border-radius: 6px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-family: var(--font-mono);
  }

  .box-title {
    font-size: 10px;
    font-weight: 700;
    color: #94a3b8;
    letter-spacing: 0.5px;
  }

  .slider-item {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .slider-head {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
  }

  label {
    color: #94a3b8;
  }

  .val {
    color: #f1f5f9;
    font-weight: 600;
  }

  input[type="range"] {
    accent-color: var(--accent-gold);
  }

  .row-params {
    display: flex;
    gap: 10px;
  }

  .micro-param {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
  }

  .micro-param input {
    width: 44px;
    background: #0b0e14;
    border: 1px solid #2e3e56;
    color: #f1f5f9;
    font-family: var(--font-mono);
    font-size: 10px;
    padding: 2px 4px;
    border-radius: 3px;
  }

  .preset-select select {
    width: 100%;
    background: #0b0e14;
    border: 1px solid #2e3e56;
    color: #f1f5f9;
    font-family: var(--font-mono);
    font-size: 10px;
    padding: 4px 6px;
    border-radius: 4px;
  }

  .mt-2 {
    margin-top: 6px;
  }

  .mt-4 {
    margin-top: 10px;
  }

  .actions-bar {
    padding: 8px 10px;
    background: #131822;
    border-top: 1px solid var(--border-color);
  }

  .btn-master {
    width: 100%;
    padding: 8px;
    background: #e5a93c;
    color: #0e1117;
    border: none;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-mono);
    letter-spacing: 0.5px;
    transition: opacity 0.15s;
  }

  .btn-master:hover:not(:disabled) {
    background: #f59e0b;
  }

  .btn-master:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>

