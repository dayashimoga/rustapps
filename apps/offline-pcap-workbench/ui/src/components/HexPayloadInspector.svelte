<script lang="ts">
  import type { HexDumpRow } from '../lib/types';

  export let packet: any = null;
  export let hexDump: HexDumpRow[] = [];

  let activeTab: 'tree' | 'hex' | 'both' = 'both';
</script>

<div class="inspector-panel">
  <div class="panel-header">
    <div class="title">
      <span class="icon">🔬</span>
      <span>Packet #{packet ? packet.index : '-'} Dissection</span>
    </div>
    <div class="tab-toggles">
      <button
        type="button"
        class:active={activeTab === 'tree'}
        on:click={() => (activeTab = 'tree')}
      >Tree</button>
      <button
        type="button"
        class:active={activeTab === 'hex'}
        on:click={() => (activeTab = 'hex')}
      >Hex</button>
      <button
        type="button"
        class:active={activeTab === 'both'}
        on:click={() => (activeTab = 'both')}
      >Split</button>
    </div>
  </div>

  <div class="panel-content" class:split-mode={activeTab === 'both'}>
    {#if packet && (activeTab === 'tree' || activeTab === 'both')}
      <div class="tree-pane">
        <!-- Frame Summary -->
        <details open class="tree-node">
          <summary>
            <strong>Frame {packet.index}</strong>: {packet.wire_len} bytes on wire, {packet.captured_len} bytes captured
          </summary>
          <div class="node-children">
            <div>Arrival Time: {new Date(packet.timestamp_epoch_ms).toISOString()}</div>
            <div>Time delta from start: +{packet.delta_time_ms.toFixed(2)} ms</div>
            <div>Stream ID: {packet.stream_id !== null && packet.stream_id !== undefined ? `TCP Stream #${packet.stream_id}` : 'None'}</div>
          </div>
        </details>

        <!-- Ethernet II Layer -->
        <details open class="tree-node">
          <summary>
            <strong>Ethernet II</strong>, Src: {packet.src_mac}, Dst: {packet.dst_mac}
          </summary>
          <div class="node-children">
            <div>Destination: {packet.dst_mac}</div>
            <div>Source: {packet.src_mac}</div>
            <div>Type: 0x{packet.ether_type.toString(16).padStart(4, '0').toUpperCase()}</div>
          </div>
        </details>

        <!-- IP Layer -->
        <details open class="tree-node">
          <summary>
            <strong>Internet Protocol</strong>, Src: {packet.src_ip}, Dst: {packet.dst_ip}
          </summary>
          <div class="node-children">
            <div>Source Address: {packet.src_ip}</div>
            <div>Destination Address: {packet.dst_ip}</div>
            <div>Protocol: {typeof packet.transport_proto === 'string' ? packet.transport_proto.toUpperCase() : 'Other'}</div>
          </div>
        </details>

        <!-- Transport Layer -->
        {#if packet.src_port !== undefined && packet.src_port !== null}
          <details open class="tree-node">
            <summary>
              <strong>{typeof packet.transport_proto === 'string' ? packet.transport_proto.toUpperCase() : 'Transport'}</strong>, Src Port: {packet.src_port}, Dst Port: {packet.dst_port}
            </summary>
            <div class="node-children">
              <div>Source Port: {packet.src_port}</div>
              <div>Destination Port: {packet.dst_port}</div>
              {#if packet.seq_number !== undefined && packet.seq_number !== null}
                <div>Sequence Number: {packet.seq_number}</div>
                <div>Acknowledgment Number: {packet.ack_number}</div>
                {#if packet.tcp_flags}
                  <div>Flags: {JSON.stringify(packet.tcp_flags)}</div>
                {/if}
              {/if}
            </div>
          </details>
        {/if}

        <!-- Application Layer -->
        {#if packet.app_layer}
          <details open class="tree-node app-layer-node">
            <summary>
              <strong>Application: {packet.app_layer.type.toUpperCase()}</strong>
            </summary>
            <div class="node-children">
              {#if packet.app_layer.type === 'tls'}
                <div>Handshake Type: {packet.app_layer.data.handshake_type}</div>
                <div>Version: {packet.app_layer.data.version}</div>
                <div>Server Name (SNI): <strong class="hl">{packet.app_layer.data.sni || 'N/A'}</strong></div>
                <div>JA3 Fingerprint: <code class="code-pill">{packet.app_layer.data.ja3_fingerprint || 'N/A'}</code></div>
                {#if packet.app_layer.data.ja3_raw}
                  <div>JA3 Raw String: <code class="code-small">{packet.app_layer.data.ja3_raw}</code></div>
                {/if}
              {:else if packet.app_layer.type === 'http'}
                {#if packet.app_layer.data.is_request}
                  <div>HTTP Request: {packet.app_layer.data.method} {packet.app_layer.data.uri}</div>
                  <div>Host: <strong class="hl">{packet.app_layer.data.host || '-'}</strong></div>
                  <div>Content-Type: {packet.app_layer.data.content_type || '-'}</div>
                {:else}
                  <div>HTTP Response: {packet.app_layer.data.status_code} {packet.app_layer.data.status_text}</div>
                  <div>Content-Type: {packet.app_layer.data.content_type || '-'}</div>
                  <div>Content-Length: {packet.app_layer.data.content_length ?? '-'} bytes</div>
                {/if}
              {:else if packet.app_layer.type === 'dns'}
                <div>Transaction ID: 0x{packet.app_layer.data.transaction_id.toString(16).toUpperCase()}</div>
                <div>Questions: {packet.app_layer.data.questions.join(', ') || 'None'}</div>
                <div>Answers: {packet.app_layer.data.answers.join(', ') || 'None'}</div>
              {:else if packet.app_layer.type === 'raw'}
                <div>Raw Payload Preview: <code class="code-small">{packet.app_layer.data.preview}</code></div>
                <div>Length: {packet.app_layer.data.length} bytes</div>
              {/if}
            </div>
          </details>
        {/if}
      </div>
    {/if}

    {#if activeTab === 'hex' || activeTab === 'both'}
      <div class="hex-pane">
        <div class="hex-header">
          <span class="offset-title">Offset</span>
          <span class="hex-bytes-title">00 01 02 03 04 05 06 07  08 09 0A 0B 0C 0D 0E 0F</span>
          <span class="ascii-title">ASCII</span>
        </div>
        <div class="hex-body">
          {#each hexDump as row}
            <div class="hex-row">
              <span class="hex-offset">0x{row.offset.toString(16).padStart(4, '0').toUpperCase()}</span>
              <span class="hex-bytes">{row.hex_bytes}</span>
              <span class="hex-ascii">{row.ascii}</span>
            </div>
          {:else}
            <div class="empty-hex">Select a packet to inspect payload bytes</div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .inspector-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #181825;
    border-top: 1px solid #313244;
  }
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 14px;
    background: #11111b;
    border-bottom: 1px solid #313244;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    color: #cdd6f4;
  }
  .tab-toggles {
    display: flex;
    gap: 4px;
  }
  .tab-toggles button {
    background: #1e1e2e;
    border: 1px solid #313244;
    color: #a6adc8;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
  }
  .tab-toggles button.active {
    background: #89b4fa;
    color: #11111b;
    border-color: #89b4fa;
    font-weight: 600;
  }
  .panel-content {
    display: flex;
    flex: 1;
    overflow: hidden;
  }
  .panel-content.split-mode .tree-pane {
    width: 50%;
    border-right: 1px solid #313244;
  }
  .panel-content.split-mode .hex-pane {
    width: 50%;
  }
  .tree-pane {
    flex: 1;
    overflow-y: auto;
    padding: 10px;
    font-size: 12px;
    color: #cdd6f4;
    background: #181825;
  }
  .tree-node {
    margin-bottom: 6px;
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 6px;
    padding: 6px 10px;
  }
  .tree-node summary {
    cursor: pointer;
    user-select: none;
    font-size: 12px;
    color: #89b4fa;
  }
  .app-layer-node summary {
    color: #cba6f7;
  }
  .node-children {
    padding-top: 6px;
    padding-left: 14px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 11px;
    color: #a6adc8;
  }
  .hl {
    color: #a6e3a1;
  }
  .code-pill {
    background: #11111b;
    color: #fab387;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
    font-family: monospace;
  }
  .code-small {
    background: #11111b;
    color: #94e2d5;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-family: monospace;
  }
  .hex-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: #11111b;
    overflow: hidden;
  }
  .hex-header {
    display: flex;
    padding: 6px 12px;
    background: #181825;
    border-bottom: 1px solid #313244;
    font-family: monospace;
    font-size: 11px;
    color: #6c7086;
  }
  .offset-title {
    width: 70px;
  }
  .hex-bytes-title {
    flex: 1;
  }
  .ascii-title {
    width: 140px;
  }
  .hex-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 12px;
    font-family: monospace;
    font-size: 11px;
  }
  .hex-row {
    display: flex;
    line-height: 1.6;
  }
  .hex-offset {
    width: 70px;
    color: #fab387;
    user-select: none;
  }
  .hex-bytes {
    flex: 1;
    color: #cdd6f4;
    letter-spacing: 0.5px;
  }
  .hex-ascii {
    width: 140px;
    color: #a6e3a1;
    border-left: 1px solid #313244;
    padding-left: 10px;
  }
  .empty-hex {
    padding: 20px;
    text-align: center;
    color: #6c7086;
  }
</style>
