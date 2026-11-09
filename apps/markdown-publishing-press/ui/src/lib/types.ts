export type DocumentTemplate =
  | 'ieee_two_column'
  | 'acm_sigconf'
  | 'springer_lncs'
  | 'arxiv_preprint'
  | 'enterprise_sop'
  | 'book_chapter';

export type TargetFormat = 'print_pdf' | 'epub3' | 'standalone_html5' | 'typst_source';

export interface BibItem {
  key: string;
  entry_type: string;
  title: string;
  authors: string;
  year: number;
  venue: string;
  doi_or_url?: string;
}

export interface PublishingProject {
  project_id: string;
  title: string;
  authors: string;
  abstract_text: string;
  template: DocumentTemplate;
  target_format: TargetFormat;
  source_markdown: string;
  bibtex_raw: string;
  bibliography: BibItem[];
  created_at_epoch_ms: number;
  updated_at_epoch_ms: number;
}

export interface ProjectSummary {
  project_id: string;
  title: string;
  authors: string;
  template: DocumentTemplate;
  word_count: number;
  citations_count: number;
  updated_at_epoch_ms: number;
}

export interface CompilationArtifact {
  artifact_id: string;
  project_id: string;
  format: TargetFormat;
  template: DocumentTemplate;
  rendered_document: string;
  page_count: number;
  figure_count: number;
  citation_count: number;
  unresolved_references: string[];
  compile_duration_ms: number;
  document_blake3_hash: string;
}

export interface CopyrightCertificate {
  certificate_id: string;
  project_id: string;
  title: string;
  primary_author: string;
  document_blake3_hash: string;
  ed25519_signature_hex: string;
  author_public_key_hex: string;
  issued_at_epoch_ms: number;
}

export interface PressStats {
  total_projects: number;
  total_compilations: number;
  total_pages_rendered: number;
  total_citations_indexed: number;
}

export interface LicenseStatus {
  tier: string;
  active: boolean;
  days_remaining?: number;
  licensee?: string;
}
