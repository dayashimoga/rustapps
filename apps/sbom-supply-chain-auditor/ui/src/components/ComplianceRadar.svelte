<script lang="ts">
  export let report: any = null;
</script>

<div class="compliance-radar-card">
  <div class="card-header">
    <span class="icon">🛡️</span>
    <span class="card-title">NTIA & EXECUTIVE ORDER 14028 COMPLIANCE RADAR</span>
    {#if report}
      <span class="grade-badge" class:f-grade={report.critical_cves > 0}>
        GRADE: {report.compliance_grade}
      </span>
    {/if}
  </div>

  <div class="radar-metrics">
    <div class="radar-box">
      <span class="r-label">Supply Chain Risk:</span>
      <span class="r-val" class:crit={report?.critical_cves > 0}>
        {report?.overall_risk_score !== undefined ? `${report.overall_risk_score.toFixed(1)}/100` : '100.0/100'}
      </span>
      <span class="r-sub">Calculated Risk Index</span>
    </div>

    <div class="radar-box">
      <span class="r-label">Critical CVEs:</span>
      <span class="r-val red">{report?.critical_cves || 0}</span>
      <span class="r-sub">CVSS 9.0–10.0 Exploits</span>
    </div>

    <div class="radar-box">
      <span class="r-label">High / Med CVEs:</span>
      <span class="r-val amber">{report ? report.high_cves + report.medium_cves : 0}</span>
      <span class="r-sub">CVSS 4.0–8.9 Advisories</span>
    </div>

    <div class="radar-box">
      <span class="r-label">Copyleft Violations:</span>
      <span class="r-val purple">{report?.copyleft_violations || 0}</span>
      <span class="r-sub">GPL/AGPL Risk Items</span>
    </div>

    <div class="radar-box ntia-box">
      <span class="r-label">Executive Order 14028:</span>
      <span class="r-val green">COMPLIANT</span>
      <span class="r-sub">NTIA Minimum Elements Met</span>
    </div>
  </div>
</div>

<style>
  .compliance-radar-card {
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

  .icon {
    font-size: 1rem;
  }

  .card-title {
    font-size: 0.75rem;
    font-weight: 700;
    color: #f1f5f9;
  }

  .grade-badge {
    margin-left: auto;
    font-size: 0.68rem;
    padding: 2px 8px;
    border-radius: 4px;
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid #10b981;
    color: #34d399;
    font-weight: 700;
  }

  .grade-badge.f-grade {
    background: rgba(239, 68, 68, 0.15);
    border-color: #ef4444;
    color: #f87171;
  }

  .radar-metrics {
    display: flex;
    gap: 12px;
  }

  .radar-box {
    flex: 1;
    background: #111722;
    border: 1px solid #1e2838;
    border-radius: 6px;
    padding: 6px 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .ntia-box {
    flex: 1.2;
  }

  .r-label {
    font-size: 0.68rem;
    color: #64748b;
  }

  .r-val {
    font-size: 0.95rem;
    font-weight: 700;
    color: #f1f5f9;
  }

  .r-val.crit {
    color: #ef4444;
  }

  .r-val.red {
    color: #ef4444;
  }

  .r-val.amber {
    color: #f59e0b;
  }

  .r-val.purple {
    color: #c084fc;
  }

  .r-val.green {
    color: #34d399;
  }

  .r-sub {
    font-size: 0.62rem;
    color: #475569;
  }
</style>
