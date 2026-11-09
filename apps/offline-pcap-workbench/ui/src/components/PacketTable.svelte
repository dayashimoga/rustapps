<script lang="ts">
  import type { DecodedPacket } from '../lib/types';

  export let packets: DecodedPacket[] = [];
  export let selectedPacketIndex: number = 1;
  export let onSelectPacket: (index: number) => void = () => {};

  function formatProto(p: DecodedPacket): string {
    if (p.app_proto && p.app_proto !== 'unknown') {
      return p.app_proto.toUpperCase();
    }
    if (typeof p.transport_proto === 'string') {
      return p.transport_proto.toUpperCase();
    }
    return 'IP';
  }

  function getProtoClass(proto: string): string {
    switch (proto.toLowerCase()) {
      case 'tls':
        return 'proto-tls';
      case 'http':
        return 'proto-http';
      case 'dns':
        return 'proto-dns';
      case 'tcp':
        return 'proto-tcp';
      case 'udp':
        return 'proto-udp';
      case 'icmp':
        return 'proto-icmp';
      default:
        return 'proto-other';
    }
  }

  function formatEndpoint(ip: string, port?: number): string {
    if (port !== undefined && port > 0) {
      return `${ip}:${port}`;
    }
    return ip;
  }
</script>

<div class="table-container">
  <table class="packet-table">
    <thead>
      <tr>
        <th class="col-num">#</th>
        <th class="col-time">Time (+ms)</th>
        <th class="col-src">Source</th>
        <th class="col-dst">Destination</th>
        <th class="col-proto">Protocol</th>
        <th class="col-len">Length</th>
        <th class="col-info">Info</th>
      </tr>
    </thead>
    <tbody>
      {#each packets as pkt}
        {@const proto = formatProto(pkt)}
        <tr
          class:active={pkt.index === selectedPacketIndex}
          on:click={() => onSelectPacket(pkt.index)}
        >
          <td class="col-num">{pkt.index}</td>
          <td class="col-time">{pkt.delta_time_ms.toFixed(1)}</td>
          <td class="col-src" title={pkt.src_ip}>{formatEndpoint(pkt.src_ip, pkt.src_port)}</td>
          <td class="col-dst" title={pkt.dst_ip}>{formatEndpoint(pkt.dst_ip, pkt.dst_port)}</td>
          <td class="col-proto">
            <span class="badge {getProtoClass(proto)}">{proto}</span>
          </td>
          <td class="col-len">{pkt.wire_len}</td>
          <td class="col-info" title={pkt.info_summary}>{pkt.info_summary}</td>
        </tr>
      {:else}
        <tr>
          <td colspan="7" class="empty-state">No packets match the active filter</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .table-container {
    height: 100%;
    overflow-y: auto;
    background: #11111b;
  }
  .packet-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace;
  }
  th {
    position: sticky;
    top: 0;
    background: #181825;
    color: #a6adc8;
    text-align: left;
    padding: 8px 10px;
    font-weight: 600;
    border-bottom: 1px solid #313244;
    user-select: none;
    z-index: 10;
  }
  td {
    padding: 6px 10px;
    border-bottom: 1px solid #1e1e2e;
    color: #cdd6f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  tr {
    cursor: pointer;
    transition: background 0.1s ease;
  }
  tr:hover {
    background: #1e1e2e;
  }
  tr.active {
    background: #313244 !important;
    color: #89b4fa;
  }
  .col-num {
    width: 50px;
    color: #6c7086;
  }
  .col-time {
    width: 80px;
    font-family: monospace;
    color: #a6adc8;
  }
  .col-src {
    width: 160px;
    font-family: monospace;
  }
  .col-dst {
    width: 160px;
    font-family: monospace;
  }
  .col-proto {
    width: 75px;
    text-align: center;
  }
  .col-len {
    width: 60px;
    text-align: right;
    font-family: monospace;
    color: #a6adc8;
  }
  .col-info {
    max-width: 450px;
  }
  .badge {
    display: inline-block;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    font-family: monospace;
  }
  .proto-tls {
    background: rgba(203, 166, 247, 0.2);
    color: #cba6f7;
    border: 1px solid rgba(203, 166, 247, 0.4);
  }
  .proto-http {
    background: rgba(166, 227, 161, 0.2);
    color: #a6e3a1;
    border: 1px solid rgba(166, 227, 161, 0.4);
  }
  .proto-dns {
    background: rgba(137, 220, 235, 0.2);
    color: #89dceb;
    border: 1px solid rgba(137, 220, 235, 0.4);
  }
  .proto-tcp {
    background: rgba(137, 180, 250, 0.2);
    color: #89b4fa;
    border: 1px solid rgba(137, 180, 250, 0.4);
  }
  .proto-udp {
    background: rgba(250, 179, 135, 0.2);
    color: #fab387;
    border: 1px solid rgba(250, 179, 135, 0.4);
  }
  .proto-icmp {
    background: rgba(249, 226, 175, 0.2);
    color: #f9e2af;
    border: 1px solid rgba(249, 226, 175, 0.4);
  }
  .proto-other {
    background: rgba(108, 112, 134, 0.2);
    color: #a6adc8;
    border: 1px solid rgba(108, 112, 134, 0.4);
  }
  .empty-state {
    text-align: center;
    padding: 30px;
    color: #6c7086;
  }
</style>
