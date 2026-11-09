/**
 * Offline QR Code Studio Desktop IPC Client
 *
 * Interfaces with the Rust backend via Tauri v2 or embedded loopback HTTP IPC bridge.
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

declare global {
  interface Window {
    __ANTIGRAVITY_TOKEN__?: string;
    __TAURI__?: {
      invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
      core?: {
        invoke: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
      };
      dialog?: {
        open?: (opts?: OpenDialogOptions) => Promise<string | string[] | null>;
        save?: (opts?: SaveDialogOptions) => Promise<string | null>;
      };
      fs?: {
        getDrivesAndPlaces?: () => Promise<FsLocationEntry[]>;
        fileUrl?: (localPath: string) => string;
      };
    };
  }
}

export async function invokeIpc<T = unknown>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (typeof window !== 'undefined') {
    if (window.__TAURI__?.core?.invoke) {
      return await window.__TAURI__.core.invoke<T>(cmd, args);
    }
    if (window.__TAURI__?.invoke) {
      return await window.__TAURI__.invoke<T>(cmd, args);
    }
  }

  // Loopback HTTP IPC fallback
  if (typeof fetch !== 'undefined' && typeof window !== 'undefined' && window.location) {
    const token = window.__ANTIGRAVITY_TOKEN__ || '';
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    if (token) {
      headers['X-Antigravity-Token'] = token;
    }

    const response = await fetch('/__ipc/' + encodeURIComponent(cmd), {
      method: 'POST',
      headers,
      body: JSON.stringify(args || {}),
    });

    const payload = await response.json().catch(() => null);
    if (response.ok && payload && payload.__ipc_ok) {
      return payload.__ipc_data as T;
    }
    const errMsg = (payload && payload.__ipc_error) || `Command "${cmd}" failed with HTTP ${response.status}`;
    throw new Error(errMsg);
  }

  throw new Error(`No IPC transport available for command "${cmd}"`);
}
