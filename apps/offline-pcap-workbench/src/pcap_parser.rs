//! Zero-copy PCAP / PCAPNG binary format parser.
//!
//! Decodes libpcap captures and next-generation PCAPNG blocks into raw packet slices.

use crate::models::DecodedPacket;
use crate::protocol_decoder::ProtocolDecoder;

pub struct PcapParser;

pub struct RawPcapPacket<'a> {
    pub timestamp_epoch_ms: i64,
    pub wire_len: usize,
    pub data: &'a [u8],
}

impl PcapParser {
    /// Ingests raw PCAP or PCAPNG bytes and parses all packets
    pub fn parse_capture(raw_bytes: &[u8]) -> Result<Vec<DecodedPacket>, String> {
        if raw_bytes.len() < 24 {
            return Err(
                "File too small to be a valid PCAP/PCAPNG capture (< 24 bytes)".to_string(),
            );
        }

        let magic = u32::from_ne_bytes([raw_bytes[0], raw_bytes[1], raw_bytes[2], raw_bytes[3]]);
        let magic_be = u32::from_be_bytes([raw_bytes[0], raw_bytes[1], raw_bytes[2], raw_bytes[3]]);

        if magic_be == 0x0A0D0D0A {
            // PCAPNG format
            Self::parse_pcapng(raw_bytes)
        } else if magic == 0xA1B2C3D4
            || magic == 0xD4C3B2A1
            || magic == 0xA1B23C4D
            || magic == 0x4D3CB2A1
        {
            // Classic PCAP format
            Self::parse_classic_pcap(raw_bytes, magic)
        } else {
            Err(format!("Unrecognized PCAP magic number 0x{:08X}", magic_be))
        }
    }

    /// Parses classic libpcap binary
    fn parse_classic_pcap(raw: &[u8], magic: u32) -> Result<Vec<DecodedPacket>, String> {
        let is_big_endian = magic == 0xA1B2C3D4 || magic == 0xA1B23C4D;
        let is_nanosec = magic == 0xA1B23C4D || magic == 0x4D3CB2A1;

        let mut offset = 24; // Skip 24-byte global header
        let mut packets = Vec::new();
        let mut first_ts_ms = None;
        let mut last_ts_ms = 0i64;

        while offset + 16 <= raw.len() {
            let (ts_sec, ts_sub, incl_len, orig_len) = if is_big_endian {
                (
                    u32::from_be_bytes([
                        raw[offset],
                        raw[offset + 1],
                        raw[offset + 2],
                        raw[offset + 3],
                    ]),
                    u32::from_be_bytes([
                        raw[offset + 4],
                        raw[offset + 5],
                        raw[offset + 6],
                        raw[offset + 7],
                    ]),
                    u32::from_be_bytes([
                        raw[offset + 8],
                        raw[offset + 9],
                        raw[offset + 10],
                        raw[offset + 11],
                    ]) as usize,
                    u32::from_be_bytes([
                        raw[offset + 12],
                        raw[offset + 13],
                        raw[offset + 14],
                        raw[offset + 15],
                    ]) as usize,
                )
            } else {
                (
                    u32::from_le_bytes([
                        raw[offset],
                        raw[offset + 1],
                        raw[offset + 2],
                        raw[offset + 3],
                    ]),
                    u32::from_le_bytes([
                        raw[offset + 4],
                        raw[offset + 5],
                        raw[offset + 6],
                        raw[offset + 7],
                    ]),
                    u32::from_le_bytes([
                        raw[offset + 8],
                        raw[offset + 9],
                        raw[offset + 10],
                        raw[offset + 11],
                    ]) as usize,
                    u32::from_le_bytes([
                        raw[offset + 12],
                        raw[offset + 13],
                        raw[offset + 14],
                        raw[offset + 15],
                    ]) as usize,
                )
            };

            offset += 16;
            if offset + incl_len > raw.len() {
                break; // Corrupted / truncated trailing packet
            }

            let pkt_data = &raw[offset..offset + incl_len];
            offset += incl_len;

            let ts_sub_ms = if is_nanosec {
                (ts_sub as i64) / 1_000_000
            } else {
                (ts_sub as i64) / 1_000
            };
            let ts_ms = (ts_sec as i64) * 1000 + ts_sub_ms;

            let delta = if let Some(first) = first_ts_ms {
                (ts_ms - first) as f64
            } else {
                first_ts_ms = Some(ts_ms);
                0.0
            };
            last_ts_ms = ts_ms;

            let decoded =
                ProtocolDecoder::decode_packet(packets.len() + 1, ts_ms, delta, pkt_data, orig_len);
            packets.push(decoded);
        }

        let _ = last_ts_ms; // Suppress unused warning
        Ok(packets)
    }

