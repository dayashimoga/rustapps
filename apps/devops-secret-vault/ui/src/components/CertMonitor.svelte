<script lang="ts">
  import { onMount } from 'svelte';
  import type { CertInspectionReport } from '../types';

  export let secretId: string;
  export let onInspect: (id: string) => Promise<CertInspectionReport>;

  let report: CertInspectionReport | null = null;
  let loading = true;
  let errorMsg = '';

  onMount(async () => {
    try {
      report = await onInspect(secretId);
    } catch (err: any) {
      errorMsg = err?.message || String(err);
    } finally {
      loading = false;
    }
  });

  function formatDate(epochSecs: number): string {
    if (!epochSecs) return 'N/A';
    return new Date(epochSecs * 1000).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  }
</script>

<div class="cert-monitor-card">
  <div class="monitor-header">
    <div class="header-title">
      <svg viewBox="0 0 20 20" fill="currentColor" class="cert-icon">
        <path fill-rule="evenodd" d="M10 1.944A11.954 11.954 0 012.166 5C2.056 5.649 2 6.319 2 7c0 5.225 3.34 9.67 8 11.317C14.66 16.67 18 12.225 18 7c0-.682-.057-1.35-.166-2.001A11.954 11.954 0 0110 1.944zM11 14a1 1 0 11-2 0 1 1 0 012 0zm0-7a1 1 0 10-2 0v3a1 1 0 102 0V7z" clip-rule="evenodd" />
      </svg>
      <span>X.509 Certificate Chain Inspection</span>
    </div>

    {#if report}
      <div class="status-badge status-{report.status}">
        {#if report.status === 'valid'}
          Valid ({report.days_until_expiration}d left)
        {:else if report.status === 'expiring_soon'}
          Expiring Soon ({report.days_until_expiration}d)
        {:else}
          Expired ({Math.abs(report.days_until_expiration)}d ago)
        {/if}
      </div>
    {/if}
  </div>

  {#if loading}
    <div class="loading-state">Inspecting ASN.1 certificate structure...</div>
  {:else if errorMsg}
    <div class="error-banner">{errorMsg}</div>
  {:else if report}
    <!-- Weak Crypto Warning -->
    {#if report.is_weak_signature || report.is_weak_key}
      <div class="warning-alert">
        <svg viewBox="0 0 20 20" fill="currentColor" class="alert-icon">
          <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
        </svg>
        <div class="alert-content">
          <strong>Weak Cryptography Detected</strong>
          <span>
            {#if report.is_weak_signature}Uses deprecated signature ({report.signature_algorithm}). {/if}
            {#if report.is_weak_key}Key size ({report.key_size_bits || 'Unknown'} bits) is below 2048-bit standard.{/if}
          </span>
        </div>
      </div>
    {/if}

    <div class="cert-grid">
      <div class="grid-item">
        <span class="label">Subject Common Name</span>
        <span class="value strong">{report.subject_cn}</span>
      </div>

      <div class="grid-item">
        <span class="label">Issuer Common Name</span>
        <span class="value">
          {report.issuer_cn}
          {#if report.is_self_signed}
            <span class="self-signed-badge">Self-Signed</span>
          {/if}
        </span>
      </div>

      <div class="grid-item">
        <span class="label">Validity Window</span>
        <span class="value font-mono">
          {formatDate(report.not_before_epoch)} &rarr; {formatDate(report.not_after_epoch)}
        </span>
      </div>

      <div class="grid-item">
        <span class="label">Signature & Key</span>
        <span class="value font-mono">
          {report.signature_algorithm} | {report.public_key_algorithm} {report.key_size_bits ? `(${report.key_size_bits} bits)` : ''}
        </span>
      </div>

      <div class="grid-item full-width">
        <span class="label">Serial Number</span>
        <span class="value font-mono serial">{report.serial_number || 'N/A'}</span>
      </div>

      {#if report.subject_alt_names && report.subject_alt_names.length > 0}
        <div class="grid-item full-width">
          <span class="label">Subject Alternative Names (SANs)</span>
          <div class="sans-list">
            {#each report.subject_alt_names as san}
              <span class="san-chip">{san}</span>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .cert-monitor-card {
    background: rgba(15, 23, 42, 0.7);
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 12px;
    padding: 16px;
    margin-top: 12px;
    backdrop-filter: blur(12px);
  }

  .monitor-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 700;
    color: #f1f5f9;
  }

  .cert-icon {
    width: 16px;
    height: 16px;
    color: #38bdf8;
  }

  .status-badge {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 3px 8px;
    border-radius: 6px;
  }

  .status-valid {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .status-expiring_soon {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
    border: 1px solid rgba(245, 158, 11, 0.3);
  }

  .status-expired {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  .warning-alert {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 8px;
    padding: 10px 12px;
    margin-bottom: 14px;
    color: #fca5a5;
    font-size: 12px;
  }

  .alert-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    color: #ef4444;
  }

  .alert-content strong {
    display: block;
    color: #fee2e2;
    margin-bottom: 2px;
  }

  .cert-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .grid-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .grid-item.full-width {
    grid-column: 1 / -1;
  }

  .label {
    font-size: 11px;
    font-weight: 600;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .value {
    font-size: 13px;
    color: #e2e8f0;
  }

  .value.strong {
    font-weight: 600;
    color: #f8fafc;
  }

  .font-mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 12px;
  }

  .serial {
    word-break: break-all;
    color: #94a3b8;
  }

  .self-signed-badge {
    background: rgba(148, 163, 184, 0.2);
    color: #cbd5e1;
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 4px;
    margin-left: 6px;
  }

  .sans-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 4px;
  }

  .san-chip {
    background: rgba(56, 189, 248, 0.1);
    border: 1px solid rgba(56, 189, 248, 0.2);
    color: #7dd3fc;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 4px;
  }

  .loading-state {
    font-size: 13px;
    color: #94a3b8;
    text-align: center;
    padding: 12px 0;
  }

  .error-banner {
    color: #f87171;
    font-size: 12px;
    padding: 8px 12px;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 6px;
  }
</style>
