#![forbid(unsafe_code)]
//! DevOps Secret Vault - Automated X.509 Certificate Chain Inspection & Expiry Monitor
//!
//! On-device, zero-network X.509 ASN.1 DER and PEM certificate parser:
//! - Validity period calculation (notBefore, notAfter, days until expiration)
//! - Expiry status classification (Valid, Expiring Soon <=30d, Expired)
//! - Weak cryptography detection (MD5, SHA-1, RSA keys < 2048 bits)
//! - Subject Alternative Names (SANs) extraction (DNS names, IPs)
//! - Subject and Issuer Common Name (CN) extraction & Self-signed identification

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::VaultError;

/// Certificate expiration and health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CertStatus {
    Valid,
    ExpiringSoon,
    Expired,
}

/// Comprehensive X.509 Certificate Inspection Report
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertInspectionReport {
    pub subject_cn: String,
    pub issuer_cn: String,
    pub not_before_epoch: i64,
    pub not_after_epoch: i64,
    pub days_until_expiration: i64,
    pub status: CertStatus,
    pub is_self_signed: bool,
    pub signature_algorithm: String,
    pub is_weak_signature: bool,
    pub public_key_algorithm: String,
    pub key_size_bits: Option<usize>,
    pub is_weak_key: bool,
    pub subject_alt_names: Vec<String>,
    pub serial_number: String,
}

// ---------------------------------------------------------------------------
// Base64 Decoder (RFC 4648)
// ---------------------------------------------------------------------------

pub(crate) fn decode_base64(input: &str) -> Result<Vec<u8>, VaultError> {
    let clean: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '=')
        .collect();

    let mut bits = 0u32;
    let mut bit_count = 0u32;
    let mut output = Vec::with_capacity(clean.len() * 3 / 4);

    for c in clean.chars() {
        let val = match c {
            'A'..='Z' => (c as u32) - ('A' as u32),
            'a'..='z' => (c as u32) - ('a' as u32) + 26,
            '0'..='9' => (c as u32) - ('0' as u32) + 52,
            '+' => 62,
            '/' => 63,
            _ => {
                return Err(VaultError::ValidationError(format!(
                    "Invalid Base64 character: '{c}'"
                )))
            }
        };

        bits = (bits << 6) | val;
        bit_count += 6;

        if bit_count >= 8 {
            bit_count -= 8;
            output.push(((bits >> bit_count) & 0xFF) as u8);
        }
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// ASN.1 DER Parser Utilities
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Asn1Element<'a> {
    tag: u8,
    value: &'a [u8],
}

impl<'a> Asn1Element<'a> {
    fn parse(input: &'a [u8]) -> Result<(Self, &'a [u8]), VaultError> {
        if input.is_empty() {
            return Err(VaultError::ValidationError("Empty ASN.1 input".to_string()));
        }

        let tag = input[0];
        let mut idx = 1;

        if idx >= input.len() {
            return Err(VaultError::ValidationError(
                "Truncated ASN.1 header".to_string(),
            ));
        }

        let first_len_byte = input[idx];
        idx += 1;

        let len = if first_len_byte < 0x80 {
            first_len_byte as usize
        } else {
            let num_octets = (first_len_byte & 0x7F) as usize;
            if num_octets == 0 || num_octets > 4 || idx + num_octets > input.len() {
                return Err(VaultError::ValidationError(
                    "Invalid ASN.1 length".to_string(),
                ));
            }
            let mut l = 0usize;
            for i in 0..num_octets {
                l = (l << 8) | (input[idx + i] as usize);
            }
            idx += num_octets;
            l
        };

        if idx + len > input.len() {
            return Err(VaultError::ValidationError(
                "ASN.1 length exceeds input boundary".to_string(),
            ));
        }

        let value = &input[idx..idx + len];
        let rest = &input[idx + len..];

        Ok((Self { tag, value }, rest))
    }

