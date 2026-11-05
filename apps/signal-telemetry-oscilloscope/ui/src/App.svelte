<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import OscilloscopeDisplay from './components/OscilloscopeDisplay.svelte';
  import SpectrumAnalyzer from './components/SpectrumAnalyzer.svelte';
  import ProtocolDecoderTable from './components/ProtocolDecoderTable.svelte';
  import ChannelControls from './components/ChannelControls.svelte';
  import LicenseModal from './components/LicenseModal.svelte';

  interface OscilloscopeStats {
    total_captures: number;
    total_samples_analyzed: number;
    total_packets_decoded: number;
    avg_dominant_freq_hz: number;
    avg_snr_db: number;
  }

  interface CaptureSession {
    session_id: string;
    title: string;
    created_at: number;
    sample_rate_hz: number;
    total_samples: number;
    time_per_div_ms: number;
    ch1_samples: number[];
    ch2_samples: number[];
    digital_samples: number[];
    stats_ch1: {
      vpp: number;
      vrms: number;
      vmin: number;
      vmax: number;
      vmean: number;
      frequency_hz: number;
      period_ms: number;
      duty_cycle_percent: number;
    };
    stats_ch2: {
      vpp: number;
      vrms: number;
      vmin: number;
      vmax: number;
      vmean: number;
      frequency_hz: number;
      period_ms: number;
      duty_cycle_percent: number;
    };
    spectrum: {
      sample_rate_hz: number;
      bin_frequencies: number[];
      magnitudes_db: number[];
      dominant_freq_hz: number;
      snr_db: number;
      thd_percent: number;
    };
    decoded_packets: any[];
  }

  let stats: OscilloscopeStats = {
    total_captures: 0,
    total_samples_analyzed: 0,
    total_packets_decoded: 0,
    avg_dominant_freq_hz: 0,
    avg_snr_db: 0,
  };

  let currentSession: CaptureSession | null = null;
  let activeTab: 'spectrum' | 'decoder' = 'spectrum';
  let isLicenseModalOpen: boolean = false;
  let licenseTier: string = 'Free';
  let isRunning: boolean = true;
  let intervalTimer: any = null;

  // Acquisition params
  let ch1Type = 'Sine';
  let ch1Freq = 1000;
  let ch1Amp = 3.3;
  let ch1VoltsPerDiv = 1.0;

  let ch2Type = 'Square';
  let ch2Freq = 500;
  let ch2Amp = 3.3;
  let ch2VoltsPerDiv = 1.0;

  let timePerDivMs = 1.0;
  let triggerLevel = 0.0;
  let triggerEdge = 'Rising';

  async function loadInitialData() {
    try {
      stats = await invokeIpc<OscilloscopeStats>('get_oscilloscope_stats');
      currentSession = await invokeIpc<CaptureSession>('get_capture', { session_id: 'scope-demo' });
      const lic = await invokeIpc<any>('get_license_status');
      if (lic && lic.tier) {
        licenseTier = lic.tier;
      }
    } catch (e) {
      console.error('Failed to load initial oscilloscope data:', e);
    }
  }

  async function triggerSweep(e?: CustomEvent) {
    if (e && e.detail) {
      ch1Type = e.detail.ch1Type;
      ch1Freq = e.detail.ch1Freq;
      ch1Amp = e.detail.ch1Amp;
      ch1VoltsPerDiv = e.detail.ch1VoltsPerDiv;
      ch2Type = e.detail.ch2Type;
      ch2Freq = e.detail.ch2Freq;
      ch2Amp = e.detail.ch2Amp;
      ch2VoltsPerDiv = e.detail.ch2VoltsPerDiv;
      timePerDivMs = e.detail.timePerDivMs;
      triggerLevel = e.detail.triggerLevel;
      triggerEdge = e.detail.triggerEdge;
    }

    try {
      const session = await invokeIpc<CaptureSession>('acquire_capture', {
        title: 'Live Oscilloscope Sweep',
        sample_rate_hz: 50000.0,
        num_samples: 1000,
        ch1_type: ch1Type,
        ch1_freq: ch1Freq,
        ch1_amp: ch1Amp,
        ch2_type: ch2Type,
        ch2_freq: ch2Freq,
        ch2_amp: ch2Amp,
        trigger: {
          channel: 'Ch1',
          level_volts: triggerLevel,
          edge: triggerEdge,
          holdoff_ms: 10.0,
          auto_mode: true,
        },
      });

      currentSession = session;
      stats = await invokeIpc<OscilloscopeStats>('get_oscilloscope_stats');
    } catch (err) {
      console.error('Sweep acquisition failed:', err);
    }
  }

  function handleToggleRun(e: CustomEvent) {
    isRunning = e.detail.isRunning;
    if (isRunning) {
      startAutoRefresh();
    } else {
      stopAutoRefresh();
    }
  }

  function startAutoRefresh() {
    stopAutoRefresh();
    intervalTimer = setInterval(() => {
      if (isRunning) {
        triggerSweep();
      }
    }, 1500);
  }

  function stopAutoRefresh() {
    if (intervalTimer) {
      clearInterval(intervalTimer);
      intervalTimer = null;
    }
  }

  onMount(() => {
    loadInitialData();
    startAutoRefresh();
    return () => {
      stopAutoRefresh();
    };
  });
