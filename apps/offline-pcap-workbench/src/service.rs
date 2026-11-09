//! Thread-safe packet capture store and service management.

use crate::models::{
    CaptureSummary, DecodedPacket, FlowWaterfallEvent, ForensicAuditReport, HexDumpRow, PcapStats,
    TcpFlow,
};
use crate::pcap_parser::PcapParser;
use crate::protocol_decoder::ProtocolDecoder;
use crate::stream_engine::StreamEngine;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct LoadedCapture {
    pub summary: CaptureSummary,
    pub packets: Vec<DecodedPacket>,
    pub flows: Vec<TcpFlow>,
    pub audit_report: ForensicAuditReport,
}

pub struct PcapService {
    captures: Arc<RwLock<HashMap<String, LoadedCapture>>>,
    stats: Arc<RwLock<PcapStats>>,
    master_key: SigningKey,
}

impl Default for PcapService {
    fn default() -> Self {
        Self::new()
    }
}

impl PcapService {
    pub fn new() -> Self {
        let master_key = SigningKey::generate(&mut OsRng);
        let service = Self {
            captures: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PcapStats {
                total_captures_analyzed: 0,
                total_packets_decoded: 0,
                total_bytes_analyzed: 0,
                total_streams_tracked: 0,
                total_anomalies_flagged: 0,
            })),
            master_key,
        };

        service.seed_demo_capture();
        service
    }

    /// Seeds realistic multi-protocol capture demonstrating DNS, TLS 1.3, HTTP Basic Auth, and TCP streams
    fn seed_demo_capture(&self) {
        let now_ms = chrono::Utc::now().timestamp_millis();

        // 1. Packet 1: DNS Query for api.internal.cloud (UDP port 53)
        // Ethernet (14) + IPv4 (20) + UDP (8) + DNS (28) = 70 bytes
        let p1 = vec![
            // Eth: Dst (6), Src (6), EtherType (2 = 0x0800)
            0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x52, 0x54, 0x00, 0x12, 0x34, 0x56, 0x08, 0x00,
            // IPv4: Version/IHL (0x45), DSCP (0), Len (56), ID (0x1001), Flags/Frag (0x4000), TTL (64), Proto (17 = UDP), Cksum (0), Src (192.168.1.50), Dst (1.1.1.1)
            0x45, 0x00, 0x00, 0x38, 0x10, 0x01, 0x40, 0x00, 0x40, 0x11, 0x00, 0x00, 192, 168, 1, 50,
            1, 1, 1, 1, // UDP: Src (53241), Dst (53), Len (36), Cksum (0)
            0xD0, 0x01, 0x00, 0x35, 0x00, 0x24, 0x00, 0x00,
            // DNS: TxID (0x7F21), Flags (0x0100 standard query), QDCount (1), ANCount (0), NSCount (0), ARCount (0)
            0x7F, 0x21, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            // QNAME: 3 'api' 8 'internal' 5 'cloud' 0
            0x03, b'a', b'p', b'i', 0x08, b'i', b'n', b't', b'e', b'r', b'n', b'a', b'l', 0x05,
            b'c', b'l', b'o', b'u', b'd', 0x00, // QTYPE: A (0x0001), QCLASS: IN (0x0001)
            0x00, 0x01, 0x00, 0x01,
        ];

        // 2. Packet 2: DNS Response with IP 10.200.4.15
        let p2 = vec![
            0x52, 0x54, 0x00, 0x12, 0x34, 0x56, 0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x08, 0x00,
            0x45, 0x00, 0x00, 0x48, 0x10, 0x02, 0x40, 0x00, 0x40, 0x11, 0x00, 0x00, 1, 1, 1, 1,
            192, 168, 1, 50, 0x00, 0x35, 0xD0, 0x01, 0x00, 0x34, 0x00, 0x00, 0x7F, 0x21, 0x81,
            0x80, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x03, b'a', b'p', b'i', 0x08,
            b'i', b'n', b't', b'e', b'r', b'n', b'a', b'l', 0x05, b'c', b'l', b'o', b'u', b'd',
            0x00, 0x00, 0x01, 0x00, 0x01,
        ];

        // 3. Packet 3: TCP Handshake [SYN] from 192.168.1.50:49210 to 10.200.4.15:443
        let p3 = vec![
            0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x52, 0x54, 0x00, 0x12, 0x34, 0x56, 0x08, 0x00,
            0x45, 0x00, 0x00, 0x28, 0x20, 0x01, 0x40, 0x00, 0x40, 0x06, 0x00, 0x00, 192, 168, 1,
            50, 10, 200, 4, 15,
            // TCP: Src (49210 = 0xC03A), Dst (443 = 0x01BB), Seq (1000), Ack (0), DataOffset/Flags (0x50, SYN=0x02), Win (64240)
            0xC0, 0x3A, 0x01, 0xBB, 0x00, 0x00, 0x03, 0xE8, 0x00, 0x00, 0x00, 0x00, 0x50, 0x02,
            0xFA, 0xF0, 0x00, 0x00, 0x00, 0x00,
        ];

        // 4. Packet 4: TCP Handshake [SYN, ACK] from 10.200.4.15:443 to 192.168.1.50:49210
        let p4 = vec![
            0x52, 0x54, 0x00, 0x12, 0x34, 0x56, 0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x08, 0x00,
            0x45, 0x00, 0x00, 0x28, 0x30, 0x01, 0x40, 0x00, 0x40, 0x06, 0x00, 0x00, 10, 200, 4, 15,
            192, 168, 1, 50, 0x01, 0xBB, 0xC0, 0x3A, 0x00, 0x00, 0x07, 0xD0, 0x00, 0x00, 0x03,
            0xE9, 0x50, 0x12, 0x72, 0x10, 0x00, 0x00, 0x00, 0x00,
        ];

        // 5. Packet 5: TCP Handshake [ACK]
        let p5 = vec![
            0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x52, 0x54, 0x00, 0x12, 0x34, 0x56, 0x08, 0x00,
            0x45, 0x00, 0x00, 0x28, 0x20, 0x02, 0x40, 0x00, 0x40, 0x06, 0x00, 0x00, 192, 168, 1,
            50, 10, 200, 4, 15, 0xC0, 0x3A, 0x01, 0xBB, 0x00, 0x00, 0x03, 0xE9, 0x00, 0x00, 0x07,
            0xD1, 0x50, 0x10, 0xFA, 0xF0, 0x00, 0x00, 0x00, 0x00,
        ];

        // 6. Packet 6: TLS 1.3 Client Hello with SNI "api.internal.cloud"
        let mut p6 = vec![
            0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x52, 0x54, 0x00, 0x12, 0x34, 0x56, 0x08, 0x00,
            0x45, 0x00, 0x00, 0x90, 0x20, 0x03, 0x40, 0x00, 0x40, 0x06, 0x00, 0x00, 192, 168, 1,
            50, 10, 200, 4, 15, 0xC0, 0x3A, 0x01, 0xBB, 0x00, 0x00, 0x03, 0xE9, 0x00, 0x00, 0x07,
            0xD1, 0x50, 0x18, 0xFA, 0xF0, 0x00, 0x00, 0x00,
            0x00, // TLS Record: ContentType 22 (Handshake), Version 0x0303, Length 96
            0x16, 0x03, 0x03, 0x00, 0x60, // Handshake: Type 1 (ClientHello), Length 92
            0x01, 0x00, 0x00, 0x5C, // Client Version: 0x0303
            0x03, 0x03,
        ];
        // 32 bytes random
        p6.extend_from_slice(&[0xAA; 32]);
        // Session ID length: 0
        p6.push(0x00);
        // Cipher Suites length: 4 (TLS_AES_128_GCM_SHA256 = 0x1301, TLS_AES_256_GCM_SHA384 = 0x1302)
        p6.extend_from_slice(&[0x00, 0x04, 0x13, 0x01, 0x13, 0x02]);
        // Compression methods: 1 (0x00 null)
        p6.extend_from_slice(&[0x01, 0x00]);
        // Extensions length: 27
        p6.extend_from_slice(&[0x00, 0x1B]);
        // Extension 0x0000 (SNI): Length 23
        p6.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x17, // SNI List length: 21
            0x00, 0x15, // Server Name type: 0 (host_name)
            0x00, // Host Name length: 18
            0x00, 0x12,
        ]);
        p6.extend_from_slice(b"api.internal.cloud");

        // 7. Packet 7: HTTP Plaintext Basic Auth Login to port 80 (Flagged Alert)
        let http_payload = b"POST /v1/auth/login HTTP/1.1\r\nHost: portal.corp.local\r\nAuthorization: Basic YWRtaW46U3VwZXJTZWNyZXQxMjMh\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 17\r\n\r\nusername=admin_corp";
        let mut p7 = vec![
            0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x52, 0x54, 0x00, 0x12, 0x34, 0x56, 0x08, 0x00,
            0x45, 0x00, 0x00, 0xC8, 0x40, 0x01, 0x40, 0x00, 0x40, 0x06, 0x00, 0x00, 192, 168, 1,
            50, 192, 168, 1, 200, 0xD0, 0x02, 0x00, 0x50, // Port 80
            0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x01, 0x50, 0x18, 0xFA, 0xF0, 0x00, 0x00,
            0x00, 0x00,
        ];
        p7.extend_from_slice(http_payload);

        let raw_packets: Vec<(&[u8], i64)> = vec![
            (&p1, now_ms),
            (&p2, now_ms + 12),
            (&p3, now_ms + 25),
            (&p4, now_ms + 34),
            (&p5, now_ms + 36),
            (&p6, now_ms + 52),
            (&p7, now_ms + 120),
        ];

        let demo_pcap_bytes = PcapParser::build_demo_pcap(&raw_packets);
        let _ = self.parse_uploaded_capture("enterprise_gateway_traffic.pcap", &demo_pcap_bytes);
    }

    /// Ingests and processes a PCAP/PCAPNG capture
    pub fn parse_uploaded_capture(
        &self,
        filename: &str,
        raw_bytes: &[u8],
    ) -> Result<CaptureSummary, String> {
        let mut packets = PcapParser::parse_capture(raw_bytes)?;
        let flows = StreamEngine::track_streams(&mut packets);
        let protocol_distribution = StreamEngine::compute_protocol_distribution(&packets);
        let top_talkers = StreamEngine::compute_top_talkers(&packets);
        let audit_report =
            StreamEngine::audit_capture(filename, &packets, &flows, &self.master_key);

        let total_packets = packets.len();
        let file_size_bytes = raw_bytes.len();
        let start_time_epoch_ms = packets
            .first()
            .map(|p| p.timestamp_epoch_ms)
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
        let end_time_epoch_ms = packets
            .last()
            .map(|p| p.timestamp_epoch_ms)
            .unwrap_or(start_time_epoch_ms);
        let duration_seconds = (end_time_epoch_ms - start_time_epoch_ms) as f64 / 1000.0;

        let capture_id = format!("cap-{}", &blake3::hash(filename.as_bytes()).to_hex()[0..12]);

        let summary = CaptureSummary {
            capture_id: capture_id.clone(),
            filename: filename.to_string(),
            file_size_bytes,
            total_packets,
            start_time_epoch_ms,
            end_time_epoch_ms,
            duration_seconds,
            protocol_distribution,
            top_talkers,
            active_tcp_flows: flows.len(),
        };

        // Update workbench global stats
        if let Ok(mut stats) = self.stats.write() {
            stats.total_captures_analyzed += 1;
            stats.total_packets_decoded += total_packets;
            stats.total_bytes_analyzed += file_size_bytes;
            stats.total_streams_tracked += flows.len();
            stats.total_anomalies_flagged += audit_report.cleartext_credentials_found;
        }

        // Store loaded capture
        if let Ok(mut lock) = self.captures.write() {
            lock.insert(
                capture_id,
                LoadedCapture {
                    summary: summary.clone(),
                    packets,
                    flows,
                    audit_report,
                },
            );
        }

        Ok(summary)
    }

    pub fn get_stats(&self) -> PcapStats {
        self.stats.read().unwrap().clone()
    }

    pub fn get_capture_summary(&self, capture_id: &str) -> Option<CaptureSummary> {
        let lock = self.captures.read().unwrap();
        guard_find_capture(&lock, capture_id, |c| c.summary.clone())
    }

    pub fn get_packets(
        &self,
        capture_id: &str,
        filter: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> (Vec<DecodedPacket>, usize) {
        let lock = self.captures.read().unwrap();
        guard_find_capture(&lock, capture_id, |c| {
            let filter_str = filter.map(|s| s.trim().to_lowercase());

            let filtered: Vec<&DecodedPacket> = c
                .packets
                .iter()
                .filter(|p| {
                    if let Some(ref q) = filter_str {
                        if q.is_empty() {
                            return true;
                        }
                        p.src_ip.to_lowercase().contains(q)
                            || p.dst_ip.to_lowercase().contains(q)
                            || p.info_summary.to_lowercase().contains(q)
                            || format!("{:?}", p.transport_proto)
                                .to_lowercase()
                                .contains(q)
                            || format!("{:?}", p.app_proto).to_lowercase().contains(q)
                    } else {
                        true
                    }
                })
                .collect();

            let total = filtered.len();
            let slice: Vec<DecodedPacket> = filtered
                .into_iter()
                .skip(offset)
                .take(limit)
                .cloned()
                .collect();

            (slice, total)
        })
        .unwrap_or((Vec::new(), 0))
    }

    pub fn get_packet_detail(
        &self,
        capture_id: &str,
        packet_index: usize,
    ) -> Option<(DecodedPacket, Vec<HexDumpRow>)> {
        let lock = self.captures.read().unwrap();
        guard_find_capture(&lock, capture_id, |c| {
            c.packets.get(packet_index.saturating_sub(1)).map(|pkt| {
                let raw_bytes = hex::decode(&pkt.raw_payload_base64).unwrap_or_default();
                let hex_dump = ProtocolDecoder::generate_hex_dump(&raw_bytes);
                (pkt.clone(), hex_dump)
            })
        })
        .flatten()
    }

    pub fn get_tcp_flows(&self, capture_id: &str) -> Vec<TcpFlow> {
        let lock = self.captures.read().unwrap();
        guard_find_capture(&lock, capture_id, |c| c.flows.clone()).unwrap_or_default()
    }

    pub fn get_stream_waterfall(
        &self,
        capture_id: &str,
        stream_id: u32,
    ) -> Vec<FlowWaterfallEvent> {
        let lock = self.captures.read().unwrap();
        guard_find_capture(&lock, capture_id, |c| {
            StreamEngine::generate_stream_waterfall(&c.packets, stream_id)
        })
        .unwrap_or_default()
    }

    pub fn get_forensic_report(&self, capture_id: &str) -> Option<ForensicAuditReport> {
        let lock = self.captures.read().unwrap();
        guard_find_capture(&lock, capture_id, |c| c.audit_report.clone())
    }
}

fn guard_find_capture<R, F>(
    lock: &HashMap<String, LoadedCapture>,
    capture_id: &str,
    f: F,
) -> Option<R>
where
    F: FnOnce(&LoadedCapture) -> R,
{
    if let Some(c) = lock.get(capture_id) {
        Some(f(c))
    } else {
        // Fallback to first available capture
        lock.values().next().map(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_seed_and_retrieval() {
        let service = PcapService::new();
        let stats = service.get_stats();
        assert_eq!(stats.total_captures_analyzed, 1);
        assert_eq!(stats.total_packets_decoded, 7);
        assert!(stats.total_anomalies_flagged >= 1); // Found HTTP Basic Auth alert

        let flows = service.get_tcp_flows("");
        assert!(!flows.is_empty());

        let waterfall = service.get_stream_waterfall("", flows[0].stream_id);
        assert!(!waterfall.is_empty());

        let report = service.get_forensic_report("").expect("Forensic report");
        assert_eq!(report.cleartext_credentials_found, 1);
    }
}
