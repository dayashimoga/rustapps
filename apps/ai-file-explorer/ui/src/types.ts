export interface FileItem {
  path: string;
  file_name: string;
  extension: string | null;
  size_bytes: number;
  is_dir: boolean;
  modified_timestamp_secs: number;
  is_readonly: boolean;
}

export interface DirectoryListing {
  current_path: string;
  parent_path: string | null;
  items: FileItem[];
  total_count: number;
  total_size_bytes: number;
}

export interface FolderStats {
  root_path: string;
  total_files: number;
  total_dirs: number;
  total_size_bytes: number;
  extension_counts: Record<string, number>;
  extension_sizes: Record<string, number>;
  largest_files: FileItem[];
}

export interface DuplicateGroup {
  file_size: number;
  blake3_hash: string;
  file_paths: string[];
  wasted_bytes: number;
}

export interface DeduplicationSummary {
  total_scanned_files: number;
  total_scanned_bytes: number;
  duplicate_groups: DuplicateGroup[];
  total_duplicate_files: number;
  potential_reclaimed_bytes: number;
  elapsed_millis: number;
}

export interface DeleteFilesResult {
  deleted_count: number;
  reclaimed_bytes: number;
  failed_paths: string[];
}

export interface TreemapRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface TreemapNode {
  name: string;
  path: string;
  size_bytes: number;
  is_dir: boolean;
  bounds: TreemapRect;
  children: TreemapNode[];
}

export type BatchRenameRule =
  | { type: 'RegexReplace'; config: { pattern: string; replacement: string } }
  | { type: 'PrefixSuffix'; config: { prefix: string; suffix: string } }
  | { type: 'Numbering'; config: { prefix: string; start_number: number; pad_digits: number } }
  | { type: 'ToLowerCase' }
  | { type: 'ToUpperCase' };

export interface RenamePreview {
  original_path: string;
  proposed_path: string;
  has_conflict: boolean;
  conflict_reason: string | null;
}

export interface BatchRenameResult {
  total_requested: number;
  successful_renames: number;
  rolled_back: boolean;
  error_message: string | null;
}

// ---------------------------------------------------------------------------
// Phase 6 Domain Types
// ---------------------------------------------------------------------------

export interface DocumentMetadata {
  title: string | null;
  author: string | null;
  page_count: number | null;
  word_count: number;
  format: string;
  creation_date: string | null;
}

export interface ExtractedDocument {
  path: string;
  file_name: string;
  extension: string | null;
  metadata: DocumentMetadata;
  content_text: string;
  language: string | null;
  extracted_at_secs: number;
}

export interface OcrWord {
  text: string;
  confidence: number;
  bbox: [number, number, number, number];
}

export interface OcrResult {
  text: string;
  words: OcrWord[];
  confidence: number;
  line_count: number;
  processing_time_ms: number;
}

export interface ScreenshotFingerprint {
  path: string;
  dhash: number;
  ocr_text: string;
  dimensions: [number, number];
}

export interface ScreenshotSearchResult {
  path: string;
  dhash: number;
  hamming_distance: number;
  similarity_pct: number;
  ocr_snippet: string | null;
}

export type SearchMode = 'Keyword' | 'Semantic' | 'Combined';

export interface ContentSearchQuery {
  query: string;
  mode: SearchMode;
  limit: number;
  extension_filter: string | null;
}

export interface SemanticSearchResult {
  path: string;
  file_name: string;
  score: number;
  snippet: string;
  format: string;
}

export interface ContentSearchResult {
  items: SemanticSearchResult[];
  total_matches: number;
  execution_time_ms: number;
}

export type FindingSeverity = 'critical' | 'high' | 'medium';
export type SensitiveDataType =
  | 'private_key'
  | 'aws_access_key'
  | 'ssn'
  | 'credit_card'
  | 'connection_string_password';

export interface SensitiveFinding {
  id: string;
  data_type: SensitiveDataType;
  severity: FindingSeverity;
  file_path: string;
  line_number: number;
  redacted_preview: string;
}

export interface ComplianceScanSummary {
  scanned_files_count: number;
  findings_count: number;
  critical_count: number;
  high_count: number;
  medium_count: number;
  findings: SensitiveFinding[];
  scan_duration_ms: number;
}

export type ActiveTab =
  | 'browser'
  | 'duplicates'
  | 'treemap'
  | 'batch_rename'
  | 'content_search'
  | 'screenshots'
  | 'compliance';

export interface FsLocationEntry {
  name: string;
  path: string;
  kind: 'drive' | 'quick_access' | string;
  available: boolean;
}

export interface ArchiveEntryInfo {
  name: string;
  size_bytes: number;
  is_dir: boolean;
}

export interface FilePreviewData {
  path: string;
  file_name: string;
  extension: string;
  size_bytes: number;
  is_text: boolean;
  is_image: boolean;
  is_pdf: boolean;
  is_media: boolean;
  is_archive?: boolean;
  archive_entries?: ArchiveEntryInfo[];
  text_content?: string | null;
  total_lines?: number | null;
  pdf_pages?: number | null;
  pdf_title?: string | null;
  pdf_author?: string | null;
  blake3_hash?: string | null;
  modified_timestamp_secs: number;
}

