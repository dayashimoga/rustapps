/**
 * Offline SBOM & Supply-Chain Auditor Desktop IPC Client
 *
 * Interfaces with the Rust backend via Tauri v2 or embedded loopback HTTP IPC bridge.
 */

declare global {
  interface Window {
    __ANTIGRAVITY_TOKEN__?: string;
    __TAURI__?: {
      invoke?: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
      core?: {
        invoke: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
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

    const token = window.__ANTIGRAVITY_TOKEN__ || '';
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'X-Antigravity-Token': token,
    };

    const res = await fetch(`/__ipc/${cmd}`, {
      method: 'POST',
      headers,
      body: JSON.stringify(args),
    });

    if (!res.ok) {
      const errText = await res.text().catch(() => 'Unknown error');
      throw new Error(`IPC command failed (${res.status}): ${errText}`);
    }

    return (await res.json()) as T;
  }

  throw new Error('Window context unavailable for IPC execution');
}
