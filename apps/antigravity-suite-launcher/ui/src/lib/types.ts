export type AppCategory = 'enterprise_core' | 'creator_media' | 'industrial_specialized';

export interface AppMetadata {
  id: string;
  name: string;
  tagline: string;
  category: AppCategory;
  binary_name: string;
  version: string;
  installed: boolean;
  cold_start_target_ms: number;
  ram_idle_budget_mb: number;
  air_gap_status: string;
  icon_glyph: string;
}

export interface SuiteStats {
  total_apps_registered: number;
  total_apps_installed: number;
  suite_license_tier: string;
  air_gap_egress_blocks: number;
}

export interface AppSelfTestResult {
  app_id: string;
  passed: boolean;
  ipc_checks_passed: number;
  ipc_checks_total: number;
  message: string;
}

export interface SuiteIntegrityManifest {
  manifest_id: string;
  suite_blake3_hash: string;
  total_apps: number;
  signer_public_key_hex: string;
  ed25519_signature_hex: string;
  issued_at_epoch_ms: number;
}

export interface LicenseStatus {
  tier: string;
  active: boolean;
  days_remaining?: number;
  licensee?: string;
}
