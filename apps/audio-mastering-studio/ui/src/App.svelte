<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import StemTracksViewer from './components/StemTracksViewer.svelte';
  import ParametricEqVisualizer from './components/ParametricEqVisualizer.svelte';
  import MasteringControls from './components/MasteringControls.svelte';
  import LoudnessMeter from './components/LoudnessMeter.svelte';
  import LicenseModal from './components/LicenseModal.svelte';
  import CommandPalette, { type CommandAction } from './components/CommandPalette.svelte';

  interface StudioStats {
    total_projects: number;
    total_stems_extracted: number;
    total_audio_minutes_processed: number;
    avg_integrated_lufs: number;
  }

  interface MasteringProject {
    project_id: string;
    title: string;
    created_at: number;
    sample_rate: number;
    duration_seconds: number;
    stems: any[];
    mastering_config: any;
    mastered_samples: number[];
    original_loudness: any;
    mastered_loudness: any;
  }

  let stats: StudioStats = {
    total_projects: 0,
    total_stems_extracted: 0,
    total_audio_minutes_processed: 0,
    avg_integrated_lufs: -14.0,
  };

  let currentProject: MasteringProject | null = null;
  let isLicenseModalOpen: boolean = false;
  let licenseTier: string = 'Free';
  let isProcessing: boolean = false;
  let isCommandPaletteOpen: boolean = false;

  $: commandActions = [
    {
      id: 'remaster-now',
      title: 'Render Audio Remaster',
      subtitle: 'Apply 5-band biquad EQ, dynamics compressor, and true-peak limiter',
      category: 'Mastering',
      icon: '⚡',
      shortcut: '⌘R',
      run: () => {
        if (currentProject) {
          handleRemaster(new CustomEvent('remaster', { detail: {
            compressorThresholdDb: currentProject.mastering_config.compressor_threshold_db,
            compressorRatio: currentProject.mastering_config.compressor_ratio,
            compressorAttackMs: currentProject.mastering_config.compressor_attack_ms,
            compressorReleaseMs: currentProject.mastering_config.compressor_release_ms,
            deReverbAmount: currentProject.mastering_config.de_reverb_amount,
            targetLufs: currentProject.mastering_config.target_lufs,
            limiterCeilingDb: currentProject.mastering_config.limiter_ceiling_db,
          }}));
        }
      },
    },
    {
      id: 'target-spotify',
      title: 'Target Spotify / Streaming (-14.0 LUFS)',
      subtitle: 'Standard loudness compliance for modern streaming services',
      category: 'Mastering',
      icon: '🎯',
      shortcut: '1',
      run: () => {
        if (currentProject) {
          currentProject.mastering_config.target_lufs = -14.0;
        }
      },
    },
    {
      id: 'target-cd',
      title: 'Target Club / CD Master (-9.0 LUFS)',
      subtitle: 'High perceived loudness with controlled inter-sample peaks',
      category: 'Mastering',
      icon: '🔊',
      shortcut: '2',
      run: () => {
        if (currentProject) {
          currentProject.mastering_config.target_lufs = -9.0;
        }
      },
    },
    {
      id: 'reset-eq',
      title: 'Flat Frequency Response (Reset EQ)',
      subtitle: 'Reset all 5 biquad parametric EQ bands to 0 dB gain',
      category: 'DSP',
      icon: '〰️',
      shortcut: '0',
      run: () => {
        if (currentProject) {
          currentProject.mastering_config.bands.forEach((b: any) => b.gain_db = 0.0);
        }
      },
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
    } else if (e.key === 'Escape' && isCommandPaletteOpen) {
      e.preventDefault();
      isCommandPaletteOpen = false;
    }
  }

  async function loadInitialData() {
    try {
      stats = await invokeIpc<StudioStats>('get_studio_stats');
      currentProject = await invokeIpc<MasteringProject>('get_project', { project_id: 'proj-demo' });
      const lic = await invokeIpc<any>('get_license_status');
      if (lic && lic.tier) {
        licenseTier = lic.tier;
      }
    } catch (e) {
      console.error('Failed to load initial mastering studio data:', e);
    }
  }

  async function handleRemaster(e: CustomEvent) {
    if (!currentProject) return;
    isProcessing = true;
    try {
      const updatedConfig = {
        bands: currentProject.mastering_config.bands,
        compressor_threshold_db: e.detail.compressorThresholdDb,
        compressor_ratio: e.detail.compressorRatio,
        compressor_attack_ms: e.detail.compressorAttackMs,
        compressor_release_ms: e.detail.compressorReleaseMs,
        de_reverb_amount: e.detail.deReverbAmount,
        target_lufs: e.detail.targetLufs,
        limiter_ceiling_db: e.detail.limiterCeilingDb,
      };

      const proj = await invokeIpc<MasteringProject>('remaster_project', {
        project_id: currentProject.project_id,
        config: updatedConfig,
      });

      currentProject = proj;
      stats = await invokeIpc<StudioStats>('get_studio_stats');
    } catch (err) {
      console.error('Remaster failed:', err);
    } finally {
      isProcessing = false;
    }
  }

  function handleEqChange(e: CustomEvent) {
    if (currentProject) {
      currentProject.mastering_config.bands = e.detail.bands;
    }
  }

  onMount(() => {
    loadInitialData();
  });
