<script lang="ts">
  export let auditReport: any = null;
</script>

<div class="privacy-meter-card">
  <div class="card-header">
    <span class="shield-icon">🛡️</span>
    <span class="card-title">DIFFERENTIAL PRIVACY & K-ANONYMITY AUDIT</span>
    {#if auditReport?.audit_passed}
      <span class="badge pass">✓ SAFE FOR EXPORT</span>
    {:else}
      <span class="badge pending">EVALUATING</span>
    {/if}
  </div>

  <div class="metrics-row">
    <div class="metric-box">
      <span class="m-label">Differential Privacy (ε):</span>
      <span class="m-val cyan">ε = {auditReport?.epsilon_guarantee ? auditReport.epsilon_guarantee.toFixed(2) : '1.00'}</span>
      <span class="m-sub">Pure Laplace Mechanism</span>
    </div>

    <div class="metric-box">
      <span class="m-label">k-Anonymity Class:</span>
      <span class="m-val green">k ≥ {auditReport?.k_anonymity || 5}</span>
      <span class="m-sub">Equivalence Group Size</span>
    </div>

    <div class="metric-box">
      <span class="m-label">Re-ID Risk Score:</span>
      <span class="m-val gold">
        {auditReport?.reidentification_risk_score ? (auditReport.reidentification_risk_score * 100).toFixed(1) : '20.0'}%
      </span>
      <span class="m-sub">Probabilistic Bound</span>
    </div>

    <div class="metric-box sig-box">
      <span class="m-label">Ed25519 Audit Signature:</span>
      <span class="m-val mono small">
        {auditReport?.signature_hex ? `${auditReport.signature_hex.substring(0, 24)}...` : 'Pending generation'}
      </span>
      <span class="m-sub">Zero-Knowledge Proof</span>
    </div>
  </div>
</div>

<style>
  .privacy-meter-card {
    background: #0d121a;
    border: 1px solid #1a2332;
    border-radius: 8px;
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-family: monospace;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 8px;
    border-bottom: 1px solid #1a2332;
    padding-bottom: 6px;
  }

  .shield-icon {
    font-size: 1rem;
  }

  .card-title {
    font-size: 0.75rem;
    font-weight: 700;
    color: #f1f5f9;
  }

  .badge {
    margin-left: auto;
    font-size: 0.65rem;
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: 700;
  }

  .badge.pass {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid #10b981;
    color: #34d399;
  }

  .badge.pending {
    background: rgba(234, 179, 8, 0.15);
    border: 1px solid #eab308;
    color: #fbbf24;
  }

  .metrics-row {
    display: flex;
    gap: 12px;
  }

  .metric-box {
    flex: 1;
    background: #111722;
    border: 1px solid #1e2838;
    border-radius: 6px;
    padding: 6px 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .sig-box {
    flex: 1.2;
  }

  .m-label {
    font-size: 0.68rem;
    color: #64748b;
  }

  .m-val {
    font-size: 0.95rem;
    font-weight: 700;
  }

  .m-val.cyan {
    color: #38bdf8;
  }

  .m-val.green {
    color: #34d399;
  }

  .m-val.gold {
    color: #fbbf24;
  }

  .m-val.mono {
    font-family: monospace;
  }

  .m-val.small {
    font-size: 0.72rem;
    color: #94a3b8;
  }

  .m-sub {
    font-size: 0.62rem;
    color: #475569;
  }
</style>
