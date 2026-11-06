#![forbid(unsafe_code)]
//! Core Domain Models & Structured Payload Schemas for Offline QR Code Studio

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// Domain errors for QR Code Studio operations
#[derive(Debug, Error)]
pub enum QrStudioError {
    #[error("Storage error: {0}")]
    Storage(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("QR encoding error: {0}")]
    EncodingError(String),

    #[error("QR decoding error: {0}")]
    DecodingError(String),

    #[error("Cryptographic or security error: {0}")]
    CryptoError(String),

    #[error("Invalid input parameter: {0}")]
    InvalidInput(String),

    #[error("License tier insufficient: {0}")]
    LicenseRequired(String),
}

/// Supported QR code payload classifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum QrPayloadType {
    #[default]
    Text,
    Url,
    WiFi,
    VCard,
    Email,
    Sms,
    Geo,
    CalendarEvent,
    TotpUri,
    CryptoPayment,
    EncryptedPayload,
    SignedPayload,
}

impl QrPayloadType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Url => "url",
            Self::WiFi => "wifi",
            Self::VCard => "vcard",
            Self::Email => "email",
            Self::Sms => "sms",
            Self::Geo => "geo",
            Self::CalendarEvent => "calendar",
            Self::TotpUri => "totp",
            Self::CryptoPayment => "crypto",
            Self::EncryptedPayload => "encrypted",
            Self::SignedPayload => "signed",
        }
    }

    pub fn from_str_name(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "url" | "link" | "http" | "https" => Self::Url,
            "wifi" | "wlan" => Self::WiFi,
            "vcard" | "contact" => Self::VCard,
            "email" | "mailto" => Self::Email,
            "sms" | "smsto" => Self::Sms,
            "geo" | "location" => Self::Geo,
            "calendar" | "vevent" => Self::CalendarEvent,
            "totp" | "otp" | "2fa" => Self::TotpUri,
            "crypto" | "bitcoin" | "ethereum" => Self::CryptoPayment,
            "encrypted" | "aes" => Self::EncryptedPayload,
            "signed" | "ed25519" => Self::SignedPayload,
            _ => Self::Text,
        }
    }

    /// Automatically classify raw decoded text into a structured QR payload type
    pub fn detect_from_raw(raw: &str) -> Self {
        let trimmed = raw.trim();
        let upper = trimmed.to_ascii_uppercase();
        if trimmed.starts_with("AGQR-ENC:v1:") {
            Self::EncryptedPayload
        } else if trimmed.starts_with("AGQR-SIG:v1:") {
            Self::SignedPayload
        } else if upper.starts_with("WIFI:") {
            Self::WiFi
        } else if upper.starts_with("BEGIN:VCARD") {
            Self::VCard
        } else if upper.starts_with("BEGIN:VEVENT") {
            Self::CalendarEvent
        } else if upper.starts_with("OTPAUTH://") {
            Self::TotpUri
        } else if upper.starts_with("MAILTO:") || upper.starts_with("MATMSG:") {
            Self::Email
        } else if upper.starts_with("SMSTO:") || upper.starts_with("SMS:") {
            Self::Sms
        } else if upper.starts_with("GEO:") {
            Self::Geo
        } else if upper.starts_with("BITCOIN:")
            || upper.starts_with("ETHEREUM:")
            || upper.starts_with("MONERO:")
        {
            Self::CryptoPayment
        } else if upper.starts_with("HTTP://")
            || upper.starts_with("HTTPS://")
            || upper.starts_with("FTP://")
            || upper.starts_with("WS://")
            || upper.starts_with("WSS://")
        {
            Self::Url
        } else {
            Self::Text
        }
    }
}

/// ISO/IEC 18004 Error Correction Level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum QrEccLevel {
    /// ~7% error recovery capacity
    Low,
    /// ~15% error recovery capacity
    #[default]
    Medium,
    /// ~25% error recovery capacity
    Quartile,
    /// ~30% error recovery capacity (recommended for logos/stylized QR codes)
    High,
}

