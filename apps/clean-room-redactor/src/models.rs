use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionCategory {
    Ssn,
    CreditCard,
    Email,
    Phone,
    IpAddress,
    MedicalRecordNumber,
    DeaNpiNumber,
    BankAccount,
    PersonName,
    CustomKeyword,
    ManualBox,
}

impl RedactionCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ssn => "Social Security Number (SSN)",
            Self::CreditCard => "Payment Card (PCI-DSS)",
            Self::Email => "Email Address",
            Self::Phone => "Telephone Number",
            Self::IpAddress => "IP Address / Network Identifier",
            Self::MedicalRecordNumber => "Medical Record Number (HIPAA)",
            Self::DeaNpiNumber => "DEA / NPI Provider ID",
            Self::BankAccount => "Bank Routing / Account Number",
            Self::PersonName => "Personal Identity / Name",
            Self::CustomKeyword => "Custom Pattern / Keyword",
            Self::ManualBox => "Manual Region Mask",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 20.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PiiFinding {
    pub id: String,
    pub page_index: usize,
    pub category: RedactionCategory,
    pub matched_text: String,
    pub mask_preview: String,
    pub confidence: f32,
    pub bbox: BoundingBox,
    pub approved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BurnStyle {
    SolidBlack,
    SolidWhite,
    PatternHatch,
    ExemptionCode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedactionMask {
    pub id: String,
    pub page_index: usize,
    pub bbox: BoundingBox,
    pub label: String,
    pub burn_style: BurnStyle,
    pub exemption_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatesPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatesConfig {
    pub prefix: String,
    pub suffix: String,
    pub start_number: u64,
    pub digits: usize,
    pub position: BatesPosition,
    pub font_size: u32,
    pub include_date: bool,
}

impl Default for BatesConfig {
    fn default() -> Self {
        Self {
            prefix: "CONFIDENTIAL-".to_string(),
            suffix: "".to_string(),
            start_number: 1,
            digits: 6,
            position: BatesPosition::BottomRight,
            font_size: 10,
            include_date: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentPage {
    pub page_index: usize,
    pub width_pts: f32,
    pub height_pts: f32,
    pub text_content: String,
    pub pii_findings: Vec<PiiFinding>,
    pub redaction_masks: Vec<RedactionMask>,
    pub thumbnail_svg: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedactionDocument {
    pub document_id: String,
    pub filename: String,
    pub file_size_bytes: u64,
    pub total_pages: usize,
    pub original_sha256: String,
    pub original_blake3: String,
    pub has_embedded_metadata: bool,
    pub metadata_keys: Vec<String>,
    pub pages: Vec<DocumentPage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SanitizationCertificate {
    pub certificate_id: String,
    pub document_id: String,
    pub filename: String,
    pub timestamp: u64,
    pub original_sha256: String,
    pub original_blake3: String,
    pub sanitized_sha256: String,
    pub sanitized_blake3: String,
    pub total_pages: usize,
    pub total_redactions_applied: usize,
    pub scrubbed_categories: Vec<String>,
    pub metadata_stripped: bool,
    pub bates_range: Option<String>,
    pub signing_key_id: String,
    pub signature_hex: String,
    pub verification_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedactionStats {
    pub total_documents_sanitized: usize,
    pub total_pages_sanitized: usize,
    pub total_pii_entities_burned: usize,
    pub total_certificates_issued: usize,
    pub compliance_score: f32,
}
