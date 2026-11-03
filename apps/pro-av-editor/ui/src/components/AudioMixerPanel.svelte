<script lang="ts">
  import { onMount } from 'svelte';
  import {
    tauriInvoke,
    type LoudnessMeterData,
    type TimelineProject,
  } from '../types';

  export let project: TimelineProject | null = null;

  let loudness: LoudnessMeterData = {
    momentary_lufs: -18.4,
    short_term_lufs: -19.2,
    integrated_lufs: -23.0,
    true_peak_dbfs: -1.2,
  };

  onMount(async () => {
    try {
      loudness = await tauriInvoke<LoudnessMeterData>('get_audio_loudness_cmd', {
        startMs: 0,
        endMs: 2000,
      });
    } catch (e) {
      console.error('Failed to get loudness readings:', e);
    }
  });

  // 8 default EQ frequency bands
  const EQ_FREQS = [40, 100, 250, 800, 2000, 5000, 10000, 18000];
  let eqGains = [0, 0, 0, 0, 0, 0, 0, 0];

  async function applyEqUpdate() {
    try {
      loudness = await tauriInvoke<LoudnessMeterData>('update_master_eq_cmd', {
        gainsDb: eqGains,
      });
    } catch (e) {
      console.error('Failed to update EQ:', e);
    }
  }

  async function handleTrackVolume(trackId: string, volume: number) {
    try {
      await tauriInvoke('update_track_state_cmd', { trackId, volume });
      loudness = await tauriInvoke<LoudnessMeterData>('get_audio_loudness_cmd', {
        startMs: 0,
        endMs: 2000,
      });
    } catch (e) {
      console.error('Failed to update track volume:', e);
    }
  }

  async function handleTrackPan(trackId: string, pan: number) {
    try {
      await tauriInvoke('update_track_state_cmd', { trackId, pan });
    } catch (e) {
      console.error('Failed to update track pan:', e);
    }
  }
</script>

