//! Data models for Offline PCAP Workbench
//!
//! Represents captured network packets, decoded protocol layers,
//! reassembled TCP streams, and forensic audit reports.

use serde::{Deserialize, Serialize};

/// High-level transport protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportProto {
    Tcp,
    Udp,
    Icmp,
    Other(u8),
}

/// Application layer protocol classification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppProto {
    Dns,
    Http,
    Tls,
    Unknown,
}

/// TCP flag bitmask breakdown
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TcpFlags {
    pub syn: bool,
    pub ack: bool,
    pub fin: bool,
    pub rst: bool,
    pub psh: bool,
    pub urg: bool,
}

impl TcpFlags {
    pub fn from_byte(b: u8) -> Self {
        Self {
            fin: (b & 0x01) != 0,
            syn: (b & 0x02) != 0,
            rst: (b & 0x04) != 0,
            psh: (b & 0x08) != 0,
            ack: (b & 0x10) != 0,
            urg: (b & 0x20) != 0,
        }
    }

    pub fn to_summary_string(&self) -> String {
        let mut flags = Vec::new();
        if self.syn {
            flags.push("SYN");
        }
        if self.ack {
            flags.push("ACK");
        }
        if self.psh {
            flags.push("PSH");
        }
        if self.fin {
            flags.push("FIN");
        }
        if self.rst {
            flags.push("RST");
        }
        if self.urg {
            flags.push("URG");
        }
        if flags.is_empty() {
            "NONE".to_string()
        } else {
            flags.join(", ")
        }
    }
}

/// DNS Query / Response record detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsInfo {
    pub transaction_id: u16,
    pub is_response: bool,
    pub questions: Vec<String>,
    pub answers: Vec<String>,
}

/// HTTP/1.x payload summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpInfo {
    pub is_request: bool,
    pub method: Option<String>,
    pub uri: Option<String>,
    pub status_code: Option<u16>,
    pub status_text: Option<String>,
    pub host: Option<String>,
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
}

/// TLS Client Hello / Handshake metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsInfo {
    pub handshake_type: String,
    pub version: String,
    pub sni: Option<String>,
    pub ja3_fingerprint: Option<String>,
    pub ja3_raw: Option<String>,
    pub cipher_suite_count: usize,
}

/// Application layer payload breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ApplicationLayer {
    Dns(DnsInfo),
    Http(HttpInfo),
    Tls(TlsInfo),
    Raw { preview: String, length: usize },
}

/// Hex Dump Row (16 bytes per row)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexDumpRow {
    pub offset: usize,
    pub hex_bytes: String,
    pub ascii: String,
}

/// Fully decoded packet representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedPacket {
    pub index: usize,
    pub timestamp_epoch_ms: i64,
    pub delta_time_ms: f64,
    pub wire_len: usize,
    pub captured_len: usize,
    pub src_mac: String,
    pub dst_mac: String,
    pub ether_type: u16,
    pub src_ip: String,
    pub dst_ip: String,
    pub transport_proto: TransportProto,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub tcp_flags: Option<TcpFlags>,
    pub seq_number: Option<u32>,
    pub ack_number: Option<u32>,
    pub app_proto: AppProto,
    pub app_layer: Option<ApplicationLayer>,
    pub info_summary: String,
    pub stream_id: Option<u32>,
    pub raw_payload_base64: String,
}

/// Stream conversation item for sequence waterfall
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowWaterfallEvent {
    pub packet_index: usize,
    pub relative_ms: f64,
    pub direction: String, // "A -> B" or "B -> A"
    pub flags: String,
    pub seq: u32,
    pub ack: u32,
    pub payload_bytes: usize,
    pub summary: String,
}

/// Reassembled TCP Flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpFlow {
    pub stream_id: u32,
    pub endpoint_a: String, // "192.168.1.50:49210"
    pub endpoint_b: String, // "10.0.0.1:443"
    pub protocol: String,
    pub start_time_ms: i64,
    pub duration_ms: f64,
    pub packet_count: usize,
    pub client_bytes: usize,
    pub server_bytes: usize,
    pub state: String, // "Handshake", "Established", "Closed", "Reset"
    pub sni: Option<String>,
}

/// Forensic Analysis & Security Audit Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForensicAuditReport {
    pub report_id: String,
    pub capture_filename: String,
    pub total_packets: usize,
    pub duration_seconds: f64,
    pub total_bytes: usize,
    pub tcp_streams: usize,
    pub cleartext_credentials_found: usize,
    pub suspicious_ports: Vec<u16>,
    pub tls_ja3_hashes: Vec<String>,
    pub signature_hex: String,
    pub ed25519_public_key: String,
    pub status: String,
}

/// Top Talker Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopTalker {
    pub ip: String,
    pub packet_count: usize,
    pub byte_count: usize,
    pub percentage: f64,
}

/// Protocol Distribution Item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMetric {
    pub protocol: String,
    pub packet_count: usize,
    pub byte_count: usize,
    pub percentage: f64,
}

/// Overall Capture Summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSummary {
    pub capture_id: String,
    pub filename: String,
    pub file_size_bytes: usize,
    pub total_packets: usize,
    pub start_time_epoch_ms: i64,
    pub end_time_epoch_ms: i64,
    pub duration_seconds: f64,
    pub protocol_distribution: Vec<ProtocolMetric>,
    pub top_talkers: Vec<TopTalker>,
    pub active_tcp_flows: usize,
}

/// Workbench Stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcapStats {
    pub total_captures_analyzed: usize,
    pub total_packets_decoded: usize,
    pub total_bytes_analyzed: usize,
    pub total_streams_tracked: usize,
    pub total_anomalies_flagged: usize,
}
