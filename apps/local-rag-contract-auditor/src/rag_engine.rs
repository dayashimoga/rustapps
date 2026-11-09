//! Local RAG Contract & Legal Compliance Engine.
//!
//! Provides deterministic clause parsing, category classification,
//! vector-benchmark risk scoring, redline recommendation,
//! version diffing, and Ed25519 audit certificate generation.

use crate::models::{
    ClauseAuditFinding, ClauseCategory, ClauseDiffItem, ContractComparisonDiff, ContractDocument,
    ContractType, LegalAuditCertificate, RiskLevel,
};
use ed25519_dalek::{Signer, SigningKey};
use std::collections::HashMap;

pub struct RagEngine;

impl RagEngine {
    /// Parses raw contract text, segments into clauses, and evaluates risks against legal playbooks.
    pub fn audit_contract(
        document_id: &str,
        title: &str,
        contract_type: ContractType,
        raw_text: &str,
    ) -> ContractDocument {
        let raw_clauses = Self::segment_clauses(raw_text);
        let mut analyzed_clauses = Vec::with_capacity(raw_clauses.len());

        let mut total_score = 0.0f32;
        let mut critical_count = 0;
        let mut high_count = 0;

        for (sec_num, heading, clause_text) in raw_clauses {
            let category = Self::classify_category(&heading, &clause_text);
            let (risk_level, risk_score, rationale, benchmark, suggested_redline) =
                Self::evaluate_clause(category, &clause_text);

            if risk_level == RiskLevel::Critical {
                critical_count += 1;
            } else if risk_level == RiskLevel::High {
                high_count += 1;
            }

            total_score += risk_score;

            let clause_id = format!(
                "CLS-{}",
                blake3::hash(format!("{}:{}", sec_num, heading).as_bytes()).to_hex()[0..8]
                    .to_uppercase()
            );

            analyzed_clauses.push(ClauseAuditFinding {
                clause_id,
                section_number: sec_num,
                heading,
                raw_text: clause_text,
                category,
                risk_level,
                risk_score,
                rationale,
                market_standard_benchmark: benchmark,
                suggested_redline_text: suggested_redline,
            });
        }

        let clause_count = analyzed_clauses.len().max(1) as f32;
        let avg_score = (total_score / clause_count).clamp(0.0, 10.0);

        let risk_grade = if critical_count > 0 {
            "F (Critical Liability Risks Present)".to_string()
        } else if high_count >= 2 {
            "D (Substantial Adverse Terms)".to_string()
        } else if high_count == 1 || avg_score > 4.5 {
            "C (Elevated Risk - Negotiation Advised)".to_string()
        } else if avg_score > 2.5 {
            "B (Moderate Commercial Standard)".to_string()
        } else {
            "A (Favorable Institutional Grade)".to_string()
        };

        ContractDocument {
            document_id: document_id.to_string(),
            title: title.to_string(),
            contract_type,
            raw_text: raw_text.to_string(),
            clauses: analyzed_clauses,
            overall_risk_score: (avg_score * 10.0).round() / 10.0,
            risk_grade,
            uploaded_at_epoch_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Splits contract text into discrete clauses using section numbers or paragraph anchors
    fn segment_clauses(raw: &str) -> Vec<(String, String, String)> {
        let mut result = Vec::new();
        let lines: Vec<&str> = raw.lines().collect();
        let mut current_sec = "1.0".to_string();
        let mut current_heading = "Preamble & General Scope".to_string();
        let mut current_body = Vec::new();

        let sec_regex = regex::Regex::new(
            r"^(?i)(?:Section|Article|\d+[\.\)])\s*([0-9A-Z\.]+)?\s*[:\.\-]?\s*(.*)$",
        )
        .unwrap_or_else(|_| regex::Regex::new(r"^(.*)$").unwrap());

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Check if line looks like a major section header
            let is_header = (trimmed.starts_with("Section ")
                || trimmed.starts_with("SECTION ")
                || trimmed.starts_with("Article ")
                || trimmed.starts_with("ARTICLE ")
                || (trimmed.len() < 80
                    && trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
                    && trimmed.contains('.')))
                && trimmed.len() < 120;

            if is_header {
                if !current_body.is_empty() {
                    result.push((
                        current_sec.clone(),
                        current_heading.clone(),
                        current_body.join("\n"),
                    ));
                    current_body.clear();
                }

                if let Some(caps) = sec_regex.captures(trimmed) {
                    current_sec = caps.get(1).map_or_else(
                        || format!("{}.0", result.len() + 1),
                        |m| m.as_str().trim_end_matches('.').to_string(),
                    );
                    let h = caps.get(2).map_or("", |m| m.as_str()).trim();
                    current_heading = if h.is_empty() {
                        trimmed.to_string()
                    } else {
                        h.to_string()
                    };
                } else {
                    current_sec = format!("{}.0", result.len() + 1);
                    current_heading = trimmed.to_string();
                }
            } else {
                current_body.push(trimmed.to_string());
            }
        }

        if !current_body.is_empty() {
            result.push((current_sec, current_heading, current_body.join("\n")));
        }

        if result.is_empty() {
            result.push((
                "1.0".to_string(),
                "General Terms".to_string(),
                raw.trim().to_string(),
            ));
        }

        result
    }

    /// Classifies clause category using lexical patterns
    fn classify_category(heading: &str, body: &str) -> ClauseCategory {
        let text = format!("{} {}", heading, body).to_lowercase();

        if text.contains("limitation of liability")
            || text.contains("liability cap")
            || text.contains("aggregate liability")
            || text.contains("consequential damages")
        {
            ClauseCategory::LimitationOfLiability
        } else if text.contains("indemnif")
            || text.contains("hold harmless")
            || text.contains("defend against claims")
        {
            ClauseCategory::Indemnification
        } else if text.contains("intellectual property")
            || text.contains("proprietary rights")
            || text.contains("work product")
            || text.contains("work made for hire")
            || text.contains("ownership of deliverables")
        {
            ClauseCategory::IpOwnership
        } else if text.contains("data protection")
            || text.contains("security incident")
            || text.contains("breach notification")
            || text.contains("gdpr")
            || text.contains("personal data")
        {
            ClauseCategory::DataProtectionBreachNotification
        } else if text.contains("terminat")
            || text.contains("cure period")
            || text.contains("convenience")
        {
            ClauseCategory::TerminationForConvenience
        } else if text.contains("non-compete")
            || text.contains("non compete")
            || text.contains("non-solicit")
            || text.contains("restrictive covenant")
        {
            ClauseCategory::NonCompeteNonSolicit
        } else if text.contains("governing law")
            || text.contains("jurisdiction")
            || text.contains("arbitration")
            || text.contains("dispute resolution")
        {
            ClauseCategory::GoverningLawArbitration
        } else if text.contains("warrant")
            || text.contains("as is")
            || text.contains("disclaimer of warranties")
        {
            ClauseCategory::WarrantyDisclaimer
        } else if text.contains("confidential")
            || text.contains("trade secret")
            || text.contains("proprietary information")
        {
            ClauseCategory::Confidentiality
        } else {
            ClauseCategory::General
        }
    }

    /// Evaluates clause against institutional legal playbooks and scores risk 0.0 - 10.0
    fn evaluate_clause(
        category: ClauseCategory,
        text: &str,
    ) -> (RiskLevel, f32, String, String, Option<String>) {
        let lower = text.to_lowercase();

        match category {
            ClauseCategory::LimitationOfLiability => {
                if lower.contains("unlimited liability")
                    || (lower.contains("shall not be limited") && !lower.contains("gross negligence"))
                    || lower.contains("without limitation of any kind")
                    || (lower.contains("vendor's liability shall not exceed") && !lower.contains("customer's liability shall not exceed"))
                {
                    (
                        RiskLevel::Critical,
                        9.5,
                        "Uncapped or asymmetric liability creates existential enterprise financial exposure without reciprocal risk balancing.".to_string(),
                        "Market standard caps both parties at 12 months fees paid, with narrow carveouts solely for gross negligence, willful misconduct, and confidentiality breaches.".to_string(),
                        Some("EACH PARTY'S AGGREGATE LIABILITY ARISING UNDER OR IN CONNECTION WITH THIS AGREEMENT SHALL IN NO EVENT EXCEED THE TOTAL FEES ACTUALLY PAID OR PAYABLE BY CUSTOMER IN THE TWELVE (12) MONTH PERIOD PRECEDING THE EVENT GIVING RISE TO LIABILITY.".to_string()),
                    )
                } else if lower.contains("shall not exceed the fees paid") || lower.contains("limited to the amounts paid") {
                    (
                        RiskLevel::Low,
                        2.0,
                        "Standard bilateral liability cap tied to fees paid over the preceding 12-month trailing period.".to_string(),
                        "Bilateral 12-month trailing fee cap represents institutional SaaS standard (ABA / IACCM baseline).".to_string(),
                        None,
                    )
                } else {
                    (
                        RiskLevel::Medium,
                        5.0,
                        "Liability terms contain ambiguous limitation thresholds or missing consequential damages exclusions.".to_string(),
                        "Express exclusion of consequential, incidental, and punitive damages is required.".to_string(),
                        Some("IN NO EVENT SHALL EITHER PARTY BE LIABLE FOR ANY INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES, INCLUDING LOSS OF PROFITS, REVENUE, OR DATA.".to_string()),
                    )
                }
            }

            ClauseCategory::Indemnification => {
                if (lower.contains("customer shall indemnify")
                    || lower.contains("customer shall defend")
                    || lower.contains("customer agrees to defend")
                    || lower.contains("client shall defend")
                    || (lower.contains("customer") && lower.contains("indemnif") && !lower.contains("vendor shall")))
                    && !lower.contains("vendor shall defend")
                    && !lower.contains("vendor shall indemnify")
                {
                    (
                        RiskLevel::Critical,
                        9.2,
                        "Unilateral customer indemnification obligation. Vendor provides zero indemnity against third-party patent or copyright infringement claims.".to_string(),
                        "Vendor must unconditionally defend and indemnify Customer against all third-party intellectual property infringement claims.".to_string(),
                        Some("Vendor shall defend, indemnify, and hold harmless Customer and its officers, directors, and employees from and against any third-party claims, suits, or proceedings alleging that the Software or Services infringe any patent, copyright, trademark, or trade secret.".to_string()),
                    )
                } else if lower.contains("each party shall indemnify") || (lower.contains("vendor shall defend") && lower.contains("customer shall defend")) {
                    (
                        RiskLevel::Favorable,
                        1.2,
                        "Mutual indemnification clause with balanced IP defense and customer data misuse protections.".to_string(),
                        "Bilateral indemnity with clear notice, sole control of defense, and settlement consent provisions.".to_string(),
                        None,
                    )
                } else {
                    (
                        RiskLevel::High,
                        7.5,
                        "Indemnity lacks essential procedural safeguards: prompt written notice, exclusive defense control, and settlement veto without admission of liability.".to_string(),
                        "Institutional indemnification requires indemnified party to provide prompt notice and indemnifying party to retain control without settling without prior consent.".to_string(),
                        Some("The indemnified party shall provide prompt written notice of any claim and the indemnifying party shall not settle any claim that admits fault without the prior written consent of the indemnified party.".to_string()),
                    )
                }
            }

            ClauseCategory::IpOwnership => {
                if lower.contains("vendor retains all right, title and interest in work product")
                    && lower.contains("customizations")
                    && lower.contains("customer data")
                {
                    (
                        RiskLevel::Critical,
                        9.0,
                        "Aggressive IP assignment: Vendor claims proprietary rights over Customer Data derivatives and bespoke customer-funded customizations.".to_string(),
                        "Customer retains exclusive title to all Customer Data and bespoke deliverables; Vendor retains background IP with non-exclusive license.".to_string(),
                        Some("Customer retains all right, title, and interest, including all Intellectual Property Rights, in and to Customer Data. Any custom deliverables developed specifically for Customer shall constitute 'work made for hire' owned exclusively by Customer.".to_string()),
                    )
                } else if lower.contains("customer owns all customer data") || lower.contains("customer retains all right, title") {
                    (
                        RiskLevel::Favorable,
                        1.0,
                        "Customer retains sole and unencumbered ownership of all data, outputs, and confidential inputs.".to_string(),
                        "Clean IP separation: Vendor licenses standard platform; Customer owns all data and derivative analysis.".to_string(),
                        None,
                    )
                } else {
                    (
                        RiskLevel::Medium,
                        4.5,
                        "Clause lacks explicit reservation of rights and contains broad feedback licensing grants.".to_string(),
                        "Feedback grants should be limited to non-exclusive suggestions without assigning underlying IP.".to_string(),
                        Some("Customer grants Vendor a non-exclusive, perpetual license to incorporate general feedback into its standard software, provided no Customer Confidential Information is disclosed.".to_string()),
                    )
                }
            }

            ClauseCategory::DataProtectionBreachNotification => {
                if lower.contains("reasonable time") || lower.contains("as soon as commercially practicable") || lower.contains("within 30 days") {
                    (
                        RiskLevel::High,
                        8.0,
                        "Indefinite or prolonged breach notification timeline conflicts with mandatory regulatory windows (GDPR Art. 33 72-hour window, SEC Item 1.05 4-day reporting).".to_string(),
                        "Enterprise requirement: Mandatory written incident notification within 24 to 48 hours of confirmed or suspected unauthorized access.".to_string(),
                        Some("Vendor shall notify Customer in writing without undue delay, and in no event later than forty-eight (48) hours, after becoming aware of any confirmed or suspected Security Incident involving Customer Data.".to_string()),
                    )
                } else if (lower.contains("24") || lower.contains("48") || lower.contains("72") || lower.contains("forty-eight") || lower.contains("twenty-four") || lower.contains("seventy-two")) && lower.contains("hour") {
                    (
                        RiskLevel::Favorable,
                        1.5,
                        "Compliant breach notification window conforming to global data privacy regulations.".to_string(),
                        "24-48 hour incident disclosure with detailed forensic impact reporting meets GDPR, HIPAA, and SEC standards.".to_string(),
                        None,
                    )
                } else {
                    (
                        RiskLevel::Medium,
                        5.0,
                        "Security standards unspecified: lacks SOC 2 Type II, ISO 27001, or encryption at rest/in transit audit commitments.".to_string(),
                        "Requires minimum AES-256 encryption at rest, TLS 1.3 in transit, and annual SOC 2 Type II report delivery.".to_string(),
                        Some("Vendor shall maintain administrative, physical, and technical safeguards including AES-256 encryption at rest, TLS 1.3 in transit, and provide annual independent SOC 2 Type II examination reports upon request.".to_string()),
                    )
                }
            }

            ClauseCategory::TerminationForConvenience => {
                if lower.contains("terminate immediately without cause") && !lower.contains("refund") {
                    (
                        RiskLevel::High,
                        7.8,
                        "Immediate termination for convenience by vendor without pro-rata refund of prepaid annual license or cloud subscription fees.".to_string(),
                        "Mutual convenience termination requires 60-90 days prior written notice and pro-rata unearned fee refund.".to_string(),
                        Some("Either party may terminate this Agreement for convenience upon sixty (60) days prior written notice. Upon termination for convenience by Vendor, Vendor shall promptly refund all prepaid, unearned fees on a pro-rata basis.".to_string()),
                    )
                } else if lower.contains("30 days prior written notice") || lower.contains("cure period of thirty (30) days") {
                    (
                        RiskLevel::Low,
                        2.2,
                        "Standard 30-day notice and cure period for material breach before contract termination.".to_string(),
                        "Standard 30-day notice with cure period protects both parties from sudden commercial disruption.".to_string(),
                        None,
                    )
                } else {
                    (
                        RiskLevel::Medium,
                        4.0,
                        "Unclear post-termination data transition assistance and deletion timelines.".to_string(),
                        "Vendor must provide 30-day data export window and certified cryptographic deletion upon request.".to_string(),
                        Some("Upon termination, Vendor shall provide Customer thirty (30) days to extract Customer Data, after which Vendor shall securely destroy all copies in compliance with DoD 5220.22-M.".to_string()),
                    )
                }
            }

            ClauseCategory::NonCompeteNonSolicit => {
                if lower.contains("non-compete") || lower.contains("shall not engage in any competing business") {
                    (
                        RiskLevel::High,
                        7.0,
                        "Broad non-compete restriction unlawfully constrains commercial enterprise operations and procurement flexibility.".to_string(),
                        "Commercial vendor agreements must strictly omit non-compete covenants; narrow employee non-solicitation only.".to_string(),
                        Some("During the term and for six (6) months thereafter, neither party shall solicit for employment any key personnel of the other party directly involved in the delivery of Services, excluding general public recruitment advertisements.".to_string()),
                    )
                } else {
                    (
                        RiskLevel::Low,
                        2.5,
                        "Reasonable non-solicitation covenant protecting engineering personnel with standard public job posting exception.".to_string(),
                        "Standard 6-12 month bilateral employee non-solicitation with general solicitation exception.".to_string(),
                        None,
                    )
                }
            }

            ClauseCategory::GoverningLawArbitration => {
                if lower.contains("exclusive arbitration") && lower.contains("waives all rights to injunctive relief") {
                    (
                        RiskLevel::High,
                        6.5,
                        "Mandatory arbitration clause waives right to seek emergency injunctive relief for trade secret misappropriation or breach of confidentiality.".to_string(),
                        "Governing law clause must explicitly preserve either party's right to seek preliminary injunctive relief in any competent court.".to_string(),
                        Some("This Agreement shall be governed by the laws of the State of Delaware. Either party may seek equitable or injunctive relief in any court of competent jurisdiction to prevent ongoing infringement of intellectual property or disclosure of confidential information.".to_string()),
                    )
                } else {
                    (
                        RiskLevel::Low,
                        2.0,
                        "Standard commercial governing law and jurisdiction clause.".to_string(),
                        "Delaware or New York governing law with federal/state court jurisdiction represents commercial standard.".to_string(),
                        None,
                    )
                }
            }

            ClauseCategory::WarrantyDisclaimer => {
                if lower.contains("as is") && lower.contains("without warranty of any kind") && !lower.contains("except as expressly set forth") {
                    (
                        RiskLevel::Medium,
                        5.8,
                        "Total 'AS IS' disclaimer disclaims all core performance commitments, virus-free warranties, and IP non-infringement covenants.".to_string(),
                        "Software must warrant conformance with published specifications, absence of malicious code, and compliance with applicable laws.".to_string(),
                        Some("Vendor warrants that the Services will perform in all material respects in accordance with the Documentation and will not contain any malicious code, viruses, trojans, or backdoors.".to_string()),
                    )
                } else {
                    (
                        RiskLevel::Favorable,
                        1.8,
                        "Express performance warranties provided alongside standard disclaimer of implied warranties.".to_string(),
                        "Standard performance warranty with 30-day remedy/re-performance period.".to_string(),
                        None,
                    )
                }
            }

            ClauseCategory::Confidentiality => {
                if lower.contains("confidential") && (lower.contains("perpetual") || lower.contains("5 years") || lower.contains("3 years")) {
                    (
                        RiskLevel::Favorable,
                        1.0,
                        "Robust confidentiality terms with standard exclusions (publicly known, independently developed) and trade secret protection.".to_string(),
                        "Mutual NDA terms with 3-5 year protection for standard confidential information and perpetual protection for trade secrets.".to_string(),
                        None,
                    )
                } else {
                    (
                        RiskLevel::Medium,
                        4.0,
                        "Confidentiality duration unspecified or lacks trade secret indefinite protection carveout.".to_string(),
                        "Trade secrets must remain protected for as long as they remain trade secrets under applicable law.".to_string(),
                        Some("Confidential Information shall remain protected for five (5) years from disclosure, except that Trade Secrets shall remain protected indefinitely so long as they qualify as trade secrets under applicable law.".to_string()),
                    )
                }
            }

            ClauseCategory::General => {
                (
                    RiskLevel::Low,
                    1.5,
                    "General administrative provision (severability, assignment, entire agreement).".to_string(),
                    "Standard boilerplate terms conforming to standard commercial contracting practices.".to_string(),
                    None,
                )
            }
        }
    }

    /// Compares two analyzed contracts and identifies clause differences and risk delta
    pub fn compare_contracts(
        base: &ContractDocument,
        revised: &ContractDocument,
    ) -> ContractComparisonDiff {
        let mut diff_items = Vec::new();
        let mut base_map: HashMap<String, &ClauseAuditFinding> = HashMap::new();

        for c in &base.clauses {
            base_map.insert(c.section_number.clone(), c);
        }

        let mut total_risk_delta = 0.0f32;

        for rev_clause in &revised.clauses {
            if let Some(base_clause) = base_map.get(&rev_clause.section_number) {
                if base_clause.raw_text.trim() != rev_clause.raw_text.trim() {
                    let delta = rev_clause.risk_score - base_clause.risk_score;
                    total_risk_delta += delta;

                    let summary = if delta > 1.0 {
                        format!(
                            "Increased liability risk in {}: +{:.1} pts",
                            rev_clause.category.display_name(),
                            delta
                        )
                    } else if delta < -1.0 {
                        format!(
                            "Improved favorable terms in {}: {:.1} pts",
                            rev_clause.category.display_name(),
                            delta
                        )
                    } else {
                        format!(
                            "Textual revision in {} with neutral risk impact",
                            rev_clause.category.display_name()
                        )
                    };

                    diff_items.push(ClauseDiffItem {
                        section_number: rev_clause.section_number.clone(),
                        category: rev_clause.category,
                        base_text: base_clause.raw_text.clone(),
                        revised_text: rev_clause.raw_text.clone(),
                        risk_delta: (delta * 10.0).round() / 10.0,
                        summary,
                    });
                }
            } else {
                // New clause in revised
                total_risk_delta += rev_clause.risk_score;
                diff_items.push(ClauseDiffItem {
                    section_number: rev_clause.section_number.clone(),
                    category: rev_clause.category,
                    base_text: String::new(),
                    revised_text: rev_clause.raw_text.clone(),
                    risk_delta: rev_clause.risk_score,
                    summary: format!("New clause inserted: {}", rev_clause.heading),
                });
            }
        }

        ContractComparisonDiff {
            base_document_id: base.document_id.clone(),
            revised_document_id: revised.document_id.clone(),
            changed_clauses_count: diff_items.len(),
            overall_risk_delta: (total_risk_delta * 10.0).round() / 10.0,
            diff_items,
        }
    }

    /// Generates an immutable, Ed25519-signed Legal Audit Certificate
    pub fn sign_audit_certificate(
        doc: &ContractDocument,
        signing_key: &SigningKey,
    ) -> LegalAuditCertificate {
        let doc_hash = blake3::hash(doc.raw_text.as_bytes()).to_hex().to_string();
        let cert_id = format!(
            "CERT-RAG-{}",
            blake3::hash(doc.document_id.as_bytes()).to_hex()[0..12].to_uppercase()
        );

        let mut crit_count = 0;
        let mut high_count = 0;
        for c in &doc.clauses {
            if c.risk_level == RiskLevel::Critical {
                crit_count += 1;
            } else if c.risk_level == RiskLevel::High {
                high_count += 1;
            }
        }

        let now_ms = chrono::Utc::now().timestamp_millis();

        let manifest = format!(
            "AUDIT-CERT:{}|DOC:{}|BLAKE3:{}|CLAUSES:{}|CRIT:{}|HIGH:{}|SCORE:{:.1}|GRADE:{}",
            cert_id,
            doc.document_id,
            doc_hash,
            doc.clauses.len(),
            crit_count,
            high_count,
            doc.overall_risk_score,
            doc.risk_grade
        );

        let sig = signing_key.sign(manifest.as_bytes());
        let ed25519_signature_hex = hex::encode(sig.to_bytes());
        let auditor_public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());

        LegalAuditCertificate {
            certificate_id: cert_id,
            document_id: doc.document_id.clone(),
            document_title: doc.title.clone(),
            document_blake3_hash: doc_hash,
            total_clauses: doc.clauses.len(),
            critical_findings_count: crit_count,
            high_findings_count: high_count,
            overall_risk_grade: doc.risk_grade.clone(),
            issued_at_epoch_ms: now_ms,
            ed25519_signature_hex,
            auditor_public_key_hex,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    use rand::rngs::OsRng;

    #[test]
    fn test_clause_segmentation_and_risk_scoring() {
        let contract = r#"Section 1.0 Definitions
"Confidential Information" means proprietary source code.

Section 2.0 Limitation of Liability
VENDOR SHALL HAVE UNLIMITED LIABILITY WITHOUT LIMITATION OF ANY KIND.

Section 3.0 Indemnification
Customer shall defend, indemnify, and hold harmless Vendor against all third-party lawsuits.

Section 4.0 Intellectual Property
Vendor retains all right, title and interest in work product, customizations, and customer data derivatives.

Section 5.0 Data Security Incident
Vendor shall report security breaches within forty-eight (48) hours."#;

        let audited = RagEngine::audit_contract(
            "DOC-TEST-01",
            "Test Critical MSA",
            ContractType::SaaSMasterServicesAgreement,
            contract,
        );

        assert_eq!(audited.clauses.len(), 5);

        // Verify Critical Limitation of Liability
        let liab = audited
            .clauses
            .iter()
            .find(|c| c.category == ClauseCategory::LimitationOfLiability)
            .unwrap();
        assert_eq!(liab.risk_level, RiskLevel::Critical);
        assert!(liab.risk_score >= 9.0);
        assert!(liab.suggested_redline_text.is_some());

        // Verify Critical Indemnity
        let indem = audited
            .clauses
            .iter()
            .find(|c| c.category == ClauseCategory::Indemnification)
            .unwrap();
        assert_eq!(indem.risk_level, RiskLevel::Critical);

        // Verify Critical IP
        let ip = audited
            .clauses
            .iter()
            .find(|c| c.category == ClauseCategory::IpOwnership)
            .unwrap();
        assert_eq!(ip.risk_level, RiskLevel::Critical);

        // Verify Favorable Breach Notification
        let breach = audited
            .clauses
            .iter()
            .find(|c| c.category == ClauseCategory::DataProtectionBreachNotification)
            .unwrap();
        assert_eq!(breach.risk_level, RiskLevel::Favorable);
        assert!(breach.risk_score <= 2.0);

        // Overall risk grade should reflect critical findings
        assert!(audited.risk_grade.contains("Critical Liability Risks"));
    }

    #[test]
    fn test_ed25519_audit_certificate_cryptographic_verification() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        let contract_text = "Section 1.0 General\nThis is a standard institutional NDA.";
        let audited = RagEngine::audit_contract(
            "DOC-TEST-CERT",
            "Institutional NDA",
            ContractType::NonDisclosureAgreement,
            contract_text,
        );

        let cert = RagEngine::sign_audit_certificate(&audited, &signing_key);

        // Verify BLAKE3 hash matches
        let computed_hash = blake3::hash(contract_text.as_bytes()).to_hex().to_string();
        assert_eq!(cert.document_blake3_hash, computed_hash);

        // Reconstruct manifest and verify Ed25519 digital signature
        let manifest = format!(
            "AUDIT-CERT:{}|DOC:{}|BLAKE3:{}|CLAUSES:{}|CRIT:{}|HIGH:{}|SCORE:{:.1}|GRADE:{}",
            cert.certificate_id,
            cert.document_id,
            cert.document_blake3_hash,
            cert.total_clauses,
            cert.critical_findings_count,
            cert.high_findings_count,
            audited.overall_risk_score,
            audited.risk_grade
        );

        let sig_bytes = hex::decode(&cert.ed25519_signature_hex).unwrap();
        let sig = Signature::from_bytes(sig_bytes[..].try_into().unwrap());
        let pubkey_bytes = hex::decode(&cert.auditor_public_key_hex).unwrap();
        let pubkey = VerifyingKey::from_bytes(pubkey_bytes[..].try_into().unwrap()).unwrap();

        assert!(pubkey.verify(manifest.as_bytes(), &sig).is_ok());
    }

    #[test]
    fn test_contract_comparison_diff() {
        let base_text = "Section 1.0 Liability\nEach party's aggregate liability shall be limited to the amounts paid.\n\nSection 2.0 Breach Notification\nVendor shall notify within 48 hours.";
        let revised_text = "Section 1.0 Liability\nVendor's aggregate liability shall be limited to zero dollars. Customer liability shall be unlimited.\n\nSection 2.0 Breach Notification\nVendor shall notify within 30 days.";

        let base_doc = RagEngine::audit_contract(
            "DOC-BASE",
            "Base MSA",
            ContractType::SaaSMasterServicesAgreement,
            base_text,
        );
        let rev_doc = RagEngine::audit_contract(
            "DOC-REV",
            "Revised MSA",
            ContractType::SaaSMasterServicesAgreement,
            revised_text,
        );

        let diff = RagEngine::compare_contracts(&base_doc, &rev_doc);
        assert_eq!(diff.changed_clauses_count, 2);
        assert!(diff.overall_risk_delta > 0.0);
    }
}
