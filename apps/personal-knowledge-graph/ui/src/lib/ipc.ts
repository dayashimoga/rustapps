/**
 * Antigravity Desktop IPC Client for Personal Knowledge Graph & Research Studio
 */

declare global {
  interface Window {
    __ANTIGRAVITY_TOKEN__?: string;
  }
}

export async function invokeIpc<T = unknown>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  const token = window.__ANTIGRAVITY_TOKEN__ || '';
  const response = await fetch('/api/ipc', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Antigravity-CSRF': token,
    },
    body: JSON.stringify({
      command,
      payload: args,
    }),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`IPC error (${response.status}): ${errorText}`);
  }

  const result = await response.json();
  if (result && typeof result === 'object' && 'error' in result) {
    throw new Error(String(result.error));
  }
  return result as T;
}

