#![forbid(unsafe_code)]
//! DevOps Secret Vault - Native RFC 6238 Real-Time TOTP Engine
//!
//! Provides an on-device, zero-network, constant-time TOTP generation engine:
//! - RFC 6238 Time-based One-time Password Algorithm
//! - RFC 4648 Base32 Secret Key Decoding with sanitization
//! - RFC 2104 HMAC with SHA-1 and SHA-256 engines
//! - Pure Rust `#![forbid(unsafe_code)]` RFC 3174 SHA-1 implementation
//! - Real-time step countdown and URI parser (`otpauth://totp/...`)

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::VaultError;

/// Supported hashing algorithms for TOTP
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum TotpAlgorithm {
    #[default]
    Sha1,
    Sha256,
}

impl TotpAlgorithm {
    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "SHA256" | "SHA-256" => Self::Sha256,
            _ => Self::Sha1,
        }
    }
}

/// TOTP generation configuration and parsed parameters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct TotpConfig {
    pub secret_base32: String,
    #[zeroize(skip)]
    pub issuer: Option<String>,
    #[zeroize(skip)]
    pub account_name: Option<String>,
    #[zeroize(skip)]
    pub algorithm: TotpAlgorithm,
    #[zeroize(skip)]
    pub digits: u32,
    #[zeroize(skip)]
    pub step_seconds: u64,
}

impl Default for TotpConfig {
    fn default() -> Self {
        Self {
            secret_base32: String::new(),
            issuer: None,
            account_name: None,
            algorithm: TotpAlgorithm::Sha1,
            digits: 6,
            step_seconds: 30,
        }
    }
}

/// Generated live TOTP token result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpCodeResponse {
    pub code: String,
    pub remaining_seconds: u64,
    pub total_step_seconds: u64,
    pub progress_ratio: f32,
}

// ---------------------------------------------------------------------------
// Base32 Decoding (RFC 4648)
// ---------------------------------------------------------------------------

/// Decode an RFC 4648 Base32 string into raw bytes
pub fn decode_base32(input: &str) -> Result<Vec<u8>, VaultError> {
    let clean: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '=')
        .collect();

    if clean.is_empty() {
        return Err(VaultError::ValidationError(
            "Base32 input is empty".to_string(),
        ));
    }

    let mut bits = 0u32;
    let mut bit_count = 0u32;
    let mut output = Vec::new();

    for c in clean.chars() {
        let val = match c {
            'A'..='Z' => (c as u32) - ('A' as u32),
            'a'..='z' => (c as u32) - ('a' as u32),
            '2'..='7' => (c as u32) - ('2' as u32) + 26,
            _ => {
                return Err(VaultError::ValidationError(format!(
                    "Invalid Base32 character: '{c}'"
                )))
            }
        };

        bits = (bits << 5) | val;
        bit_count += 5;

        if bit_count >= 8 {
            bit_count -= 8;
            output.push(((bits >> bit_count) & 0xFF) as u8);
        }
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// Pure Rust RFC 3174 SHA-1 Engine (#![forbid(unsafe_code)])
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Sha1 {
    state: [u32; 5],
    count: u64,
    buffer: [u8; 64],
    buf_len: usize,
}

impl Sha1 {
    fn new() -> Self {
        Self {
            state: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
            count: 0,
            buffer: [0u8; 64],
            buf_len: 0,
        }
    }

    fn update(&mut self, data: &[u8]) {
        self.count += data.len() as u64;
        let mut idx = 0;

        // If buffer already has partial data
        if self.buf_len > 0 {
            let needed = 64 - self.buf_len;
            if data.len() >= needed {
                self.buffer[self.buf_len..64].copy_from_slice(&data[..needed]);
                let block = self.buffer;
                self.transform(&block);
                self.buf_len = 0;
                idx = needed;
            } else {
                self.buffer[self.buf_len..self.buf_len + data.len()].copy_from_slice(data);
                self.buf_len += data.len();
                return;
            }
        }

        while idx + 64 <= data.len() {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[idx..idx + 64]);
            self.transform(&block);
            idx += 64;
        }

        let remaining = &data[idx..];
        if !remaining.is_empty() {
            self.buffer[..remaining.len()].copy_from_slice(remaining);
            self.buf_len = remaining.len();
        }
    }

    fn transform(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            let b0 = block[i * 4] as u32;
            let b1 = block[i * 4 + 1] as u32;
            let b2 = block[i * 4 + 2] as u32;
            let b3 = block[i * 4 + 3] as u32;
            w[i] = (b0 << 24) | (b1 << 16) | (b2 << 8) | b3;
        }
        for i in 16..80 {
            let v = w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16];
            w[i] = v.rotate_left(1);
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];

        for (i, &w_item) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };

            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w_item);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
    }

    fn finalize(mut self) -> [u8; 20] {
        let total_bits = self.count * 8;
        // Append 0x80 byte
        self.buffer[self.buf_len] = 0x80;
        self.buf_len += 1;

        if self.buf_len > 56 {
            for i in self.buf_len..64 {
                self.buffer[i] = 0;
            }
            let block = self.buffer;
            self.transform(&block);
            self.buf_len = 0;
        }

        for i in self.buf_len..56 {
            self.buffer[i] = 0;
        }

        let len_bytes = total_bits.to_be_bytes();
        self.buffer[56..64].copy_from_slice(&len_bytes);
        let block = self.buffer;
        self.transform(&block);

        let mut out = [0u8; 20];
        for i in 0..5 {
            let be = self.state[i].to_be_bytes();
            out[i * 4..i * 4 + 4].copy_from_slice(&be);
        }
        out
    }

    fn digest(data: &[u8]) -> [u8; 20] {
        let mut hasher = Self::new();
        hasher.update(data);
        hasher.finalize()
    }
}

