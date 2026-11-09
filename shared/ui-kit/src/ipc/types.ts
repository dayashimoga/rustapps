/**
 * Antigravity IPC & Dialog Transport Types
 */

export interface DialogFilter {
  name: string;
  extensions: string[];
}

export interface OpenDialogOptions {
  title?: string;
  directory?: boolean;
  multiple?: boolean;
  filter?: string;
  filters?: DialogFilter[];
  defaultPath?: string;
}

export interface SaveDialogOptions {
  title?: string;
  defaultPath?: string;
  filter?: string;
  filters?: DialogFilter[];
}

export interface FsLocationEntry {
  name: string;
  path: string;
  kind: string;
  available: boolean;
}

export interface IpcOkEnvelope<T = unknown> {
  __ipc_ok: true;
  __ipc_data: T;
}

export interface IpcErrorEnvelope {
  __ipc_ok: false;
  __ipc_error: string;
}

export type IpcEnvelope<T = unknown> = IpcOkEnvelope<T> | IpcErrorEnvelope;

export type MockIpcHandler<T = unknown> = (args: Record<string, unknown>) => Promise<T> | T;

export interface LicenseStatusInfo {
  app_id: string;
  tier: 'Free' | 'Pro' | 'ProLifetime' | 'Business' | 'Enterprise';
  is_valid: boolean;
  expires_at: number | null;
  licensed_to: string;
  hardware_id: string;
  days_remaining?: number;
  features: string[];
}

