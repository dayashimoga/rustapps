<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import LicenseModal, { type LicenseStatusReport } from './components/LicenseModal.svelte';
  import VideoPlayerPreview from './components/VideoPlayerPreview.svelte';
  import TimelineViewer from './components/TimelineViewer.svelte';
  import TutorialStepList from './components/TutorialStepList.svelte';
  import SubtitleEditor from './components/SubtitleEditor.svelte';
  import ExportModal from './components/ExportModal.svelte';
  import CommandPalette, { type CommandAction } from './components/CommandPalette.svelte';

  export interface CursorEvent {
    timestamp_ms: number;
    x: number;
    y: number;
    click: 'None' | 'Left' | 'Right' | 'Double';
  }

  export interface ZoomKeyframe {
    start_ms: number;
    end_ms: number;
    focus_x: number;
    focus_y: number;
    scale: number;
    easing: string;
    label: string;
  }

  export interface SubtitleCue {
    start_ms: number;
    end_ms: number;
    text: string;
    speaker?: string;
  }

  export interface TutorialStep {
    step_number: number;
    timestamp_ms: number;
    title: string;
    instruction: string;
    action_type: string;
    target_x: number;
    target_y: number;
  }

  export interface RecordingSession {
    session_id: string;
    title: string;
    created_at: number;
    duration_ms: number;
    width: number;
    height: number;
    fps: number;
    total_clicks: number;
    auto_zooms_count: number;
    silence_trimmed_ms: number;
    cursor_events: CursorEvent[];
    zoom_keyframes: ZoomKeyframe[];
    subtitles: SubtitleCue[];
    tutorial_steps: TutorialStep[];
  }

  export interface StudioStats {
    total_recordings: number;
    total_duration_secs: number;
    total_zooms_generated: number;
    total_silence_saved_secs: number;
    total_tutorial_steps: number;
  }

  let sessions: RecordingSession[] = [];
  let activeSession: RecordingSession | null = null;
  let stats: StudioStats = {
    total_recordings: 0,
    total_duration_secs: 0,
    total_zooms_generated: 0,
    total_silence_saved_secs: 0,
    total_tutorial_steps: 0,
  };

  let currentTimeMs: number = 0;
  let isPlaying: boolean = false;
  let playInterval: any = null;
  let currentZoomScale: number = 1.45;

  let activeRightTab: 'steps' | 'subtitles' | 'stats' = 'steps';
  let isLicenseModalOpen: boolean = false;
  let isExportModalOpen: boolean = false;
  let isCommandPaletteOpen: boolean = false;
  let errorMessage: string = '';

  let licenseTier: 'Free' | 'Premium' | 'Business' = 'Free';
  let isLicenseActive: boolean = false;

  $: commandActions = [
    {
      id: 'toggle-play',
      title: isPlaying ? 'Pause Walkthrough Preview' : 'Play Walkthrough Preview',
      subtitle: isPlaying ? 'Freeze playback at current timeline position' : 'Simulate cursor trajectories and auto-zooms',
      category: 'Playback',
      icon: isPlaying ? '⏸️' : '▶️',
      shortcut: 'Space',
      run: () => {
        if (isPlaying) pause();
        else play();
      },
    },
    {
      id: 'tab-steps',
      title: 'Tutorial Steps & Milestones',
      subtitle: 'View generated Markdown SOP instructions and click coordinates',
      category: 'Tutorial',
      icon: '📝',
      shortcut: '1',
      run: () => { activeRightTab = 'steps'; },
    },
    {
      id: 'tab-subtitles',
      title: 'Speech Subtitles & SRT Cues',
      subtitle: 'Inspect timestamped transcription cues',
      category: 'Tutorial',
      icon: '💬',
      shortcut: '2',
      run: () => { activeRightTab = 'subtitles'; },
    },
    {
      id: 'tab-stats',
      title: 'Studio Synthesis Metrics',
      subtitle: 'Review silence trimming and keyframe compression ratio',
      category: 'Tutorial',
      icon: '📊',
      shortcut: '3',
      run: () => { activeRightTab = 'stats'; },
    },
    {
      id: 'export-sop',
      title: 'Export Tutorial SOP & Bundle',
      subtitle: 'Export interactive HTML/Markdown guide with screenshot assets',
      category: 'Export',
      icon: '🚀',
      shortcut: '⌘E',
      run: () => { isExportModalOpen = true; },
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
    } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'e') {
      e.preventDefault();
      isExportModalOpen = true;
    } else if (e.key === 'Escape' && isCommandPaletteOpen) {
      e.preventDefault();
      isCommandPaletteOpen = false;
    }
  }

  onMount(async () => {
    await refreshSessions();
    await checkLicense();
  });

  onDestroy(() => {
    if (playInterval) clearInterval(playInterval);
  });

  async function checkLicense() {
    try {
      const rep = await invokeIpc<LicenseStatusReport>('get_license_status');
      if (rep) {
        licenseTier = rep.tier;
        isLicenseActive = rep.is_active;
      }
    } catch (e) {
      console.error('License status check error:', e);
    }
  }

  async function refreshSessions() {
    try {
      const list = await invokeIpc<RecordingSession[]>('list_recordings');
      sessions = list;
      if (list.length > 0) {
        activeSession = list[0];
        currentTimeMs = 0;
      }
      const st = await invokeIpc<StudioStats>('get_studio_stats');
      if (st) stats = st;
    } catch (e: any) {
      errorMessage = e?.message || 'Failed to load recordings';
    }
  }

  async function selectSession(id: string) {
    try {
      const sess = await invokeIpc<RecordingSession>('get_recording', { session_id: id });
      activeSession = sess;
      currentTimeMs = 0;
      pause();
    } catch (e: any) {
      errorMessage = e?.message || 'Failed to select recording';
    }
  }

  async function handleScaleChange(scale: number) {
    currentZoomScale = scale;
    if (!activeSession) return;
    try {
      const updated = await invokeIpc<RecordingSession>('adjust_auto_zooms', {
        session_id: activeSession.session_id,
        scale,
      });
      activeSession = updated;
    } catch (e: any) {
      console.error('Failed to adjust auto zooms:', e);
    }
  }

  function togglePlay() {
    if (isPlaying) {
      pause();
    } else {
      play();
    }
  }

  function play() {
    if (!activeSession) return;
    isPlaying = true;
    const step = 50; // 50ms tick (20fps update)
    if (playInterval) clearInterval(playInterval);
    playInterval = setInterval(() => {
      if (!activeSession) return;
      currentTimeMs += step;
      if (currentTimeMs >= activeSession.duration_ms) {
        currentTimeMs = 0;
        pause();
      }
    }, step);
  }

  function pause() {
    isPlaying = false;
    if (playInterval) {
      clearInterval(playInterval);
      playInterval = null;
    }
  }

  function handleSeek(targetMs: number) {
    if (!activeSession) return;
    currentTimeMs = Math.max(0, Math.min(targetMs, activeSession.duration_ms));
  }