impl QrEccLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "L",
            Self::Medium => "M",
            Self::Quartile => "Q",
            Self::High => "H",
        }
    }

    pub fn from_str_code(code: &str) -> Self {
        match code.trim().to_ascii_uppercase().as_str() {
            "L" | "LOW" => Self::Low,
            "Q" | "QUARTILE" => Self::Quartile,
            "H" | "HIGH" => Self::High,
            _ => Self::Medium,
        }
    }

    /// 2-bit format indicator per ISO/IEC 18004 Table 25
    pub fn format_bits(&self) -> u8 {
        match self {
            Self::Low => 0b01,
            Self::Medium => 0b00,
            Self::Quartile => 0b11,
            Self::High => 0b10,
        }
    }

    pub fn from_format_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0b01 => Self::Low,
            0b00 => Self::Medium,
            0b11 => Self::Quartile,
            _ => Self::High,
        }
    }

    pub fn recovery_percentage(&self) -> u8 {
        match self {
            Self::Low => 7,
            Self::Medium => 15,
            Self::Quartile => 25,
            Self::High => 30,
        }
    }
}

/// Visual geometry for individual QR data modules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ModuleShape {
    #[default]
    Square,
    Rounded,
    Dots,
    Diamond,
}

impl ModuleShape {
    pub fn from_str_name(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "rounded" => Self::Rounded,
            "dots" | "circle" => Self::Dots,
            "diamond" | "rhombus" => Self::Diamond,
            _ => Self::Square,
        }
    }
}

/// Visual geometry for the three 7x7 Finder Patterns ("Eyes")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum FinderStyle {
    #[default]
    Square,
    Rounded,
    Circle,
}

impl FinderStyle {
    pub fn from_str_name(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "rounded" => Self::Rounded,
            "circle" | "dot" => Self::Circle,
            _ => Self::Square,
        }
    }
}

/// Complete visual customization configuration for QR rendering
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QrStyleConfig {
    pub ecc_level: QrEccLevel,
    pub module_shape: ModuleShape,
    pub finder_style: FinderStyle,
    pub fg_color: String,
    pub fg_gradient_end: Option<String>,
    pub bg_color: String,
    pub quiet_zone_modules: u8,
    pub size_px: u32,
    pub center_logo_text: Option<String>,
}

impl Default for QrStyleConfig {
    fn default() -> Self {
        Self {
            ecc_level: QrEccLevel::Medium,
            module_shape: ModuleShape::Square,
            finder_style: FinderStyle::Square,
            fg_color: "#0F172A".to_string(),
            fg_gradient_end: None,
            bg_color: "#FFFFFF".to_string(),
            quiet_zone_modules: 4,
            size_px: 512,
            center_logo_text: None,
        }
    }
}

/// Structured Wi-Fi Network Configuration (`WIFI:T:WPA;S:MyNet;P:Secret;H:false;;`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiCredentials {
    pub ssid: String,
    pub password: String,
    pub encryption: String,
    pub hidden: bool,
}

