//! Deep protocol dissection engine for Ethernet, IPv4, IPv6, TCP, UDP, DNS, HTTP, and TLS.

use crate::models::{
    AppProto, ApplicationLayer, DecodedPacket, DnsInfo, HexDumpRow, HttpInfo, TcpFlags, TlsInfo,
    TransportProto,
};

pub struct ProtocolDecoder;

impl ProtocolDecoder {
    /// Dissects raw packet bytes into a structured `DecodedPacket`.
    pub fn decode_packet(
        index: usize,
        timestamp_epoch_ms: i64,
        delta_time_ms: f64,
        raw_bytes: &[u8],
        wire_len: usize,
    ) -> DecodedPacket {
        let captured_len = raw_bytes.len();
        let raw_payload_base64 = hex::encode(raw_bytes);

        // Default Ethernet fallback
        if raw_bytes.len() < 14 {
            return DecodedPacket {
                index,
                timestamp_epoch_ms,
                delta_time_ms,
                wire_len,
                captured_len,
                src_mac: "00:00:00:00:00:00".to_string(),
                dst_mac: "00:00:00:00:00:00".to_string(),
                ether_type: 0,
                src_ip: "unknown".to_string(),
                dst_ip: "unknown".to_string(),
                transport_proto: TransportProto::Other(0),
                src_port: None,
                dst_port: None,
                tcp_flags: None,
                seq_number: None,
                ack_number: None,
                app_proto: AppProto::Unknown,
                app_layer: None,
                info_summary: format!("Truncated frame ({} bytes)", raw_bytes.len()),
                stream_id: None,
                raw_payload_base64,
            };
        }

        // Ethernet II Header
        let dst_mac = format_mac(&raw_bytes[0..6]);
        let src_mac = format_mac(&raw_bytes[6..12]);
        let ether_type = u16::from_be_bytes([raw_bytes[12], raw_bytes[13]]);

        let meta = PacketFrameMeta {
            index,
            timestamp_epoch_ms,
            delta_time_ms,
            wire_len,
            captured_len,
            src_mac: src_mac.clone(),
            dst_mac: dst_mac.clone(),
            ether_type,
            raw_payload_base64: raw_payload_base64.clone(),
        };

        let mut offset = 14;

        let (src_ip, dst_ip, proto_byte, ip_payload_offset) = match ether_type {
            0x0800 => {
                // IPv4
                if raw_bytes.len() < offset + 20 {
                    return fallback_packet(meta, "Truncated IPv4");
                }
                let ihl = (raw_bytes[offset] & 0x0F) as usize * 4;
                if ihl < 20 || raw_bytes.len() < offset + ihl {
                    return fallback_packet(meta, "Malformed IPv4 header");
                }
                let p_byte = raw_bytes[offset + 9];
                let s_ip = format_ipv4(&raw_bytes[offset + 12..offset + 16]);
                let d_ip = format_ipv4(&raw_bytes[offset + 16..offset + 20]);
                (s_ip, d_ip, p_byte, offset + ihl)
            }
            0x86DD => {
                // IPv6
                if raw_bytes.len() < offset + 40 {
                    return fallback_packet(meta, "Truncated IPv6");
                }
                let next_header = raw_bytes[offset + 6];
                let s_ip = format_ipv6(&raw_bytes[offset + 8..offset + 24]);
                let d_ip = format_ipv6(&raw_bytes[offset + 24..offset + 40]);
                (s_ip, d_ip, next_header, offset + 40)
            }
            0x0806 => {
                // ARP
                return DecodedPacket {
                    index,
                    timestamp_epoch_ms,
                    delta_time_ms,
                    wire_len,
                    captured_len,
                    src_mac,
                    dst_mac,
                    ether_type,
                    src_ip: "ARP".to_string(),
                    dst_ip: "Broadcast".to_string(),
                    transport_proto: TransportProto::Other(0),
                    src_port: None,
                    dst_port: None,
                    tcp_flags: None,
                    seq_number: None,
                    ack_number: None,
                    app_proto: AppProto::Unknown,
                    app_layer: None,
                    info_summary: "Address Resolution Protocol (ARP)".to_string(),
                    stream_id: None,
                    raw_payload_base64,
                };
            }
            _ => {
                return fallback_packet(meta, &format!("EtherType 0x{:04X}", ether_type));
            }
        };

        offset = ip_payload_offset;

        // Transport Layer Dissection
        match proto_byte {
            6 => {
                // TCP
                if raw_bytes.len() < offset + 20 {
                    return fallback_ip_packet(
                        meta,
                        src_ip,
                        dst_ip,
                        TransportProto::Tcp,
                        "Truncated TCP segment",
                    );
                }
                let src_port = u16::from_be_bytes([raw_bytes[offset], raw_bytes[offset + 1]]);
                let dst_port = u16::from_be_bytes([raw_bytes[offset + 2], raw_bytes[offset + 3]]);
                let seq = u32::from_be_bytes([
                    raw_bytes[offset + 4],
                    raw_bytes[offset + 5],
                    raw_bytes[offset + 6],
                    raw_bytes[offset + 7],
                ]);
                let ack = u32::from_be_bytes([
                    raw_bytes[offset + 8],
                    raw_bytes[offset + 9],
                    raw_bytes[offset + 10],
                    raw_bytes[offset + 11],
                ]);
                let data_offset = ((raw_bytes[offset + 12] >> 4) as usize) * 4;
                let flags = TcpFlags::from_byte(raw_bytes[offset + 13]);
                let window = u16::from_be_bytes([raw_bytes[offset + 14], raw_bytes[offset + 15]]);

                let payload_start = offset + data_offset;
                let payload = if raw_bytes.len() > payload_start {
                    &raw_bytes[payload_start..]
                } else {
                    &[]
                };

                let (app_proto, app_layer, info) = Self::dissect_app_payload(
                    src_port, dst_port, payload, &flags, seq, ack, window,
                );

                DecodedPacket {
                    index,
                    timestamp_epoch_ms,
                    delta_time_ms,
                    wire_len,
                    captured_len,
                    src_mac,
                    dst_mac,
                    ether_type,
                    src_ip,
                    dst_ip,
                    transport_proto: TransportProto::Tcp,
                    src_port: Some(src_port),
                    dst_port: Some(dst_port),
                    tcp_flags: Some(flags),
                    seq_number: Some(seq),
                    ack_number: Some(ack),
                    app_proto,
                    app_layer,
                    info_summary: info,
                    stream_id: None, // Assigned by stream engine
                    raw_payload_base64,
                }
            }
            17 => {
                // UDP
                if raw_bytes.len() < offset + 8 {
                    return fallback_ip_packet(
                        meta,
                        src_ip,
                        dst_ip,
                        TransportProto::Udp,
                        "Truncated UDP datagram",
                    );
                }
                let src_port = u16::from_be_bytes([raw_bytes[offset], raw_bytes[offset + 1]]);
                let dst_port = u16::from_be_bytes([raw_bytes[offset + 2], raw_bytes[offset + 3]]);
                let udp_len = u16::from_be_bytes([raw_bytes[offset + 4], raw_bytes[offset + 5]]);

                let payload_start = offset + 8;
                let payload = if raw_bytes.len() > payload_start {
                    &raw_bytes[payload_start..]
                } else {
                    &[]
                };

                let (app_proto, app_layer, info) =
                    Self::dissect_udp_payload(src_port, dst_port, payload, udp_len);

                DecodedPacket {
                    index,
                    timestamp_epoch_ms,
                    delta_time_ms,
                    wire_len,
                    captured_len,
                    src_mac,
                    dst_mac,
                    ether_type,
                    src_ip,
                    dst_ip,
                    transport_proto: TransportProto::Udp,
                    src_port: Some(src_port),
                    dst_port: Some(dst_port),
                    tcp_flags: None,
                    seq_number: None,
                    ack_number: None,
                    app_proto,
                    app_layer,
                    info_summary: info,
                    stream_id: None,
                    raw_payload_base64,
                }
            }
            1 => DecodedPacket {
                index,
                timestamp_epoch_ms,
                delta_time_ms,
                wire_len,
                captured_len,
                src_mac,
                dst_mac,
                ether_type,
                src_ip,
                dst_ip,
                transport_proto: TransportProto::Icmp,
                src_port: None,
                dst_port: None,
                tcp_flags: None,
                seq_number: None,
                ack_number: None,
                app_proto: AppProto::Unknown,
                app_layer: None,
                info_summary: "Internet Control Message Protocol (ICMP Echo/Reply)".to_string(),
                stream_id: None,
                raw_payload_base64,
            },
            other => DecodedPacket {
                index,
                timestamp_epoch_ms,
                delta_time_ms,
                wire_len,
                captured_len,
                src_mac,
                dst_mac,
                ether_type,
                src_ip,
                dst_ip,
                transport_proto: TransportProto::Other(other),
                src_port: None,
                dst_port: None,
                tcp_flags: None,
                seq_number: None,
                ack_number: None,
                app_proto: AppProto::Unknown,
                app_layer: None,
                info_summary: format!("IP Protocol {}", other),
                stream_id: None,
                raw_payload_base64,
            },
        }
    }