</script>

<svelte:window on:keydown={handleGlobalKeyDown} />

<div class="audio-studio-app">
  <!-- Top Bar -->
  <header class="app-header">
    <div class="brand">
      <span class="brand-icon">🎹</span>
      <div class="brand-text">
        <h1>AI AUDIO MASTERING & STEM SPLITTER STUDIO</h1>
        <span class="subtitle">100% Offline 4-Stem Demixing, 5-Band Biquad EQ & EBU R128 Loudness Mastering</span>
      </div>
    </div>

    <div class="header-stats">
      <button class="cmd-palette-btn" on:click={() => (isCommandPaletteOpen = true)} title="Commands (⌘K)">
        <span class="cmd-icon">⌘</span>
        <span>Commands</span>
        <kbd>⌘K</kbd>
      </button>

      <div class="stat-pill">
        <span class="stat-label">PROJECTS:</span>
        <span class="stat-value">{stats.total_projects}</span>
      </div>
      <div class="stat-pill">
        <span class="stat-label">STEMS:</span>
        <span class="stat-value">{stats.total_stems_extracted}</span>
      </div>
      <div class="stat-pill">
        <span class="stat-label">PROCESSED:</span>
        <span class="stat-value">{stats.total_audio_minutes_processed.toFixed(1)}m</span>
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

  <!-- Main DAW Body -->
  <main class="app-body">
    {#if currentProject}
      <!-- Upper Section: 4-Stem Waveform Player -->
      <section class="stems-section">
        <StemTracksViewer
          stems={currentProject.stems}
          durationSeconds={currentProject.duration_seconds}
        />
      </section>

      <!-- Lower Section: Professional Mastering Rack -->
      <section class="mastering-rack">
        <div class="rack-col eq-col">
          <ParametricEqVisualizer
            bands={currentProject.mastering_config.bands}
            on:change={handleEqChange}
          />
        </div>

        <div class="rack-col controls-col">
          <MasteringControls
            compressorThresholdDb={currentProject.mastering_config.compressor_threshold_db}
            compressorRatio={currentProject.mastering_config.compressor_ratio}
            compressorAttackMs={currentProject.mastering_config.compressor_attack_ms}
            compressorReleaseMs={currentProject.mastering_config.compressor_release_ms}
            deReverbAmount={currentProject.mastering_config.de_reverb_amount}
            targetLufs={currentProject.mastering_config.target_lufs}
            limiterCeilingDb={currentProject.mastering_config.limiter_ceiling_db}
            {isProcessing}
            on:remaster={handleRemaster}
          />
        </div>

        <div class="rack-col meter-col">
          <LoudnessMeter
            original={currentProject.original_loudness}
            mastered={currentProject.mastered_loudness}
            targetLufs={currentProject.mastering_config.target_lufs}
          />
        </div>
      </section>
    {:else}
      <div class="loading-state">
        <span class="loading-spinner"></span>
        <span>Initializing offline mastering studio engine...</span>
      </div>
    {/if}
  </main>

  <!-- Footer Status Bar -->
  <footer class="app-footer">
    <span class="status-item">
      PROJECT: {currentProject ? currentProject.title : 'None'}
    </span>
    <span class="status-item">
      SAMPLE RATE: {currentProject ? `${currentProject.sample_rate} Hz` : '44100 Hz'}
    </span>
    <span class="status-item">
      INTEGRATED LOUDNESS: {currentProject ? `${currentProject.mastered_loudness.integrated_lufs.toFixed(1)} LUFS` : '-14.0 LUFS'}
    </span>
    <span class="status-item security">
      OFFLINE AIR-GAP: ZERO NETWORK EGRESS ASSURED
    </span>
    <span class="status-item version">v{0.1}</span>
  </footer>

  <!-- Command Palette -->
  <CommandPalette
    isOpen={isCommandPaletteOpen}
    actions={commandActions}
    onClose={() => (isCommandPaletteOpen = false)}
  />

  <!-- Licensing Modal -->
  <LicenseModal
    isOpen={isLicenseModalOpen}
    appName="AI Audio Mastering & Stem Separation Studio"
    on:close={() => (isLicenseModalOpen = false)}
    on:statusChange={(e) => (licenseTier = e.detail.tier)}
  />
</div>

<style>
  .audio-studio-app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #0e1117;
    color: #f0f6fc;
  }

  .app-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 18px;
    background: #141820;
    border-bottom: 1px solid var(--border-color);
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
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.8px;
    color: var(--accent-gold);
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

  .cmd-palette-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: #1b222d;
    border: 1px solid #283547;
    color: #cbd5e1;
    font-size: 0.78rem;
    font-weight: 500;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .cmd-palette-btn:hover {
    background: rgba(139, 92, 246, 0.15);
    border-color: #8b5cf6;
    color: #f8fafc;
  }

  .cmd-icon {
    color: #a78bfa;
    font-weight: 700;
  }

  .cmd-palette-btn kbd {
    background: #0e1117;
    border: 1px solid #334155;
    padding: 1px 4px;
    border-radius: 3px;
    font-size: 0.68rem;
    color: #94a3b8;
    font-family: inherit;
  }

  .stat-pill {
    display: flex;
    gap: 6px;
    background: #1b222d;
    border: 1px solid #283547;
    padding: 4px 8px;
    border-radius: 4px;
  }

  .stat-label {
    color: #8b949e;
  }

  .stat-value {
    color: var(--accent-gold);
    font-weight: 600;
  }

  .air-gap {
    background: #10b98122;
    border-color: #10b98166;
    color: #34d399;
    font-weight: 600;
    align-items: center;
  }

  .dot {
    width: 6px;
    height: 6px;
    background: #34d399;
    border-radius: 50%;
  }

  .license-btn {
    background: #1f2733;
    border: 1px solid #e5a93c66;
    color: #fbbf24;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-mono);
  }

  .license-btn:hover {
    background: #2a3547;
  }

  .app-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    padding: 8px 10px;
    gap: 8px;
  }

  .stems-section {
    flex: 1.1;
    min-height: 220px;
  }

  .mastering-rack {
    flex: 0.95;
    display: flex;
    gap: 8px;
    min-height: 210px;
  }

  .rack-col {
    display: flex;
    flex-direction: column;
  }

  .eq-col {
    flex: 1.3;
  }

  .controls-col {
    flex: 1.1;
  }

  .meter-col {
    flex: 0.9;
  }

  .loading-state {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
    color: #8b949e;
    font-family: var(--font-mono);
    gap: 12px;
  }

  .app-footer {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 4px 16px;
    background: #11141b;
    border-top: 1px solid var(--border-color);
    font-size: 10px;
    font-family: var(--font-mono);
    color: #8b949e;
  }

  .status-item.security {
    margin-left: auto;
    color: #34d399;
    font-weight: 600;
  }

  .status-item.version {
    color: var(--accent-gold);
  }
</style>