/// Helper to split string on unescaped delimiter
fn split_unescaped(s: &str, delimiter: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if next == delimiter || next == '\\' || next == ':' || next == ',' {
                    current.push(next);
                    chars.next();
                    continue;
                }
            }
            current.push(c);
        } else if c == delimiter {
            parts.push(current);
            current = String::new();
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

impl WifiCredentials {
    pub fn to_qr_string(&self) -> String {
        let enc = if self.encryption.trim().is_empty() {
            "WPA"
        } else {
            self.encryption.trim()
        };
        let escape = |s: &str| {
            s.replace('\\', "\\\\")
                .replace(';', "\\;")
                .replace(',', "\\,")
                .replace(':', "\\:")
        };
        format!(
            "WIFI:T:{};S:{};P:{};H:{};;",
            enc,
            escape(&self.ssid),
            escape(&self.password),
            if self.hidden { "true" } else { "false" }
        )
    }

    pub fn from_qr_string(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if !trimmed.to_ascii_uppercase().starts_with("WIFI:") {
            return None;
        }
        let body = &trimmed[5..];
        let mut ssid = String::new();
        let mut password = String::new();
        let mut encryption = "WPA".to_string();
        let mut hidden = false;

        let parts = split_unescaped(body, ';');
        for part in parts {
            let p = part.trim();
            if let Some(val) = p.strip_prefix("S:").or_else(|| p.strip_prefix("s:")) {
                ssid = val.to_string();
            } else if let Some(val) = p.strip_prefix("P:").or_else(|| p.strip_prefix("p:")) {
                password = val.to_string();
            } else if let Some(val) = p.strip_prefix("T:").or_else(|| p.strip_prefix("t:")) {
                encryption = val.to_string();
            } else if let Some(val) = p.strip_prefix("H:").or_else(|| p.strip_prefix("h:")) {
                hidden = val.eq_ignore_ascii_case("true");
            }
        }
        if ssid.is_empty() {
            return None;
        }
        Some(Self {
            ssid,
            password,
            encryption,
            hidden,
        })
    }
}

/// Structured vCard 3.0 Contact (`BEGIN:VCARD...END:VCARD`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VCardContact {
    pub full_name: String,
    pub organization: String,
    pub title: String,
    pub phone: String,
    pub email: String,
    pub url: String,
}

impl VCardContact {
    pub fn to_qr_string(&self) -> String {
        let mut lines = vec![
            "BEGIN:VCARD".to_string(),
            "VERSION:3.0".to_string(),
            format!("FN:{}", self.full_name.trim()),
        ];
        if !self.organization.trim().is_empty() {
            lines.push(format!("ORG:{}", self.organization.trim()));
        }
        if !self.title.trim().is_empty() {
            lines.push(format!("TITLE:{}", self.title.trim()));
        }
        if !self.phone.trim().is_empty() {
            lines.push(format!("TEL:{}", self.phone.trim()));
        }
        if !self.email.trim().is_empty() {
            lines.push(format!("EMAIL:{}", self.email.trim()));
        }
        if !self.url.trim().is_empty() {
            lines.push(format!("URL:{}", self.url.trim()));
        }
        lines.push("END:VCARD".to_string());
        lines.join("\n")
    }

    pub fn from_qr_string(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if !trimmed.to_ascii_uppercase().starts_with("BEGIN:VCARD") {
            return None;
        }
        let mut full_name = String::new();
        let mut organization = String::new();
        let mut title = String::new();
        let mut phone = String::new();
        let mut email = String::new();
        let mut url = String::new();

        for line in trimmed.lines() {
            let l = line.trim();
            if let Some(v) = l.strip_prefix("FN:") {
                full_name = v.trim().to_string();
            } else if let Some(v) = l.strip_prefix("ORG:") {
                organization = v.trim().to_string();
            } else if let Some(v) = l.strip_prefix("TITLE:") {
                title = v.trim().to_string();
            } else if let Some(v) = l.strip_prefix("TEL:") {
                phone = v.trim().to_string();
            } else if let Some(v) = l.strip_prefix("EMAIL:") {
                email = v.trim().to_string();
            } else if let Some(v) = l.strip_prefix("URL:") {
                url = v.trim().to_string();
            }
        }
        if full_name.is_empty() {
            return None;
        }
        Some(Self {
            full_name,
            organization,
            title,
            phone,
            email,
            url,
        })
    }
}

