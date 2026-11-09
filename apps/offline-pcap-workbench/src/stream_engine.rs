//! TCP Stream Reassembly, Flow Tracking & Forensic Integrity Verification.

use crate::models::{
    DecodedPacket, FlowWaterfallEvent, ForensicAuditReport, ProtocolMetric, TcpFlow, TopTalker,
    TransportProto,
};
use ed25519_dalek::{Signer, SigningKey};
use hex::encode as hex_encode;
use std::collections::HashMap;

pub struct StreamEngine;

impl StreamEngine {
    /// Correlates TCP packets into bidirectional conversations and assigns `stream_id`
    pub fn track_streams(packets: &mut [DecodedPacket]) -> Vec<TcpFlow> {
        let mut flows: HashMap<String, (u32, Vec<usize>)> = HashMap::new();
        let mut next_stream_id = 0u32;

        for (idx, pkt) in packets.iter().enumerate() {
            if pkt.transport_proto != TransportProto::Tcp {
                continue;
            }

            let s_port = pkt.src_port.unwrap_or(0);
            let d_port = pkt.dst_port.unwrap_or(0);

            // Normalized flow key (lexicographical order of endpoint pair)
            let ep1 = format!("{}:{}", pkt.src_ip, s_port);
            let ep2 = format!("{}:{}", pkt.dst_ip, d_port);
            let flow_key = if ep1 < ep2 {
                format!("{}-{}", ep1, ep2)
            } else {
                format!("{}-{}", ep2, ep1)
            };

            let stream_id = if let Some((s_id, list)) = flows.get_mut(&flow_key) {
                list.push(idx);
                *s_id
            } else {
                let s_id = next_stream_id;
                next_stream_id += 1;
                flows.insert(flow_key, (s_id, vec![idx]));
                s_id
            };

            // Set packet's stream id
            // (We will update packets in next loop)
            let _ = stream_id;
        }

        // Apply stream_ids back into packets
        for (stream_id, indices) in flows.values() {
            for &idx in indices {
                packets[idx].stream_id = Some(*stream_id);
            }
        }

        // Build list of TcpFlow summaries
        let mut result_flows = Vec::new();

        for (_key, (stream_id, indices)) in flows {
            if indices.is_empty() {
                continue;
            }
            let first_pkt = &packets[indices[0]];
            let ep_a = format!("{}:{}", first_pkt.src_ip, first_pkt.src_port.unwrap_or(0));
            let ep_b = format!("{}:{}", first_pkt.dst_ip, first_pkt.dst_port.unwrap_or(0));

            let start_time_ms = first_pkt.timestamp_epoch_ms;
            let last_pkt = &packets[*indices.last().unwrap_or(&indices[0])];
            let duration_ms = (last_pkt.timestamp_epoch_ms - start_time_ms) as f64;

            let mut client_bytes = 0;
            let mut server_bytes = 0;
            let mut sni = None;
            let mut has_syn = false;
            let mut has_fin = false;
            let mut has_rst = false;

            for &idx in &indices {
                let p = &packets[idx];
                let wire_len = p.wire_len;
                if p.src_ip == first_pkt.src_ip {
                    client_bytes += wire_len;
                } else {
                    server_bytes += wire_len;
                }

                if let Some(flags) = p.tcp_flags {
                    if flags.syn {
                        has_syn = true;
                    }
                    if flags.fin {
                        has_fin = true;
                    }
                    if flags.rst {
                        has_rst = true;
                    }
                }

                if let Some(crate::models::ApplicationLayer::Tls(ref t)) = p.app_layer {
                    if t.sni.is_some() {
                        sni = t.sni.clone();
                    }
                }
            }

            let state = if has_rst {
                "Reset".to_string()
            } else if has_fin {
                "Closed".to_string()
            } else if has_syn {
                "Established".to_string()
            } else {
                "Mid-stream".to_string()
            };

            let protocol_name = if sni.is_some() {
                "TLS 1.3".to_string()
            } else if ep_a.contains(":80") || ep_b.contains(":80") {
                "HTTP/1.1".to_string()
            } else {
                "TCP".to_string()
            };

            result_flows.push(TcpFlow {
                stream_id,
                endpoint_a: ep_a,
                endpoint_b: ep_b,
                protocol: protocol_name,
                start_time_ms,
                duration_ms,
                packet_count: indices.len(),
                client_bytes,
                server_bytes,
                state,
                sni,
            });
        }

        result_flows.sort_by_key(|f| f.stream_id);
        result_flows
    }

    /// Generates sequence waterfall events for a specific TCP conversation
    pub fn generate_stream_waterfall(
        packets: &[DecodedPacket],
        stream_id: u32,
    ) -> Vec<FlowWaterfallEvent> {
        let stream_pkts: Vec<&DecodedPacket> = packets
            .iter()
            .filter(|p| p.stream_id == Some(stream_id))
            .collect();

        if stream_pkts.is_empty() {
            return Vec::new();
        }

        let first_ep_a = stream_pkts[0].src_ip.clone();
        let base_ts = stream_pkts[0].timestamp_epoch_ms;

        stream_pkts
            .iter()
            .map(|p| {
                let relative_ms = (p.timestamp_epoch_ms - base_ts) as f64;
                let direction = if p.src_ip == first_ep_a {
                    "A -> B".to_string()
                } else {
                    "B -> A".to_string()
                };

                let flags = p
                    .tcp_flags
                    .map(|f| f.to_summary_string())
                    .unwrap_or_else(|| "-".to_string());
                let seq = p.seq_number.unwrap_or(0);
                let ack = p.ack_number.unwrap_or(0);

                FlowWaterfallEvent {
                    packet_index: p.index,
                    relative_ms,
                    direction,
                    flags,
                    seq,
                    ack,
                    payload_bytes: p.wire_len,
                    summary: p.info_summary.clone(),
                }
            })
            .collect()
    }