    fn as_sequence(&self) -> Result<Vec<Asn1Element<'a>>, VaultError> {
        if self.tag != 0x30 && (self.tag & 0x20 == 0) {
            return Err(VaultError::ValidationError(
                "Expected constructed ASN.1 sequence".to_string(),
            ));
        }
        let mut elements = Vec::new();
        let mut cursor = self.value;
        while !cursor.is_empty() {
            let (el, rest) = Self::parse(cursor)?;
            elements.push(el);
            cursor = rest;
        }
        Ok(elements)
    }

    fn parse_oid(&self) -> Result<String, VaultError> {
        if self.tag != 0x06 || self.value.is_empty() {
            return Err(VaultError::ValidationError(
                "Invalid OID tag or empty value".to_string(),
            ));
        }

        let mut parts = Vec::new();
        let first = self.value[0];
        parts.push((first / 40).to_string());
        parts.push((first % 40).to_string());

        let mut current = 0u64;
        for &b in &self.value[1..] {
            current = (current << 7) | ((b & 0x7F) as u64);
            if (b & 0x80) == 0 {
                parts.push(current.to_string());
                current = 0;
            }
        }

        Ok(parts.join("."))
    }

    fn parse_time_to_epoch(&self) -> Result<i64, VaultError> {
        let text = std::str::from_utf8(self.value)
            .map_err(|_| VaultError::ValidationError("Invalid ASN.1 time string".to_string()))?;

        // UTCTime: YYMMDDHHMMSSZ
        // GeneralizedTime: YYYYMMDDHHMMSSZ
        if self.tag == 0x17 && text.len() >= 12 {
            let yy: i64 = text[0..2].parse().unwrap_or(0);
            let year = if yy >= 50 { 1900 + yy } else { 2000 + yy };
            let month: i64 = text[2..4].parse().unwrap_or(1);
            let day: i64 = text[4..6].parse().unwrap_or(1);
            let hour: i64 = text[6..8].parse().unwrap_or(0);
            let min: i64 = text[8..10].parse().unwrap_or(0);
            let sec: i64 = text[10..12].parse().unwrap_or(0);
            Ok(date_to_unix_epoch(year, month, day, hour, min, sec))
        } else if self.tag == 0x18 && text.len() >= 14 {
            let year: i64 = text[0..4].parse().unwrap_or(2000);
            let month: i64 = text[4..6].parse().unwrap_or(1);
            let day: i64 = text[6..8].parse().unwrap_or(1);
            let hour: i64 = text[8..10].parse().unwrap_or(0);
            let min: i64 = text[10..12].parse().unwrap_or(0);
            let sec: i64 = text[12..14].parse().unwrap_or(0);
            Ok(date_to_unix_epoch(year, month, day, hour, min, sec))
        } else {
            Err(VaultError::ValidationError(
                "Unsupported ASN.1 time tag".to_string(),
            ))
        }
    }
}

/// Convert Y/M/D H:M:S to Unix timestamp (days since 1970-01-01 * 86400)
fn date_to_unix_epoch(year: i64, month: i64, day: i64, hour: i64, min: i64, sec: i64) -> i64 {
    // Days before each month (normal year)
    let days_before_month = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let y = year - 1970;
    // Count leap days between 1970 and year
    let leap_days = ((year - 1969) / 4) - ((year - 1901) / 100) + ((year - 1601) / 400);
    let is_leap_year = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let mut m_days = days_before_month[(month.clamp(1, 12) - 1) as usize];
    if month > 2 && is_leap_year {
        m_days += 1;
    }

    let total_days = y * 365 + leap_days + m_days + (day - 1);
    total_days * 86400 + hour * 3600 + min * 60 + sec
}

// ---------------------------------------------------------------------------
// Common Name and SAN Extraction
// ---------------------------------------------------------------------------