    /// Parses PCAPNG Enhanced Packet Blocks (EPB)
    fn parse_pcapng(raw: &[u8]) -> Result<Vec<DecodedPacket>, String> {
        let mut offset = 0;
        let mut packets = Vec::new();
        let mut first_ts_ms = None;

        while offset + 12 <= raw.len() {
            let block_type = u32::from_le_bytes([
                raw[offset],
                raw[offset + 1],
                raw[offset + 2],
                raw[offset + 3],
            ]);
            let block_total_len = u32::from_le_bytes([
                raw[offset + 4],
                raw[offset + 5],
                raw[offset + 6],
                raw[offset + 7],
            ]) as usize;

            if block_total_len < 12 || offset + block_total_len > raw.len() {
                break;
            }

            // Enhanced Packet Block (EPB = 0x00000006)
            if block_type == 0x00000006 && block_total_len >= 32 {
                let ts_high = u32::from_le_bytes([
                    raw[offset + 12],
                    raw[offset + 13],
                    raw[offset + 14],
                    raw[offset + 15],
                ]);
                let ts_low = u32::from_le_bytes([
                    raw[offset + 16],
                    raw[offset + 17],
                    raw[offset + 18],
                    raw[offset + 19],
                ]);
                let cap_len = u32::from_le_bytes([
                    raw[offset + 20],
                    raw[offset + 21],
                    raw[offset + 22],
                    raw[offset + 23],
                ]) as usize;
                let orig_len = u32::from_le_bytes([
                    raw[offset + 24],
                    raw[offset + 25],
                    raw[offset + 26],
                    raw[offset + 27],
                ]) as usize;

                let pkt_start = offset + 28;
                if pkt_start + cap_len <= offset + block_total_len {
                    let pkt_data = &raw[pkt_start..pkt_start + cap_len];

                    // Standard PCAPNG microsecond timestamp (64-bit microsecond counter)
                    let ts_us = ((ts_high as u64) << 32) | (ts_low as u64);
                    let ts_ms = (ts_us / 1000) as i64;

                    let delta = if let Some(first) = first_ts_ms {
                        (ts_ms - first) as f64
                    } else {
                        first_ts_ms = Some(ts_ms);
                        0.0
                    };

                    let decoded = ProtocolDecoder::decode_packet(
                        packets.len() + 1,
                        ts_ms,
                        delta,
                        pkt_data,
                        orig_len,
                    );
                    packets.push(decoded);
                }
            }

            offset += block_total_len;
        }

        Ok(packets)
    }

    /// Builder helper to generate synthetically valid PCAP file bytes
    /// for offline testing and demos
    pub fn build_demo_pcap(packets: &[(&[u8], i64)]) -> Vec<u8> {
        let mut buf = Vec::new();

        // 1. Global Header (24 bytes)
        // Magic 0xA1B2C3D4 (LE: D4 C3 B2 A1)
        buf.extend_from_slice(&0xD4C3B2A1u32.to_ne_bytes());
        buf.extend_from_slice(&2u16.to_le_bytes()); // Major 2
        buf.extend_from_slice(&4u16.to_le_bytes()); // Minor 4
        buf.extend_from_slice(&0i32.to_le_bytes()); // Thiszone 0
        buf.extend_from_slice(&0u32.to_le_bytes()); // Sigfigs 0
        buf.extend_from_slice(&65535u32.to_le_bytes()); // Snaplen 65535
        buf.extend_from_slice(&1u32.to_le_bytes()); // Network 1 (Ethernet)

        // 2. Write each packet
        for (pkt_bytes, ts_ms) in packets {
            let ts_sec = (*ts_ms / 1000) as u32;
            let ts_usec = ((*ts_ms % 1000) * 1000) as u32;
            let len = pkt_bytes.len() as u32;

            buf.extend_from_slice(&ts_sec.to_le_bytes());
            buf.extend_from_slice(&ts_usec.to_le_bytes());
            buf.extend_from_slice(&len.to_le_bytes());
            buf.extend_from_slice(&len.to_le_bytes());
            buf.extend_from_slice(pkt_bytes);
        }

        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcap_build_and_parse_roundtrip() {
        let fake_pkt = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0x08, 0x06,
        ];
        let pcap_bytes = PcapParser::build_demo_pcap(&[(&fake_pkt, 1000), (&fake_pkt, 1500)]);
        let pkts = PcapParser::parse_capture(&pcap_bytes).expect("parse pcap");
        assert_eq!(pkts.len(), 2);
        assert_eq!(pkts[0].src_mac, "66:77:88:99:AA:BB");
        assert_eq!(pkts[0].dst_mac, "00:11:22:33:44:55");
        assert_eq!(pkts[0].ether_type, 0x0806);
    }
}
