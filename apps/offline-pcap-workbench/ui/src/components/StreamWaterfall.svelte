<script lang="ts">
  import { invokeIpc } from '../lib/ipc';
  import type { TcpFlow, FlowWaterfallEvent } from '../lib/types';

  export let flows: TcpFlow[] = [];
  export let onSelectPacket: (index: number) => void = () => {};

  let selectedStreamId: number = 0;
  let waterfallEvents: FlowWaterfallEvent[] = [];
  let isLoading = false;

  $: if (flows.length > 0 && selectedStreamId === 0) {
    selectedStreamId = flows[0].stream_id;
  }

  $: if (selectedStreamId !== undefined) {
    loadWaterfall(selectedStreamId);
  }

  async function loadWaterfall(streamId: number) {
    isLoading = true;
    try {
      waterfallEvents = await invokeIpc<FlowWaterfallEvent[]>('get_stream_waterfall', {
        stream_id: streamId,
      });
    } catch {
      waterfallEvents = [];
    } finally {
      isLoading = false;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
</script>

<div class="waterfall-layout">
  <!-- Left: Flows List -->
  <div class="flows-sidebar">
    <div class="sidebar-header">
      <span>TCP Conversations ({flows.length})</span>
    </div>
    <div class="flows-list">
      {#each flows as f}
        <button
          type="button"
          class="flow-card"
          class:active={f.stream_id === selectedStreamId}
          on:click={() => (selectedStreamId = f.stream_id)}
        >
          <div class="flow-header">
            <span class="flow-id">Stream #{f.stream_id}</span>
            <span class="flow-proto">{f.protocol}</span>
          </div>
          <div class="flow-endpoints">
            <div class="ep" title={f.endpoint_a}>{f.endpoint_a}</div>
            <div class="arrow">⇄</div>
            <div class="ep" title={f.endpoint_b}>{f.endpoint_b}</div>
          </div>
          {#if f.sni}
            <div class="flow-sni">SNI: {f.sni}</div>
          {/if}
          <div class="flow-metrics">
            <span>{f.packet_count} pkts</span>
            <span>{formatBytes(f.client_bytes + f.server_bytes)}</span>
            <span class="flow-state state-{f.state.toLowerCase()}">{f.state}</span>
          </div>
        </button>
      {:else}
        <div class="empty-sidebar">No TCP streams found</div>
      {/each}
    </div>
  </div>

  <!-- Right: Waterfall Timeline -->
  <div class="waterfall-timeline">
    <div class="timeline-header">
      <div class="endpoint-col left-ep">Client (Endpoint A)</div>
      <div class="center-channel">Conversation Sequence Waterfall</div>
      <div class="endpoint-col right-ep">Server (Endpoint B)</div>
    </div>

    <div class="timeline-body">
      {#if isLoading}
        <div class="loading-state">Rendering sequence waterfall...</div>
      {:else if waterfallEvents.length === 0}
        <div class="loading-state">No packets in selected TCP conversation</div>
      {:else}
        <div class="events-sequence">
          {#each waterfallEvents as ev}
            {@const isAtoB = ev.direction === 'A -> B'}
            <div
              class="event-row"
              role="button"
              tabindex="0"
              on:click={() => onSelectPacket(ev.packet_index)}
              on:keydown={(e) => {
                if (e.key === 'Enter') onSelectPacket(ev.packet_index);
              }}
            >
              <div class="time-pill">+{ev.relative_ms.toFixed(1)} ms</div>

              <div class="sequence-track" class:is-reverse={!isAtoB}>
                <div class="seq-arrow" class:dir-right={isAtoB} class:dir-left={!isAtoB}>
                  <div class="arrow-line"></div>
                  <div class="arrow-head"></div>
                </div>

                <div class="seq-metadata">
                  <span class="pkt-num">#{ev.packet_index}</span>
                  <span class="flags-badge">{ev.flags}</span>
                  <span class="seq-ack">Seq={ev.seq} Ack={ev.ack}</span>
                  <span class="len-pill">{ev.payload_bytes} B</span>
                  <span class="info-text">{ev.summary}</span>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .waterfall-layout {
    display: flex;
    height: 100%;
    background: #11111b;
  }
  .flows-sidebar {
    width: 320px;
    border-right: 1px solid #313244;
    display: flex;
    flex-direction: column;
    background: #181825;
  }
  .sidebar-header {
    padding: 10px 14px;
    background: #11111b;
    border-bottom: 1px solid #313244;
    font-size: 12px;
    font-weight: 600;
    color: #cdd6f4;
  }
  .flows-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .flow-card {
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 8px;
    padding: 8px 10px;
    text-align: left;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .flow-card:hover {
    border-color: #89b4fa;
  }
  .flow-card.active {
    border-color: #89b4fa;
    background: #252739;
  }
  .flow-header {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    font-weight: 600;
  }
  .flow-id {
    color: #89b4fa;
  }
  .flow-proto {
    color: #cba6f7;
    font-family: monospace;
    font-size: 11px;
  }
  .flow-endpoints {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: #a6adc8;
    margin: 4px 0;
    font-family: monospace;
  }
  .flow-endpoints .ep {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 120px;
  }
  .flow-sni {
    font-size: 10px;
    color: #a6e3a1;
    font-family: monospace;
    margin-bottom: 4px;
  }
  .flow-metrics {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: #6c7086;
  }
  .flow-state {
    font-weight: 600;
  }
  .state-established {
    color: #a6e3a1;
  }
  .state-closed {
    color: #fab387;
  }
  .state-reset {
    color: #f38ba8;
  }
  .waterfall-timeline {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .timeline-header {
    display: flex;
    align-items: center;
    padding: 8px 16px;
    background: #181825;
    border-bottom: 1px solid #313244;
    font-size: 12px;
    font-weight: 600;
  }
  .endpoint-col {
    width: 200px;
    color: #89b4fa;
    font-family: monospace;
  }
  .right-ep {
    text-align: right;
    color: #fab387;
  }
  .center-channel {
    flex: 1;
    text-align: center;
    color: #6c7086;
  }
  .timeline-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }
  .events-sequence {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .event-row {
    display: flex;
    align-items: center;
    gap: 12px;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 6px;
    transition: background 0.1s ease;
  }
  .event-row:hover {
    background: #1e1e2e;
  }
  .time-pill {
    width: 80px;
    font-size: 11px;
    font-family: monospace;
    color: #6c7086;
  }
  .sequence-track {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .sequence-track.is-reverse {
    align-items: flex-end;
  }
  .seq-arrow {
    display: flex;
    align-items: center;
    width: 100%;
    position: relative;
  }
  .seq-arrow.dir-right {
    flex-direction: row;
  }
  .seq-arrow.dir-left {
    flex-direction: row-reverse;
  }
  .arrow-line {
    flex: 1;
    height: 2px;
    background: #89b4fa;
  }
  .dir-left .arrow-line {
    background: #fab387;
  }
  .arrow-head {
    width: 0;
    height: 0;
    border-top: 5px solid transparent;
    border-bottom: 5px solid transparent;
  }
  .dir-right .arrow-head {
    border-left: 8px solid #89b4fa;
  }
  .dir-left .arrow-head {
    border-right: 8px solid #fab387;
  }
  .seq-metadata {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    font-family: monospace;
  }
  .pkt-num {
    color: #6c7086;
  }
  .flags-badge {
    background: #313244;
    color: #cdd6f4;
    padding: 1px 6px;
    border-radius: 4px;
    font-weight: 600;
  }
  .seq-ack {
    color: #a6adc8;
  }
  .len-pill {
    background: #11111b;
    color: #94e2d5;
    padding: 1px 5px;
    border-radius: 3px;
  }
  .info-text {
    color: #cdd6f4;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 320px;
  }
  .loading-state,
  .empty-sidebar {
    padding: 30px;
    text-align: center;
    color: #6c7086;
    font-size: 13px;
  }
</style>
