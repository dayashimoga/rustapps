<script lang="ts">
  export interface DecodedPacket {
    packet_id: number;
    timestamp_us: number;
    protocol: string;
    address_hex: string | null;
    data_hex: string;
    ascii_preview: string;
    is_ack: boolean;
    error_flags: string | null;
  }

  export let packets: DecodedPacket[] = [];
</script>

<div class="protocol-table-container">
  <div class="protocol-header">
    <div class="header-title">
      <span class="icon">⚡</span>
      <span class="title">SERIAL PROTOCOL BUS DECODER (UART / I2C / SPI / CAN)</span>
    </div>
    <span class="count-badge">{packets.length} Packets Decoded</span>
  </div>

  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th>#</th>
          <th>Time (μs)</th>
          <th>Protocol</th>
          <th>Address</th>
          <th>Data (Hex)</th>
          <th>ASCII Preview</th>
          <th>Status</th>
        </tr>
      </thead>
      <tbody>
        {#if packets.length === 0}
          <tr>
            <td colspan="7" class="empty-row">No serial protocol packets detected in current capture buffer</td>
          </tr>
        {:else}
          {#each packets as pkt}
            <tr>
              <td class="mono">{pkt.packet_id}</td>
              <td class="mono">{pkt.timestamp_us}</td>
              <td><span class="proto-tag">{pkt.protocol}</span></td>
              <td class="mono">{pkt.address_hex || '—'}</td>
              <td class="mono hex-col">{pkt.data_hex}</td>
              <td class="mono ascii-col">{pkt.ascii_preview}</td>
              <td>
                {#if pkt.error_flags}
                  <span class="status-err">{pkt.error_flags}</span>
                {:else if pkt.is_ack}
                  <span class="status-ack">ACK</span>
                {:else}
                  <span class="status-ok">OK</span>
                {/if}
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>
</div>

<style>
  .protocol-table-container {
    display: flex;
    flex-direction: column;
    background: #0d131a;
    border: 1px solid #1e2c3d;
    border-radius: 8px;
    overflow: hidden;
  }

  .protocol-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 14px;
    background: #141c26;
    border-bottom: 1px solid #1e2c3d;
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    font-weight: 600;
    color: #ffaa00;
    font-family: var(--font-mono);
  }

  .count-badge {
    background: #ffaa0022;
    color: #ffaa00;
    border: 1px solid #ffaa0044;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 10px;
    font-weight: 600;
    font-family: var(--font-mono);
  }

  .table-wrap {
    max-height: 180px;
    overflow-y: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
    text-align: left;
  }

  th {
    background: #101620;
    color: #8b949e;
    padding: 6px 10px;
    font-weight: 600;
    font-size: 10px;
    border-bottom: 1px solid #1e2c3d;
    position: sticky;
    top: 0;
  }

  td {
    padding: 6px 10px;
    border-bottom: 1px solid #16202c;
  }

  .mono {
    font-family: var(--font-mono);
  }

  .hex-col {
    color: #58a6ff;
  }

  .ascii-col {
    color: #7ee787;
  }

  .proto-tag {
    background: #1f2d3d;
    color: #c9d1d9;
    padding: 2px 5px;
    border-radius: 4px;
    font-size: 10px;
  }

  .status-ok {
    color: #7ee787;
    font-weight: 600;
  }

  .status-ack {
    color: #79c0ff;
    font-weight: 600;
  }

  .status-err {
    color: #f85149;
    font-weight: 600;
  }

  .empty-row {
    text-align: center;
    padding: 18px;
    color: #8b949e;
    font-style: italic;
  }
</style>

