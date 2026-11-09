use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SbomFormat {
    CycloneDxJson,
    CycloneDxXml,
    SpdxJson,
    SpdxTagValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentType {
    Library,
    Application,
    Framework,
    Container,
    OperatingSystem,
    Device,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseRisk {
    Permissive,
    WeakCopyleft,
    StrongCopyleft,
    CommercialProprietary,
    Unknown,
}

impl LicenseRisk {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Permissive => "Permissive (MIT/Apache/BSD)",
            Self::WeakCopyleft => "Weak Copyleft (LGPL/MPL)",
            Self::StrongCopyleft => "Strong Copyleft (GPL/AGPL)",
            Self::CommercialProprietary => "Commercial / Proprietary",
            Self::Unknown => "Unknown / Unspecified",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VulnerabilitySeverity {
    Critical,
    High,
    Medium,
    Low,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VulnerabilityAdvisory {
    pub cve_id: String,
    pub package_name: String,
    pub affected_version_range: String,
    pub fixed_version: Option<String>,
    pub severity: VulnerabilitySeverity,
    pub cvss_score: f32,
    pub summary: String,
    pub epss_score: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SbomComponent {
    pub bom_ref: String,
    pub name: String,
    pub version: String,
    pub purl: Option<String>,
    pub component_type: ComponentType,
    pub license_expression: String,
    pub license_risk: LicenseRisk,
    pub vulnerabilities: Vec<VulnerabilityAdvisory>,
    pub dependencies: Vec<String>,
    pub hashes: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SbomDocument {
    pub document_id: String,
    pub filename: String,
    pub format: SbomFormat,
    pub spec_version: String,
    pub serial_number: String,
    pub timestamp: u64,
    pub author: String,
    pub total_components: usize,
    pub components: Vec<SbomComponent>,
    pub ntia_minimum_elements_compliant: bool,
    pub executive_order_14028_compliant: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SbomAuditReport {
    pub report_id: String,
    pub document_id: String,
    pub filename: String,
    pub total_components: usize,
    pub critical_cves: usize,
    pub high_cves: usize,
    pub medium_cves: usize,
    pub low_cves: usize,
    pub copyleft_violations: usize,
    pub overall_risk_score: f32,
    pub compliance_grade: String,
    pub signing_key_id: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditorStats {
    pub total_sboms_audited: usize,
    pub total_components_analyzed: usize,
    pub total_cves_flagged: usize,
    pub total_copyleft_detected: usize,
    pub avg_compliance_score: f32,
}