/// Parse structured key-value attributes from any decoded QR string
pub fn extract_structured_fields(
    raw: &str,
    payload_type: QrPayloadType,
) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    map.insert("type".to_string(), payload_type.as_str().to_string());

    match payload_type {
        QrPayloadType::WiFi => {
            if let Some(wifi) = WifiCredentials::from_qr_string(raw) {
                map.insert("ssid".to_string(), wifi.ssid);
                map.insert("encryption".to_string(), wifi.encryption);
                map.insert("hidden".to_string(), wifi.hidden.to_string());
                map.insert(
                    "password_present".to_string(),
                    (!wifi.password.is_empty()).to_string(),
                );
            }
        }
        QrPayloadType::VCard => {
            if let Some(vc) = VCardContact::from_qr_string(raw) {
                map.insert("full_name".to_string(), vc.full_name);
                if !vc.organization.is_empty() {
                    map.insert("organization".to_string(), vc.organization);
                }
                if !vc.phone.is_empty() {
                    map.insert("phone".to_string(), vc.phone);
                }
                if !vc.email.is_empty() {
                    map.insert("email".to_string(), vc.email);
                }
            }
        }
        QrPayloadType::Url => {
            if let Some(scheme_idx) = raw.find("://") {
                let scheme = &raw[..scheme_idx].to_ascii_lowercase();
                let rest = &raw[scheme_idx + 3..];
                let host = rest.split(['/', '?', '#']).next().unwrap_or("");
                map.insert("scheme".to_string(), scheme.clone());
                map.insert("host".to_string(), host.to_string());
            }
        }
        QrPayloadType::Geo => {
            if let Some(coords) = raw
                .strip_prefix("geo:")
                .or_else(|| raw.strip_prefix("GEO:"))
            {
                let mut parts = coords.split('?');
                if let Some(lat_lon) = parts.next() {
                    map.insert("coordinates".to_string(), lat_lon.to_string());
                }
            }
        }
        _ => {}
    }
    map
}

/// Threat risk severity classification for scanned or generated QR payloads
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum SecurityRiskLevel {
    #[default]
    Safe,
    LowRisk,
    MediumRisk,
    HighRisk,
    Critical,
}

impl SecurityRiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "SAFE",
            Self::LowRisk => "LOW_RISK",
            Self::MediumRisk => "MEDIUM_RISK",
            Self::HighRisk => "HIGH_RISK",
            Self::Critical => "CRITICAL",
        }
    }
}

/// Cryptographic authenticity state of a QR payload
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SignatureVerificationStatus {
    #[default]
    Unsigned,
    SignatureValid,
    SignatureInvalid,
    EncryptedLocked,
    DecryptedVerified,
}

/// Individual security heuristic finding
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub code: String,
    pub severity: SecurityRiskLevel,
    pub title: String,
    pub description: String,
    pub remediation: String,
}

/// Comprehensive offline anti-quishing & cryptographic analysis report
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityAnalysisReport {
    pub risk_level: SecurityRiskLevel,
    pub risk_score: u8,
    pub payload_type: QrPayloadType,
    pub signature_status: SignatureVerificationStatus,
    pub signer_public_key_hex: Option<String>,
    pub verified_cleartext: Option<String>,
    pub host_domain: Option<String>,
    pub findings: Vec<SecurityFinding>,
}

/// Output of a QR generation request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QrGenerationResult {
    pub id: String,
    pub label: String,
    pub payload_type: QrPayloadType,
    pub raw_payload: String,
    pub encoded_payload: String,
    pub version: u8,
    pub dimension: usize,
    pub ecc_level: QrEccLevel,
    pub mask_pattern: u8,
    pub svg_markup: String,
    pub png_base64: String,
    pub contrast_ratio: f64,
    pub scannability_warning: Option<String>,
    pub security_report: SecurityAnalysisReport,
    pub created_at: u64,
}

/// Output of scanning a QR image or frame
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QrScanResult {
    pub id: String,
    pub decoded_text: String,
    pub payload_type: QrPayloadType,
    pub version: u8,
    pub dimension: usize,
    pub ecc_level: QrEccLevel,
    pub mask_pattern: u8,
    pub parsed_fields: BTreeMap<String, String>,
    pub security_report: SecurityAnalysisReport,
    pub decode_time_us: u64,
}

