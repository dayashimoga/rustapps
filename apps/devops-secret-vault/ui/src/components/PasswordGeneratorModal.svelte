<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';

  export let isOpen = false;
  export let onGenerate: (params: {
    length: number;
    includeUppercase: boolean;
    includeLowercase: boolean;
    includeNumbers: boolean;
    includeSymbols: boolean;
    mode: 'password' | 'passphrase';
    wordCount: number;
  }) => Promise<{ password: string; entropy_bits: number; strength_label: string }>;

  const dispatch = createEventDispatcher<{
    close: void;
    usePassword: { password: string };
  }>();

  let mode: 'password' | 'passphrase' = 'password';
  let length = 24;
  let wordCount = 5;
  let includeUppercase = true;
  let includeLowercase = true;
  let includeNumbers = true;
  let includeSymbols = true;

  let generatedPassword = '';
  let entropyBits = 0;
  let strengthLabel = '';
  let copied = false;

  $: if (isOpen) {
    regenerate();
  }

  async function regenerate() {
    try {
      const res = await onGenerate({
        length,
        includeUppercase,
        includeLowercase,
        includeNumbers,
        includeSymbols,
        mode,
        wordCount,
      });
      generatedPassword = res.password;
      entropyBits = Math.round(res.entropy_bits);
      strengthLabel = res.strength_label;
    } catch (_) {}
  }

  async function copyToClipboard() {
    if (!generatedPassword) return;
    try {
      await navigator.clipboard.writeText(generatedPassword);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (_) {}
  }

  function handleUse() {
    dispatch('usePassword', { password: generatedPassword });
    dispatch('close');
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="modal-backdrop" on:click={() => dispatch('close')}>
    <div class="gen-modal glass-panel" on:click|stopPropagation>
      <div class="modal-header">
        <div class="title-row">
          <span class="icon">⚡</span>
          <div>
            <h3>Cryptographic Secret Generator</h3>
            <p class="subtitle">NIST 800-63B compliant high-entropy offline password &amp; Diceware generator</p>
          </div>
        </div>
        <button class="btn-close" on:click={() => dispatch('close')}>✕</button>
      </div>

      <!-- Password Output Display -->
      <div class="output-card">
        <div class="password-box">
          <span class="password-text">{generatedPassword}</span>
          <button class="btn-copy" on:click={copyToClipboard} title="Copy to Clipboard">
            {copied ? '✓ Copied' : '📋 Copy'}
          </button>
        </div>

        <div class="entropy-bar-row">
          <div class="entropy-track">
            <div
              class="entropy-fill"
              style="width: {Math.min(100, (entropyBits / 128) * 100)}%;"
            ></div>
          </div>
          <span class="entropy-label">{entropyBits} bits ({strengthLabel})</span>
        </div>
      </div>

      <!-- Mode Selector -->
      <div class="mode-toggles">
        <button
          class="mode-btn {mode === 'password' ? 'active' : ''}"
          on:click={() => { mode = 'password'; regenerate(); }}
        >
          Password Mode
        </button>
        <button
          class="mode-btn {mode === 'passphrase' ? 'active' : ''}"
          on:click={() => { mode = 'passphrase'; regenerate(); }}
        >
          Diceware Passphrase
        </button>
      </div>

      <!-- Parameter Sliders -->
      <div class="params-panel">
        {#if mode === 'password'}
          <div class="param-row">
            <span class="param-title">Password Length: {length} chars</span>
            <input
              type="range"
              min="8"
              max="64"
              bind:value={length}
              on:input={regenerate}
            />
          </div>

          <div class="checkbox-grid">
            <label class="check-label">
              <input type="checkbox" bind:checked={includeUppercase} on:change={regenerate} />
              Uppercase (A-Z)
            </label>
            <label class="check-label">
              <input type="checkbox" bind:checked={includeLowercase} on:change={regenerate} />
              Lowercase (a-z)
            </label>
            <label class="check-label">
              <input type="checkbox" bind:checked={includeNumbers} on:change={regenerate} />
              Numbers (0-9)
            </label>
            <label class="check-label">
              <input type="checkbox" bind:checked={includeSymbols} on:change={regenerate} />
              Symbols (!@#$%)
            </label>
          </div>
        {:else}
          <div class="param-row">
            <span class="param-title">Word Count: {wordCount} words</span>
            <input
              type="range"
              min="3"
              max="8"
              bind:value={wordCount}
              on:input={regenerate}
            />
          </div>
          <p class="passphrase-hint">
            Diceware passphrases are extremely memorable while offering 60-100+ bits of cryptographic entropy resistant to brute force and dictionary attacks.
          </p>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn btn-secondary" on:click={regenerate}>↻ Regenerate</button>
        <div class="action-buttons">
          <button class="btn btn-secondary" on:click={() => dispatch('close')}>Cancel</button>
          <button class="btn btn-primary" on:click={handleUse}>Use Secret</button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .gen-modal {
    width: 580px;
    border-radius: 14px;
    padding: 24px;
    background: hsla(222, 24%, 7%, 0.98);
    border: 1px solid var(--border-color);
  }

  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .icon {
    font-size: 24px;
  }

  .title-row h3 {
    font-size: 17px;
    font-weight: 700;
  }

  .subtitle {
    font-size: 11.5px;
    color: var(--text-secondary);
  }

  .btn-close {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 18px;
    cursor: pointer;
  }

  .output-card {
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 16px;
    margin-bottom: 16px;
  }

  .password-box {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .password-text {
    font-family: var(--font-mono);
    font-size: 16px;
    font-weight: 600;
    color: var(--accent-cyan);
    word-break: break-all;
    user-select: all;
  }

  .btn-copy {
    background: rgba(6, 182, 212, 0.16);
    border: 1px solid rgba(6, 182, 212, 0.4);
    color: var(--accent-cyan);
    border-radius: 6px;
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    margin-left: 12px;
  }

  .entropy-bar-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .entropy-track {
    flex: 1;
    height: 6px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 9999px;
    overflow: hidden;
  }

  .entropy-fill {
    height: 100%;
    background: linear-gradient(90deg, #ef4444, #f59e0b, #10b981);
    transition: width 0.2s ease;
  }

  .entropy-label {
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .mode-toggles {
    display: flex;
    gap: 8px;
    margin-bottom: 16px;
  }

  .mode-btn {
    flex: 1;
    padding: 8px 12px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .mode-btn.active {
    background: hsla(217, 91%, 60%, 0.2);
    border-color: var(--accent-blue);
    color: #fff;
  }

  .params-panel {
    display: flex;
    flex-direction: column;
    gap: 14px;
    margin-bottom: 20px;
  }

  .param-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .param-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .checkbox-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .check-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .passphrase-hint {
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 14px;
    border-top: 1px solid var(--border-color);
  }

  .action-buttons {
    display: flex;
    gap: 8px;
  }
</style>
