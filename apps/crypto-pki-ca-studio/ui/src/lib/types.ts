export interface SubjectDn {
  common_name: string;
  organization: string;
  organizational_unit: string;
  country: string;
  state_or_province: string;
  locality: string;
}

export interface KeyUsageFlags {
  digital_signature: boolean;
  key_encipherment: boolean;
  key_cert_sign: boolean;
  crl_sign: boolean;
}

export interface ExtendedKeyUsageFlags {
  server_auth: boolean;
  client_auth: boolean;
  code_signing: boolean;
  email_protection: boolean;
}

export interface X509Certificate {
  certificate_id: string;
  serial_number_hex: string;
  cert_type: 'root_ca' | 'intermediate_ca' | 'tls_server' | 'tls_client' | 'code_signing';
  subject: SubjectDn;
  issuer_dn: string;
  issuer_ca_id?: string;
  key_algorithm: string;
  validity_not_before_epoch_ms: number;
  validity_not_after_epoch_ms: number;
  subject_alt_names: string[];
  key_usage: KeyUsageFlags;
  ext_key_usage: ExtendedKeyUsageFlags;
  is_ca: boolean;
  path_len_constraint?: number;
  fingerprint_sha256: string;
  certificate_pem: string;
  public_key_pem: string;
  is_revoked: boolean;
  revocation_reason?: string;
  revocation_epoch_ms?: number;
}

export interface RevokedCertificateEntry {
  serial_number_hex: string;
  revocation_epoch_ms: number;
  reason: string;
}

export interface CertificateRevocationList {
  crl_id: string;
  issuer_ca_id: string;
  issuer_dn: string;
  this_update_epoch_ms: number;
  next_update_epoch_ms: number;
  revoked_entries: RevokedCertificateEntry[];
  signature_hex: string;
  crl_pem: string;
}

export interface ShamirShard {
  shard_index: number;
  shard_hex: string;
  custodian_label: string;
  commitment_hash: string;
}

export interface ShamirCeremony {
  ceremony_id: string;
  ca_id: string;
  ca_common_name: string;
  threshold_m: number;
  total_n: number;
  shards: ShamirShard[];
  created_at_epoch_ms: number;
  status: string;
}

export interface PkiStats {
  total_cas: number;
  total_issued_certificates: number;
  active_certificates: number;
  revoked_certificates: number;
  active_crls: number;
  shamir_ceremonies_count: number;
}
