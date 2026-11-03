<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import {
    tauriInvoke,
    type ExportPreset,
    type TimelineProject,
  } from '../types';

  export let show: boolean = false;
  export let project: TimelineProject | null = null;

  const dispatch = createEventDispatcher<{
    close: void;
    refreshProject: void;
  }>();

  type ActiveTab = 'silence' | 'beats' | 'scenecut' | 'captions' | 'export';
  let activeTab: ActiveTab = 'silence';

  // Silence Cutter state
  let silenceThresholdDb: number = -35.0;
  let minSilenceMs: number = 400;
  let padMs: number = 80;
  let silenceRunning: boolean = false;
  let silenceResult: string | null = null;

  // Beat Detector state
  let beatThreshold: number = 0.25;
  let beatRunning: boolean = false;
  let beatResult: string | null = null;

  // Scene Cut state
  let sceneThreshold: number = 25.0;
  let sceneRunning: boolean = false;
  let sceneResult: string | null = null;

  // Captions state
  let captionsRunning: boolean = false;
  let captionsResult: string | null = null;

  // Export state
  let exportPresets: ExportPreset[] = [];
  let selectedPresetId: string = 'preset_1080p60';
  let exportInterchangeType: 'edl' | 'fcpxml' = 'edl';
  let exportOutputText: string = '';
  let exportRunning: boolean = false;
  let exportStatus: string | null = null;

  onMount(async () => {
    try {
      exportPresets = await tauriInvoke<ExportPreset[]>('get_export_presets_cmd');
      if (exportPresets.length > 0) {
        selectedPresetId = exportPresets[0].id;
      }
    } catch (e) {
      console.warn('Failed to fetch export presets:', e);
    }
  });

  async function runSilenceRippleCut() {
    if (!project) return;
    silenceRunning = true;
    silenceResult = null;
    try {
      // Find first audio track
      const audioTrack = project.tracks.find(t => t.track_type === 'Audio') || project.tracks[0];
      if (!audioTrack) {
        silenceResult = 'No track found in project to ripple cut.';
        silenceRunning = false;
        return;
      }
      const intervals = await tauriInvoke<Array<[number, number]>>('ai_ripple_cut_silence_cmd', {
        trackId: audioTrack.id,
        thresholdDb: silenceThresholdDb,
        minDurationMs: minSilenceMs,
        padMs: padMs,
      });
      silenceResult = `Detected and removed ${intervals.length} silent segments. Timeline rippled seamlessly.`;
      dispatch('refreshProject');
    } catch (e) {
      silenceResult = `Silence cut error: ${e}`;
    } finally {
      silenceRunning = false;
    }
  }

  async function runBeatDetection() {
    beatRunning = true;
    beatResult = null;
    try {
      const beats = await tauriInvoke<number[]>('ai_detect_beats_cmd', {
        threshold: beatThreshold,
      });
      beatResult = `Successfully detected ${beats.length} rhythm transients. Markers added to timeline.`;
      dispatch('refreshProject');
    } catch (e) {
      beatResult = `Beat detection error: ${e}`;
    } finally {
      beatRunning = false;
    }
  }

  async function runSceneCutDetection() {
    sceneRunning = true;
    sceneResult = null;
    try {
      const cuts = await tauriInvoke<number[]>('ai_detect_scene_cuts_cmd', {
        threshold: sceneThreshold,
      });
      sceneResult = `Analyzed video frames: found ${cuts.length} scene boundaries.`;
      dispatch('refreshProject');
    } catch (e) {
      sceneResult = `Scene cut detection error: ${e}`;
    } finally {
      sceneRunning = false;
    }
  }

  async function runAutoCaptions() {
    captionsRunning = true;
    captionsResult = null;
    try {
      const subs = await tauriInvoke<Array<[number, number, string]>>('ai_generate_auto_captions_cmd', {
        sampleRate: 48000,
      });
      captionsResult = `Generated ${subs.length} offline speech-to-text subtitle cues.`;
      dispatch('refreshProject');
    } catch (e) {
      captionsResult = `Auto-captions error: ${e}`;
    } finally {
      captionsRunning = false;
    }
  }

  async function handleExportInterchange() {
    exportRunning = true;
    exportStatus = null;
    try {
      if (exportInterchangeType === 'edl') {
        exportOutputText = await tauriInvoke<string>('export_edl_cmd');
        exportStatus = 'CMX 3600 EDL exported successfully.';
      } else {
        exportOutputText = await tauriInvoke<string>('export_fcpxml_cmd');
        exportStatus = 'Final Cut Pro XML (FCPXML) exported successfully.';
      }
    } catch (e) {
      exportStatus = `Interchange export failed: ${e}`;
    } finally {
      exportRunning = false;
    }
  }

  async function handleRenderMaster() {
    exportRunning = true;
    exportStatus = 'Rendering broadcast master offline...';
    try {
      const outPath = `exports/master_${Date.now()}.wav`;
      const res = await tauriInvoke<string>('render_audio_master_cmd', {
        outputPath: outPath,
      });
      exportStatus = `Broadcast render complete: ${res}`;
    } catch (e) {
      exportStatus = `Export render failed: ${e}`;
    } finally {
      exportRunning = false;
    }
  }