<div class="audio-mixer-container">
  <!-- Track Channel Strips -->
  <div class="channel-strips">
    {#if project}
      {#each project.tracks.filter(t => t.track_type === 'Audio') as track, i}
        <div class="channel-strip">
          <span class="track-title">{track.name}</span>
          <div class="pan-control">
            <span class="pan-tag">Pan</span>
            <input
              type="range"
              min="-1.0"
              max="1.0"
              step="0.05"
              value="0.0"
              class="pan-slider"
              on:change={(e) => handleTrackPan(track.id, parseFloat(e.currentTarget.value))}
            />
          </div>
          <div class="fader-box">
            <input
              type="range"
              orient="vertical"
              min="0.0"
              max="1.5"
              step="0.02"
              value="1.0"
              class="fader"
              on:change={(e) => handleTrackVolume(track.id, parseFloat(e.currentTarget.value))}
            />
            <div class="meter-bar">
              <div class="meter-fill" style={`height: ${Math.max(10, 80 - i * 15)}%;`}></div>
            </div>
          </div>
          <div class="strip-footer">
            <span class="gain-val">0.0 dB</span>
          </div>
        </div>
      {/each}
    {/if}

    <!-- Master Output Bus Strip -->
    <div class="channel-strip master-strip">
      <span class="track-title master-title">MASTER</span>
      <div class="pan-control">
        <span class="pan-tag">Stereo</span>
      </div>
      <div class="fader-box">
        <input type="range" orient="vertical" min="0.0" max="1.5" step="0.02" value="1.0" class="fader" />
        <div class="meter-bar master-meter">
          <div class="meter-fill master-fill" style="height: 72%;"></div>
        </div>
      </div>
      <div class="strip-footer">
        <span class="gain-val">0.0 dB</span>
      </div>
    </div>
  </div>

  <!-- Master Processing: 8-Band EQ & EBU R128 Meters -->
  <div class="master-processing">
    <!-- 8-Band Parametric EQ Graph -->
    <div class="eq-section">
      <div class="eq-header">
        <span class="section-title">8-Band Parametric Equalizer</span>
        <button class="reset-eq-btn" on:click={() => { eqGains = [0, 0, 0, 0, 0, 0, 0, 0]; applyEqUpdate(); }}>
          Flat EQ
        </button>
      </div>
      <div class="eq-bands-grid">
        {#each EQ_FREQS as freq, idx}
          <div class="eq-band-col">
            <span class="band-gain">{eqGains[idx] > 0 ? `+${eqGains[idx]}` : eqGains[idx]} dB</span>
            <input
              type="range"
              orient="vertical"
              min="-12"
              max="12"
              step="0.5"
              bind:value={eqGains[idx]}
              on:change={applyEqUpdate}
              class="eq-slider"
            />
            <span class="band-freq">{freq >= 1000 ? `${freq / 1000}k` : freq}</span>
          </div>
        {/each}
      </div>
    </div>

    <!-- EBU R128 Broadcast Loudness Meter -->
    <div class="loudness-section">
      <span class="section-title">EBU R128 / ITU-R BS.1770 Broadcast Loudness</span>
      <div class="loudness-grid">
        <div class="meter-stat-card">
          <span class="stat-label">Integrated</span>
          <span class="stat-value" class:target-match={Math.abs(loudness.integrated_lufs - -23.0) < 1.0}>
            {loudness.integrated_lufs.toFixed(1)} LUFS
          </span>
          <span class="stat-sub">Target: -23.0 LUFS</span>
        </div>

        <div class="meter-stat-card">
          <span class="stat-label">Short-Term (3s)</span>
          <span class="stat-value">{loudness.short_term_lufs.toFixed(1)} LUFS</span>
        </div>

        <div class="meter-stat-card">
          <span class="stat-label">Momentary (400ms)</span>
          <span class="stat-value">{loudness.momentary_lufs.toFixed(1)} LUFS</span>
        </div>

        <div class="meter-stat-card">
          <span class="stat-label">True Peak</span>
          <span class="stat-value" class:peak-warning={loudness.true_peak_dbfs > -1.0}>
            {loudness.true_peak_dbfs.toFixed(1)} dBFS
          </span>
          <span class="stat-sub">Max: -1.0 dBFS</span>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .audio-mixer-container {
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: 1rem;
    padding: 1rem;
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    height: 100%;
    overflow-y: auto;
  }

  .channel-strips {
    display: flex;
    gap: 0.5rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.75rem;
    overflow-x: auto;
  }

  .channel-strip {
    width: 68px;
    display: flex;
    flex-direction: column;
    align-items: center;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.5rem 0.25rem;
    gap: 0.5rem;
  }

  .master-strip {
    border-color: var(--accent-amber);
    background: rgba(245, 158, 11, 0.08);
  }

  .track-title {
    font-size: 0.65rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .master-title {
    color: var(--accent-amber);
  }

  .pan-control {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .pan-tag {
    font-size: 0.55rem;
    color: var(--text-faint);
  }

  .pan-slider {
    width: 44px;
    height: 3px;
  }

  .fader-box {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 120px;
  }

  .fader {
    writing-mode: vertical-lr;
    direction: rtl;
    width: 8px;
    height: 100%;
    cursor: pointer;
  }

  .meter-bar {
    width: 6px;
    height: 100%;
    background: #000;
    border-radius: 2px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    overflow: hidden;
  }

  .meter-fill {
    width: 100%;
    background: linear-gradient(to top, #10b981 70%, #f59e0b 90%, #ef4444 100%);
  }

  .strip-footer {
    font-family: var(--font-mono);
    font-size: 0.6rem;
    color: var(--text-faint);
  }

  .master-processing {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .eq-section, .loudness-section {
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .eq-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .section-title {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .reset-eq-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    font-size: 0.7rem;
    padding: 0.2rem 0.5rem;
    border-radius: var(--radius-sm);
  }

  .eq-bands-grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 0.5rem;
    align-items: center;
    background: #000;
    border-radius: var(--radius-sm);
    padding: 0.75rem 0.5rem;
  }

  .eq-band-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.35rem;
  }

  .band-gain, .band-freq {
    font-family: var(--font-mono);
    font-size: 0.6rem;
    color: var(--text-faint);
  }

  .band-gain { color: var(--accent-cyan); }

  .eq-slider {
    writing-mode: vertical-lr;
    direction: rtl;
    width: 6px;
    height: 80px;
    cursor: pointer;
  }

  .loudness-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.75rem;
  }

  .meter-stat-card {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .stat-label {
    font-size: 0.65rem;
    color: var(--text-muted);
  }

  .stat-value {
    font-family: var(--font-mono);
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-main);
  }

  .target-match { color: var(--accent-emerald); }
  .peak-warning { color: var(--accent-rose); }

  .stat-sub {
    font-size: 0.55rem;
    color: var(--text-faint);
    font-family: var(--font-mono);
  }
</style>

