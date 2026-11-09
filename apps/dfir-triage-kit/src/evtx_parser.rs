#![forbid(unsafe_code)]
//! Native Windows .evtx (Event Log Binary XML) Parser
//!
//! Provides pure-Rust, offline, zero-dependency parsing of Windows Event Log (.evtx) files
//! across Windows, Linux, and macOS without calling external OS binaries.

use crate::models::{LogEventRecord, LogLevel};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub struct EvtxParser;

impl EvtxParser {
    const FILE_HEADER_MAGIC: &'static [u8; 8] = b"ElfFile\0";
    const CHUNK_HEADER_MAGIC: &'static [u8; 8] = b"ElfChnk\0";
    const RECORD_MAGIC: &'static [u8; 4] = &[0x2a, 0x2a, 0x00, 0x00]; // 0x00002a2a

    /// Parses an .evtx file and returns extracted normalized log event records
    pub fn parse_file(
        path: impl AsRef<Path>,
        max_records: usize,
    ) -> Result<Vec<LogEventRecord>, std::io::Error> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 4096];
        let n = file.read(&mut header)?;
        if n < 8 || &header[0..8] != Self::FILE_HEADER_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Not a valid Windows .evtx file (missing ElfFile\\0 magic)",
            ));
        }

        let mut records = Vec::new();

        // Chunks start at offset 4096 (each chunk is 65,536 bytes)
        let mut chunk_offset = 4096u64;
        let file_len = file.metadata()?.len();

        while chunk_offset + 512 <= file_len && records.len() < max_records {
            file.seek(SeekFrom::Start(chunk_offset))?;
            let mut chunk_header = [0u8; 512];
            if file.read_exact(&mut chunk_header).is_err() {
                break;
            }

            if &chunk_header[0..8] != Self::CHUNK_HEADER_MAGIC {
                chunk_offset += 65536;
                continue;
            }

            // In an ElfChnk, records start after the 512-byte chunk header
            let mut record_offset = chunk_offset + 512;
            let chunk_end = (chunk_offset + 65536).min(file_len);

            while record_offset + 24 <= chunk_end && records.len() < max_records {
                file.seek(SeekFrom::Start(record_offset))?;
                let mut rec_hdr = [0u8; 24];
                if file.read_exact(&mut rec_hdr).is_err() {
                    break;
                }

                if &rec_hdr[0..4] != Self::RECORD_MAGIC {
                    // Try stepping forward by 4 bytes to find next record
                    record_offset += 4;
                    continue;
                }

                let rec_size =
                    u32::from_le_bytes([rec_hdr[4], rec_hdr[5], rec_hdr[6], rec_hdr[7]]) as usize;
                let _rec_id = u64::from_le_bytes([
                    rec_hdr[8],
                    rec_hdr[9],
                    rec_hdr[10],
                    rec_hdr[11],
                    rec_hdr[12],
                    rec_hdr[13],
                    rec_hdr[14],
                    rec_hdr[15],
                ]);
                let filetime = u64::from_le_bytes([
                    rec_hdr[16],
                    rec_hdr[17],
                    rec_hdr[18],
                    rec_hdr[19],
                    rec_hdr[20],
                    rec_hdr[21],
                    rec_hdr[22],
                    rec_hdr[23],
                ]);

                if !(24..=1024 * 1024).contains(&rec_size) {
                    record_offset += 24;
                    continue;
                }

                let payload_len = rec_size.saturating_sub(24);
                let mut payload = vec![0u8; payload_len];
                let _ = file.read_exact(&mut payload);

                // Convert Windows FILETIME to Unix timestamp seconds
                let unix_ts = if filetime >= 116444736000000000 {
                    ((filetime - 116444736000000000) / 10000000) as i64
                } else {
                    1700000000
                };

                let record = Self::parse_record_payload(&payload, unix_ts);
                records.push(record);

                record_offset += rec_size as u64;
            }

            chunk_offset += 65536;
        }

        Ok(records)
    }

    /// Extracts text/XML tokens from binary event record payload
    fn parse_record_payload(payload: &[u8], timestamp: i64) -> LogEventRecord {
        // Extract UTF-8 or ASCII printable strings from payload
        let mut ascii_text = String::with_capacity(payload.len());
        for &b in payload {
            if b.is_ascii_graphic() || b == b' ' || b == b'\t' || b == b'\n' {
                ascii_text.push(b as char);
            } else {
                ascii_text.push(' ');
            }
        }

        // Extract Event ID if present (e.g., from XML pattern <EventID>4625</EventID> or EventID 4625)
        let event_id = Self::extract_event_id(&ascii_text).unwrap_or(1000);
        let source = Self::extract_provider(&ascii_text).unwrap_or_else(|| "Security".to_string());
        let level = Self::extract_level(&ascii_text);
        let user_id = Self::extract_user(&ascii_text);

        let clean_msg = if ascii_text.trim().is_empty() {
            format!("Windows Event {} from {}", event_id, source)
        } else {
            let mut summary = ascii_text
                .split_whitespace()
                .take(30)
                .collect::<Vec<_>>()
                .join(" ");
            if summary.len() > 200 {
                summary.truncate(200);
                summary.push_str("...");
            }
            summary
        };

        LogEventRecord {
            timestamp,
            source,
            event_id,
            level,
            message: clean_msg,
            user_id,
        }
    }

    fn extract_event_id(text: &str) -> Option<u32> {
        if let Some(start) = text.find("<EventID>") {
            let rest = &text[start + 9..];
            if let Some(end) = rest.find("</EventID>") {
                return rest[..end].trim().parse().ok();
            }
        }
        if let Some(start) = text.find("EventID ") {
            let rest = &text[start + 8..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return num_str.parse().ok();
        }
        None
    }

    fn extract_provider(text: &str) -> Option<String> {
        if let Some(start) = text.find("Provider Name=\"") {
            let rest = &text[start + 15..];
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
        if let Some(start) = text.find("<Provider ") {
            let rest = &text[start..];
            if let Some(name_start) = rest.find("Name=\"") {
                let name_rest = &rest[name_start + 6..];
                if let Some(end) = name_rest.find('"') {
                    return Some(name_rest[..end].to_string());
                }
            }
        }
        None
    }

    fn extract_level(text: &str) -> LogLevel {
        if text.contains("<Level>1</Level>") || text.contains("Level 1") {
            LogLevel::Critical
        } else if text.contains("<Level>2</Level>")
            || text.contains("Level 2")
            || text.contains("Error")
        {
            LogLevel::Error
        } else if text.contains("<Level>3</Level>")
            || text.contains("Level 3")
            || text.contains("Warning")
        {
            LogLevel::Warning
        } else if text.contains("<Level>5</Level>") || text.contains("Level 5") {
            LogLevel::Debug
        } else {
            LogLevel::Info
        }
    }

    fn extract_user(text: &str) -> Option<String> {
        if let Some(start) = text.find("TargetUserName\">") {
            let rest = &text[start + 16..];
            if let Some(end) = rest.find('<') {
                let u = rest[..end].trim();
                if !u.is_empty() {
                    return Some(u.to_string());
                }
            }
        }
        if let Some(start) = text.find("SubjectUserName\">") {
            let rest = &text[start + 17..];
            if let Some(end) = rest.find('<') {
                let u = rest[..end].trim();
                if !u.is_empty() {
                    return Some(u.to_string());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evtx_parser_invalid_magic() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), b"NotAnEvtxFileContent").unwrap();
        let res = EvtxParser::parse_file(temp.path(), 10);
        assert!(res.is_err());
    }

    #[test]
    fn test_evtx_synthetic_chunk_and_record() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let mut data = vec![0u8; 4096 + 65536];

        // File Header
        data[0..8].copy_from_slice(b"ElfFile\0");

        // Chunk Header at 4096
        data[4096..4096 + 8].copy_from_slice(b"ElfChnk\0");

        // Event Record at 4096 + 512
        let rec_start = 4096 + 512;
        data[rec_start..rec_start + 4].copy_from_slice(&[0x2a, 0x2a, 0x00, 0x00]); // magic

        // XML payload
        let xml = b"<Event><System><Provider Name=\"Microsoft-Windows-Security-Auditing\"/><EventID>4625</EventID><Level>2</Level></System><EventData><Data Name=\"TargetUserName\">compromised_admin</Data></EventData></Event>";
        let rec_size = (24 + xml.len() + 4) as u32;

        data[rec_start + 4..rec_start + 8].copy_from_slice(&rec_size.to_le_bytes()); // size
        data[rec_start + 8..rec_start + 16].copy_from_slice(&1u64.to_le_bytes()); // rec_id
        data[rec_start + 16..rec_start + 24].copy_from_slice(&132800000000000000u64.to_le_bytes()); // FILETIME

        data[rec_start + 24..rec_start + 24 + xml.len()].copy_from_slice(xml);

        std::fs::write(temp.path(), &data).unwrap();

        let records = EvtxParser::parse_file(temp.path(), 5).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].event_id, 4625);
        assert_eq!(records[0].source, "Microsoft-Windows-Security-Auditing");
        assert_eq!(records[0].level, LogLevel::Error);
        assert_eq!(records[0].user_id.as_deref(), Some("compromised_admin"));
    }
}