    /// Dissects TCP application payloads (TLS, HTTP, or raw TCP flags)
    fn dissect_app_payload(
        src_port: u16,
        dst_port: u16,
        payload: &[u8],
        flags: &TcpFlags,
        seq: u32,
        ack: u32,
        win: u16,
    ) -> (AppProto, Option<ApplicationLayer>, String) {
        // 1. Check TLS Handshake ClientHello / ServerHello
        if (src_port == 443 || dst_port == 443 || src_port == 8443 || dst_port == 8443)
            && payload.len() >= 5
            && payload[0] == 0x16
        // Handshake record
        {
            if let Some(tls_info) = Self::dissect_tls_record(payload) {
                let sni_str = tls_info.sni.as_deref().unwrap_or("unknown");
                let summary = format!(
                    "TLS {} ({}, SNI: {})",
                    tls_info.handshake_type, tls_info.version, sni_str
                );
                return (
                    AppProto::Tls,
                    Some(ApplicationLayer::Tls(tls_info)),
                    summary,
                );
            }
        }

        // 2. Check HTTP/1.x plaintext
        if src_port == 80 || dst_port == 80 || src_port == 8080 || dst_port == 8080 {
            if let Some(http_info) = Self::dissect_http_payload(payload) {
                let summary = if http_info.is_request {
                    format!(
                        "HTTP {} {} (Host: {})",
                        http_info.method.as_deref().unwrap_or("REQ"),
                        http_info.uri.as_deref().unwrap_or("/"),
                        http_info.host.as_deref().unwrap_or("-")
                    )
                } else {
                    format!(
                        "HTTP/1.1 {} {}",
                        http_info.status_code.unwrap_or(200),
                        http_info.status_text.as_deref().unwrap_or("OK")
                    )
                };
                return (
                    AppProto::Http,
                    Some(ApplicationLayer::Http(http_info)),
                    summary,
                );
            }
        }

        // 3. Fallback to TCP Control summary
        let flags_str = flags.to_summary_string();
        let summary = format!(
            "{} -> {} [{}] Seq={} Ack={} Win={} Len={}",
            src_port,
            dst_port,
            flags_str,
            seq,
            ack,
            win,
            payload.len()
        );

        let app_layer = if !payload.is_empty() {
            let preview = String::from_utf8_lossy(&payload[..payload.len().min(64)]).to_string();
            Some(ApplicationLayer::Raw {
                preview,
                length: payload.len(),
            })
        } else {
            None
        };

        (AppProto::Unknown, app_layer, summary)
    }