</script>

{#if show}
  <div class="modal-backdrop" on:click|self={() => dispatch('close')} role="presentation">
    <div class="modal-dialog">
      <!-- Modal Header -->
      <div class="modal-header">
        <div class="title-with-badge">
          <span class="ai-spark-icon">✨</span>
          <h2>AI Studio Tools &amp; Master Exporter</h2>
          <span class="offline-tag">100% OFFLINE</span>
        </div>
        <button class="close-btn" on:click={() => dispatch('close')} title="Close Modal">✕</button>
      </div>

      <!-- Tabs Navigation -->
      <div class="modal-tabs">
        <button
          class="tab-link"
          class:active={activeTab === 'silence'}
          on:click={() => (activeTab = 'silence')}
        >
          🔇 Silence Ripple-Cut
        </button>
        <button
          class="tab-link"
          class:active={activeTab === 'beats'}
          on:click={() => (activeTab = 'beats')}
        >
          🥁 Beat Detection
        </button>
        <button
          class="tab-link"
          class:active={activeTab === 'scenecut'}
          on:click={() => (activeTab = 'scenecut')}
        >
          🎬 Scene-Cut Detection
        </button>
        <button
          class="tab-link"
          class:active={activeTab === 'captions'}
          on:click={() => (activeTab = 'captions')}
        >
          💬 Auto-Captions
        </button>
        <button
          class="tab-link"
          class:active={activeTab === 'export'}
          on:click={() => (activeTab = 'export')}
        >
          📦 Master &amp; Interchange
        </button>
      </div>

      <!-- Modal Content Body -->
      <div class="modal-body">
        <!-- Silence Cutter Tab -->
        {#if activeTab === 'silence'}
          <div class="tool-pane">
            <p class="tool-desc">
              Automatically identify dead air, hesitations, and pauses across recorded takes.
              Ripple cuts all subsequent audio/video tracks while preserving natural conversational cadence with speech padding.
            </p>
            <div class="controls-grid">
              <div class="control-field">
                <label for="silence-thresh">Silence Threshold: {silenceThresholdDb.toFixed(1)} dB</label>
                <input
                  id="silence-thresh"
                  type="range"
                  min="-60"
                  max="-15"
                  step="1"
                  bind:value={silenceThresholdDb}
                  class="slider"
                />
              </div>
              <div class="control-field">
                <label for="min-silence">Minimum Silence: {minSilenceMs} ms</label>
                <input
                  id="min-silence"
                  type="range"
                  min="100"
                  max="1500"
                  step="50"
                  bind:value={minSilenceMs}
                  class="slider"
                />
              </div>
              <div class="control-field">
                <label for="speech-pad">Speech Safety Pad: {padMs} ms</label>
                <input
                  id="speech-pad"
                  type="range"
                  min="20"
                  max="250"
                  step="10"
                  bind:value={padMs}
                  class="slider"
                />
              </div>
            </div>
            <div class="action-footer">
              <button
                class="primary-action-btn"
                disabled={silenceRunning}
                on:click={runSilenceRippleCut}
              >
                {silenceRunning ? 'Analyzing Waveforms...' : '✂️ Execute AI Silence Ripple-Cut'}
              </button>
              {#if silenceResult}
                <div class="result-banner">{silenceResult}</div>
              {/if}
            </div>
          </div>
        {/if}

        <!-- Beat Detection Tab -->
        {#if activeTab === 'beats'}
          <div class="tool-pane">
            <p class="tool-desc">
              Analyze audio tracks with multi-band onset energy difference equations to detect musical beats,
              transients, and downbeats. Adds color-coded snap markers directly to the project timeline.
            </p>
            <div class="controls-grid">
              <div class="control-field">
                <label for="beat-thresh">Onset Sensitivity Threshold: {beatThreshold.toFixed(2)}</label>
                <input
                  id="beat-thresh"
                  type="range"
                  min="0.05"
                  max="0.8"
                  step="0.05"
                  bind:value={beatThreshold}
                  class="slider"
                />
              </div>
            </div>
            <div class="action-footer">
              <button
                class="primary-action-btn"
                disabled={beatRunning}
                on:click={runBeatDetection}
              >
                {beatRunning ? 'Computing Spectral Flux...' : '🎵 Detect Musical Beats & Snap Points'}
              </button>
              {#if beatResult}
                <div class="result-banner">{beatResult}</div>
              {/if}
            </div>
          </div>
        {/if}

        <!-- Scene Cut Tab -->
        {#if activeTab === 'scenecut'}
          <div class="tool-pane">
            <p class="tool-desc">
              Scan raw camera footage or rendered clips using 64-bit difference hashing (dHash) and 3D RGB color histogram
              distance metrics to find abrupt shot cuts without manual scrubbing.
            </p>
            <div class="controls-grid">
              <div class="control-field">
                <label for="scene-thresh">Visual Difference Threshold: {sceneThreshold.toFixed(1)}</label>
                <input
                  id="scene-thresh"
                  type="range"
                  min="10.0"
                  max="60.0"
                  step="2.5"
                  bind:value={sceneThreshold}
                  class="slider"
                />
              </div>
            </div>
            <div class="action-footer">
              <button
                class="primary-action-btn"
                disabled={sceneRunning}
                on:click={runSceneCutDetection}
              >
                {sceneRunning ? 'Scanning Video Frames...' : '🔍 Detect Scene Transitions'}
              </button>
              {#if sceneResult}
                <div class="result-banner">{sceneResult}</div>
              {/if}
            </div>
          </div>
        {/if}

        <!-- Captions Tab -->
        {#if activeTab === 'captions'}
          <div class="tool-pane">
            <p class="tool-desc">
              Generate timed subtitles and closed captions completely offline using local acoustic models.
              Adds a dedicated Subtitle Track with synchronized text overlays.
            </p>
            <div class="action-footer">
              <button
                class="primary-action-btn"
                disabled={captionsRunning}
                on:click={runAutoCaptions}
              >
                {captionsRunning ? 'Transcribing Speech Offline...' : '📝 Generate Auto-Captions Track'}
              </button>
              {#if captionsResult}
                <div class="result-banner">{captionsResult}</div>
              {/if}
            </div>
          </div>
        {/if}

        <!-- Export & Interchange Tab -->
        {#if activeTab === 'export'}
          <div class="tool-pane">
            <div class="export-split">
              <!-- Broadcast Master Export -->
              <div class="export-col">
                <h3>Master Export Presets</h3>
                <p class="sub-text">Render full timeline mixdown to pristine lossless or broadcast formats.</p>
                <div class="preset-selector">
                  {#each exportPresets as preset}
                    <label class="preset-option" class:selected={selectedPresetId === preset.id}>
                      <input
                        type="radio"
                        name="preset"
                        value={preset.id}
                        bind:group={selectedPresetId}
                      />
                      <div class="preset-meta">
                        <span class="preset-name">{preset.name}</span>
                        <span class="preset-specs">{preset.width}x{preset.height} • {preset.container.toUpperCase()}</span>
                      </div>
                    </label>
                  {/each}
                </div>
                <button
                  class="primary-action-btn"
                  disabled={exportRunning}
                  on:click={handleRenderMaster}
                >
                  🚀 Render Broadcast Master
                </button>
              </div>

              <!-- NLE Interchange -->
              <div class="export-col">
                <h3>NLE Interchange (EDL / FCPXML)</h3>
                <p class="sub-text">Export multi-track timeline cuts to DaVinci Resolve, Premiere Pro, or Final Cut Pro.</p>
                <div class="interchange-type-selector">
                  <button
                    class="type-btn"
                    class:active={exportInterchangeType === 'edl'}
                    on:click={() => (exportInterchangeType = 'edl')}
                  >
                    CMX 3600 EDL
                  </button>
                  <button
                    class="type-btn"
                    class:active={exportInterchangeType === 'fcpxml'}
                    on:click={() => (exportInterchangeType = 'fcpxml')}
                  >
                    Final Cut Pro XML
                  </button>
                </div>
                <button
                  class="secondary-action-btn"
                  disabled={exportRunning}
                  on:click={handleExportInterchange}
                >
                  Generate {exportInterchangeType.toUpperCase()}
                </button>
                {#if exportOutputText}
                  <div class="code-preview-box">
                    <pre><code>{exportOutputText}</code></pre>
                  </div>
                {/if}
              </div>
            </div>
            {#if exportStatus}
              <div class="result-banner">{exportStatus}</div>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-dialog {
    width: 90%;
    max-width: 820px;
    max-height: 85vh;
    background: #11141c;
    border: 1px solid #2d3748;
    border-radius: 12px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    background: #181d28;
    border-bottom: 1px solid #2d3748;
  }

  .title-with-badge {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .ai-spark-icon {
    font-size: 1.25rem;
  }

  .modal-header h2 {
    font-size: 1.1rem;
    font-weight: 700;
    color: #f3f4f6;
    margin: 0;
  }

  .offline-tag {
    font-size: 0.65rem;
    font-weight: 800;
    padding: 2px 6px;
    background: #065f46;
    color: #34d399;
    border-radius: 4px;
    letter-spacing: 0.05em;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 1.2rem;
    cursor: pointer;
    line-height: 1;
    padding: 4px 8px;
    border-radius: 4px;
    transition: background 0.15s, color 0.15s;
  }

  .close-btn:hover {
    background: #2d3748;
    color: #f8fafc;
  }

  .modal-tabs {
    display: flex;
    background: #0f131a;
    border-bottom: 1px solid #2d3748;
    overflow-x: auto;
  }

  .tab-link {
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: #94a3b8;
    padding: 12px 18px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s;
  }

  .tab-link:hover {
    color: #f1f5f9;
  }

  .tab-link.active {
    color: #38bdf8;
    border-bottom-color: #38bdf8;
    background: rgba(56, 189, 248, 0.05);
  }

  .modal-body {
    padding: 20px;
    overflow-y: auto;
    flex: 1;
  }

  .tool-desc {
    color: #94a3b8;
    font-size: 0.9rem;
    line-height: 1.5;
    margin-bottom: 20px;
  }

  .controls-grid {
    display: flex;
    flex-direction: column;
    gap: 16px;
    background: #181d28;
    padding: 16px;
    border-radius: 8px;
    border: 1px solid #262f3f;
    margin-bottom: 20px;
  }

  .control-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .control-field label {
    font-size: 0.8rem;
    font-weight: 600;
    color: #cbd5e1;
  }

  .slider {
    accent-color: #38bdf8;
    width: 100%;
    cursor: pointer;
  }

  .action-footer {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .primary-action-btn {
    background: #0284c7;
    color: #ffffff;
    border: none;
    padding: 10px 18px;
    border-radius: 6px;
    font-size: 0.88rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  .primary-action-btn:hover:not(:disabled) {
    background: #0369a1;
  }

  .primary-action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .secondary-action-btn {
    background: #334155;
    color: #f8fafc;
    border: 1px solid #475569;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s;
    width: 100%;
    margin-bottom: 12px;
  }

  .secondary-action-btn:hover:not(:disabled) {
    background: #475569;
  }

  .result-banner {
    padding: 10px 14px;
    background: rgba(34, 197, 94, 0.12);
    border: 1px solid #16a34a;
    border-radius: 6px;
    color: #86efac;
    font-size: 0.85rem;
  }

  .export-split {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 24px;
    margin-bottom: 16px;
  }

  .export-col h3 {
    font-size: 0.95rem;
    font-weight: 700;
    color: #f3f4f6;
    margin-bottom: 4px;
  }

  .sub-text {
    font-size: 0.8rem;
    color: #94a3b8;
    margin-bottom: 14px;
  }

  .preset-selector {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 16px;
  }

  .preset-option {
    display: flex;
    align-items: center;
    gap: 10px;
    background: #181d28;
    border: 1px solid #2d3748;
    padding: 10px 12px;
    border-radius: 6px;
    cursor: pointer;
    transition: border-color 0.15s;
  }

  .preset-option.selected {
    border-color: #38bdf8;
    background: rgba(56, 189, 248, 0.08);
  }

  .preset-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .preset-name {
    font-size: 0.85rem;
    font-weight: 600;
    color: #f8fafc;
  }

  .preset-specs {
    font-size: 0.72rem;
    color: #94a3b8;
  }

  .interchange-type-selector {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
  }

  .type-btn {
    flex: 1;
    background: #1e293b;
    border: 1px solid #334155;
    color: #94a3b8;
    padding: 6px 10px;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .type-btn.active {
    background: #0284c7;
    border-color: #0284c7;
    color: #ffffff;
  }

  .code-preview-box {
    max-height: 180px;
    overflow-y: auto;
    background: #0a0d14;
    border: 1px solid #1e293b;
    border-radius: 6px;
    padding: 10px;
  }

  .code-preview-box pre {
    margin: 0;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 0.75rem;
    color: #cbd5e1;
    white-space: pre-wrap;
  }
</style>
