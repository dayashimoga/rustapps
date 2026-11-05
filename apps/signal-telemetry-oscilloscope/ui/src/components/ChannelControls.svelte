<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  export let ch1Type: string = 'Sine';
  export let ch1Freq: number = 1000;
  export let ch1Amp: number = 3.3;
  export let ch1VoltsPerDiv: number = 1.0;

  export let ch2Type: string = 'Square';
  export let ch2Freq: number = 500;
  export let ch2Amp: number = 3.3;
  export let ch2VoltsPerDiv: number = 1.0;

  export let timePerDivMs: number = 1.0;
  export let triggerLevel: number = 0.0;
  export let triggerEdge: string = 'Rising';
  export let isRunning: boolean = true;

  const waveformOptions = ['Sine', 'Square', 'Triangle', 'Sawtooth', 'Noise', 'UartStream'];
  const voltsDivOptions = [0.2, 0.5, 1.0, 2.0, 5.0];
  const timeDivOptions = [0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0];

  function triggerSweep() {
    dispatch('acquire', {
      ch1Type,
      ch1Freq,
      ch1Amp,
      ch1VoltsPerDiv,
      ch2Type,
      ch2Freq,
      ch2Amp,
      ch2VoltsPerDiv,
      timePerDivMs,
      triggerLevel,
      triggerEdge,
    });
  }

  function toggleRun() {
    isRunning = !isRunning;
    dispatch('toggleRun', { isRunning });
  }
</script>

