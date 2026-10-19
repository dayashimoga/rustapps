<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { TotpCodeResponse } from '../types';

  export let secretId: string;
  export let onGenerate: (id: string) => Promise<TotpCodeResponse>;
  export let onCopy: (text: string) => void;

  let totpData: TotpCodeResponse | null = null;
  let copied = false;
  let timer: any = null;
  let loading = true;
  let errorMessage = '';

  async function fetchCode() {
    try {
      totpData = await onGenerate(secretId);
      errorMessage = '';
    } catch (err: any) {
      errorMessage = err?.message || String(err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    fetchCode();
    // Poll every 1s for accurate countdown and refresh
    timer = setInterval(fetchCode, 1000);
  });

  onDestroy(() => {
    if (timer) clearInterval(timer);
  });

  function handleCopy() {
    if (!totpData) return;
    onCopy(totpData.code);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 2000);
  }

  function formatCode(c: string): string {
    if (c.length === 6) {
      return `${c.slice(0, 3)} ${c.slice(3)}`;
    }
    if (c.length === 8) {
      return `${c.slice(0, 4)} ${c.slice(4)}`;
    }
    return c;
  }
</script>

<div class="totp-card">
  <div class="totp-header">
    <div class="header-left">
      <span class="totp-badge">RFC 6238 TOTP</span>
      <span class="live-pulse"></span>
    </div>
    {#if totpData}
      <span class="step-label">{totpData.remaining_seconds}s remaining</span>
    {/if}
  </div>

  {#if loading && !totpData}
    <div class="totp-loading">Generating secure 2FA token...</div>
  {:else if errorMessage}
    <div class="totp-error">{errorMessage}</div>
  {:else if totpData}
    <div class="totp-body">
      <!-- Circular countdown indicator -->
      <div class="timer-dial">
        <svg viewBox="0 0 40 40" class="dial-svg">
          <circle cx="20" cy="20" r="16" class="dial-track" />
          <circle
            cx="20"
            cy="20"
            r="16"
            class="dial-progress"
            style="stroke-dasharray: 100; stroke-dashoffset: {totpData.progress_ratio * 100};"
            class:urgent={totpData.remaining_seconds <= 5}
          />
        </svg>
        <span class="dial-seconds">{totpData.remaining_seconds}</span>
      </div>

      <div class="code-container">
        <span class="code-digits">{formatCode(totpData.code)}</span>
      </div>

      <button
        type="button"
        class="copy-totp-btn"
        class:copied
        on:click={handleCopy}
        title="Copy 2FA code"
      >
        {#if copied}
          <svg viewBox="0 0 20 20" fill="currentColor" class="btn-icon">
            <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
          </svg>
          <span>Copied</span>
        {:else}
          <svg viewBox="0 0 20 20" fill="currentColor" class="btn-icon">
            <path d="M8 3a1 1 0 011-1h2a1 1 0 110 2H9a1 1 0 01-1-1z" />
            <path d="M6 3a2 2 0 00-2 2v11a2 2 0 002 2h8a2 2 0 002-2V5a2 2 0 00-2-2 3 3 0 01-3 2H9a3 3 0 01-3-2z" />
          </svg>
          <span>Copy Code</span>
        {/if}
      </button>
    </div>
  {/if}
</div>

<style>
  .totp-card {
    background: rgba(15, 23, 42, 0.65);
    border: 1px solid rgba(56, 189, 248, 0.25);
    border-radius: 12px;
    padding: 16px;
    margin-top: 12px;
    backdrop-filter: blur(12px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  }

  .totp-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .totp-badge {
    background: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.3);
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 8px;
    border-radius: 6px;
  }

  .live-pulse {
    width: 6px;
    height: 6px;
    background: #10b981;
    border-radius: 50%;
    box-shadow: 0 0 8px #10b981;
    animation: pulse 1.5s infinite;
  }

  @keyframes pulse {
    0% { transform: scale(0.9); opacity: 0.7; }
    50% { transform: scale(1.3); opacity: 1; }
    100% { transform: scale(0.9); opacity: 0.7; }
  }

  .step-label {
    font-size: 12px;
    color: #94a3b8;
    font-variant-numeric: tabular-nums;
  }

  .totp-body {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .timer-dial {
    position: relative;
    width: 44px;
    height: 44px;
    flex-shrink: 0;
  }

  .dial-svg {
    transform: rotate(-90deg);
    width: 100%;
    height: 100%;
  }

  .dial-track {
    fill: none;
    stroke: rgba(255, 255, 255, 0.1);
    stroke-width: 3.5;
  }

  .dial-progress {
    fill: none;
    stroke: #38bdf8;
    stroke-width: 3.5;
    stroke-linecap: round;
    transition: stroke-dashoffset 0.8s linear, stroke 0.3s ease;
  }

  .dial-progress.urgent {
    stroke: #ef4444;
  }

  .dial-seconds {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 700;
    color: #f8fafc;
    font-variant-numeric: tabular-nums;
  }

  .code-container {
    flex: 1;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 8px 16px;
    text-align: center;
  }

  .code-digits {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 26px;
    font-weight: 700;
    letter-spacing: 4px;
    color: #f1f5f9;
    text-shadow: 0 0 16px rgba(56, 189, 248, 0.5);
  }

  .copy-totp-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(56, 189, 248, 0.15);
    border: 1px solid rgba(56, 189, 248, 0.3);
    color: #e0f2fe;
    padding: 10px 14px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .copy-totp-btn:hover {
    background: rgba(56, 189, 248, 0.25);
    border-color: rgba(56, 189, 248, 0.5);
    transform: translateY(-1px);
  }

  .copy-totp-btn.copied {
    background: rgba(16, 185, 129, 0.2);
    border-color: rgba(16, 185, 129, 0.4);
    color: #a7f3d0;
  }

  .btn-icon {
    width: 16px;
    height: 16px;
  }

  .totp-loading {
    color: #94a3b8;
    font-size: 13px;
    text-align: center;
    padding: 12px 0;
  }

  .totp-error {
    color: #f87171;
    font-size: 13px;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: 6px;
    padding: 8px 12px;
  }
</style>