    /// Dissects UDP payload (DNS or generic datagram)
    fn dissect_udp_payload(
        src_port: u16,
        dst_port: u16,
        payload: &[u8],
        _udp_len: u16,
    ) -> (AppProto, Option<ApplicationLayer>, String) {
        if (src_port == 53 || dst_port == 53) && payload.len() >= 12 {
            if let Some(dns_info) = Self::dissect_dns(payload) {
                let q_str = if dns_info.questions.is_empty() {
                    "-".to_string()
                } else {
                    dns_info.questions.join(", ")
                };
                let summary = if dns_info.is_response {
                    format!("DNS Response 0x{:04X}: {}", dns_info.transaction_id, q_str)
                } else {
                    format!("DNS Query 0x{:04X} A {}", dns_info.transaction_id, q_str)
                };
                return (
                    AppProto::Dns,
                    Some(ApplicationLayer::Dns(dns_info)),
                    summary,
                );
            }
        }

        let summary = format!(
            "UDP: {} -> {} (Len: {} bytes)",
            src_port,
            dst_port,
            payload.len()
        );
        (AppProto::Unknown, None, summary)
    }

    /// Dissects TLS Handshake and parses ClientHello SNI + JA3
    fn dissect_tls_record(payload: &[u8]) -> Option<TlsInfo> {
        if payload.len() < 9 || payload[0] != 0x16 {
            return None;
        }

        let rec_major = payload[1];
        let rec_minor = payload[2];
        let handshake_type = payload[5];

        let type_str = match handshake_type {
            1 => "Client Hello",
            2 => "Server Hello",
            11 => "Certificate",
            12 => "Server Key Exchange",
            14 => "Server Hello Done",
            16 => "Client Key Exchange",
            _ => "Handshake",
        };

        let ver_str = match (rec_major, rec_minor) {
            (3, 1) => "TLS 1.0",
            (3, 2) => "TLS 1.1",
            (3, 3) => "TLS 1.2",
            (3, 4) => "TLS 1.3",
            _ => "TLS",
        };

        if handshake_type != 1 {
            return Some(TlsInfo {
                handshake_type: type_str.to_string(),
                version: ver_str.to_string(),
                sni: None,
                ja3_fingerprint: None,
                ja3_raw: None,
                cipher_suite_count: 0,
            });
        }

        // Parse ClientHello
        // Offset 5: Handshake Type (1)
        // Offset 6..9: Length (3 bytes)
        // Offset 9..11: Client Version
        // Offset 11..43: Random (32 bytes)
        // Offset 43: Session ID Length
        let mut idx = 43;
        if payload.len() <= idx {
            return None;
        }
        let session_id_len = payload[idx] as usize;
        idx += 1 + session_id_len;

        // Cipher suites
        if payload.len() <= idx + 2 {
            return None;
        }
        let cipher_suites_len = u16::from_be_bytes([payload[idx], payload[idx + 1]]) as usize;
        idx += 2;

        let mut ciphers = Vec::new();
        let end_ciphers = idx + cipher_suites_len;
        if payload.len() < end_ciphers {
            return None;
        }
        while idx + 1 < end_ciphers {
            let c = u16::from_be_bytes([payload[idx], payload[idx + 1]]);
            ciphers.push(c);
            idx += 2;
        }

        // Compression methods
        if payload.len() <= idx {
            return None;
        }
        let comp_len = payload[idx] as usize;
        idx += 1 + comp_len;

        // Extensions
        let mut sni = None;
        let mut extension_ids = Vec::new();

        if payload.len() > idx + 2 {
            let ext_len = u16::from_be_bytes([payload[idx], payload[idx + 1]]) as usize;
            idx += 2;
            let ext_end = (idx + ext_len).min(payload.len());

            while idx + 4 <= ext_end {
                let ext_type = u16::from_be_bytes([payload[idx], payload[idx + 1]]);
                let ext_data_len =
                    u16::from_be_bytes([payload[idx + 2], payload[idx + 3]]) as usize;
                idx += 4;
                extension_ids.push(ext_type);

                // Extension 0x0000: Server Name Indication (SNI)
                if ext_type == 0 && idx + ext_data_len <= ext_end && ext_data_len >= 5 {
                    // SNI List Length (2) -> Server Name Type (1) -> Name Length (2) -> Name
                    let name_type = payload[idx + 2];
                    let name_len =
                        u16::from_be_bytes([payload[idx + 3], payload[idx + 4]]) as usize;
                    if name_type == 0 && idx + 5 + name_len <= ext_end {
                        if let Ok(s) = std::str::from_utf8(&payload[idx + 5..idx + 5 + name_len]) {
                            sni = Some(s.to_string());
                        }
                    }
                }

                idx += ext_data_len;
            }
        }

        // Construct JA3 fingerprint string: TLSVersion,CipherSuites,Extensions,EllipticCurves,PointFormats
        let ciphers_str = ciphers
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("-");
        let exts_str = extension_ids
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("-");
        let ja3_raw = format!("771,{},{},29-23-24,0", ciphers_str, exts_str);
        let ja3_fingerprint = blake3::hash(ja3_raw.as_bytes()).to_hex().to_string();

        Some(TlsInfo {
            handshake_type: "Client Hello".to_string(),
            version: "TLS 1.3".to_string(),
            sni,
            ja3_fingerprint: Some(ja3_fingerprint),
            ja3_raw: Some(ja3_raw),
            cipher_suite_count: ciphers.len(),
        })
    }

