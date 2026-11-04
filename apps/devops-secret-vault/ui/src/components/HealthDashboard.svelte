<script lang="ts">
  import { onMount } from 'svelte';
  import type { VaultHealthReport, SecurityRecommendation } from '../types';
  import { invokeIpc } from '../lib/ipc';

  export let onFetchHealth: () => Promise<VaultHealthReport>;
  export let onSelectSecret: (secretId: string) => void;
  export let onClose: () => void;
  export let onVerifyAuditChain: (() => Promise<boolean>) | undefined = undefined;

  let report: VaultHealthReport | null = null;
  let loading = true;
  let errorMsg = '';

  let auditStatus: 'unverified' | 'verifying' | 'valid' | 'tampered' = 'unverified';
  let isVerifyingAudit = false;

  async function verifyAuditChain() {
    isVerifyingAudit = true;
    auditStatus = 'verifying';
    try {
      let ok: boolean;
      if (onVerifyAuditChain) {
        ok = await onVerifyAuditChain();
      } else {
        ok = await invokeIpc<boolean>('verify_audit_chain', {});
      }
      auditStatus = ok ? 'valid' : 'tampered';
    } catch {
      auditStatus = 'tampered';
    } finally {
      isVerifyingAudit = false;
    }
  }

  async function loadHealth() {
    loading = true;
    errorMsg = '';
    try {
      report = await onFetchHealth();
      await verifyAuditChain();
    } catch (err: unknown) {
      const e = err as { message?: string };
      errorMsg = e?.message || String(err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadHealth();
  });

  function getGradeColor(grade: string): string {
    switch (grade) {
      case 'A+':
      case 'A':
        return '#10b981'; // emerald
      case 'B':
        return '#38bdf8'; // sky
      case 'C':
        return '#f59e0b'; // amber
      case 'D':
        return '#f97316'; // orange
      default:
        return '#ef4444'; // red
    }
  }

  function getSeverityClass(sev: string): string {
    switch (sev.toUpperCase()) {
      case 'CRITICAL':
        return 'sev-critical';
      case 'HIGH':
        return 'sev-high';
      case 'MEDIUM':
        return 'sev-medium';
      default:
        return 'sev-low';
    }
  }
</script>

<div class="health-modal-overlay">
  <div class="health-modal">
    <div class="modal-header">
      <div class="title-wrap">
        <svg viewBox="0 0 20 20" fill="currentColor" class="header-icon">
          <path fill-rule="evenodd" d="M2.166 4.999A11.954 11.954 0 0010 1.944 11.954 11.954 0 0017.834 5c.11.65.166 1.32.166 2.001 0 5.225-3.34 9.67-8 11.317C5.34 16.67 2 12.225 2 7c0-.682.057-1.35.166-2.001zm11.541 3.708a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
        </svg>
        <div>
          <h2>Security Health & Hygiene Dashboard</h2>
          <p class="subtitle">On-device 5-dimensional cryptographic posture evaluation</p>
        </div>
      </div>

      <div class="header-actions">
        <button type="button" class="refresh-btn" on:click={loadHealth} disabled={loading}>
          <svg viewBox="0 0 20 20" fill="currentColor" class="btn-icon">
            <path fill-rule="evenodd" d="M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z" clip-rule="evenodd" />
          </svg>
          Re-evaluate
        </button>
        <button type="button" class="close-btn" on:click={onClose}>&times;</button>
      </div>
    </div>

    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <span>Evaluating entropy, keyed hashes, rotation age, and X.509 chains...</span>
      </div>
    {:else if errorMsg}
      <div class="error-box">{errorMsg}</div>
    {:else if report}
      <div class="modal-body">
        <!-- Top Score Section -->
        <div class="score-banner">
          <div class="dial-container">
            <svg viewBox="0 0 120 120" class="score-dial">
              <circle cx="60" cy="60" r="50" class="dial-bg" />
              <circle
                cx="60"
                cy="60"
                r="50"
                class="dial-fill"
                style="stroke: {getGradeColor(report.grade)}; stroke-dasharray: 314; stroke-dashoffset: {314 - (report.overall_score / 100) * 314};"
              />
            </svg>
            <div class="dial-text">
              <span class="score-grade" style="color: {getGradeColor(report.grade)}">{report.grade}</span>
              <span class="score-num">{report.overall_score}%</span>
            </div>
          </div>

          <div class="score-details">
            <h3>Overall Security Posture</h3>
            <p>
              Analysis completed entirely offline across <strong>{report.total_secrets}</strong> secrets.
              {#if report.overall_score >= 90}
                Your vault adheres to enterprise security standards with zero known exposed defaults.
              {:else if report.overall_score >= 70}
                Good security baseline, but actionable recommendations require remediation.
              {:else}
                Critical security vulnerabilities detected. Rotate compromised or reused credentials immediately.
              {/if}
            </p>
          </div>
        </div>

        <!-- Cryptographic Audit Chain Verification Card -->
        <div class="audit-chain-card {auditStatus}">
          <div class="audit-left">
            <div class="chain-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="22" height="22">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
              </svg>
            </div>
            <div class="audit-info">
              <h4>Tamper-Evident Audit Hash Chain (SHA-256)</h4>
              <p class="audit-desc">
                {#if auditStatus === 'valid'}
                  Cryptographic linear hash chain verified intact from genesis block. Zero records tampered or altered.
                {:else if auditStatus === 'tampered'}
                  ⚠️ Cryptographic hash mismatch detected! An audit record was modified, reordered, or deleted in SQLite.
                {:else if auditStatus === 'verifying'}
                  Verifying SHA-256 sequential entry hashes across all historical audit records...
                {:else}
                  Verifies unbroken linear cryptographic hashes against unauthorized SQLite record manipulation.
                {/if}
              </p>
            </div>
          </div>

          <div class="audit-right">
            {#if auditStatus === 'valid'}
              <span class="status-pill valid">✓ Chain Intact</span>
            {:else if auditStatus === 'tampered'}
              <span class="status-pill tampered">⚠️ Hash Mismatch</span>
            {/if}
            <button
              type="button"
              class="verify-chain-btn"
              on:click={verifyAuditChain}
              disabled={isVerifyingAudit}
            >
              {#if isVerifyingAudit}
                <span class="spinner-sm"></span> Verifying...
              {:else}
                Verify Audit Chain
              {/if}
            </button>
          </div>
        </div>

        <!-- 5 Category Metrics -->
        <div class="metrics-grid">
          <div class="metric-card" class:has-issues={report.compromised_count > 0}>
            <span class="metric-num">{report.compromised_count}</span>
            <span class="metric-title">Exposed / Defaults</span>
            <span class="metric-desc">Matched offline known list</span>
          </div>

          <div class="metric-card" class:has-issues={report.reused_count > 0}>
            <span class="metric-num">{report.reused_count}</span>
            <span class="metric-title">Shared / Reused</span>
            <span class="metric-desc">Zero-knowledge detected</span>
          </div>

          <div class="metric-card" class:has-issues={report.weak_count > 0}>
            <span class="metric-num">{report.weak_count}</span>
            <span class="metric-title">Weak Entropy</span>
            <span class="metric-desc">&lt; 12 chars / 40 bits</span>
          </div>

          <div class="metric-card" class:has-issues={report.stale_count > 0}>
            <span class="metric-num">{report.stale_count}</span>
            <span class="metric-title">Stale Credentials</span>
            <span class="metric-desc">&gt; 90 days rotation age</span>
          </div>

          <div class="metric-card" class:has-issues={report.cert_issues_count > 0}>
            <span class="metric-num">{report.cert_issues_count}</span>
            <span class="metric-title">Certificate Alerts</span>
            <span class="metric-desc">Expiring / Weak algos</span>
          </div>
        </div>

        <!-- Actionable Recommendations -->
        <div class="recommendations-section">
          <h4>Prioritized Remediation Recommendations ({report.recommendations.length})</h4>

          {#if report.recommendations.length === 0}
            <div class="empty-recs">
              <svg viewBox="0 0 20 20" fill="currentColor" class="empty-icon">
                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
              </svg>
              <span>Zero vulnerabilities detected! Excellent cryptographic hygiene.</span>
            </div>
          {:else}
            <div class="recs-list">
              {#each report.recommendations as rec}
                <div class="rec-card {getSeverityClass(rec.severity)}">
                  <div class="rec-header">
                    <span class="sev-badge">{rec.severity}</span>
                    <span class="rec-title">{rec.title}</span>
                  </div>
                  <p class="rec-desc">{rec.description}</p>
                  <div class="rec-footer">
                    <span class="rec-target">Secret: <strong>{rec.secret_title}</strong></span>
                    <button
                      type="button"
                      class="jump-btn"
                      on:click={() => {
                        onSelectSecret(rec.secret_id);
                        onClose();
                      }}
                    >
                      Jump to Secret &rarr;
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .health-modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .health-modal {
    width: 860px;
    max-width: 95vw;
    max-height: 90vh;
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6);
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 24px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .title-wrap {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .header-icon {
    width: 28px;
    height: 28px;
    color: #38bdf8;
  }

  h2 {
    font-size: 18px;
    font-weight: 700;
    color: #f8fafc;
    margin: 0;
  }

  .subtitle {
    font-size: 12px;
    color: #94a3b8;
    margin: 2px 0 0 0;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .refresh-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(56, 189, 248, 0.12);
    border: 1px solid rgba(56, 189, 248, 0.25);
    color: #e0f2fe;
    font-size: 12px;
    font-weight: 600;
    padding: 6px 12px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .refresh-btn:hover {
    background: rgba(56, 189, 248, 0.22);
  }

  .btn-icon {
    width: 14px;
    height: 14px;
  }

  .close-btn {
    background: none;
    border: none;
    color: #94a3b8;
    font-size: 24px;
    cursor: pointer;
  }

  .close-btn:hover {
    color: #f1f5f9;
  }

  .modal-body {
    padding: 24px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .score-banner {
    display: flex;
    align-items: center;
    gap: 24px;
    background: rgba(30, 41, 59, 0.5);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 16px 20px;
  }

  .dial-container {
    position: relative;
    width: 90px;
    height: 90px;
    flex-shrink: 0;
  }

  .score-dial {
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
  }

  .dial-bg {
    fill: none;
    stroke: rgba(255, 255, 255, 0.08);
    stroke-width: 8;
  }

  .dial-fill {
    fill: none;
    stroke-width: 8;
    stroke-linecap: round;
    transition: stroke-dashoffset 1s ease;
  }

  .dial-text {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }

  .score-grade {
    font-size: 24px;
    font-weight: 800;
    line-height: 1;
  }

  .score-num {
    font-size: 11px;
    color: #94a3b8;
    font-weight: 600;
  }

  .score-details h3 {
    margin: 0 0 6px 0;
    font-size: 16px;
    color: #f8fafc;
  }

  .score-details p {
    margin: 0;
    font-size: 13px;
    color: #94a3b8;
    line-height: 1.5;
  }

  .audit-chain-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 18px;
    border-radius: 10px;
    background: rgba(30, 41, 59, 0.45);
    border: 1px solid rgba(255, 255, 255, 0.08);
    gap: 16px;
  }

  .audit-chain-card.valid {
    border-color: rgba(16, 185, 129, 0.35);
    background: rgba(16, 185, 129, 0.05);
  }

  .audit-chain-card.tampered {
    border-color: rgba(239, 68, 68, 0.45);
    background: rgba(239, 68, 68, 0.08);
  }

  .audit-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .chain-icon {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    background: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .audit-chain-card.valid .chain-icon {
    background: rgba(16, 185, 129, 0.15);
    color: #10b981;
  }

  .audit-chain-card.tampered .chain-icon {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
  }

  .audit-info h4 {
    margin: 0 0 3px 0;
    font-size: 14px;
    font-weight: 600;
    color: #f8fafc;
  }

  .audit-desc {
    margin: 0;
    font-size: 12px;
    color: #94a3b8;
    line-height: 1.4;
  }

  .audit-right {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .status-pill {
    font-size: 11px;
    font-weight: 700;
    padding: 3px 8px;
    border-radius: 4px;
    text-transform: uppercase;
  }

  .status-pill.valid {
    background: rgba(16, 185, 129, 0.15);
    color: #10b981;
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .status-pill.tampered {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  .verify-chain-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: #e2e8f0;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .verify-chain-btn:hover:not(:disabled) {
    background: rgba(56, 189, 248, 0.15);
    border-color: rgba(56, 189, 248, 0.4);
    color: #38bdf8;
  }

  .verify-chain-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .spinner-sm {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(255, 255, 255, 0.2);
    border-top-color: #38bdf8;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 12px;
  }

  .metric-card {
    background: rgba(30, 41, 59, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 10px;
    padding: 12px;
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .metric-card.has-issues {
    border-color: rgba(239, 68, 68, 0.4);
    background: rgba(239, 68, 68, 0.06);
  }

  .metric-num {
    font-size: 24px;
    font-weight: 800;
    color: #f8fafc;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  }

  .metric-card.has-issues .metric-num {
    color: #f87171;
  }

  .metric-title {
    font-size: 12px;
    font-weight: 600;
    color: #e2e8f0;
  }

  .metric-desc {
    font-size: 10px;
    color: #64748b;
  }

  .recommendations-section h4 {
    margin: 0 0 12px 0;
    font-size: 14px;
    font-weight: 700;
    color: #cbd5e1;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .recs-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .rec-card {
    background: rgba(30, 41, 59, 0.5);
    border-left: 4px solid #38bdf8;
    border-radius: 8px;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .rec-card.sev-critical {
    border-left-color: #ef4444;
    background: rgba(239, 68, 68, 0.08);
  }

  .rec-card.sev-high {
    border-left-color: #f97316;
    background: rgba(249, 115, 22, 0.08);
  }

  .rec-card.sev-medium {
    border-left-color: #f59e0b;
    background: rgba(245, 158, 11, 0.08);
  }

  .rec-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .sev-badge {
    font-size: 10px;
    font-weight: 800;
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
  }

  .sev-critical .sev-badge {
    background: rgba(239, 68, 68, 0.2);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  .sev-high .sev-badge {
    background: rgba(249, 115, 22, 0.2);
    color: #fb923c;
    border: 1px solid rgba(249, 115, 22, 0.3);
  }

  .sev-medium .sev-badge {
    background: rgba(245, 158, 11, 0.2);
    color: #fcd34d;
    border: 1px solid rgba(245, 158, 11, 0.3);
  }

  .rec-title {
    font-size: 13px;
    font-weight: 600;
    color: #f1f5f9;
  }

  .rec-desc {
    margin: 0;
    font-size: 12px;
    color: #cbd5e1;
    line-height: 1.4;
  }

  .rec-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 4px;
    font-size: 12px;
  }

  .rec-target {
    color: #94a3b8;
  }

  .jump-btn {
    background: none;
    border: none;
    color: #38bdf8;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    padding: 0;
  }

  .jump-btn:hover {
    text-decoration: underline;
  }

  .empty-recs {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(16, 185, 129, 0.1);
    border: 1px solid rgba(16, 185, 129, 0.25);
    color: #34d399;
    padding: 16px;
    border-radius: 8px;
    font-size: 13px;
  }

  .empty-icon {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
  }

  .loading-state {
    padding: 48px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    color: #94a3b8;
  }

  .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid rgba(56, 189, 248, 0.2);
    border-top-color: #38bdf8;
    border-radius: 50%;
    animation: spin 1s infinite linear;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
