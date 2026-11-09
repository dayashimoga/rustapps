//! Service Layer for Offline Local RAG Contract & Legal Compliance Auditor.
//!
//! Manages repository of analyzed contracts, version comparisons,
//! and cryptographic audit certificate signing.

use crate::models::{
    AuditorStats, ContractComparisonDiff, ContractDocument, ContractSummary, ContractType,
    LegalAuditCertificate, RiskLevel,
};
use crate::rag_engine::RagEngine;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct ContractService {
    contracts: RwLock<HashMap<String, ContractDocument>>,
    signing_key: SigningKey,
}

impl Default for ContractService {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractService {
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        let service = Self {
            contracts: RwLock::new(HashMap::new()),
            signing_key,
        };

        service.seed_sample_contracts();
        service
    }

    fn seed_sample_contracts(&self) {
        let vendor_draft = r#"Section 1.0 Scope and Definitions
This Master Services Agreement ("Agreement") governs access to and use of the Apex Distributed Cloud Platform. "Customer" means the enterprise entity subscribing to the Services.

Section 2.0 Cloud Services and Availability
Vendor shall provide the Services with commercially reasonable efforts targeting ninety-nine point five percent (99.5%) platform availability during normal business hours, excluding scheduled maintenance.

Section 3.0 Proprietary Rights and Work Product
Customer acknowledges and agrees that Vendor retains all right, title and interest in work product, inventions, customizations, and all analytical derivatives derived from Customer Data processed on the platform.

Section 4.0 Fees, Payment and Taxes
Customer shall pay all undisputed invoiced fees within thirty (30) days of receipt. Late payments shall accrue interest at the rate of one point five percent (1.5%) per month.

Section 5.0 Confidentiality Obligations
Each party agrees to safeguard the Confidential Information of the other party using the same degree of care it uses for its own proprietary information of like character, but not less than reasonable care.

Section 6.0 Warranties and Total As-Is Disclaimer
THE SERVICES ARE PROVIDED ENTIRELY "AS IS" AND WITHOUT WARRANTY OF ANY KIND. VENDOR DISCLAIMS ALL IMPLIED WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, AND NON-INFRINGEMENT.

Section 7.0 Indemnification Obligations
Customer shall indemnify, defend, and hold harmless Vendor and its affiliates from and against any third-party claims, liabilities, damages, and legal expenses arising out of Customer's use of the platform.

Section 8.0 Limitation of Liability
VENDOR'S AGGREGATE LIABILITY ARISING UNDER THIS AGREEMENT SHALL BE STRICTLY LIMITED TO ONE HUNDRED DOLLARS ($100.00). CUSTOMER'S LIABILITY UNDER THIS AGREEMENT SHALL BE UNLIMITED AND SHALL NOT BE LIMITED TO FEES PAID.

Section 9.0 Data Protection and Incident Notification
Vendor shall maintain standard administrative safeguards. In the event of an unauthorized security breach affecting Customer records, Vendor shall provide written notification to Customer within thirty (30) business days.

Section 10.0 Term and Termination for Convenience
Vendor may terminate this Agreement immediately without cause upon written notice. Customer shall forfeit all prepaid annual subscription fees upon any early termination.

Section 11.0 Restrictive Covenants and Non-Compete
During the term of this Agreement and for a period of two (2) years thereafter, Customer shall not engage in any competing cloud infrastructure software business.

Section 12.0 Governing Law and Dispute Resolution
This Agreement shall be construed under the laws of the State of Delaware, without regard to conflict of laws principles. Either party may seek equitable relief in state or federal court."#;

        let clean_playbook = r#"Section 1.0 Scope and Definitions
This Master Services Agreement ("Agreement") governs access to and use of the Apex Distributed Cloud Platform. "Customer" means the enterprise entity subscribing to the Services.

Section 2.0 Cloud Services and Availability
Vendor warrants ninety-nine point nine percent (99.9%) platform availability backed by service level credits as set forth in the SLA Schedule.

Section 3.0 Proprietary Rights and Customer Data
Customer retains all right, title, and interest, including all Intellectual Property Rights, in and to Customer Data. Any custom deliverables developed specifically for Customer shall constitute work made for hire owned by Customer.

Section 4.0 Fees, Payment and Taxes
Customer shall pay all undisputed invoiced fees within thirty (30) days of receipt. Late payments shall accrue interest at the lesser of one percent (1.0%) per month or the statutory maximum.

Section 5.0 Confidentiality Obligations
Each party agrees to safeguard the Confidential Information of the other party for five (5) years, and indefinitely for Trade Secrets, using the highest standard of institutional care.

Section 6.0 Warranties and Specifications
Vendor warrants that the Services will perform in all material respects in accordance with published documentation and will not contain any malware, backdoors, or malicious code.

Section 7.0 Indemnification Obligations
Vendor shall defend, indemnify, and hold harmless Customer against any third-party claim alleging that the Services or Software infringe any patent, copyright, trademark, or trade secret.

Section 8.0 Limitation of Liability
EACH PARTY'S AGGREGATE LIABILITY ARISING UNDER OR IN CONNECTION WITH THIS AGREEMENT SHALL IN NO EVENT EXCEED THE TOTAL FEES ACTUALLY PAID OR PAYABLE BY CUSTOMER IN THE TWELVE (12) MONTH PERIOD PRECEDING THE EVENT GIVING RISE TO LIABILITY.

Section 9.0 Data Protection and Incident Notification
Vendor shall notify Customer in writing without undue delay, and in no event later than forty-eight (48) hours, after becoming aware of any confirmed or suspected Security Incident involving Customer Data.

Section 10.0 Term and Termination for Convenience
Either party may terminate this Agreement for convenience upon sixty (60) days prior written notice. Upon termination for convenience by Vendor, Vendor shall promptly refund all prepaid, unearned fees on a pro-rata basis.

Section 11.0 Employee Non-Solicitation
Neither party shall solicit for employment any employee of the other party directly involved in the Services, excluding responses to general public recruitments.

Section 12.0 Governing Law and Dispute Resolution
This Agreement shall be governed by the laws of the State of Delaware. Either party may seek equitable or injunctive relief in any competent court to prevent ongoing irreparable harm."#;

        let doc1 = RagEngine::audit_contract(
            "DOC-APEX-MSA-32",
            "Apex Enterprise Cloud Services Master Agreement v3.2 (Vendor Draft)",
            ContractType::SaaSMasterServicesAgreement,
            vendor_draft,
        );

        let doc2 = RagEngine::audit_contract(
            "DOC-CLEAN-PLAYBOOK-10",
            "Apex Institutional SaaS Clean Baseline v1.0 (Customer Playbook)",
            ContractType::SaaSMasterServicesAgreement,
            clean_playbook,
        );

        if let Ok(mut lock) = self.contracts.write() {
            lock.insert(doc1.document_id.clone(), doc1);
            lock.insert(doc2.document_id.clone(), doc2);
        }
    }

