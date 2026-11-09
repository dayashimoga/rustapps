//! Data Models for Offline Academic & Technical Publishing Press.

use serde::{Deserialize, Serialize};

/// Supported Academic and Enterprise Publishing Layout Templates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentTemplate {
    /// IEEE Standard Two-Column Transactions / Conference format
    IeeeTwoColumn,
    /// ACM SIGCONF Two-Column Computer Science Proceedings format
    AcmSigconf,
    /// Springer Lecture Notes in Computer Science (LNCS) single-column
    SpringerLncs,
    /// arXiv.org Single-Column Open Research Preprint format
    ArxivPreprint,
    /// ISO 9001 / Engineering Standard Operating Procedure (SOP) format
    EnterpriseSop,
    /// Academic Monograph / Technical Book Chapter format
    BookChapter,
}

impl DocumentTemplate {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::IeeeTwoColumn => "IEEE Two-Column Transactions",
            Self::AcmSigconf => "ACM SIGCONF Proceedings",
            Self::SpringerLncs => "Springer LNCS Monograph",
            Self::ArxivPreprint => "arXiv.org Research Preprint",
            Self::EnterpriseSop => "ISO 9001 Enterprise SOP",
            Self::BookChapter => "Academic Book Chapter",
        }
    }
}

/// Output distribution document formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetFormat {
    /// Vector Print-Ready PDF
    PrintPdf,
    /// EPUB3 Reflowable eBook
    Epub3,
    /// Self-Contained Offline HTML5 Bundle with Embedded KaTeX Math & CSS
    StandaloneHtml5,
    /// Native Typst Typesetting Source Markup
    TypstSource,
}

/// Parsed BibTeX Citation Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BibItem {
    pub key: String,
    pub entry_type: String,
    pub title: String,
    pub authors: String,
    pub year: u32,
    pub venue: String,
    pub doi_or_url: Option<String>,
}

/// Document Cross Reference Item (Figure, Table, Equation, Section)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossReference {
    pub ref_id: String,
    pub ref_type: String,
    pub label: String,
    pub seq_number: usize,
}

/// A Managed Publishing Project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishingProject {
    pub project_id: String,
    pub title: String,
    pub authors: String,
    pub abstract_text: String,
    pub template: DocumentTemplate,
    pub target_format: TargetFormat,
    pub source_markdown: String,
    pub bibtex_raw: String,
    pub bibliography: Vec<BibItem>,
    pub created_at_epoch_ms: i64,
    pub updated_at_epoch_ms: i64,
}

/// Project Summary for Navigation List
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub project_id: String,
    pub title: String,
    pub authors: String,
    pub template: DocumentTemplate,
    pub word_count: usize,
    pub citations_count: usize,
    pub updated_at_epoch_ms: i64,
}

/// Typeset Compilation Output Artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationArtifact {
    pub artifact_id: String,
    pub project_id: String,
    pub format: TargetFormat,
    pub template: DocumentTemplate,
    pub rendered_document: String,
    pub page_count: usize,
    pub figure_count: usize,
    pub citation_count: usize,
    pub unresolved_references: Vec<String>,
    pub compile_duration_ms: u64,
    pub document_blake3_hash: String,
}

/// Ed25519-Signed Copyright & Intellectual Property Certificate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyrightCertificate {
    pub certificate_id: String,
    pub project_id: String,
    pub title: String,
    pub primary_author: String,
    pub document_blake3_hash: String,
    pub ed25519_signature_hex: String,
    pub author_public_key_hex: String,
    pub issued_at_epoch_ms: i64,
}

/// Executive Workbench Metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressStats {
    pub total_projects: usize,
    pub total_compilations: usize,
    pub total_pages_rendered: usize,
    pub total_citations_indexed: usize,
}
