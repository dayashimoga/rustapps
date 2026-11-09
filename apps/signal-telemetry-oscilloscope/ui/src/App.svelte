<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import OscilloscopeDisplay from './components/OscilloscopeDisplay.svelte';
  import SpectrumAnalyzer from './components/SpectrumAnalyzer.svelte';
  import ProtocolDecoderTable from './components/ProtocolDecoderTable.svelte';
  import ChannelControls from './components/ChannelControls.svelte';
  import LicenseModal from './components/LicenseModal.svelte';
  import CommandPalette, { type CommandAction } from './components/CommandPalette.svelte';

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
    math_samples: number[];
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
    cursors?: {
      x1_time_ms: number;
      x2_time_ms: number;
      delta_t_ms: number;
      frequency_hz: number;
      y1_volts: number;
      y2_volts: number;
      delta_v_volts: number;
    };
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
  let isCommandPaletteOpen: boolean = false;
  let fileInput: HTMLInputElement;
  let statusBanner: string = '';

  // Math Channel State
  let showMath: boolean = true;
  let mathOperation: 'Add' | 'Subtract' | 'Multiply' | 'InvertCh1' = 'Add';

  // Cursors State
  let showCursors: boolean = true;

  $: commandActions = [
    {
      id: 'toggle-run',
      title: isRunning ? 'Pause Acquisition (HOLD)' : 'Resume Acquisition (RUN)',
      subtitle: isRunning ? 'Freeze waveform display and analysis' : 'Stream continuous synthetic acquisition frames',
      category: 'Acquisition',
      icon: isRunning ? '⏸️' : '▶️',
      shortcut: 'Space',
      run: () => { toggleRunStop(); },
    },
    {
      id: 'tab-fft',
      title: 'FFT Spectrum Decomposition View',
      subtitle: 'Analyze spectral power density, SNR (dB), and THD%',
      category: 'Analysis',
      icon: '📊',
      shortcut: '1',
      run: () => { activeTab = 'spectrum'; },
    },
    {
      id: 'tab-bus',
      title: 'Protocol Bus Decoder View',
      subtitle: 'Inspect UART, SPI, and I2C hex frame payloads',
      category: 'Analysis',
      icon: '⚡',
      shortcut: '2',
      run: () => { activeTab = 'decoder'; },
    },
    {
      id: 'toggle-cursors',
      title: showCursors ? 'Hide Measurement Cursors' : 'Show Measurement Cursors',
      subtitle: 'Toggle interactive ΔT and ΔV measurement cursors',
      category: 'Measurement',
      icon: '📏',
      shortcut: 'C',
      run: () => { showCursors = !showCursors; },
    },
    {
      id: 'math-op',
      title: `Cycle Channel Math (${mathOperation})`,
      subtitle: 'Compute realtime A+B, A-B, A*B, or -A',
      category: 'Math',
      icon: '∑',
      shortcut: 'M',
      run: () => { cycleMathOp(); },
    },
    {
      id: 'import-file',
      title: 'Import Signal File (.csv, .wav, .vcd)',
      subtitle: 'Load laboratory telemetry or audio oscilloscope traces',
      category: 'Import',
      icon: '📥',
      shortcut: '⌘O',
      run: () => { fileInput?.click(); },
    },
    {
      id: 'single-sweep',
      title: 'Trigger Single Shot Sweep',
      subtitle: 'Execute immediate single acquisition pass',
      category: 'Acquisition',
      icon: '🎯',
      shortcut: 'S',
      run: () => { triggerSweep(); },
    },
    {
      id: 'manage-license',
      title: 'Manage Offline Ed25519 License',
      subtitle: `Current tier: ${licenseTier.toUpperCase()}`,
      category: 'System',
      icon: '🔑',
      shortcut: 'L',
      run: () => { isLicenseModalOpen = true; },
    },
  ] as CommandAction[];

  function handleGlobalKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      isCommandPaletteOpen = !isCommandPaletteOpen;
    } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'o') {
      e.preventDefault();
      fileInput?.click();
    } else if (e.key === 'Escape' && isCommandPaletteOpen) {
      e.preventDefault();
      isCommandPaletteOpen = false;
    }
  }

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

      if (showMath) {
        const mathSession = await invokeIpc<CaptureSession>('compute_channel_math', {
          session_id: session.session_id,
          operation: mathOperation,
        });
        currentSession = mathSession;
      } else {
        currentSession = session;
      }

      stats = await invokeIpc<OscilloscopeStats>('get_oscilloscope_stats');
    } catch (err) {
      console.error('Sweep acquisition failed:', err);
    }
  }

  async function cycleMathOp() {
    const ops: ('Add' | 'Subtract' | 'Multiply' | 'InvertCh1')[] = ['Add', 'Subtract', 'Multiply', 'InvertCh1'];
    const idx = ops.indexOf(mathOperation);
    mathOperation = ops[(idx + 1) % ops.length];
    if (currentSession) {
      try {
        const updated = await invokeIpc<CaptureSession>('compute_channel_math', {
          session_id: currentSession.session_id,
          operation: mathOperation,
        });
        currentSession = updated;
        statusBanner = `Computed Math Channel: ${mathOperation}`;
        setTimeout(() => { statusBanner = ''; }, 3000);
      } catch (e) {
        console.error('Math compute error:', e);
      }
    }
  }

  async function handleCursorUpdate(e: CustomEvent<{ x1: number; x2: number; y1: number; y2: number }>) {
    if (!currentSession) return;
    try {
      const updated = await invokeIpc<CaptureSession>('update_measurement_cursors', {
        session_id: currentSession.session_id,
        cursors: {
          x1_time_ms: e.detail.x1,
          x2_time_ms: e.detail.x2,
          y1_volts: e.detail.y1,
          y2_volts: e.detail.y2,
          delta_t_ms: Math.abs(e.detail.x2 - e.detail.x1),
          frequency_hz: Math.abs(e.detail.x2 - e.detail.x1) > 0.001 ? 1000.0 / Math.abs(e.detail.x2 - e.detail.x1) : 0,
          delta_v_volts: Math.abs(e.detail.y2 - e.detail.y1),
        },
      });
      currentSession.cursors = updated.cursors;
    } catch (err) {
      console.error('Cursor update failed:', err);
    }
  }

  async function handleFileImport(e: Event) {
    const target = e.target as HTMLInputElement;
    if (!target.files || target.files.length === 0) return;
    const file = target.files[0];
    const name = file.name;
    const ext = name.split('.').pop()?.toLowerCase() || 'csv';

    try {
      let format = 'csv';
      if (ext === 'wav') format = 'wav';
      else if (ext === 'vcd') format = 'vcd';

      const text = await file.text();
      const session = await invokeIpc<CaptureSession>('import_signal_file', {
        title: `Import: ${name}`,
        format,
        content: text,
      });

      currentSession = session;
      statusBanner = `Successfully imported ${session.total_samples} samples from ${name}`;
      setTimeout(() => { statusBanner = ''; }, 4000);
    } catch (err: any) {
      alert(`Import error: ${err?.message || err}`);
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

  function toggleRunStop() {
    isRunning = !isRunning;
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

<svelte:window on:keydown={handleGlobalKeyDown} />

<!-- Hidden File Import Input -->
<input
  type="file"
  accept=".csv,.txt,.wav,.vcd"
  bind:this={fileInput}
  on:change={handleFileImport}
  style="display: none;"
/>

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
      <button class="cmd-palette-btn" on:click={() => (isCommandPaletteOpen = true)} title="Commands (⌘K)">
        <span class="cmd-icon">⌘</span>
        <span>Commands</span>
        <kbd>⌘K</kbd>
      </button>

      <button class="action-top-btn" on:click={() => fileInput?.click()} title="Import signal file">
        📥 Import
      </button>

      <button
        class="action-top-btn"
        class:active={showCursors}
        on:click={() => (showCursors = !showCursors)}
        title="Toggle ΔT / ΔV cursors"
      >
        📏 Cursors
      </button>

      <button
        class="action-top-btn math"
        class:active={showMath}
        on:click={cycleMathOp}
        title="Cycle Channel Math (A+B, A-B, A*B, -A)"
      >
        ∑ Math: {mathOperation}
      </button>

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

  {#if statusBanner}
    <div class="status-banner">
      <span>✨ {statusBanner}</span>
    </div>
  {/if}

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
            mathSamples={currentSession.math_samples || []}
            triggerLevel={triggerLevel}
            timePerDivMs={timePerDivMs}
            ch1VoltsPerDiv={ch1VoltsPerDiv}
            ch2VoltsPerDiv={ch2VoltsPerDiv}
            vppCh1={currentSession.stats_ch1.vpp}
            freqCh1={currentSession.stats_ch1.frequency_hz}
            vppCh2={currentSession.stats_ch2.vpp}
            mathOperation={mathOperation}
            showMath={showMath}
            showCursors={showCursors}
            cursors={currentSession.cursors || null}
            on:updateCursors={handleCursorUpdate}
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
        on:sweep={triggerSweep}
        on:toggleRun={handleToggleRun}
      />
    </aside>
  </main>

  <!-- Bottom Laboratory Status Bar -->
  <footer class="app-footer">
    <div class="footer-left">
      <span class="status-indicator live"></span>
      <span class="status-text">{isRunning ? 'ACQUIRING (RUN)' : 'ACQUISITION PAUSED (HOLD)'}</span>
      <span class="divider">|</span>
      <span class="status-text">SQLite WAL: <code>oscilloscope.db</code></span>
    </div>

    <div class="footer-right">
      <span>Zero Unsolicited Network Egress Guaranteed</span>
      <span class="divider">|</span>
      <span>Session: <code>{currentSession?.session_id || 'IDLE'}</code></span>
    </div>
  </footer>

  <!-- Command Palette Modal -->
  <CommandPalette
    isOpen={isCommandPaletteOpen}
    actions={commandActions}
    onClose={() => (isCommandPaletteOpen = false)}
  />

  <!-- Offline License Verification Modal -->
  <LicenseModal
    isOpen={isLicenseModalOpen}
    appName="Scientific Signal Oscilloscope"
    on:close={() => (isLicenseModalOpen = false)}
    on:statusChange={(e) => {
      licenseTier = e.detail.tier;
    }}
  />
</div>

<style>
  .oscilloscope-app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #06090e;
    color: #e6edf3;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
  }

  .app-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 18px;
    background: #0d131a;
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
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.5px;
    color: #58a6ff;
  }

  .subtitle {
    font-size: 11px;
    color: #8b949e;
  }

  .header-stats {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .cmd-palette-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: #161b22;
    border: 1px solid #30363d;
    color: #8b949e;
    font-size: 0.8rem;
    font-weight: 500;
    padding: 4px 9px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .cmd-palette-btn:hover {
    background: rgba(244, 63, 94, 0.15);
    border-color: #f43f5e;
    color: #f0f6fc;
  }

  .cmd-icon {
    color: #fb7185;
    font-weight: 700;
  }

  .cmd-palette-btn kbd {
    background: #0d1117;
    border: 1px solid #21262d;
    padding: 1px 5px;
    border-radius: 4px;
    font-size: 0.7rem;
    color: #8b949e;
    font-family: inherit;
  }

  .action-top-btn {
    background: #161b22;
    border: 1px solid #30363d;
    color: #c9d1d9;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.76rem;
    font-weight: 600;
    cursor: pointer;
  }

  .action-top-btn:hover {
    background: #21262d;
    border-color: #58a6ff;
  }

  .action-top-btn.active {
    background: rgba(56, 189, 248, 0.15);
    border-color: #38bdf8;
    color: #38bdf8;
  }

  .action-top-btn.math.active {
    background: rgba(217, 70, 239, 0.15);
    border-color: #d946ef;
    color: #d946ef;
  }

  .stat-pill {
    display: flex;
    gap: 5px;
    padding: 4px 10px;
    background: #141c26;
    border: 1px solid #1e2c3d;
    border-radius: 4px;
    font-size: 11px;
    font-family: var(--font-mono);
  }

  .stat-label {
    color: #8b949e;
  }

  .stat-value {
    color: #58a6ff;
    font-weight: bold;
  }

  .air-gap {
    align-items: center;
    background: #0f241a;
    border-color: #1a422d;
    color: #3fb950;
    font-weight: bold;
  }

  .dot {
    width: 6px;
    height: 6px;
    background: #3fb950;
    border-radius: 50%;
  }

  .license-btn {
    background: #21262d;
    border: 1px solid #30363d;
    color: #e6edf3;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 11px;
    font-family: var(--font-mono);
    font-weight: bold;
    cursor: pointer;
  }

  .status-banner {
    background: rgba(16, 185, 129, 0.15);
    border-bottom: 1px solid var(--accent-green);
    padding: 6px 18px;
    font-size: 0.8rem;
    color: #6ee7b7;
    font-weight: 600;
  }

  .app-body {
    display: flex;
    flex: 1;
    overflow: hidden;
    padding: 12px 16px;
    gap: 16px;
  }

  .main-workspace {
    display: flex;
    flex-direction: column;
    flex: 1;
    gap: 12px;
    overflow: hidden;
  }

  .screen-box {
    flex: 6;
    min-height: 0;
  }

  .loading-screen {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    background: #0d131a;
    border: 1px solid #1e2c3d;
    border-radius: 8px;
    color: #8b949e;
    font-family: var(--font-mono);
  }

  .analysis-deck {
    display: flex;
    flex-direction: column;
    flex: 4;
    background: #0d131a;
    border: 1px solid #1e2c3d;
    border-radius: 8px;
    overflow: hidden;
    min-height: 0;
  }

  .deck-tabs {
    display: flex;
    background: #141c26;
    border-bottom: 1px solid #1e2c3d;
  }

  .tab-btn {
    padding: 8px 16px;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: #8b949e;
    font-size: 11px;
    font-weight: bold;
    font-family: var(--font-mono);
    cursor: pointer;
  }

  .tab-btn.active {
    color: #58a6ff;
    border-bottom-color: #58a6ff;
    background: #0d131a;
  }

  .deck-content {
    flex: 1;
    overflow: hidden;
  }

  .side-rack {
    width: 320px;
    overflow-y: auto;
  }

  .app-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 18px;
    background: #0d131a;
    border-top: 1px solid #1e2c3d;
    font-size: 11px;
    font-family: var(--font-mono);
    color: #8b949e;
  }

  .footer-left, .footer-right {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .status-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .status-indicator.live {
    background: #3fb950;
    box-shadow: 0 0 6px #3fb95088;
  }

  .status-text {
    font-weight: bold;
    color: #e6edf3;
  }

  .divider {
    color: #30363d;
  }

  code {
    color: #58a6ff;
  }
</style>