    /// Evaluates metrics across all stored contracts
    pub fn get_stats(&self) -> AuditorStats {
        let lock = match self.contracts.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let total_contracts = lock.len();
        let mut total_clauses = 0;
        let mut total_risk = 0.0f32;
        let mut crit_prevented = 0;

        for doc in lock.values() {
            total_clauses += doc.clauses.len();
            total_risk += doc.overall_risk_score;
            for c in &doc.clauses {
                if c.risk_level == RiskLevel::Critical {
                    crit_prevented += 1;
                }
            }
        }

        let avg_risk = if total_contracts > 0 {
            (total_risk / total_contracts as f32 * 10.0).round() / 10.0
        } else {
            0.0
        };

        AuditorStats {
            total_contracts_audited: total_contracts,
            total_clauses_analyzed: total_clauses,
            critical_risks_prevented: crit_prevented,
            avg_risk_score: avg_risk,
        }
    }

    /// Returns list of summaries for all loaded contracts
    pub fn list_contracts(&self) -> Vec<ContractSummary> {
        let lock = match self.contracts.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let mut list: Vec<ContractSummary> = lock
            .values()
            .map(|doc| {
                let crit = doc
                    .clauses
                    .iter()
                    .filter(|c| c.risk_level == RiskLevel::Critical)
                    .count();
                let high = doc
                    .clauses
                    .iter()
                    .filter(|c| c.risk_level == RiskLevel::High)
                    .count();
                ContractSummary {
                    document_id: doc.document_id.clone(),
                    title: doc.title.clone(),
                    contract_type: doc.contract_type,
                    total_clauses: doc.clauses.len(),
                    overall_risk_score: doc.overall_risk_score,
                    risk_grade: doc.risk_grade.clone(),
                    critical_count: crit,
                    high_count: high,
                    uploaded_at_epoch_ms: doc.uploaded_at_epoch_ms,
                }
            })
            .collect();

        list.sort_by_key(|b| std::cmp::Reverse(b.uploaded_at_epoch_ms));
        list
    }

    /// Retrieves full parsed contract by ID
    pub fn get_contract(&self, document_id: &str) -> Option<ContractDocument> {
        let lock = match self.contracts.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if document_id.is_empty() {
            // Return first contract if available
            return lock.values().next().cloned();
        }

        lock.get(document_id).cloned()
    }

    /// Audits, scores, and stores a new contract document
    pub fn audit_contract_text(
        &self,
        title: &str,
        contract_type: ContractType,
        raw_text: &str,
    ) -> ContractDocument {
        let doc_id = format!(
            "DOC-{}",
            blake3::hash(format!("{}:{}", title, raw_text).as_bytes()).to_hex()[0..10]
                .to_uppercase()
        );

        let doc = RagEngine::audit_contract(&doc_id, title, contract_type, raw_text);

        if let Ok(mut lock) = self.contracts.write() {
            lock.insert(doc.document_id.clone(), doc.clone());
        }

        doc
    }

    /// Compares two contracts and calculates clause risk deltas
    pub fn compare_contracts(
        &self,
        base_id: &str,
        revised_id: &str,
    ) -> Result<ContractComparisonDiff, String> {
        let lock = match self.contracts.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let base = lock
            .get(base_id)
            .ok_or_else(|| format!("Base contract '{}' not found", base_id))?;
        let revised = lock
            .get(revised_id)
            .ok_or_else(|| format!("Revised contract '{}' not found", revised_id))?;

        Ok(RagEngine::compare_contracts(base, revised))
    }

    /// Generates Ed25519-signed Legal Audit Certificate
    pub fn sign_audit_certificate(
        &self,
        document_id: &str,
    ) -> Result<LegalAuditCertificate, String> {
        let lock = match self.contracts.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let doc = lock
            .get(document_id)
            .ok_or_else(|| format!("Contract '{}' not found", document_id))?;

        Ok(RagEngine::sign_audit_certificate(doc, &self.signing_key))
    }
}