// ---------------------------------------------------------------------------
// RFC 2104 HMAC Implementation
// ---------------------------------------------------------------------------

fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; 20] {
    let mut k_pad = [0u8; 64];
    if key.len() > 64 {
        let key_hash = Sha1::digest(key);
        k_pad[..20].copy_from_slice(&key_hash);
    } else {
        k_pad[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0u8; 64];
    let mut opad = [0u8; 64];
    for i in 0..64 {
        ipad[i] = k_pad[i] ^ 0x36;
        opad[i] = k_pad[i] ^ 0x5c;
    }

    let mut inner = Sha1::new();
    inner.update(&ipad);
    inner.update(message);
    let inner_hash = inner.finalize();

    let mut outer = Sha1::new();
    outer.update(&opad);
    outer.update(&inner_hash);
    outer.finalize()
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut k_pad = [0u8; 64];
    if key.len() > 64 {
        let mut hasher = Sha256::new();
        hasher.update(key);
        let key_hash = hasher.finalize();
        k_pad[..32].copy_from_slice(&key_hash);
    } else {
        k_pad[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0u8; 64];
    let mut opad = [0u8; 64];
    for i in 0..64 {
        ipad[i] = k_pad[i] ^ 0x36;
        opad[i] = k_pad[i] ^ 0x5c;
    }

    let mut inner = Sha256::new();
    inner.update(&ipad[..]);
    inner.update(message);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(&opad[..]);
    outer.update(&inner_hash[..]);
    let outer_hash = outer.finalize();

    let mut out = [0u8; 32];
    out.copy_from_slice(&outer_hash);
    out
}

// ---------------------------------------------------------------------------
// TOTP Token Generation
// ---------------------------------------------------------------------------

impl TotpConfig {
    /// Generate the TOTP code for a specific Unix timestamp (seconds)
    pub fn generate_at_timestamp(&self, timestamp: u64) -> Result<TotpCodeResponse, VaultError> {
        let key_bytes = decode_base32(&self.secret_base32)?;
        let step = if self.step_seconds == 0 {
            30
        } else {
            self.step_seconds
        };
        let counter = timestamp / step;
        let counter_bytes = counter.to_be_bytes();

        let digits = if self.digits == 0 { 6 } else { self.digits };
        let code_int = match self.algorithm {
            TotpAlgorithm::Sha1 => {
                let hmac = hmac_sha1(&key_bytes, &counter_bytes);
                let offset = (hmac[19] & 0x0F) as usize;
                let binary = (((hmac[offset] & 0x7F) as u32) << 24)
                    | ((hmac[offset + 1] as u32) << 16)
                    | ((hmac[offset + 2] as u32) << 8)
                    | (hmac[offset + 3] as u32);
                binary % 10u32.pow(digits)
            }
            TotpAlgorithm::Sha256 => {
                let hmac = hmac_sha256(&key_bytes, &counter_bytes);
                let offset = (hmac[31] & 0x0F) as usize;
                let binary = (((hmac[offset] & 0x7F) as u32) << 24)
                    | ((hmac[offset + 1] as u32) << 16)
                    | ((hmac[offset + 2] as u32) << 8)
                    | (hmac[offset + 3] as u32);
                binary % 10u32.pow(digits)
            }
        };

        let formatted = format!("{:0width$}", code_int, width = digits as usize);
        let elapsed = timestamp % step;
        let remaining_seconds = step - elapsed;
        let progress_ratio = elapsed as f32 / step as f32;

        Ok(TotpCodeResponse {
            code: formatted,
            remaining_seconds,
            total_step_seconds: step,
            progress_ratio,
        })
    }

    /// Generate TOTP code at the current wall clock time
    pub fn generate_current(&self) -> Result<TotpCodeResponse, VaultError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| VaultError::ValidationError("System clock before Unix Epoch".to_string()))?
            .as_secs();
        self.generate_at_timestamp(now)
    }

    /// Parse standard otpauth://totp URI
    pub fn parse_otpauth_uri(uri: &str) -> Result<Self, VaultError> {
        let trimmed = uri.trim();
        if !trimmed.starts_with("otpauth://totp/") {
            return Err(VaultError::ValidationError(
                "Invalid TOTP URI scheme, must start with otpauth://totp/".to_string(),
            ));
        }

        let after_scheme = &trimmed["otpauth://totp/".len()..];
        let (label_part, query_part) = after_scheme.split_once('?').ok_or_else(|| {
            VaultError::ValidationError("Missing query parameters in TOTP URI".to_string())
        })?;

        let decoded_label = urlencoding_decode(label_part);
        let (issuer_from_label, account_name) =
            if let Some((iss, acc)) = decoded_label.split_once(':') {
                (Some(iss.trim().to_string()), Some(acc.trim().to_string()))
            } else {
                (None, Some(decoded_label.trim().to_string()))
            };

        let mut secret = None;
        let mut issuer = issuer_from_label;
        let mut algorithm = TotpAlgorithm::Sha1;
        let mut digits = 6u32;
        let mut period = 30u64;

        for param in query_part.split('&') {
            if let Some((k, v)) = param.split_once('=') {
                let k_lower = k.to_lowercase();
                let v_dec = urlencoding_decode(v);
                match k_lower.as_str() {
                    "secret" => secret = Some(v_dec),
                    "issuer" => issuer = Some(v_dec),
                    "algorithm" => algorithm = TotpAlgorithm::from_str_lossy(&v_dec),
                    "digits" => {
                        if let Ok(d) = v_dec.parse::<u32>() {
                            digits = d;
                        }
                    }
                    "period" => {
                        if let Ok(p) = v_dec.parse::<u64>() {
                            period = p;
                        }
                    }
                    _ => {}
                }
            }
        }

        let secret_val = secret.ok_or_else(|| {
            VaultError::ValidationError("Missing 'secret' parameter in TOTP URI".to_string())
        })?;

        Ok(Self {
            secret_base32: secret_val,
            issuer,
            account_name,
            algorithm,
            digits,
            step_seconds: period,
        })
    }
}