<div class="channel-controls-container">
  <div class="control-header">
    <h3>ACQUISITION & CHANNELS</h3>
    <div class="sweep-actions">
      <button class="run-btn" class:running={isRunning} on:click={toggleRun}>
        {isRunning ? 'RUNNING' : 'STOPPED'}
      </button>
      <button class="single-btn" on:click={triggerSweep}>SINGLE SWEEP</button>
    </div>
  </div>

  <!-- Timebase Section -->
  <div class="control-group">
    <div class="group-title">HORIZONTAL TIMEBASE</div>
    <div class="input-row">
      <label for="timebase-select">Time / Div</label>
      <select id="timebase-select" bind:value={timePerDivMs} on:change={triggerSweep}>
        {#each timeDivOptions as td}
          <option value={td}>{td} ms/div</option>
        {/each}
      </select>
    </div>
  </div>

  <!-- Channel 1 Section -->
  <div class="control-group ch1-group">
    <div class="group-title ch1-title">CHANNEL 1 (ANALOG / SENSOR)</div>
    <div class="input-row">
      <label for="ch1-vdiv">Volts / Div</label>
      <select id="ch1-vdiv" bind:value={ch1VoltsPerDiv} on:change={triggerSweep}>
        {#each voltsDivOptions as vd}
          <option value={vd}>{vd} V/div</option>
        {/each}
      </select>
    </div>
    <div class="input-row">
      <label for="ch1-wf">Waveform</label>
      <select id="ch1-wf" bind:value={ch1Type} on:change={triggerSweep}>
        {#each waveformOptions as wf}
          <option value={wf}>{wf}</option>
        {/each}
      </select>
    </div>
    <div class="slider-row">
      <div class="slider-header">
        <span>Frequency</span>
        <span class="slider-val">{ch1Freq} Hz</span>
      </div>
      <input type="range" min="50" max="10000" step="50" bind:value={ch1Freq} on:change={triggerSweep} />
    </div>
    <div class="slider-row">
      <div class="slider-header">
        <span>Amplitude</span>
        <span class="slider-val">{ch1Amp.toFixed(1)} V</span>
      </div>
      <input type="range" min="0.5" max="10.0" step="0.1" bind:value={ch1Amp} on:change={triggerSweep} />
    </div>
  </div>

  <!-- Channel 2 Section -->
  <div class="control-group ch2-group">
    <div class="group-title ch2-title">CHANNEL 2 (DIGITAL / BUS)</div>
    <div class="input-row">
      <label for="ch2-vdiv">Volts / Div</label>
      <select id="ch2-vdiv" bind:value={ch2VoltsPerDiv} on:change={triggerSweep}>
        {#each voltsDivOptions as vd}
          <option value={vd}>{vd} V/div</option>
        {/each}
      </select>
    </div>
    <div class="input-row">
      <label for="ch2-wf">Waveform</label>
      <select id="ch2-wf" bind:value={ch2Type} on:change={triggerSweep}>
        {#each waveformOptions as wf}
          <option value={wf}>{wf}</option>
        {/each}
      </select>
    </div>
    <div class="slider-row">
      <div class="slider-header">
        <span>Frequency</span>
        <span class="slider-val">{ch2Freq} Hz</span>
      </div>
      <input type="range" min="50" max="10000" step="50" bind:value={ch2Freq} on:change={triggerSweep} />
    </div>
    <div class="slider-row">
      <div class="slider-header">
        <span>Amplitude</span>
        <span class="slider-val">{ch2Amp.toFixed(1)} V</span>
      </div>
      <input type="range" min="0.5" max="10.0" step="0.1" bind:value={ch2Amp} on:change={triggerSweep} />
    </div>
  </div>

  <!-- Trigger Section -->
  <div class="control-group">
    <div class="group-title">HARDWARE TRIGGER</div>
    <div class="input-row">
      <label for="trig-edge">Edge</label>
      <select id="trig-edge" bind:value={triggerEdge} on:change={triggerSweep}>
        <option value="Rising">Rising Edge (↑)</option>
        <option value="Falling">Falling Edge (↓)</option>
        <option value="Either">Either Edge (↕)</option>
      </select>
    </div>
    <div class="slider-row">
      <div class="slider-header">
        <span>Trigger Level</span>
        <span class="slider-val">{triggerLevel.toFixed(2)} V</span>
      </div>
      <input type="range" min="-5.0" max="5.0" step="0.1" bind:value={triggerLevel} on:change={triggerSweep} />
    </div>
  </div>
</div>

<style>
  .channel-controls-container {
    display: flex;
    flex-direction: column;
    gap: 12px;
    background: #0d131a;
    border: 1px solid #1e2c3d;
    border-radius: 8px;
    padding: 14px;
    height: 100%;
    overflow-y: auto;
    font-size: 11px;
    font-family: var(--font-mono);
  }

  .control-header {
    display: flex;
    flex-direction: column;
    gap: 10px;
    border-bottom: 1px solid #1e2c3d;
    padding-bottom: 12px;
  }

  h3 {
    font-size: 12px;
    font-weight: 700;
    color: #e6edf3;
    letter-spacing: 0.5px;
  }

  .sweep-actions {
    display: flex;
    gap: 8px;
  }

  .run-btn {
    flex: 1;
    padding: 8px 10px;
    background: #238636;
    color: #fff;
    border: 1px solid #2ea043;
    border-radius: 4px;
    font-weight: 700;
    font-size: 11px;
    transition: background 0.15s;
  }

  .run-btn:not(.running) {
    background: #da3633;
    border-color: #f85149;
  }

  .single-btn {
    flex: 1;
    padding: 8px 10px;
    background: #1f6feb;
    color: #fff;
    border: 1px solid #388bfd;
    border-radius: 4px;
    font-weight: 700;
    font-size: 11px;
  }

  .control-group {
    background: #141c26;
    border: 1px solid #1e2c3d;
    border-radius: 6px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .group-title {
    font-size: 10px;
    font-weight: 700;
    color: #8b949e;
    letter-spacing: 0.5px;
  }

  .ch1-title {
    color: #00ff66;
  }

  .ch2-title {
    color: #00d4ff;
  }

  .input-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  label {
    color: #8b949e;
  }

  select {
    background: #0d131a;
    border: 1px solid #2a3d54;
    color: #e6edf3;
    border-radius: 4px;
    padding: 3px 6px;
    font-size: 11px;
  }

  .slider-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .slider-header {
    display: flex;
    justify-content: space-between;
    color: #8b949e;
    font-size: 10px;
  }

  .slider-val {
    color: #e6edf3;
    font-weight: 600;
  }

  input[type="range"] {
    accent-color: #58a6ff;
  }
</style>

