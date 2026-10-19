export type SecretType =
  | 'password'
  | 'ssh_key'
  | 'certificate'
  | 'api_key'
  | 'database_credential';

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