fn urlencoding_decode(s: &str) -> String {
    let mut res = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(b) = u8::from_str_radix(&hex, 16) {
                res.push(b as char);
            } else {
                res.push('%');
                res.push_str(&hex);
            }
        } else if c == '+' {
            res.push(' ');
        } else {
            res.push(c);
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base32_decode() {
        // "Hello!" in ASCII is 0x48 0x65 0x6C 0x6C 0x6F 0x21
        // Base32 is JBSWY3DPEE======
        let decoded = decode_base32("JBSWY3DPEE").expect("Decodes valid base32");
        assert_eq!(decoded, b"Hello!");

        // Case insensitivity and whitespace stripping
        let decoded_spaced = decode_base32("jbsw y3dp ee==").expect("Decodes spaced base32");
        assert_eq!(decoded_spaced, b"Hello!");
    }

    #[test]
    fn test_sha1_known_vector() {
        // SHA-1("abc") = a9993e364706816aba3e25717850c26c9cd0d89d
        let hash = Sha1::digest(b"abc");
        let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn test_rfc6238_sha1_official_test_vectors() {
        // RFC 6238 Appendix B test vector key for SHA1 is ASCII "12345678901234567890" (20 bytes)
        // Base32 representation of "12345678901234567890" is GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ
        let config = TotpConfig {
            secret_base32: "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".to_string(),
            issuer: Some("RFC6238".to_string()),
            account_name: Some("test@ietf.org".to_string()),
            algorithm: TotpAlgorithm::Sha1,
            digits: 8,
            step_seconds: 30,
        };

        // RFC 6238 Appendix B:
        // Time = 59 -> TOTP = 94287082
        let resp_59 = config.generate_at_timestamp(59).unwrap();
        assert_eq!(resp_59.code, "94287082");

        // Time = 1111111109 -> TOTP = 07081804
        let resp_1109 = config.generate_at_timestamp(1111111109).unwrap();
        assert_eq!(resp_1109.code, "07081804");

        // Time = 1111111111 -> TOTP = 14050471
        let resp_1111 = config.generate_at_timestamp(1111111111).unwrap();
        assert_eq!(resp_1111.code, "14050471");

        // Time = 1234567890 -> TOTP = 89005924
        let resp_1234 = config.generate_at_timestamp(1234567890).unwrap();
        assert_eq!(resp_1234.code, "89005924");

        // Time = 2000000000 -> TOTP = 69279037
        let resp_2000 = config.generate_at_timestamp(2000000000).unwrap();
        assert_eq!(resp_2000.code, "69279037");
    }

    #[test]
    fn test_otpauth_uri_parser() {
        let uri = "otpauth://totp/GitHub:octocat?secret=JBSWY3DPEHPK3PXP&issuer=GitHub&digits=6&period=30";
        let config = TotpConfig::parse_otpauth_uri(uri).expect("Should parse valid URI");
        assert_eq!(config.secret_base32, "JBSWY3DPEHPK3PXP");
        assert_eq!(config.issuer, Some("GitHub".to_string()));
        assert_eq!(config.account_name, Some("octocat".to_string()));
        assert_eq!(config.digits, 6);
        assert_eq!(config.step_seconds, 30);
    }
}
