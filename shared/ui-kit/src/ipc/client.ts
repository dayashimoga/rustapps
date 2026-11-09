/**
 * Unified Antigravity Desktop IPC, Binary Streaming & Dialog Client
 */

import type {
  DialogFilter,
  OpenDialogOptions,
  SaveDialogOptions,
  FsLocationEntry,
  MockIpcHandler,
} from './types';

declare global {
  interface Window {
    __ANTIGRAVITY_TOKEN__?: string;
    __TAURI__?: {
      invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
      core?: {
        invoke: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
      };
      event?: {
        listen?: <T = unknown>(event: string, handler: (event: { payload: T }) => void) => Promise<() => void>;
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

// In-memory test mock registry
const mockHandlers = new Map<string, MockIpcHandler>();
let isDialogPromptActive = false;

/**
 * Register a mock IPC handler for automated tests or headless simulation.
 */
export function registerIpcMock<T = unknown>(cmd: string, handler: MockIpcHandler<T>): void {
  mockHandlers.set(cmd, handler as MockIpcHandler);
}

/**
 * Clear all registered mock IPC handlers.
 */
export function clearIpcMocks(): void {
  mockHandlers.clear();
}

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
  // 1. Check registered mock handler first (for Vitest/mock environments)
  const mock = mockHandlers.get(cmd);
  if (mock) {
    return (await mock(args)) as T;
  }

  // 2. Tauri IPC bridge (Tauri v2 core.invoke or v1 invoke)
  if (typeof window !== 'undefined') {
    if (window.__TAURI__?.core?.invoke) {
      return await window.__TAURI__.core.invoke<T>(cmd, args);
    }
    if (window.__TAURI__?.invoke) {
      return await window.__TAURI__.invoke<T>(cmd, args);
    }
  }

  // 3. Loopback HTTP IPC fallback
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
 * Binary streaming IPC for high-throughput waveform, frame buffer, and model pack data.
 */
export async function invokeIpcBin(
  cmd: string,
  body?: BodyInit | Uint8Array
): Promise<ArrayBuffer> {
  if (
    typeof fetch !== 'undefined' &&
    typeof window !== 'undefined' &&
    window.location &&
    window.location.protocol.startsWith('http')
  ) {
    const token = window.__ANTIGRAVITY_TOKEN__ || '';
    const headers: Record<string, string> = {
      'Content-Type': 'application/octet-stream',
    };
    if (token) {
      headers['X-Antigravity-Token'] = token;
    }

    const payloadBody = body instanceof Uint8Array ? body : body ?? new Uint8Array(0);
    const response = await fetch('/__ipc_bin/' + encodeURIComponent(cmd), {
      method: 'POST',
      headers,
      body: payloadBody as BodyInit,
    });

    if (!response.ok) {
      throw new Error(`Binary IPC command "${cmd}" failed with HTTP ${response.status}`);
    }

    return await response.arrayBuffer();
  }

  throw new Error(`Binary IPC transport unavailable for command "${cmd}"`);
}

/**
 * Subscribe to server-sent events (SSE) or Tauri events with automatic cleanup.
 */
export function subscribeEvents<T = unknown>(
  eventName: string,
  handler: (payload: T) => void
): () => void {
  // If Tauri event listener is available
  if (typeof window !== 'undefined' && window.__TAURI__?.event?.listen) {
    let unlisten: (() => void) | null = null;
    window.__TAURI__.event.listen<T>(eventName, (e) => handler(e.payload)).then((fn) => {
      unlisten = fn;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }

  // SSE Fallback
  if (typeof EventSource !== 'undefined' && typeof window !== 'undefined') {
    const token = window.__ANTIGRAVITY_TOKEN__ || '';
    const url = `/__events?name=${encodeURIComponent(eventName)}${token ? `&token=${encodeURIComponent(token)}` : ''}`;
    const eventSource = new EventSource(url);

    eventSource.onmessage = (e) => {
      try {
        const parsed = JSON.parse(e.data) as T;
        handler(parsed);
      } catch {
        handler(e.data as unknown as T);
      }
    };

    return () => {
      eventSource.close();
    };
  }

  return () => {};
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