fn extract_cn_from_dn_sequence(dn_seq: &Asn1Element) -> String {
    if let Ok(rdn_sets) = dn_seq.as_sequence() {
        for rdn_set in rdn_sets {
            if let Ok(atvs) = rdn_set.as_sequence() {
                for atv in atvs {
                    if let Ok(pair) = atv.as_sequence() {
                        if pair.len() >= 2 {
                            if let Ok(oid) = pair[0].parse_oid() {
                                // 2.5.4.3 is id-at-commonName
                                if oid == "2.5.4.3" {
                                    if let Ok(val) = std::str::from_utf8(pair[1].value) {
                                        return val.to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    "Unknown".to_string()
}

fn extract_sans(ext_element: &Asn1Element) -> Vec<String> {
    let mut sans = Vec::new();
    if let Ok(exts) = ext_element.as_sequence() {
        for ext in exts {
            if let Ok(parts) = ext.as_sequence() {
                if parts.is_empty() {
                    continue;
                }
                if let Ok(oid) = parts[0].parse_oid() {
                    // 2.5.4.3 = CN, 2.5.29.17 = subjectAltName
                    if oid == "2.5.29.17" {
                        let octet_value = if parts.len() == 2 {
                            parts[1].value
                        } else if parts.len() >= 3 {
                            parts[2].value
                        } else {
                            &[]
                        };

                        if let Ok((general_names, _)) = Asn1Element::parse(octet_value) {
                            if let Ok(items) = general_names.as_sequence() {
                                for item in items {
                                    // tag 0x82 is dNSName [2]
                                    if item.tag == 0x82 {
                                        if let Ok(name) = std::str::from_utf8(item.value) {
                                            sans.push(name.to_string());
                                        }
                                    } else if item.tag == 0x87 {
                                        // tag 0x87 is iPAddress [7]
                                        if item.value.len() == 4 {
                                            sans.push(format!(
                                                "{}.{}.{}.{}",
                                                item.value[0],
                                                item.value[1],
                                                item.value[2],
                                                item.value[3]
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    sans
}

// ---------------------------------------------------------------------------
// Certificate Inspection Engine
// ---------------------------------------------------------------------------

/// Parse and inspect an X.509 certificate from PEM or DER formatted bytes
pub fn inspect_x509_pem(pem_or_der_text: &str) -> Result<CertInspectionReport, VaultError> {
    let der_bytes = if pem_or_der_text.contains("-----BEGIN CERTIFICATE-----") {
        let mut in_cert = false;
        let mut b64 = String::new();
        for line in pem_or_der_text.lines() {
            let t = line.trim();
            if t == "-----BEGIN CERTIFICATE-----" {
                in_cert = true;
                continue;
            }
            if t == "-----END CERTIFICATE-----" {
                break;
            }
            if in_cert {
                b64.push_str(t);
            }
        }
        decode_base64(&b64)?
    } else {
        // Attempt raw base64 or fail
        decode_base64(pem_or_der_text.trim())?
    };

    inspect_x509_der(&der_bytes)
}

/// Inspect an X.509 DER byte payload
pub fn inspect_x509_der(der_bytes: &[u8]) -> Result<CertInspectionReport, VaultError> {
    let (top_seq, _) = Asn1Element::parse(der_bytes)?;
    let top_elements = top_seq.as_sequence()?;

    if top_elements.len() < 3 {
        return Err(VaultError::ValidationError(
            "X.509 DER does not have at least 3 sequence elements (TBS, Alg, Sig)".to_string(),
        ));
    }

    let tbs_element = &top_elements[0];
    let tbs_items = tbs_element.as_sequence()?;

    if tbs_items.len() < 6 {
        return Err(VaultError::ValidationError(
            "TBSCertificate has fewer than 6 required components".to_string(),
        ));
    }

    // Version is optional explicit tag [0] (0xA0)
    let mut offset = 0;
    if tbs_items[0].tag == 0xA0 {
        offset = 1;
    }

    // Serial Number
    let serial_el = &tbs_items[offset];
    let serial_number = serial_el
        .value
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(":");

    // Signature algorithm in TBS
    let sig_alg_el = &tbs_items[offset + 1];
    let sig_alg_oid = if let Ok(items) = sig_alg_el.as_sequence() {
        if !items.is_empty() {
            items[0]
                .parse_oid()
                .unwrap_or_else(|_| "Unknown".to_string())
        } else {
            "Unknown".to_string()
        }
    } else {
        "Unknown".to_string()
    };

    let (signature_algorithm, is_weak_signature) = match sig_alg_oid.as_str() {
        "1.2.840.113549.1.1.5" => ("SHA1withRSA".to_string(), true),
        "1.2.840.113549.1.1.4" => ("MD5withRSA".to_string(), true),
        "1.3.14.3.2.29" => ("SHA1withRSA_OIW".to_string(), true),
        "1.2.840.113549.1.1.11" => ("SHA256withRSA".to_string(), false),
        "1.2.840.113549.1.1.12" => ("SHA384withRSA".to_string(), false),
        "1.2.840.113549.1.1.13" => ("SHA512withRSA".to_string(), false),
        "1.2.840.10045.4.3.2" => ("ECDSA-with-SHA256".to_string(), false),
        "1.2.840.10045.4.3.3" => ("ECDSA-with-SHA384".to_string(), false),
        other => (format!("OID({other})"), false),
    };

    // Issuer
    let issuer_el = &tbs_items[offset + 2];
    let issuer_cn = extract_cn_from_dn_sequence(issuer_el);

    // Validity
    let validity_el = &tbs_items[offset + 3];
    let validity_items = validity_el.as_sequence()?;
    if validity_items.len() < 2 {
        return Err(VaultError::ValidationError(
            "Invalid Validity structure in X.509".to_string(),
        ));
    }
    let not_before_epoch = validity_items[0].parse_time_to_epoch()?;
    let not_after_epoch = validity_items[1].parse_time_to_epoch()?;

    // Subject
    let subject_el = &tbs_items[offset + 4];
    let subject_cn = extract_cn_from_dn_sequence(subject_el);

    // SubjectPublicKeyInfo
    let spki_el = &tbs_items[offset + 5];
    let spki_items = spki_el.as_sequence()?;
    let mut public_key_algorithm = "Unknown".to_string();
    let mut key_size_bits = None;
    let mut is_weak_key = false;

    if spki_items.len() >= 2 {
        if let Ok(alg_items) = spki_items[0].as_sequence() {
            if !alg_items.is_empty() {
                if let Ok(pk_oid) = alg_items[0].parse_oid() {
                    match pk_oid.as_str() {
                        "1.2.840.113549.1.1.1" => {
                            public_key_algorithm = "RSA".to_string();
                            // Parse RSA BitString to inspect modulus length
                            let bit_string_el = &spki_items[1];
                            if bit_string_el.tag == 0x03 && bit_string_el.value.len() > 1 {
                                // Skip unused bits byte
                                let rsa_bytes = &bit_string_el.value[1..];
                                if let Ok((rsa_seq, _)) = Asn1Element::parse(rsa_bytes) {
                                    if let Ok(rsa_parts) = rsa_seq.as_sequence() {
                                        if !rsa_parts.is_empty() && rsa_parts[0].tag == 0x02 {
                                            // Modulus length in bits (ignore leading 0x00 sign byte)
                                            let mut mod_len = rsa_parts[0].value.len();
                                            if !rsa_parts[0].value.is_empty()
                                                && rsa_parts[0].value[0] == 0x00
                                            {
                                                mod_len -= 1;
                                            }
                                            let bits = mod_len * 8;
                                            key_size_bits = Some(bits);
                                            if bits < 2048 {
                                                is_weak_key = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        "1.2.840.10045.2.1" => {
                            public_key_algorithm = "ECC".to_string();
                            key_size_bits = Some(256);
                        }
                        other => {
                            public_key_algorithm = format!("OID({other})");
                        }
                    }
                }
            }
        }
    }

    // Extensions [3] (0xA3)
    let mut subject_alt_names = Vec::new();
    for item in &tbs_items[offset + 6..] {
        if item.tag == 0xA3 {
            subject_alt_names = extract_sans(item);
            break;
        }
    }

    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| VaultError::ValidationError("System clock invalid".to_string()))?
        .as_secs() as i64;

    let seconds_remaining = not_after_epoch - now_epoch;
    let days_until_expiration = seconds_remaining / 86400;

    let status = if days_until_expiration < 0 {
        CertStatus::Expired
    } else if days_until_expiration <= 30 {
        CertStatus::ExpiringSoon
    } else {
        CertStatus::Valid
    };

    let is_self_signed = issuer_cn == subject_cn && issuer_cn != "Unknown";

    Ok(CertInspectionReport {
        subject_cn,
        issuer_cn,
        not_before_epoch,
        not_after_epoch,
        days_until_expiration,
        status,
        is_self_signed,
        signature_algorithm,
        is_weak_signature,
        public_key_algorithm,
        key_size_bits,
        is_weak_key,
        subject_alt_names,
        serial_number,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_to_unix_epoch() {
        // 1970-01-01 00:00:00 = 0
        assert_eq!(date_to_unix_epoch(1970, 1, 1, 0, 0, 0), 0);
        // 2026-10-19 00:00:00
        let epoch_2026 = date_to_unix_epoch(2026, 10, 19, 0, 0, 0);
        assert!(epoch_2026 > 1_700_000_000);
    }

    #[test]
    fn test_base64_decode() {
        let b64 = "VGVzdFhYWFhYWA==";
        let decoded = decode_base64(b64).expect("Decodes valid base64");
        assert_eq!(decoded, b"TestXXXXXX");
    }

    #[test]
    fn test_asn1_oid_parser() {
        // OID 1.2.840.113549.1.1.11 is sha256WithRSAEncryption
        // Bytes: 0x2A 0x86 0x48 0x86 0xF7 0x0D 0x01 0x01 0x0B
        let oid_bytes = [0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B];
        let el = Asn1Element {
            tag: 0x06,
            value: &oid_bytes,
        };
        let oid_str = el.parse_oid().unwrap();
        assert_eq!(oid_str, "1.2.840.113549.1.1.11");
    }
}
