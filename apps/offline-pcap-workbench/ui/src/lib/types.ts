export interface DecodedPacket {
  index: number;
  timestamp_epoch_ms: number;
  delta_time_ms: number;
  wire_len: number;
  captured_len: number;
  src_mac: string;
  dst_mac: string;
  ether_type: number;
  src_ip: string;
  dst_ip: string;
  src_port?: number;
  dst_port?: number;
  transport_proto: string | { other: number };
  app_proto: string;
  info_summary: string;
  stream_id?: number;
  app_layer?: any;
}

export interface HexDumpRow {
  offset: number;
  hex_bytes: string;
  ascii: string;
}

export interface TcpFlow {
  stream_id: number;
  endpoint_a: string;
  endpoint_b: string;
  protocol: string;
  duration_ms: number;
  packet_count: number;
  client_bytes: number;
  server_bytes: number;
  state: string;
  sni?: string;
}

export interface FlowWaterfallEvent {
  packet_index: number;
  relative_ms: number;
  direction: string;
  flags: string;
  seq: number;
  ack: number;
  payload_bytes: number;
  summary: string;
}

export interface ForensicAuditReport {
  report_id: string;
  capture_filename: string;
  total_packets: number;
  duration_seconds: number;
  total_bytes: number;
  tcp_streams: number;
  cleartext_credentials_found: number;
  suspicious_ports: number[];
  tls_ja3_hashes: string[];
  signature_hex: string;
  ed25519_public_key: string;
  status: string;
}