/// Persistent SQLite history record for generated and scanned QR codes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QrHistoryRecord {
    pub id: String,
    pub direction: String,
    pub label: String,
    pub payload_type: QrPayloadType,
    pub raw_payload: String,
    pub version: u8,
    pub ecc_level: QrEccLevel,
    pub svg_markup: String,
    pub risk_level: SecurityRiskLevel,
    pub is_favorite: bool,
    pub created_at: u64,
}

/// Single item in a batch QR generation job
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchQrItem {
    pub label: String,
    #[serde(alias = "raw_text")]
    pub payload: String,
    #[serde(default)]
    pub payload_type: Option<QrPayloadType>,
}

/// Result entry for a batch QR generation item
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchQrGeneratedEntry {
    pub label: String,
    pub payload: String,
    pub version: u8,
    pub dimension: usize,
    pub svg_markup: String,
    pub png_base64: String,
    pub risk_level: SecurityRiskLevel,
}

/// Summary report of a completed batch generation operation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchJobReport {
    pub total_requested: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub elapsed_ms: u64,
    pub entries: Vec<BatchQrGeneratedEntry>,
}

/// Studio-wide offline telemetry & analytics counters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct QrStudioStats {
    pub total_generated: usize,
    pub total_scanned: usize,
    pub favorites_count: usize,
    pub threats_flagged: usize,
    pub encrypted_count: usize,
    pub signed_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wifi_credentials_roundtrip() {
        let wifi = WifiCredentials {
            ssid: "CorpNet;5G".to_string(),
            password: "My:Secret;Password".to_string(),
            encryption: "WPA".to_string(),
            hidden: true,
        };
        let qr_str = wifi.to_qr_string();
        assert!(qr_str.starts_with("WIFI:T:WPA;"));
        let parsed = WifiCredentials::from_qr_string(&qr_str).expect("Should parse WiFi QR");
        assert_eq!(parsed.ssid, "CorpNet;5G");
        assert_eq!(parsed.password, "My:Secret;Password");
        assert_eq!(parsed.encryption, "WPA");
        assert!(parsed.hidden);
        assert_eq!(QrPayloadType::detect_from_raw(&qr_str), QrPayloadType::WiFi);
    }

    #[test]
    fn test_vcard_contact_roundtrip() {
        let contact = VCardContact {
            full_name: "Ada Lovelace".to_string(),
            organization: "Analytical Engine Labs".to_string(),
            title: "Chief Architect".to_string(),
            phone: "+44-20-7946-0192".to_string(),
            email: "ada@analytical.org".to_string(),
            url: "https://analytical.org".to_string(),
        };
        let qr_str = contact.to_qr_string();
        let parsed = VCardContact::from_qr_string(&qr_str).expect("Should parse vCard");
        assert_eq!(parsed, contact);
        assert_eq!(
            QrPayloadType::detect_from_raw(&qr_str),
            QrPayloadType::VCard
        );
    }

    #[test]
    fn test_payload_type_detection() {
        assert_eq!(
            QrPayloadType::detect_from_raw("https://rust-lang.org"),
            QrPayloadType::Url
        );
        assert_eq!(
            QrPayloadType::detect_from_raw("otpauth://totp/GitHub:user?secret=JBSWY3DPEHPK3PXP"),
            QrPayloadType::TotpUri
        );
        assert_eq!(
            QrPayloadType::detect_from_raw("AGQR-ENC:v1:aa:bb:cc"),
            QrPayloadType::EncryptedPayload
        );
        assert_eq!(
            QrPayloadType::detect_from_raw("AGQR-SIG:v1:aa:bb:hello"),
            QrPayloadType::SignedPayload
        );
        assert_eq!(
            QrPayloadType::detect_from_raw("Plain inventory serial #94810"),
            QrPayloadType::Text
        );
    }
}
