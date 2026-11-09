export type ContractType =
  | 'saas_master_services_agreement'
  | 'non_disclosure_agreement'
  | 'data_processing_agreement'
  | 'vendor_procurement'
  | 'employment_agreement'
  | 'other';

export type ClauseCategory =
  | 'limitation_of_liability'
  | 'indemnification'
  | 'ip_ownership'
  | 'termination_for_convenience'
  | 'non_compete_non_solicit'
  | 'governing_law_arbitration'
  | 'data_protection_breach_notification'
  | 'warranty_disclaimer'
  | 'confidentiality'
  | 'general';

export type RiskLevel = 'critical' | 'high' | 'medium' | 'low' | 'favorable';

export interface ClauseAuditFinding {
  clause_id: string;
  section_number: string;
  heading: string;
  raw_text: string;
  category: ClauseCategory;
  risk_level: RiskLevel;
  risk_score: number;
  rationale: string;
  market_standard_benchmark: string;
  suggested_redline_text?: string | null;
}

export interface ContractDocument {
  document_id: string;
  title: string;
  contract_type: ContractType;
  raw_text: string;
  clauses: ClauseAuditFinding[];
  overall_risk_score: number;
  risk_grade: string;
  uploaded_at_epoch_ms: number;
}

export interface ContractSummary {
  document_id: string;
  title: string;
  contract_type: ContractType;
  total_clauses: number;
  overall_risk_score: number;
  risk_grade: string;
  critical_count: number;
  high_count: number;
  uploaded_at_epoch_ms: number;
}

export interface ClauseDiffItem {
  section_number: string;
  category: ClauseCategory;
  base_text: string;
  revised_text: string;
  risk_delta: number;
  summary: string;
}

export interface ContractComparisonDiff {
  base_document_id: string;
  revised_document_id: string;
  changed_clauses_count: number;
  overall_risk_delta: number;
  diff_items: ClauseDiffItem[];
}

export interface LegalAuditCertificate {
  certificate_id: string;
  document_id: string;
  document_title: string;
  document_blake3_hash: string;
  total_clauses: number;
  critical_findings_count: number;
  high_findings_count: number;
  overall_risk_grade: string;
  issued_at_epoch_ms: number;
  ed25519_signature_hex: string;
  auditor_public_key_hex: string;
}

export interface AuditorStats {
  total_contracts_audited: number;
  total_clauses_analyzed: number;
  critical_risks_prevented: number;
  avg_risk_score: number;
}

export interface LicenseStatusReport {
  tier: string;
  is_valid: boolean;
  features: string[];
  days_remaining?: number | null;
}
