//! Data models for Forensic Air-Gapped Media & Document Privacy Sanitizer.
//!
//! Models metadata tags, risk classifications, GPS extractions,
//! zero-trace sanitization results, and Ed25519 privacy audit certificates.

use serde::{Deserialize, Serialize};

/// Recognized Media and Document File Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Jpeg,
    Png,
    WebP,
    Pdf,
    Tiff,
    Mp4,
    GenericBinary,
}

impl MediaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Jpeg => "JPEG Image",
            MediaType::Png => "PNG Image",
            MediaType::WebP => "WebP Image",
            MediaType::Pdf => "PDF Document",
            MediaType::Tiff => "TIFF Image",
            MediaType::Mp4 => "MP4 Video",
            MediaType::GenericBinary => "Generic Binary File",
        }
    }
}

/// Severity Rating for Metadata Privacy Leakage
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataRiskGrade {
    Clean,
    Medium,
    High,
    Critical,
}

impl MetadataRiskGrade {
    pub fn as_str(&self) -> &'static str {
        match self {
            MetadataRiskGrade::Clean => "CLEAN",
            MetadataRiskGrade::Medium => "MEDIUM_RISK",
            MetadataRiskGrade::High => "HIGH_RISK",
            MetadataRiskGrade::Critical => "CRITICAL_RISK",
        }
    }
}

/// Extracted Metadata Property Tag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataTag {
    pub tag_name: String,
    pub category: String, // "GPS Geolocation", "Device Identifiers", "Author & Legal", "Chronological", "Technical"
    pub raw_value: String,
    pub risk_grade: MetadataRiskGrade,
    pub explanation: String,
}

/// Detailed Forensic Inspection of Media Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInspectionReport {
    pub file_id: String,
    pub filename: String,
    pub media_type: MediaType,
    pub original_size_bytes: usize,
    pub original_blake3: String,
    pub metadata_tags: Vec<MetadataTag>,
    pub has_gps: bool,
    pub gps_coordinates: Option<String>,
    pub device_fingerprint: Option<String>,
    pub overall_privacy_risk: MetadataRiskGrade,
}

/// Media Summary for Navigation List
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSummary {
    pub file_id: String,
    pub filename: String,
    pub media_type: MediaType,
    pub size_bytes: usize,
    pub tags_detected_count: usize,
    pub has_gps: bool,
    pub is_sanitized: bool,
    pub overall_privacy_risk: MetadataRiskGrade,
}

/// Result of Zero-Trace Sanitization Operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizedResult {
    pub file_id: String,
    pub filename: String,
    pub original_size_bytes: usize,
    pub sanitized_size_bytes: usize,
    pub bytes_removed: usize,
    pub original_blake3: String,
    pub sanitized_blake3: String,
    pub tags_stripped_count: usize,
    pub gps_purged: bool,
    pub sanitized_at_epoch_ms: i64,
}

/// Cryptographically Signed Privacy & Zero-Trace Audit Certificate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyAuditCertificate {
    pub certificate_id: String,
    pub file_id: String,
    pub filename: String,
    pub original_blake3: String,
    pub sanitized_blake3: String,
    pub tags_purged_count: usize,
    pub zero_trace_verified: bool,
    pub ed25519_signature_hex: String,
    pub auditor_public_key_hex: String,
    pub issued_at_epoch_ms: i64,
}

/// Executive Workbench Metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizerStats {
    pub total_files_sanitized: usize,
    pub total_tags_purged: usize,
    pub gps_leaks_prevented: usize,
    pub device_fingerprints_destroyed: usize,
}