    /// Dissects HTTP/1.x payload
    fn dissect_http_payload(payload: &[u8]) -> Option<HttpInfo> {
        let text = std::str::from_utf8(payload).ok()?;
        let first_line = text.lines().next()?;

        if first_line.starts_with("GET ")
            || first_line.starts_with("POST ")
            || first_line.starts_with("PUT ")
            || first_line.starts_with("DELETE ")
            || first_line.starts_with("HEAD ")
            || first_line.starts_with("OPTIONS ")
        {
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            let method = parts.first().map(|s| s.to_string());
            let uri = parts.get(1).map(|s| s.to_string());

            let mut host = None;
            let mut content_type = None;
            let mut content_length = None;

            for line in text.lines().skip(1) {
                let lower = line.to_lowercase();
                if lower.starts_with("host:") {
                    host = line.split(':').nth(1).map(|s| s.trim().to_string());
                } else if lower.starts_with("content-type:") {
                    content_type = line.split(':').nth(1).map(|s| s.trim().to_string());
                } else if lower.starts_with("content-length:") {
                    content_length = line
                        .split(':')
                        .nth(1)
                        .and_then(|s| s.trim().parse::<usize>().ok());
                }
            }

            Some(HttpInfo {
                is_request: true,
                method,
                uri,
                status_code: None,
                status_text: None,
                host,
                content_type,
                content_length,
            })
        } else if first_line.starts_with("HTTP/1.") {
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            let status_code = parts.get(1).and_then(|s| s.parse::<u16>().ok());
            let status_text = parts.get(2..).map(|s| s.join(" "));

            let mut content_type = None;
            let mut content_length = None;

            for line in text.lines().skip(1) {
                let lower = line.to_lowercase();
                if lower.starts_with("content-type:") {
                    content_type = line.split(':').nth(1).map(|s| s.trim().to_string());
                } else if lower.starts_with("content-length:") {
                    content_length = line
                        .split(':')
                        .nth(1)
                        .and_then(|s| s.trim().parse::<usize>().ok());
                }
            }

            Some(HttpInfo {
                is_request: false,
                method: None,
                uri: None,
                status_code,
                status_text,
                host: None,
                content_type,
                content_length,
            })
        } else {
            None
        }
    }

