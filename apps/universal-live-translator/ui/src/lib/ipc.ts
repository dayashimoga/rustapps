/**
 * Unified Antigravity Desktop IPC & Native Dialog Client
 * 
 * Provides type-safe access to Rust backend commands, native file dialogs,
 * system drive enumeration, and streaming file URLs with CSRF session token attachment.
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

let isDialogPromptActive = false;

/**
 * Returns true if running inside the Antigravity desktop host or Tauri runtime.
 */
export function isDesktopApp(): boolean {
  return typeof window !== 'undefined' && Boolean(window.__TAURI__?.invoke || window.__TAURI__?.core?.invoke);
}

/**
 * Invokes a backend Rust IPC command with typed payload and response.
 */
export async function invokeIpc<T = unknown>(
  cmd: string,
  args: Record<string, unknown> = {}
): Promise<T> {
  if (typeof window !== 'undefined') {
    if (window.__TAURI__?.core?.invoke) {
      return await window.__TAURI__.core.invoke<T>(cmd, args);
    }
    if (window.__TAURI__?.invoke) {
      return await window.__TAURI__.invoke<T>(cmd, args);
    }
  }

  // Loopback HTTP IPC fallback
  if (
    typeof fetch !== 'undefined' &&
    typeof window !== 'undefined' &&
    window.location &&
    window.location.protocol.startsWith('http')
  ) {
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

/**
 * Opens a native OS file or directory picker with concurrency protection.
 */
export async function openNativeDialog(
  options: OpenDialogOptions = {}
): Promise<string | string[] | null> {
  if (isDialogPromptActive) {
    return null;
  }
  isDialogPromptActive = true;
  try {
    if (typeof window !== 'undefined' && window.__TAURI__?.dialog?.open) {
      return await window.__TAURI__.dialog.open(options);
    }
    if (
      typeof fetch !== 'undefined' &&
      typeof window !== 'undefined' &&
      window.location?.protocol.startsWith('http')
    ) {
      const token = window.__ANTIGRAVITY_TOKEN__ || '';
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
      };
      if (token) {
        headers['X-Antigravity-Token'] = token;
      }
      const response = await fetch('/__dialog/open', {
        method: 'POST',
        headers,
        body: JSON.stringify(options || {}),
      });
      if (response.ok) {
        const data = await response.json().catch(() => null);
        return data?.path ?? null;
      }
    }
    return null;
  } finally {
    isDialogPromptActive = false;
  }
}

/**
 * Opens a native OS save file dialog with concurrency protection.
 */
export async function saveNativeDialog(
  options: SaveDialogOptions = {}
): Promise<string | null> {
  if (isDialogPromptActive) {
    return null;
  }
  isDialogPromptActive = true;
  try {
    if (typeof window !== 'undefined' && window.__TAURI__?.dialog?.save) {
      return await window.__TAURI__.dialog.save(options);
    }
    if (
      typeof fetch !== 'undefined' &&
      typeof window !== 'undefined' &&
      window.location?.protocol.startsWith('http')
    ) {
      const token = window.__ANTIGRAVITY_TOKEN__ || '';
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
      };
      if (token) {
        headers['X-Antigravity-Token'] = token;
      }
      const response = await fetch('/__dialog/save', {
        method: 'POST',
        headers,
        body: JSON.stringify(options || {}),
      });
      if (response.ok) {
        const data = await response.json().catch(() => null);
        return data?.path ?? null;
      }
    }
    return null;
  } finally {
    isDialogPromptActive = false;
  }
}

/**
 * Enumerates system drives and quick access places.
 */
export async function getDrivesAndPlaces(): Promise<FsLocationEntry[]> {
  if (typeof window !== 'undefined' && window.__TAURI__?.fs?.getDrivesAndPlaces) {
    return await window.__TAURI__.fs.getDrivesAndPlaces();
  }
  if (
    typeof fetch !== 'undefined' &&
    typeof window !== 'undefined' &&
    window.location?.protocol.startsWith('http')
  ) {
    const token = window.__ANTIGRAVITY_TOKEN__ || '';
    const headers: Record<string, string> = {};
    if (token) {
      headers['X-Antigravity-Token'] = token;
    }
    const response = await fetch('/__fs/drives', { headers }).catch(() => null);
    if (response && response.ok) {
      const data = await response.json().catch(() => null);
      return data?.entries ?? [];
    }
  }
  return [];
}

/**
 * Constructs a streaming local file URL with CSRF token authentication.
 */
export function getLocalFileUrl(localPath: string): string {
  if (!localPath) return '';
  if (typeof window !== 'undefined' && window.__TAURI__?.fs?.fileUrl) {
    return window.__TAURI__.fs.fileUrl(localPath);
  }
  const token = typeof window !== 'undefined' ? window.__ANTIGRAVITY_TOKEN__ || '' : '';
  return `/__file?path=${encodeURIComponent(localPath)}${token ? `&token=${encodeURIComponent(token)}` : ''}`;
}