</script>

<div class="oscilloscope-app">
  <!-- Top Navigation Bar -->
  <header class="app-header">
    <div class="brand">
      <span class="brand-icon">🔬</span>
      <div class="brand-text">
        <h1>SCIENTIFIC SIGNAL OSCILLOSCOPE</h1>
        <span class="subtitle">100% Offline Digital Signal Processing, FFT Spectrum & Protocol Decoding</span>
      </div>
    </div>

    <div class="header-stats">
      <div class="stat-pill">
        <span class="stat-label">CAPTURES:</span>
        <span class="stat-value">{stats.total_captures}</span>
      </div>
      <div class="stat-pill">
        <span class="stat-label">SAMPLES:</span>
        <span class="stat-value">{stats.total_samples_analyzed.toLocaleString()}</span>
      </div>
      <div class="stat-pill">
        <span class="stat-label">PACKETS:</span>
        <span class="stat-value">{stats.total_packets_decoded}</span>
      </div>
      <div class="stat-pill air-gap">
        <span class="dot"></span>
        <span>AIR-GAPPED</span>
      </div>
      <button class="license-btn" on:click={() => (isLicenseModalOpen = true)}>
        TIER: {licenseTier.toUpperCase()}
      </button>
    </div>
  </header>

  <!-- Workspace Body -->
  <main class="app-body">
    <!-- Left / Center Primary Analysis Workspace -->
    <section class="main-workspace">
      <!-- Oscilloscope Dual-Channel Screen -->
      <div class="screen-box">
        {#if currentSession}
          <OscilloscopeDisplay
            ch1Samples={currentSession.ch1_samples}
            ch2Samples={currentSession.ch2_samples}
            triggerLevel={triggerLevel}
            timePerDivMs={timePerDivMs}
            ch1VoltsPerDiv={ch1VoltsPerDiv}
            ch2VoltsPerDiv={ch2VoltsPerDiv}
            vppCh1={currentSession.stats_ch1.vpp}
            freqCh1={currentSession.stats_ch1.frequency_hz}
            vppCh2={currentSession.stats_ch2.vpp}
          />
        {:else}
          <div class="loading-screen">Acquiring initial oscilloscope sweep...</div>
        {/if}
      </div>

      <!-- Lower Tabbed Analysis Deck (Spectrum & Protocol Decoder) -->
      <div class="analysis-deck">
        <div class="deck-tabs">
          <button
            class="tab-btn"
            class:active={activeTab === 'spectrum'}
            on:click={() => (activeTab = 'spectrum')}
          >
            📊 FFT SPECTRUM ANALYZER
          </button>
          <button
            class="tab-btn"
            class:active={activeTab === 'decoder'}
            on:click={() => (activeTab = 'decoder')}
          >
            ⚡ PROTOCOL BUS DECODER
          </button>
        </div>

        <div class="deck-content">
          {#if activeTab === 'spectrum' && currentSession}
            <SpectrumAnalyzer
              binFrequencies={currentSession.spectrum.bin_frequencies}
              magnitudesDb={currentSession.spectrum.magnitudes_db}
              dominantFreqHz={currentSession.spectrum.dominant_freq_hz}
              snrDb={currentSession.spectrum.snr_db}
              thdPercent={currentSession.spectrum.thd_percent}
            />
          {:else if activeTab === 'decoder' && currentSession}
            <ProtocolDecoderTable packets={currentSession.decoded_packets} />
          {/if}
        </div>
      </div>
    </section>

    <!-- Right Side Channel Rack -->
    <aside class="side-rack">
      <ChannelControls
        bind:ch1Type
        bind:ch1Freq
        bind:ch1Amp
        bind:ch1VoltsPerDiv
        bind:ch2Type
        bind:ch2Freq
        bind:ch2Amp
        bind:ch2VoltsPerDiv
        bind:timePerDivMs
        bind:triggerLevel
        bind:triggerEdge
        bind:isRunning
        on:acquire={triggerSweep}
        on:toggleRun={handleToggleRun}
      />
    </aside>
  </main>

  <!-- Status Bar Footer -->
  <footer class="app-footer">
    <span class="status-item">
      SESSION: {currentSession ? currentSession.session_id : 'Initializing'}
    </span>
    <span class="status-item">
      SAMPLE RATE: {currentSession ? (currentSession.sample_rate_hz / 1000).toFixed(1) + ' kSa/s' : '50.0 kSa/s'}
    </span>
    <span class="status-item">
      DOMINANT FREQ: {currentSession ? currentSession.spectrum.dominant_freq_hz.toFixed(1) + ' Hz' : '0.0 Hz'}
    </span>
    <span class="status-item security">
      OFFLINE AIR-GAP: ZERO NETWORK EGRESS ASSURED
    </span>
    <span class="status-item version">v{0.1}</span>
  </footer>

  <!-- Offline Licensing Modal -->
  <LicenseModal
    isOpen={isLicenseModalOpen}
    appName="Scientific Signal & Telemetry Oscilloscope"
    on:close={() => (isLicenseModalOpen = false)}
    on:statusChange={(e) => (licenseTier = e.detail.tier)}
  />
</div>

<style>
  .oscilloscope-app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #0a0e14;
    color: #e6edf3;
  }

  .app-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 18px;
    background: #121820;
    border-bottom: 1px solid #1e2c3d;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .brand-icon {
    font-size: 24px;
  }

  .brand-text h1 {
    font-size: 14px;
    font-weight: 700;
    letter-spacing: 0.8px;
    color: #58a6ff;
    font-family: var(--font-mono);
  }

  .subtitle {
    font-size: 10px;
    color: #8b949e;
  }

  .header-stats {
    display: flex;
    align-items: center;
    gap: 10px;
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .stat-pill {
    display: flex;
    gap: 6px;
    background: #16202c;
    border: 1px solid #243347;
    padding: 4px 8px;
    border-radius: 4px;
  }

  .stat-label {
    color: #8b949e;
  }

  .stat-value {
    color: #58a6ff;
    font-weight: 600;
  }

  .air-gap {
    background: #23863622;
    border-color: #2ea04366;
    color: #3fb950;
    font-weight: 600;
    align-items: center;
  }

  .dot {
    width: 6px;
    height: 6px;
    background: #3fb950;
    border-radius: 50%;
  }

  .license-btn {
    background: #1f2d3d;
    border: 1px solid #388bfd66;
    color: #79c0ff;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-mono);
  }

  .license-btn:hover {
    background: #27394d;
  }

  .app-body {
    flex: 1;
    display: flex;
    overflow: hidden;
    padding: 10px;
    gap: 10px;
  }

  .main-workspace {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow: hidden;
  }

  .screen-box {
    flex: 1.2;
    min-height: 300px;
  }

  .loading-screen {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
    background: #0d131a;
    border: 1px solid #1e2c3d;
    border-radius: 8px;
    color: #8b949e;
    font-family: var(--font-mono);
  }

  .analysis-deck {
    flex: 0.9;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .deck-tabs {
    display: flex;
    gap: 8px;
  }

  .tab-btn {
    background: #121820;
    border: 1px solid #1e2c3d;
    color: #8b949e;
    padding: 5px 12px;
    border-radius: 4px;
    font-size: 11px;
    font-family: var(--font-mono);
    font-weight: 600;
  }

  .tab-btn.active {
    background: #1e2c3d;
    color: #e6edf3;
    border-color: #388bfd;
  }

  .deck-content {
    flex: 1;
  }

  .side-rack {
    width: 320px;
    display: flex;
    flex-direction: column;
  }

  .app-footer {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 4px 16px;
    background: #101620;
    border-top: 1px solid #1e2c3d;
    font-size: 10px;
    font-family: var(--font-mono);
    color: #8b949e;
  }

  .status-item.security {
    margin-left: auto;
    color: #3fb950;
    font-weight: 600;
  }

  .status-item.version {
    color: #58a6ff;
  }
</style>