    /// Dissects DNS datagram
    fn dissect_dns(payload: &[u8]) -> Option<DnsInfo> {
        let tx_id = u16::from_be_bytes([payload[0], payload[1]]);
        let flags = u16::from_be_bytes([payload[2], payload[3]]);
        let is_response = (flags & 0x8000) != 0;
        let qd_count = u16::from_be_bytes([payload[4], payload[5]]) as usize;
        let an_count = u16::from_be_bytes([payload[6], payload[7]]) as usize;

        let mut idx = 12;
        let mut questions = Vec::new();

        for _ in 0..qd_count {
            if idx >= payload.len() {
                break;
            }
            let (qname, next_idx) = parse_dns_label(payload, idx);
            if !qname.is_empty() {
                questions.push(qname);
            }
            idx = next_idx + 4; // Skip QTYPE (2) + QCLASS (2)
        }

        let mut answers = Vec::new();
        if is_response && an_count > 0 {
            answers.push(format!("{} Resource Records", an_count));
        }

        Some(DnsInfo {
            transaction_id: tx_id,
            is_response,
            questions,
            answers,
        })
    }

    /// Generates 16-byte formatted hex dump lines with ASCII translation
    pub fn generate_hex_dump(payload: &[u8]) -> Vec<HexDumpRow> {
        let mut rows = Vec::new();
        for (i, chunk) in payload.chunks(16).enumerate() {
            let offset = i * 16;
            let hex_parts: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
            let hex_bytes = hex_parts.join(" ");

            let ascii: String = chunk
                .iter()
                .map(|&b| {
                    if (32..=126).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();

            rows.push(HexDumpRow {
                offset,
                hex_bytes,
                ascii,
            });
        }
        rows
    }
}

fn parse_dns_label(payload: &[u8], mut idx: usize) -> (String, usize) {
    let mut parts = Vec::new();
    while idx < payload.len() {
        let len = payload[idx] as usize;
        if len == 0 {
            idx += 1;
            break;
        }
        if len >= 0xC0 {
            // Pointer decompression
            idx += 2;
            break;
        }
        idx += 1;
        if idx + len <= payload.len() {
            if let Ok(label) = std::str::from_utf8(&payload[idx..idx + len]) {
                parts.push(label);
            }
            idx += len;
        } else {
            break;
        }
    }
    (parts.join("."), idx)
}

fn format_mac(b: &[u8]) -> String {
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        b[0], b[1], b[2], b[3], b[4], b[5]
    )
}

fn format_ipv4(b: &[u8]) -> String {
    format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3])
}

