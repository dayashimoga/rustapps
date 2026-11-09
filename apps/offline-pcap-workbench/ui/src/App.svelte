<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import type { DecodedPacket, HexDumpRow, TcpFlow, ForensicAuditReport } from './lib/types';
  import PacketTable from './components/PacketTable.svelte';
  import HexPayloadInspector from './components/HexPayloadInspector.svelte';
  import StreamWaterfall from './components/StreamWaterfall.svelte';
  import CommandPalette from './components/CommandPalette.svelte';
  import LicenseModal from './components/LicenseModal.svelte';
  import ForensicModal from './components/ForensicModal.svelte';

  // State
  let captureSummary: any = null;
  let packets: DecodedPacket[] = [];
  let totalPackets: number = 0;
  let selectedPacketIndex: number = 1;
  let selectedPacketDetail: any = null;
  let hexDump: HexDumpRow[] = [];
  let tcpFlows: TcpFlow[] = [];
  let forensicReport: ForensicAuditReport | null = null;
  let licenseStatus: any = null;

  // View state
  let currentView: 'packets' | 'waterfall' = 'packets';
  let filterText = '';
  let isCommandPaletteOpen = false;
  let isLicenseModalOpen = false;
  let isForensicModalOpen = false;
  let isLoading = true;

  onMount(async () => {
    await loadInitialData();
    window.addEventListener('keydown', handleGlobalKeyDown);
    return () => window.removeEventListener('keydown', handleGlobalKeyDown);
  });

  function handleGlobalKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      isCommandPaletteOpen = !isCommandPaletteOpen;
    }
  }

  async function loadInitialData() {
    isLoading = true;
    try {
      captureSummary = await invokeIpc<any>('get_capture_summary', {});
      tcpFlows = await invokeIpc<TcpFlow[]>('get_tcp_flows', {});
      forensicReport = await invokeIpc<ForensicAuditReport>('get_forensic_report', {});
      licenseStatus = await invokeIpc<any>('get_license_status', {});
      await fetchPackets();
      await fetchPacketDetail(1);
    } catch (e) {
      console.error('Failed to load initial PCAP workbench state', e);
    } finally {
      isLoading = false;
    }
  }

  async function fetchPackets() {
    try {
      const res = await invokeIpc<{
        packets: DecodedPacket[];
        total: number;
      }>('get_packets', {
        filter: filterText,
        offset: 0,
        limit: 100,
      });
      packets = res.packets;
      totalPackets = res.total;
      if (packets.length > 0 && !packets.some((p) => p.index === selectedPacketIndex)) {
        await selectPacket(packets[0].index);
      }
    } catch (e) {
      console.error('Failed to fetch packets', e);
    }
  }

  async function selectPacket(index: number) {
    selectedPacketIndex = index;
    await fetchPacketDetail(index);
  }

  async function fetchPacketDetail(index: number) {
    try {
      const res = await invokeIpc<{
        packet: any;
        hex_dump: HexDumpRow[];
      }>('get_packet_detail', {
        packet_index: index,
      });
      selectedPacketDetail = res.packet;
      hexDump = res.hex_dump;
    } catch (e) {
      console.error('Failed to get packet detail', e);
    }
  }

  function handleCommand(cmdId: string) {
    if (cmdId === 'filter_all') {
      filterText = '';
      fetchPackets();
    } else if (cmdId === 'filter_tls') {
      filterText = 'tls';
      fetchPackets();
    } else if (cmdId === 'filter_http') {
      filterText = 'http';
      fetchPackets();
    } else if (cmdId === 'filter_dns') {
      filterText = 'dns';
      fetchPackets();
    } else if (cmdId === 'view_packets') {
      currentView = 'packets';
    } else if (cmdId === 'view_waterfall') {
      currentView = 'waterfall';
    } else if (cmdId === 'open_forensics') {
      isForensicModalOpen = true;
    } else if (cmdId === 'open_license') {
      isLicenseModalOpen = true;
    }
  }

  function handleFilterInput() {
    fetchPackets();
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
</script>

<div class="workbench-root">
  <!-- Top Application Bar -->
  <header class="app-header">
    <div class="brand">
      <span class="brand-badge">PAW</span>
      <span class="brand-title">Offline PCAP Workbench</span>
      <span class="air-gap-pill">100% AIR-GAPPED</span>
    </div>

    <!-- Wireshark Display Filter Input -->
    <div class="filter-box">
      <span class="filter-icon">🔍</span>
      <input
        type="text"
        placeholder="Apply display filter (e.g. dns, http, tls, tcp, 192.168.1.50)..."
        bind:value={filterText}
        on:input={handleFilterInput}
      />
      {#if filterText}
        <button type="button" class="clear-btn" on:click={() => { filterText = ''; fetchPackets(); }}>✕</button>
      {/if}
    </div>

    <!-- Header Actions -->
    <div class="header-actions">
      <button
        type="button"
        class="nav-tab"
        class:active={currentView === 'packets'}
        on:click={() => (currentView = 'packets')}
      >Packets & Hex</button>

      <button
        type="button"
        class="nav-tab"
        class:active={currentView === 'waterfall'}
        on:click={() => (currentView = 'waterfall')}
      >Stream Waterfall</button>

      <button
        type="button"
        class="btn-tool"
        on:click={() => (isForensicModalOpen = true)}
        title="View Forensic Audit Report"
      >
        <span class="btn-icon">🛡️</span>
        <span>Forensics</span>
        {#if forensicReport && forensicReport.cleartext_credentials_found > 0}
          <span class="alert-dot"></span>
        {/if}
      </button>

      <button
        type="button"
        class="btn-tool"
        on:click={() => (isLicenseModalOpen = true)}
        title="License Manager"
      >
        <span class="btn-icon">🔑</span>
        <span>{licenseStatus?.tier ? licenseStatus.tier.split(' ')[0] : 'License'}</span>
      </button>

      <button
        type="button"
        class="btn-cmd"
        on:click={() => (isCommandPaletteOpen = true)}
        title="Open Command Palette (Ctrl+K)"
      >
        <kbd>Ctrl+K</kbd>
      </button>
    </div>
  </header>

  <!-- Capture Metadata & Metrics Ribbon -->
  {#if captureSummary}
    <div class="metrics-ribbon">
      <div class="meta-item">
        <span class="lbl">Capture:</span>
        <span class="val mono">{captureSummary.filename}</span>
      </div>
      <div class="meta-item">
        <span class="lbl">Size:</span>
        <span class="val">{formatBytes(captureSummary.file_size_bytes)}</span>
      </div>
      <div class="meta-item">
        <span class="lbl">Packets:</span>
        <span class="val">{captureSummary.total_packets} ({totalPackets} filtered)</span>
      </div>
      <div class="meta-item">
        <span class="lbl">Duration:</span>
        <span class="val">{captureSummary.duration_seconds.toFixed(2)}s</span>
      </div>
      <div class="meta-item">
        <span class="lbl">Conversations:</span>
        <span class="val">{captureSummary.active_tcp_flows} TCP</span>
      </div>

      <div class="spacer"></div>

      <!-- Protocol Distribution Pills -->
      <div class="proto-pills">
        {#each captureSummary.protocol_distribution as p}
          <span class="proto-pill">
            <strong>{p.protocol}:</strong> {p.packet_count} ({p.percentage.toFixed(0)}%)
          </span>
        {/each}
      </div>

      <!-- Anomaly Alert Indicator -->
      {#if forensicReport && forensicReport.cleartext_credentials_found > 0}
        <button
          type="button"
          class="alert-pill"
          on:click={() => (isForensicModalOpen = true)}
        >
          ⚠️ {forensicReport.cleartext_credentials_found} Cleartext Credential Alert!
        </button>
      {/if}
    </div>
  {/if}

  <!-- Main View Area -->
  <main class="workbench-body">
    {#if isLoading}
      <div class="loading-container">Ingesting zero-copy PCAP packet streams...</div>
    {:else if currentView === 'packets'}
      <div class="split-view">
        <div class="table-pane">
          <PacketTable
            {packets}
            {selectedPacketIndex}
            onSelectPacket={selectPacket}
          />
        </div>
        <div class="detail-pane">
          <HexPayloadInspector
            packet={selectedPacketDetail}
            {hexDump}
          />
        </div>
      </div>
    {:else if currentView === 'waterfall'}
      <StreamWaterfall
        flows={tcpFlows}
        onSelectPacket={(idx) => {
          selectPacket(idx);
          currentView = 'packets';
        }}
      />
    {/if}
  </main>

  <!-- Modals -->
  <CommandPalette
    isOpen={isCommandPaletteOpen}
    onClose={() => (isCommandPaletteOpen = false)}
    onSelectCommand={handleCommand}
  />

  <LicenseModal
    isOpen={isLicenseModalOpen}
    onClose={() => (isLicenseModalOpen = false)}
    onLicenseUpdated={loadInitialData}
  />

  <ForensicModal
    isOpen={isForensicModalOpen}
    report={forensicReport}
    onClose={() => (isForensicModalOpen = false)}
  />
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    background: #11111b;
    color: #cdd6f4;
    overflow: hidden;
    user-select: none;
  }
  .workbench-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
  }
  .app-header {
    height: 48px;
    background: #181825;
    border-bottom: 1px solid #313244;
    display: flex;
    align-items: center;
    padding: 0 16px;
    gap: 16px;
    z-index: 100;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .brand-badge {
    background: #89b4fa;
    color: #11111b;
    font-weight: 900;
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: monospace;
  }
  .brand-title {
    font-size: 13px;
    font-weight: 700;
    color: #cdd6f4;
  }
  .air-gap-pill {
    background: #1e1e2e;
    color: #a6e3a1;
    font-size: 9px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 10px;
    border: 1px solid rgba(166, 227, 161, 0.4);
    letter-spacing: 0.5px;
  }
  .filter-box {
    flex: 1;
    max-width: 500px;
    display: flex;
    align-items: center;
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 6px;
    padding: 4px 10px;
    gap: 8px;
  }
  .filter-box:focus-within {
    border-color: #89b4fa;
  }
  .filter-icon {
    font-size: 13px;
    opacity: 0.6;
  }
  .filter-box input {
    flex: 1;
    background: transparent;
    border: none;
    color: #cdd6f4;
    font-size: 12px;
    outline: none;
    font-family: monospace;
  }
  .clear-btn {
    background: none;
    border: none;
    color: #6c7086;
    cursor: pointer;
    font-size: 11px;
  }
  .clear-btn:hover {
    color: #cdd6f4;
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
  }
  .nav-tab {
    background: #1e1e2e;
    border: 1px solid #313244;
    color: #a6adc8;
    padding: 5px 12px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .nav-tab.active {
    background: #89b4fa;
    color: #11111b;
    border-color: #89b4fa;
    font-weight: 600;
  }
  .btn-tool {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #1e1e2e;
    border: 1px solid #313244;
    color: #cdd6f4;
    padding: 5px 10px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    position: relative;
  }
  .btn-tool:hover {
    background: #313244;
  }
  .alert-dot {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #f38ba8;
  }
  .btn-cmd {
    background: transparent;
    border: none;
    cursor: pointer;
  }
  .btn-cmd kbd {
    background: #313244;
    color: #89b4fa;
    padding: 3px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-family: monospace;
  }
  .metrics-ribbon {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 6px 16px;
    background: #181825;
    border-bottom: 1px solid #313244;
    font-size: 11px;
  }
  .meta-item {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .meta-item .lbl {
    color: #6c7086;
  }
  .meta-item .val {
    color: #cdd6f4;
    font-weight: 500;
  }
  .mono {
    font-family: monospace;
  }
  .spacer {
    flex: 1;
  }
  .proto-pills {
    display: flex;
    gap: 6px;
  }
  .proto-pill {
    background: #11111b;
    border: 1px solid #313244;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    color: #a6adc8;
    font-family: monospace;
  }
  .proto-pill strong {
    color: #89b4fa;
  }
  .alert-pill {
    background: rgba(243, 139, 168, 0.2);
    border: 1px solid #f38ba8;
    color: #f38ba8;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    cursor: pointer;
  }
  .workbench-body {
    flex: 1;
    overflow: hidden;
    display: flex;
  }
  .split-view {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
  }
  .table-pane {
    height: 52%;
    overflow: hidden;
  }
  .detail-pane {
    height: 48%;
    overflow: hidden;
  }
  .loading-container {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #6c7086;
    font-size: 14px;
  }
</style>
