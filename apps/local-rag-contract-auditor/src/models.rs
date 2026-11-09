//! Data models for Offline Local RAG Contract & Compliance Auditor.
//!
//! Represents contracts, clause findings, risk classifications,
//! semantic embeddings, redlines, and Ed25519 legal audit certificates.

use serde::{Deserialize, Serialize};

/// High-level Contract Type Classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractType {
    SaaSMasterServicesAgreement,
    NonDisclosureAgreement,
    DataProcessingAgreement,
    VendorProcurement,
    EmploymentAgreement,
    Other,
}

impl ContractType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContractType::SaaSMasterServicesAgreement => "SaaS Master Services Agreement (MSA)",
            ContractType::NonDisclosureAgreement => "Non-Disclosure Agreement (NDA)",
            ContractType::DataProcessingAgreement => "Data Processing Agreement (DPA / GDPR)",
            ContractType::VendorProcurement => "Vendor Procurement & Licensing Agreement",
            ContractType::EmploymentAgreement => "Employment / Contractor Agreement",
            ContractType::Other => "General Commercial Agreement",
        }
    }
}

/// Standard Legal Clause Categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClauseCategory {
    LimitationOfLiability,
    Indemnification,
    IpOwnership,
    TerminationForConvenience,
    NonCompeteNonSolicit,
    GoverningLawArbitration,
    DataProtectionBreachNotification,
    WarrantyDisclaimer,
    Confidentiality,
    General,
}

impl ClauseCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            ClauseCategory::LimitationOfLiability => "Limitation of Liability",
            ClauseCategory::Indemnification => "Indemnification",
            ClauseCategory::IpOwnership => "Intellectual Property Ownership",
            ClauseCategory::TerminationForConvenience => "Termination & Cure Period",
            ClauseCategory::NonCompeteNonSolicit => "Non-Compete & Non-Solicitation",
            ClauseCategory::GoverningLawArbitration => "Governing Law & Dispute Resolution",
            ClauseCategory::DataProtectionBreachNotification => {
                "Data Protection & Breach Notification"
            }
            ClauseCategory::WarrantyDisclaimer => "Warranties & Disclaimers",
            ClauseCategory::Confidentiality => "Confidentiality & Non-Disclosure",
            ClauseCategory::General => "General Provisions",
        }
    }
}

/// Severity Rating for Legal Contract Risks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Favorable,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Critical => "CRITICAL",
            RiskLevel::High => "HIGH",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::Low => "LOW",
            RiskLevel::Favorable => "FAVORABLE",
        }
    }
}

/// Single Evaluated Contract Clause Finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClauseAuditFinding {
    pub clause_id: String,
    pub section_number: String,
    pub heading: String,
    pub raw_text: String,
    pub category: ClauseCategory,
    pub risk_level: RiskLevel,
    pub risk_score: f32, // 0.0 (safest) to 10.0 (maximum liability)
    pub rationale: String,
    pub market_standard_benchmark: String,
    pub suggested_redline_text: Option<String>,
}

/// Full Parsed & Analyzed Contract Document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDocument {
    pub document_id: String,
    pub title: String,
    pub contract_type: ContractType,
    pub raw_text: String,
    pub clauses: Vec<ClauseAuditFinding>,
    pub overall_risk_score: f32,
    pub risk_grade: String, // "A (Low Risk)", "B (Moderate Risk)", "C (Elevated Risk)", "F (High Risk)"
    pub uploaded_at_epoch_ms: i64,
}

/// Lightweight Document Summary for Navigation List
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSummary {
    pub document_id: String,
    pub title: String,
    pub contract_type: ContractType,
    pub total_clauses: usize,
    pub overall_risk_score: f32,
    pub risk_grade: String,
    pub critical_count: usize,
    pub high_count: usize,
    pub uploaded_at_epoch_ms: i64,
}

/// Itemized Clause Diff between Contract Versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClauseDiffItem {
    pub section_number: String,
    pub category: ClauseCategory,
    pub base_text: String,
    pub revised_text: String,
    pub risk_delta: f32,
    pub summary: String,
}

/// Contract Version Comparison Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractComparisonDiff {
    pub base_document_id: String,
    pub revised_document_id: String,
    pub changed_clauses_count: usize,
    pub overall_risk_delta: f32,
    pub diff_items: Vec<ClauseDiffItem>,
}

/// Cryptographically Signed Legal Audit Certificate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalAuditCertificate {
    pub certificate_id: String,
    pub document_id: String,
    pub document_title: String,
    pub document_blake3_hash: String,
    pub total_clauses: usize,
    pub critical_findings_count: usize,
    pub high_findings_count: usize,
    pub overall_risk_grade: String,
    pub issued_at_epoch_ms: i64,
    pub ed25519_signature_hex: String,
    pub auditor_public_key_hex: String,
}

/// Executive Workbench Metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditorStats {
    pub total_contracts_audited: usize,
    pub total_clauses_analyzed: usize,
    pub critical_risks_prevented: usize,
    pub avg_risk_score: f32,
}
