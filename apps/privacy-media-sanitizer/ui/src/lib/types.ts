export type MediaType =
  | 'jpeg'
  | 'png'
  | 'webp'
  | 'pdf'
  | 'tiff'
  | 'mp4'
  | 'generic_binary';

export type MetadataRiskGrade = 'clean' | 'medium' | 'high' | 'critical';

export interface MetadataTag {
  tag_name: string;
  category: string;
  raw_value: string;
  risk_grade: MetadataRiskGrade;
  explanation: string;
}

export interface MediaInspectionReport {
  file_id: string;
  filename: string;
  media_type: MediaType;
  original_size_bytes: number;
  original_blake3: string;
  metadata_tags: MetadataTag[];
  has_gps: boolean;
  gps_coordinates?: string | null;
  device_fingerprint?: string | null;
  overall_privacy_risk: MetadataRiskGrade;
}

export interface MediaSummary {
  file_id: string;
  filename: string;
  media_type: MediaType;
  size_bytes: number;
  tags_detected_count: number;
  has_gps: boolean;
  is_sanitized: boolean;
  overall_privacy_risk: MetadataRiskGrade;
}

export interface SanitizedResult {
  file_id: string;
  filename: string;
  original_size_bytes: number;
  sanitized_size_bytes: number;
  bytes_removed: number;
  original_blake3: string;
  sanitized_blake3: string;
  tags_stripped_count: number;
  gps_purged: boolean;
  sanitized_at_epoch_ms: number;
}

export interface PrivacyAuditCertificate {
  certificate_id: string;
  file_id: string;
  filename: string;
  original_blake3: string;
  sanitized_blake3: string;
  tags_purged_count: number;
  zero_trace_verified: boolean;
  ed25519_signature_hex: string;
  auditor_public_key_hex: string;
  issued_at_epoch_ms: number;
}

export interface SanitizerStats {
  total_files_sanitized: number;
  total_tags_purged: number;
  gps_leaks_prevented: number;
  device_fingerprints_destroyed: number;
}

export interface LicenseStatusReport {
  tier: string;
  is_valid: boolean;
  features: string[];
  days_remaining?: number | null;
}