</script>

<svelte:window on:keydown={handleGlobalKeyDown} />

<div class="studio-layout">
  <!-- Top Application Bar -->
  <header class="app-header">
    <div class="header-left">
      <div class="brand">
        <span class="logo">🎬</span>
        <div class="brand-titles">
          <h1>Screen Recorder Studio</h1>
          <span class="sub">100% Offline AI Capture, Auto-Zoom & Interactive Tutorials</span>
        </div>
      </div>
      <div class="airgap-badge">
        <span class="pulse"></span>
        <span>100% OFFLINE AIR-GAPPED</span>
      </div>
    </div>

    <div class="header-right">
      <button class="cmd-palette-btn" on:click={() => (isCommandPaletteOpen = true)} title="Commands (⌘K)">
        <span class="cmd-icon">⌘</span>
        <span>Commands</span>
        <kbd>⌘K</kbd>
      </button>

      {#if sessions.length > 0}
        <div class="session-picker">
          <label for="session-select">Session:</label>
          <select
            id="session-select"
            class="select-input"
            value={activeSession?.session_id || ''}
            on:change={(e) => selectSession(e.currentTarget.value)}
          >
            {#each sessions as s}
              <option value={s.session_id}>
                {s.title} ({Math.round(s.duration_ms / 1000)}s)
              </option>
            {/each}
          </select>
        </div>
      {/if}

      <button class="export-btn" on:click={() => (isExportModalOpen = true)}>
        🚀 Export Tutorial
      </button>

      <button
        class="license-btn"
        class:active={isLicenseActive}
        on:click={() => (isLicenseModalOpen = true)}
      >
        <span>🔑</span>
        <span>{licenseTier.toUpperCase()}</span>
      </button>
    </div>
  </header>

  {#if errorMessage}
    <div class="error-banner">
      <span>⚠️ {errorMessage}</span>
      <button class="close-error" on:click={() => (errorMessage = '')}>✕</button>
    </div>
  {/if}

  <!-- Main Split Studio Body -->
  <main class="studio-body">
    <!-- Left Workspace: Video Preview & Multi-Track Timeline -->
    <section class="left-workspace">
      <VideoPlayerPreview
        {currentTimeMs}
        durationMs={activeSession?.duration_ms || 10_000}
        {isPlaying}
        zoomKeyframes={activeSession?.zoom_keyframes || []}
        cursorEvents={activeSession?.cursor_events || []}
        {currentZoomScale}
        on:togglePlay={togglePlay}
        on:seek={(e) => handleSeek(e.detail)}
        on:scaleChange={(e) => handleScaleChange(e.detail)}
      />

      <TimelineViewer
        durationMs={activeSession?.duration_ms || 10_000}
        {currentTimeMs}
        cursorEvents={activeSession?.cursor_events || []}
        zoomKeyframes={activeSession?.zoom_keyframes || []}
        subtitles={activeSession?.subtitles || []}
        silenceTrimmedMs={activeSession?.silence_trimmed_ms || 0}
        on:seek={(e) => handleSeek(e.detail)}
      />
    </section>

    <!-- Right Workspace: Tutorial Steps & Subtitles -->
    <aside class="right-workspace">
      <div class="right-tabs">
        <button
          class="tab-btn"
          class:active={activeRightTab === 'steps'}
          on:click={() => (activeRightTab = 'steps')}
        >
          Steps ({activeSession?.tutorial_steps.length || 0})
        </button>
        <button
          class="tab-btn"
          class:active={activeRightTab === 'subtitles'}
          on:click={() => (activeRightTab = 'subtitles')}
        >
          Subtitles ({activeSession?.subtitles.length || 0})
        </button>
        <button
          class="tab-btn"
          class:active={activeRightTab === 'stats'}
          on:click={() => (activeRightTab = 'stats')}
        >
          Studio Stats
        </button>
      </div>

      <div class="tab-content">
        {#if activeRightTab === 'steps'}
          <TutorialStepList
            steps={activeSession?.tutorial_steps || []}
            {currentTimeMs}
            on:seek={(e) => handleSeek(e.detail)}
          />
        {:else if activeRightTab === 'subtitles'}
          <SubtitleEditor
            subtitles={activeSession?.subtitles || []}
            {currentTimeMs}
            on:seek={(e) => handleSeek(e.detail)}
            on:exportSrt={() => (isExportModalOpen = true)}
          />
        {:else if activeRightTab === 'stats'}
          <div class="stats-panel">
            <h3>Studio Synthesis Metrics</h3>
            <div class="stat-card">
              <span class="label">Total Walkthroughs Logged</span>
              <span class="val">{stats.total_recordings}</span>
            </div>
            <div class="stat-card">
              <span class="label">Total Recorded Duration</span>
              <span class="val">{stats.total_duration_secs.toFixed(1)}s</span>
            </div>
            <div class="stat-card">
              <span class="label">AI Auto-Zooms Generated</span>
              <span class="val">{stats.total_zooms_generated} keyframes</span>
            </div>
            <div class="stat-card">
              <span class="label">Dead-Air Silence Trimmed</span>
              <span class="val">{stats.total_silence_saved_secs.toFixed(1)}s saved</span>
            </div>
            <div class="stat-card">
              <span class="label">Interactive Tutorial Milestones</span>
              <span class="val">{stats.total_tutorial_steps} steps</span>
            </div>
          </div>
        {/if}
      </div>
    </aside>
  </main>

  <!-- Bottom Status Bar -->
  <footer class="app-footer">
    <div class="footer-left">
      <span class="dot-online"></span>
      <span>SQLite WAL Storage: <code>screen_recorder.db</code></span>
    </div>
    <div class="footer-right">
      <span>Active Session: <code>{activeSession?.session_id || 'NONE'}</code></span>
      <span class="divider">|</span>
      <span>Zero Unsolicited Egress Guaranteed</span>
    </div>
  </footer>

  <!-- Command Palette -->
  <CommandPalette
    isOpen={isCommandPaletteOpen}
    actions={commandActions}
    onClose={() => (isCommandPaletteOpen = false)}
  />

  <!-- Export Dialog -->
  <ExportModal
    isOpen={isExportModalOpen}
    sessionId={activeSession?.session_id || ''}
    sessionTitle={activeSession?.title || 'Tutorial Walkthrough'}
    on:close={() => (isExportModalOpen = false)}
  />

  <!-- Licensing Modal -->
  <LicenseModal
    isOpen={isLicenseModalOpen}
    appName="Screen Recorder Studio"
    on:close={() => (isLicenseModalOpen = false)}
    on:statusChange={(e) => {
      licenseTier = e.detail.tier;
      isLicenseActive = e.detail.is_active;
    }}
  />
</div>

<style>
  .studio-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
  }

  .app-header {
    padding: 12px 20px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .logo {
    font-size: 1.8rem;
    line-height: 1;
  }

  .brand-titles h1 {
    font-size: 1.15rem;
    color: var(--text-main);
  }

  .brand-titles .sub {
    font-size: 0.72rem;
    color: var(--text-sub);
  }

  .airgap-badge {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.3);
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 0.68rem;
    font-weight: 700;
    color: var(--accent-green);
  }

  .pulse {
    width: 6px;
    height: 6px;
    background: var(--accent-green);
    border-radius: 50%;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .cmd-palette-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 0.8rem;
    font-weight: 500;
    padding: 5px 10px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .cmd-palette-btn:hover {
    background: rgba(244, 63, 94, 0.15);
    border-color: #f43f5e;
    color: var(--text-main);
  }

  .cmd-icon {
    color: #fb7185;
    font-weight: 700;
  }

  .cmd-palette-btn kbd {
    background: #0f172a;
    border: 1px solid #334155;
    padding: 1px 5px;
    border-radius: 4px;
    font-size: 0.7rem;
    color: var(--text-muted);
    font-family: inherit;
  }

  .session-picker {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .select-input {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 5px 8px;
    color: var(--text-main);
    font-size: 0.78rem;
    font-family: var(--font-mono);
  }

  .export-btn {
    background: linear-gradient(135deg, #2563eb, #1d4ed8);
    border: 1px solid #3b82f6;
    color: #fff;
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 600;
  }

  .export-btn:hover {
    background: #1d4ed8;
  }

  .license-btn {
    background: var(--bg-card);
    border: 1px solid var(--border);
    padding: 5px 10px;
    border-radius: 6px;
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .license-btn.active {
    border-color: var(--accent-green);
    color: var(--accent-green);
    background: rgba(16, 185, 129, 0.1);
  }

  .error-banner {
    background: rgba(239, 68, 68, 0.15);
    border-bottom: 1px solid var(--accent-red);
    padding: 8px 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: #fca5a5;
    font-size: 0.82rem;
  }

  .close-error {
    background: transparent;
    border: none;
    color: #fca5a5;
  }

  .studio-body {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 380px;
    gap: 16px;
    padding: 16px 20px;
    overflow: hidden;
  }

  @media (max-width: 1024px) {
    .studio-body {
      grid-template-columns: 1fr;
    }
  }

  .left-workspace {
    display: flex;
    flex-direction: column;
    gap: 14px;
    overflow-y: auto;
  }

  .right-workspace {
    display: flex;
    flex-direction: column;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }

  .right-tabs {
    display: flex;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
  }

  .tab-btn {
    flex: 1;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 10px 0;
    color: var(--text-muted);
    font-size: 0.78rem;
    font-weight: 600;
  }

  .tab-btn:hover {
    color: var(--text-main);
  }

  .tab-btn.active {
    color: var(--accent-cyan);
    border-bottom-color: var(--accent-cyan);
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
  }

  .stats-panel {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .stats-panel h3 {
    font-size: 0.95rem;
    color: var(--text-main);
  }

  .stat-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .stat-card .label {
    font-size: 0.72rem;
    color: var(--text-sub);
    text-transform: uppercase;
  }

  .stat-card .val {
    font-size: 1.2rem;
    font-weight: 700;
    font-family: var(--font-mono);
    color: var(--text-main);
  }

  .app-footer {
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    padding: 6px 18px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.72rem;
    color: var(--text-sub);
  }

  .footer-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .dot-online {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-green);
  }

  .footer-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  code {
    font-family: var(--font-mono);
    color: var(--text-muted);
  }

  .divider {
    color: var(--border);
  }
</style>
