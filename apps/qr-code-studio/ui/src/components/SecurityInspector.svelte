<script lang="ts">
  import { invokeIpc } from '../lib/ipc';

  export let initialPayload = '';

  let inputPayload = initialPayload || 'https://paypal.com@login-secure.phishing-test.org/auth';
  let isAnalyzing = false;
  let errorMessage = '';

  let report: {
    risk_score: number;
    risk_level: 'Safe' | 'Low' | 'Medium' | 'High' | 'Critical';
    threat_categories: string[];
    findings: string[];
    sanitized_url: string | null;
    is_air_gap_safe: boolean;
  } | null = null;

  async function analyze() {
    if (!inputPayload.trim()) return;
    isAnalyzing = true;
    errorMessage = '';

    try {
      const res = await invokeIpc<any>('analyze_payload_security', {
        payload: inputPayload.trim(),
      });
      report = res;
    } catch (e: any) {
      errorMessage = e?.message || String(e);
    } finally {
      isAnalyzing = false;
    }
  }

  function getRiskColor(level: string): string {
    switch (level) {
      case 'Safe': return 'var(--risk-safe)';
      case 'Low': return 'var(--risk-low)';
      case 'Medium': return 'var(--risk-medium)';
      case 'High': return 'var(--risk-high)';
      case 'Critical': return 'var(--risk-critical)';
      default: return 'var(--text-muted)';
    }
  }

  // Pre-analyze on mount
  if (inputPayload) {
    analyze();
  }
</script>

<div class="security-container">
  <div class="security-header">
    <h2>Anti-Quishing Threat Inspector</h2>
    <p class="subtitle">Offline Heuristic Security Analysis & Malicious Payload Detection</p>
  </div>

  <div class="input-card">
    <label for="inspect-input">Inspect Arbitrary QR Payload or URI</label>
    <div class="input-action-row">
      <input
        id="inspect-input"
        type="text"
        bind:value={inputPayload}
        on:keydown={(e) => e.key === 'Enter' && analyze()}
        placeholder="Paste QR payload, URL, or script command to evaluate..."
        class="inspect-input"
      />
      <button class="btn-analyze" on:click={analyze} disabled={isAnalyzing}>
        {isAnalyzing ? 'Analyzing...' : 'Analyze Threat'}
      </button>
    </div>
  </div>

  {#if errorMessage}
    <div class="error-banner">⚠️ {errorMessage}</div>
  {/if}

  {#if report}
    <div class="analysis-grid">
      <!-- Left: Dial & Score -->
      <div class="score-card">
        <div class="score-circle" style="border-color: {getRiskColor(report.risk_level)};">
          <span class="score-num" style="color: {getRiskColor(report.risk_level)};">{report.risk_score}</span>
          <span class="score-label">/ 100</span>
        </div>
        <div class="level-tag" style="background: {getRiskColor(report.risk_level)}22; color: {getRiskColor(report.risk_level)}; border: 1px solid {getRiskColor(report.risk_level)}55;">
          {report.risk_level} Risk
        </div>
        <div class="airgap-pill" class:safe={report.is_air_gap_safe}>
          {report.is_air_gap_safe ? '✓ Air-Gap Safe' : '⚠️ Potential Network Egress Threat'}
        </div>
      </div>

      <!-- Right: Detailed Findings & Categories -->
      <div class="findings-card">
        <h3>Detected Security Categories</h3>
        {#if report.threat_categories.length > 0}
          <div class="categories-wrap">
            {#each report.threat_categories as cat}
              <span class="cat-pill">{cat}</span>
            {/each}
          </div>
        {:else}
          <p class="clean-text">No malicious threat categories matched.</p>
        {/if}

        <h3 class="findings-title">Detailed Heuristic Findings</h3>
        {#if report.findings.length > 0}
          <div class="findings-list">
            {#each report.findings as item}
              <div class="finding-item">
                <span class="bullet">⚠️</span>
                <span>{item}</span>
              </div>
            {/each}
          </div>
        {:else}
          <div class="finding-item safe-item">
            <span class="bullet">✓</span>
            <span>No heuristic threat anomalies detected. Payload conforms to offline safety standards.</span>
          </div>
        {/if}

        {#if report.sanitized_url}
          <div class="sanitized-box">
            <span class="box-label">Sanitized Target URL</span>
            <div class="sanitized-val">{report.sanitized_url}</div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .security-container {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    height: 100%;
    overflow-y: auto;
  }

  .security-header h2 {
    font-size: 1.25rem;
    font-weight: 700;
  }

  .subtitle {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-top: 0.25rem;
  }

  .input-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .input-card label {
    font-size: 0.8rem;
    font-weight: 700;
    color: var(--text-secondary);
    text-transform: uppercase;
  }

  .input-action-row {
    display: flex;
    gap: 0.75rem;
  }

  .inspect-input {
    flex: 1;
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: 6px;
    padding: 0.6rem 0.85rem;
    font-size: 0.9rem;
    font-family: var(--font-mono);
  }

  .inspect-input:focus {
    outline: none;
    border-color: var(--accent-indigo);
  }

  .btn-analyze {
    background: var(--accent-indigo);
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 0 1.25rem;
    font-weight: 600;
    font-size: 0.9rem;
    cursor: pointer;
  }

  .btn-analyze:hover {
    background: #4f46e5;
  }

  .error-banner {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: #fca5a5;
    padding: 0.75rem 1rem;
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .analysis-grid {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: 1.5rem;
  }

  .score-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 2rem 1.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.25rem;
  }

  .score-circle {
    width: 140px;
    height: 140px;
    border-radius: 50%;
    border: 6px solid var(--accent-emerald);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background: var(--bg-elevated);
  }

  .score-num {
    font-size: 2.75rem;
    font-weight: 800;
    font-family: var(--font-mono);
  }

  .score-label {
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .level-tag {
    font-size: 0.85rem;
    font-weight: 700;
    padding: 0.35rem 0.85rem;
    border-radius: 9999px;
    text-transform: uppercase;
  }

  .airgap-pill {
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.35rem 0.65rem;
    border-radius: 6px;
    background: rgba(239, 68, 68, 0.1);
    color: var(--accent-rose);
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  .airgap-pill.safe {
    background: rgba(16, 185, 129, 0.1);
    color: var(--accent-emerald);
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .findings-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .findings-card h3 {
    font-size: 0.95rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .categories-wrap {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .cat-pill {
    background: rgba(245, 158, 11, 0.15);
    border: 1px solid rgba(245, 158, 11, 0.3);
    color: var(--accent-amber);
    padding: 0.25rem 0.6rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .clean-text {
    font-size: 0.85rem;
    color: var(--accent-emerald);
  }

  .findings-title {
    margin-top: 0.5rem;
  }

  .findings-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .finding-item {
    display: flex;
    gap: 0.6rem;
    align-items: flex-start;
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: 0.6rem 0.8rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .finding-item.safe-item {
    color: var(--accent-emerald);
    border-color: rgba(16, 185, 129, 0.3);
  }

  .sanitized-box {
    margin-top: 0.5rem;
    background: rgba(6, 182, 212, 0.08);
    border: 1px solid rgba(6, 182, 212, 0.3);
    border-radius: 6px;
    padding: 0.75rem;
  }

  .sanitized-box .box-label {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--accent-cyan);
    text-transform: uppercase;
  }

  .sanitized-val {
    margin-top: 0.25rem;
    font-family: var(--font-mono);
    font-size: 0.85rem;
    color: var(--text-primary);
    word-break: break-all;
  }
</style>
