<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import type {
    DebuggerStats,
    FieldbusAuditManifest,
    IndustrialBusFrame,
    IndustrialTraceSession,
    LicenseStatus,
    SessionSummary,
  } from './lib/types';

  import CommandPalette from './components/CommandPalette.svelte';
  import LicenseModal from './components/LicenseModal.svelte';
  import RawFrameModal from './components/RawFrameModal.svelte';
  import AuditManifestModal from './components/AuditManifestModal.svelte';
  import PacketWaterfallPane from './components/PacketWaterfallPane.svelte';
  import RegisterInspectorPane from './components/RegisterInspectorPane.svelte';

  let stats: DebuggerStats = {
    total_traces_captured: 0,
    total_frames_decoded: 0,
    security_intrusions_blocked: 0,
    crc_bus_faults_isolated: 0,
  };

  let sessionSummaries: SessionSummary[] = [];
  let currentSessionId = '';
  let activeSession: IndustrialTraceSession | null = null;
  let selectedFrame: IndustrialBusFrame | null = null;
  let licenseStatus: LicenseStatus | null = null;
  let auditManifest: FieldbusAuditManifest | null = null;

  let showCommandPalette = false;
  let showLicenseModal = false;
  let showRawFrameModal = false;
  let showAuditModal = false;

  onMount(async () => {
    await reloadData();
  });

  async function reloadData() {
    try {
      stats = await invokeIpc<DebuggerStats>('get_stats');
      sessionSummaries = await invokeIpc<SessionSummary[]>('list_sessions');
      licenseStatus = await invokeIpc<LicenseStatus>('get_license_status');

      if (sessionSummaries.length > 0) {
        if (!currentSessionId || !sessionSummaries.find(s => s.session_id === currentSessionId)) {
          currentSessionId = sessionSummaries[0].session_id;
        }
        await loadSession(currentSessionId);
      }
    } catch (e) {
      console.error('Failed to load debugger data', e);
    }
  }

  async function loadSession(sessionId: string) {
    try {
      currentSessionId = sessionId;
      activeSession = await invokeIpc<IndustrialTraceSession>('get_session', { session_id: sessionId });
      if (activeSession && activeSession.frames.length > 0) {
        selectedFrame = activeSession.frames[0];
      } else {
        selectedFrame = null;
      }
    } catch (e) {
      console.error('Failed to load session', e);
    }
  }

  async function openAuditManifest() {
    try {
      auditManifest = await invokeIpc<FieldbusAuditManifest>('generate_audit_manifest', {
        session_id: currentSessionId,
      });
      showAuditModal = true;
    } catch (e) {
      console.error('Failed to generate audit manifest', e);
    }
  }

  function handleCommandSelect(e: CustomEvent<{ id: string }>) {
    switch (e.detail.id) {
      case 'decode_frame':
        showRawFrameModal = true;
        break;
      case 'sign_audit':
        openAuditManifest();
        break;
      case 'manage_license':
        showLicenseModal = true;
        break;
      case 'reload_sessions':
        reloadData();
        break;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
      e.preventDefault();
      showCommandPalette = !showCommandPalette;
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="flex flex-col h-screen w-screen bg-slate-950 text-slate-100 font-sans overflow-hidden select-none">
  <!-- Top Navigation & Metrics Header -->
  <header class="px-5 py-3 bg-slate-900 border-b border-slate-800 flex items-center justify-between shrink-0">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-amber-500/10 border border-amber-500/30 flex items-center justify-center text-amber-400 font-bold text-base">
        ⚡
      </div>
      <div>
        <div class="flex items-center gap-2">
          <h1 class="text-sm font-bold text-slate-100 tracking-wide">
            Industrial Fieldbus Protocol Debugger
          </h1>
          <span class="px-2 py-0.5 rounded text-[10px] font-mono font-medium bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
            AIR-GAPPED 100%
          </span>
          <span class="px-2 py-0.5 rounded text-[10px] font-mono font-medium bg-amber-500/20 text-amber-300 border border-amber-500/30">
            IEC 62443 COMPLIANT
          </span>
        </div>
        <p class="text-[11px] text-slate-400">
          Modbus RTU/TCP, CAN 2.0B / CAN-FD Dissection & Industrial Cyber-Anomaly Monitor
        </p>
      </div>
    </div>

    <!-- Live Telemetry Stats -->
    <div class="hidden lg:flex items-center gap-6 font-mono text-xs border-x border-slate-800 px-6">
      <div>
        <div class="text-[10px] text-slate-500 uppercase">Traces Captured</div>
        <div class="text-slate-200 font-bold">{stats.total_traces_captured}</div>
      </div>
      <div>
        <div class="text-[10px] text-slate-500 uppercase">Frames Decoded</div>
        <div class="text-slate-200 font-bold">{stats.total_frames_decoded}</div>
      </div>
      <div>
        <div class="text-[10px] text-slate-500 uppercase">Intrusions Blocked</div>
        <div class="text-red-400 font-bold">{stats.security_intrusions_blocked}</div>
      </div>
      <div>
        <div class="text-[10px] text-slate-500 uppercase">CRC Faults Isolated</div>
        <div class="text-amber-400 font-bold">{stats.crc_bus_faults_isolated}</div>
      </div>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-2">
      <button
        class="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg text-xs font-mono font-medium transition-colors border border-slate-700"
        on:click={() => showRawFrameModal = true}
      >
        Dissect Frame
      </button>

      <button
        class="px-3 py-1.5 bg-emerald-950/80 hover:bg-emerald-900 text-emerald-300 border border-emerald-700/60 rounded-lg text-xs font-mono font-medium transition-colors"
        on:click={openAuditManifest}
      >
        Audit Certificate
      </button>

      <button
        class="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-amber-300 rounded-lg text-xs font-mono font-medium transition-colors border border-slate-700"
        on:click={() => showLicenseModal = true}
      >
        {licenseStatus?.tier?.toUpperCase() || 'LICENSE'}
      </button>

      <button
        class="px-2.5 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-slate-200 rounded-lg text-xs font-mono border border-slate-700"
        on:click={() => showCommandPalette = true}
        title="Command Palette (Ctrl+K)"
      >
        ⌘K
      </button>
    </div>
  </header>

  <!-- Session Selector Sub-header -->
  <div class="px-5 py-2 bg-slate-900/60 border-b border-slate-800 flex items-center justify-between shrink-0 font-mono text-xs">
    <div class="flex items-center gap-3">
      <span class="text-slate-400 uppercase text-[10px] font-bold">Active Trace Session:</span>
      <select
        bind:value={currentSessionId}
        on:change={() => loadSession(currentSessionId)}
        class="px-3 py-1 bg-slate-950 border border-slate-700 rounded text-slate-200 font-semibold focus:border-amber-500 focus:outline-none"
      >
        {#each sessionSummaries as s}
          <option value={s.session_id}>
            {s.name} ({s.protocol.toUpperCase()} @ {s.baudrate_bps / 1000} kbps - {s.total_frames} frames)
          </option>
        {/each}
      </select>
    </div>

    {#if activeSession}
      <div class="flex items-center gap-4 text-[11px] text-slate-400">
        <div>Baudrate: <span class="text-slate-200">{activeSession.baudrate_bps} bps</span></div>
        <div>Frames: <span class="text-slate-200">{activeSession.total_frames}</span></div>
        {#if activeSession.security_alerts_count > 0}
          <div class="text-red-400 font-bold">⚠️ {activeSession.security_alerts_count} Security Alerts</div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Main Split Layout -->
  <main class="flex-1 flex overflow-hidden">
    <!-- Left Pane: Packet Waterfall (45%) -->
    <section class="w-5/12 h-full">
      <PacketWaterfallPane
        session={activeSession}
        selectedFrame={selectedFrame}
        on:select={(e) => selectedFrame = e.detail}
      />
    </section>

    <!-- Right Pane: Register & Anomaly Inspector (55%) -->
    <section class="w-7/12 h-full">
      <RegisterInspectorPane frame={selectedFrame} />
    </section>
  </main>

  <!-- Modals -->
  <CommandPalette
    bind:show={showCommandPalette}
    on:select={handleCommandSelect}
  />

  <LicenseModal
    bind:show={showLicenseModal}
    status={licenseStatus}
    on:activated={reloadData}
  />

  <RawFrameModal
    bind:show={showRawFrameModal}
    on:decoded={(e) => {
      selectedFrame = e.detail;
    }}
  />

  <AuditManifestModal
    bind:show={showAuditModal}
    manifest={auditManifest}
  />
</div>