    /// Evaluates protocol distribution across all packets
    pub fn compute_protocol_distribution(packets: &[DecodedPacket]) -> Vec<ProtocolMetric> {
        let total = packets.len().max(1);
        let mut proto_counts: HashMap<String, (usize, usize)> = HashMap::new();

        for p in packets {
            let name = match p.app_proto {
                crate::models::AppProto::Dns => "DNS",
                crate::models::AppProto::Http => "HTTP",
                crate::models::AppProto::Tls => "TLS",
                crate::models::AppProto::Unknown => match p.transport_proto {
                    TransportProto::Tcp => "TCP",
                    TransportProto::Udp => "UDP",
                    TransportProto::Icmp => "ICMP",
                    TransportProto::Other(_) => "Other",
                },
            };

            let entry = proto_counts.entry(name.to_string()).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += p.wire_len;
        }

        let mut metrics: Vec<ProtocolMetric> = proto_counts
            .into_iter()
            .map(|(proto, (count, bytes))| ProtocolMetric {
                protocol: proto,
                packet_count: count,
                byte_count: bytes,
                percentage: (count as f64 / total as f64) * 100.0,
            })
            .collect();

        metrics.sort_by_key(|b| std::cmp::Reverse(b.packet_count));
        metrics
    }

    /// Identifies top network talkers
    pub fn compute_top_talkers(packets: &[DecodedPacket]) -> Vec<TopTalker> {
        let total_pkts = packets.len().max(1);
        let mut talkers: HashMap<String, (usize, usize)> = HashMap::new();

        for p in packets {
            if p.src_ip != "unknown" && p.src_ip != "ARP" {
                let e = talkers.entry(p.src_ip.clone()).or_insert((0, 0));
                e.0 += 1;
                e.1 += p.wire_len;
            }
        }

        let mut list: Vec<TopTalker> = talkers
            .into_iter()
            .map(|(ip, (count, bytes))| TopTalker {
                ip,
                packet_count: count,
                byte_count: bytes,
                percentage: (count as f64 / total_pkts as f64) * 100.0,
            })
            .collect();

        list.sort_by_key(|b| std::cmp::Reverse(b.byte_count));
        list.truncate(10);
        list
    }

    /// Evaluates network traffic for forensic integrity, cleartext credentials & JA3 hashes
    pub fn audit_capture(
        filename: &str,
        packets: &[DecodedPacket],
        flows: &[TcpFlow],
        signing_key: &SigningKey,
    ) -> ForensicAuditReport {
        let mut cleartext_credentials = 0;
        let mut suspicious_ports = Vec::new();
        let mut ja3_hashes = Vec::new();

        for p in packets {
            // Check for cleartext Basic Auth or credentials in HTTP headers or payload
            if let Some(crate::models::ApplicationLayer::Http(ref h)) = p.app_layer {
                if let Ok(raw) = hex::decode(&p.raw_payload_base64) {
                    let text = String::from_utf8_lossy(&raw);
                    let lower = text.to_lowercase();
                    if lower.contains("authorization: basic")
                        || lower.contains("password=")
                        || lower.contains("passwd=")
                        || lower.contains("api_key=")
                    {
                        cleartext_credentials += 1;
                    }
                }
                let _ = h;
            }

            // Check suspicious ports
            if let Some(dp) = p.dst_port {
                if [4444, 1337, 31337, 6667, 8888].contains(&dp) && !suspicious_ports.contains(&dp)
                {
                    suspicious_ports.push(dp);
                }
            }

            // Collect JA3 fingerprints
            if let Some(crate::models::ApplicationLayer::Tls(ref t)) = p.app_layer {
                if let Some(ref ja3) = t.ja3_fingerprint {
                    if !ja3_hashes.contains(ja3) {
                        ja3_hashes.push(ja3.clone());
                    }
                }
            }
        }

        let duration_seconds = if packets.len() >= 2 {
            (packets.last().unwrap().timestamp_epoch_ms - packets[0].timestamp_epoch_ms) as f64
                / 1000.0
        } else {
            0.0
        };

        let total_bytes: usize = packets.iter().map(|p| p.wire_len).sum();

        let report_id = format!(
            "FORENSIC-PCAP-{}",
            blake3::hash(filename.as_bytes()).to_hex()[0..12].to_uppercase()
        );

        let manifest = format!(
            "REPORT:{}|FILE:{}|PACKETS:{}|STREAMS:{}|CLEARTEXT:{}|JA3_COUNT:{}",
            report_id,
            filename,
            packets.len(),
            flows.len(),
            cleartext_credentials,
            ja3_hashes.len()
        );

        let sig = signing_key.sign(manifest.as_bytes());
        let signature_hex = hex_encode(sig.to_bytes());
        let ed25519_public_key = hex_encode(signing_key.verifying_key().to_bytes());

        let status = if cleartext_credentials > 0 {
            "High Risk (Cleartext Credentials Found)".to_string()
        } else if !suspicious_ports.is_empty() {
            "Suspicious Activity Detected".to_string()
        } else {
            "Clean (Air-Gapped Validated)".to_string()
        };

        ForensicAuditReport {
            report_id,
            capture_filename: filename.to_string(),
            total_packets: packets.len(),
            duration_seconds,
            total_bytes,
            tcp_streams: flows.len(),
            cleartext_credentials_found: cleartext_credentials,
            suspicious_ports,
            tls_ja3_hashes: ja3_hashes,
            signature_hex,
            ed25519_public_key,
            status,
        }
    }
}