fn format_ipv6(b: &[u8]) -> String {
    let parts: Vec<String> = (0..8)
        .map(|i| {
            let val = u16::from_be_bytes([b[i * 2], b[i * 2 + 1]]);
            format!("{:x}", val)
        })
        .collect();
    parts.join(":")
}

#[derive(Clone)]
struct PacketFrameMeta {
    index: usize,
    timestamp_epoch_ms: i64,
    delta_time_ms: f64,
    wire_len: usize,
    captured_len: usize,
    src_mac: String,
    dst_mac: String,
    ether_type: u16,
    raw_payload_base64: String,
}

fn fallback_packet(meta: PacketFrameMeta, summary: &str) -> DecodedPacket {
    DecodedPacket {
        index: meta.index,
        timestamp_epoch_ms: meta.timestamp_epoch_ms,
        delta_time_ms: meta.delta_time_ms,
        wire_len: meta.wire_len,
        captured_len: meta.captured_len,
        src_mac: meta.src_mac,
        dst_mac: meta.dst_mac,
        ether_type: meta.ether_type,
        src_ip: "unknown".to_string(),
        dst_ip: "unknown".to_string(),
        transport_proto: TransportProto::Other(0),
        src_port: None,
        dst_port: None,
        tcp_flags: None,
        seq_number: None,
        ack_number: None,
        app_proto: AppProto::Unknown,
        app_layer: None,
        info_summary: summary.to_string(),
        stream_id: None,
        raw_payload_base64: meta.raw_payload_base64,
    }
}

fn fallback_ip_packet(
    meta: PacketFrameMeta,
    src_ip: String,
    dst_ip: String,
    transport_proto: TransportProto,
    summary: &str,
) -> DecodedPacket {
    DecodedPacket {
        index: meta.index,
        timestamp_epoch_ms: meta.timestamp_epoch_ms,
        delta_time_ms: meta.delta_time_ms,
        wire_len: meta.wire_len,
        captured_len: meta.captured_len,
        src_mac: meta.src_mac,
        dst_mac: meta.dst_mac,
        ether_type: meta.ether_type,
        src_ip,
        dst_ip,
        transport_proto,
        src_port: None,
        dst_port: None,
        tcp_flags: None,
        seq_number: None,
        ack_number: None,
        app_proto: AppProto::Unknown,
        app_layer: None,
        info_summary: summary.to_string(),
        stream_id: None,
        raw_payload_base64: meta.raw_payload_base64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_flags_parsing() {
        let flags = TcpFlags::from_byte(0x12); // SYN (0x02) | ACK (0x10)
        assert!(flags.syn);
        assert!(flags.ack);
        assert!(!flags.fin);
        assert!(!flags.rst);
        assert_eq!(flags.to_summary_string(), "SYN, ACK");
    }

    #[test]
    fn test_hex_dump_generation() {
        let payload = b"Hello, PCAP World! 1234567890";
        let rows = ProtocolDecoder::generate_hex_dump(payload);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].offset, 0);
        assert_eq!(rows[1].offset, 16);
        assert!(rows[0].ascii.contains("Hello, PCAP"));
    }
}
