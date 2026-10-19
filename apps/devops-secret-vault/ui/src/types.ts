export type SecretType =
  | 'password'
  | 'ssh_key'
  | 'certificate'
  | 'api_key'
  | 'database_credential'
  | 'aws_credential'
  | 'azure_credential'
  | 'terraform_secret'
  | 'totp';

export interface SecretMetadata {
  id: string;
  title: string;
  secret_type: SecretType;
  tags: string[];
  username?: string | null;
  url?: string | null;
  notes?: string | null;
  version: number;
  created_at: number;
  updated_at: number;
}

export type SecretPayload =
  | { type: 'password'; data: { password: string } }
  | {
      type: 'ssh_key';
      data: {
        private_key: string;
        public_key?: string | null;
        passphrase?: string | null;
      };
    }
  | {
      type: 'certificate';
      data: {
        private_key: string;
        certificate_pem: string;
        chain_pem?: string | null;
      };
    }
  | {
      type: 'api_key';
      data: {
        api_key: string;
        api_secret?: string | null;
        endpoint_url?: string | null;
      };
    }
  | {
      type: 'database_credential';
      data: {
        engine: string;
        host: string;
        port: number;
        database: string;
        username: string;
        password: string;
        connection_string?: string | null;
      };
    }
  | {
      type: 'aws_credential';
      data: {
        access_key_id: string;
        secret_access_key: string;
        session_token?: string | null;
        region?: string | null;
        profile_name?: string | null;
      };
    }
  | {
      type: 'azure_credential';
      data: {
        subscription_id: string;
        tenant_id: string;
        client_id: string;
        client_secret: string;
        cloud_environment?: string | null;
      };
    }
  | {
      type: 'terraform_secret';
      data: {
        state_key?: string | null;
        tf_vars?: string | null;
        provider_token?: string | null;
        backend_config?: string | null;
      };
    }
  | {
      type: 'totp';
      data: {
        secret_base32: string;
        issuer?: string | null;
        account_name?: string | null;
        algorithm?: string | null;
        digits?: number | null;
        period_seconds?: number | null;
      };
    };

export interface DecryptedSecret {
  metadata: SecretMetadata;
  payload: SecretPayload;
}

export interface SecretSearchResult {
  id: string;
  title: string;
  secret_type: SecretType;
  tags: string[];
  username?: string | null;
  score: number;
  snippet?: string | null;
}

export type EntropyRating =
  | 'very_weak'
  | 'weak'
  | 'fair'
  | 'strong'
  | 'very_strong';

export interface PasswordStrength {
  entropy_bits: number;
  rating: EntropyRating;
  length: number;
  has_lowercase: boolean;
  has_uppercase: boolean;
  has_digits: boolean;
  has_symbols: boolean;
}

export type SecretAuditAction =
  | 'created'
  | 'read'
  | 'updated'
  | 'deleted'
  | 'exported'
  | 'copied_to_clipboard';

export interface SecretAuditEntry {
  id: string;
  secret_id: string;
  action: SecretAuditAction;
  timestamp: number;
  client: string;
}

export interface VaultStatusResponse {
  is_initialized: boolean;
  is_locked: boolean;
}

// ---------------------------------------------------------------------------
// Phase 4: Advanced Features
// ---------------------------------------------------------------------------

export interface TotpCodeResponse {
  code: string;
  remaining_seconds: number;
  total_step_seconds: number;
  progress_ratio: number;
}

export type CertStatus = 'valid' | 'expiring_soon' | 'expired';

export interface CertInspectionReport {
  subject_cn: string;
  issuer_cn: string;
  not_before_epoch: number;
  not_after_epoch: number;
  days_until_expiration: number;
  status: CertStatus;
  is_self_signed: boolean;
  signature_algorithm: string;
  is_weak_signature: boolean;
  public_key_algorithm: string;
  key_size_bits?: number | null;
  is_weak_key: boolean;
  subject_alt_names: string[];
  serial_number: string;
}

export type Severity = 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';

export interface SecurityRecommendation {
  id: string;
  severity: Severity;
  title: string;
  description: string;
  secret_id: string;
  secret_title: string;
}

export interface VaultHealthReport {
  overall_score: number;
  grade: string;
  total_secrets: number;
  compromised_count: number;
  reused_count: number;
  weak_count: number;
  stale_count: number;
  cert_issues_count: number;
  recommendations: SecurityRecommendation[];
}

export interface BackupExportSummary {
  output_path: string;
  total_secrets: number;
  file_size_bytes: number;
  exported_at: number;
}

export interface BackupImportSummary {
  total_imported: number;
  total_skipped: number;
  total_overwritten: number;
  original_exported_at: number;
}
