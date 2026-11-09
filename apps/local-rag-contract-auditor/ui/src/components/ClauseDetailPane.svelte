<script lang="ts">
  import type { ClauseAuditFinding } from '../lib/types';

  export let clause: ClauseAuditFinding | null = null;

  let copied = false;

  function copyRedline() {
    if (!clause?.suggested_redline_text) return;
    navigator.clipboard.writeText(clause.suggested_redline_text);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 2000);
  }
</script>

{#if clause}
  <div class="clause-detail-container">
    <div class="detail-header">
      <div class="title-group">
        <div class="sec-meta">
          <span class="sec-num">Section {clause.section_number}</span>
          <span class="cat-pill">{clause.category.replace(/_/g, ' ')}</span>
        </div>
        <h2 class="heading">{clause.heading}</h2>
      </div>

      <div class="score-card">
        <div class="gauge-label">RISK SCORE</div>
        <div
          class="gauge-value"
          class:crit={clause.risk_level === 'critical'}
          class:high={clause.risk_level === 'high'}
          class:med={clause.risk_level === 'medium'}
          class:low={clause.risk_level === 'low'}
          class:fav={clause.risk_level === 'favorable'}
        >
          {clause.risk_score.toFixed(1)} <span class="denom">/ 10</span>
        </div>
        <div class="level-badge {clause.risk_level}">{clause.risk_level.toUpperCase()}</div>
      </div>
    </div>

    <div class="section-card original">
      <div class="card-header">
        <span class="card-icon">📄</span>
        <span class="card-title">CURRENT CONTRACT CLAUSE TEXT</span>
      </div>
      <div class="card-body">
        <p class="clause-text">{clause.raw_text}</p>
      </div>
    </div>

    <div class="grid-two">
      <div class="section-card rationale">
        <div class="card-header">
          <span class="card-icon">⚠️</span>
          <span class="card-title">LEGAL RATIONALE & RISK EXPOSURE</span>
        </div>
        <div class="card-body">
          <p>{clause.rationale}</p>
        </div>
      </div>

      <div class="section-card benchmark">
        <div class="card-header">
          <span class="card-icon">⚖️</span>
          <span class="card-title">MARKET STANDARD BENCHMARK</span>
        </div>
        <div class="card-body">
          <p>{clause.market_standard_benchmark}</p>
        </div>
      </div>
    </div>

    {#if clause.suggested_redline_text}
      <div class="section-card redline">
        <div class="card-header redline-hdr">
          <div class="rh-left">
            <span class="card-icon">✏️</span>
            <span class="card-title">SUGGESTED INSTITUTIONAL REDLINE</span>
          </div>
          <button type="button" class="copy-btn" on:click={copyRedline}>
            {copied ? '✓ Copied!' : 'Copy Redline Text'}
          </button>
        </div>
        <div class="card-body">
          <pre class="redline-text">{clause.suggested_redline_text}</pre>
        </div>
      </div>
    {/if}
  </div>
{:else}
  <div class="empty-detail">
    <div class="empty-icon">📜</div>
    <h3>No Clause Selected</h3>
    <p>Select a clause from the left checklist to view risk scoring, market benchmarks, and suggested redlines.</p>
  </div>
{/if}

<style>
  .clause-detail-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 20px;
    overflow-y: auto;
    height: 100%;
  }

  .detail-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding-bottom: 16px;
    border-bottom: 1px solid #1e293b;
  }

  .title-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .sec-meta {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .sec-num {
    background: #1e293b;
    color: #38bdf8;
    padding: 3px 10px;
    border-radius: 4px;
    font-size: 0.8rem;
    font-weight: 700;
  }

  .cat-pill {
    background: #132338;
    color: #93c5fd;
    padding: 3px 10px;
    border-radius: 4px;
    font-size: 0.8rem;
    text-transform: capitalize;
  }

  .heading {
    margin: 0;
    font-size: 1.35rem;
    font-weight: 700;
    color: #f8fafc;
  }

  .score-card {
    background: #060a12;
    border: 1px solid #1e293b;
    border-radius: 8px;
    padding: 10px 18px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .gauge-label {
    font-size: 0.68rem;
    font-weight: 700;
    color: #64748b;
    letter-spacing: 0.08em;
  }

  .gauge-value {
    font-size: 1.6rem;
    font-weight: 800;
    line-height: 1.1;
  }

  .denom {
    font-size: 0.8rem;
    font-weight: 500;
    color: #64748b;
  }

  .gauge-value.crit { color: #ef4444; }
  .gauge-value.high { color: #f97316; }
  .gauge-value.med { color: #f59e0b; }
  .gauge-value.low { color: #3b82f6; }
  .gauge-value.fav { color: #10b981; }

  .level-badge {
    font-size: 0.72rem;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 4px;
  }

  .level-badge.critical { background: rgba(239, 68, 68, 0.2); color: #f87171; }
  .level-badge.high { background: rgba(249, 115, 22, 0.2); color: #fb923c; }
  .level-badge.medium { background: rgba(245, 158, 11, 0.2); color: #fcd34d; }
  .level-badge.low { background: rgba(59, 130, 246, 0.2); color: #60a5fa; }
  .level-badge.favorable { background: rgba(16, 185, 129, 0.2); color: #34d399; }

  .section-card {
    background: #090e18;
    border: 1px solid #1e293b;
    border-radius: 8px;
    overflow: hidden;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    background: #060a12;
    border-bottom: 1px solid #1a2336;
  }

  .card-icon {
    font-size: 0.95rem;
  }

  .card-title {
    font-size: 0.76rem;
    font-weight: 700;
    color: #94a3b8;
    letter-spacing: 0.05em;
  }

  .card-body {
    padding: 16px;
    font-size: 0.9rem;
    line-height: 1.6;
    color: #cbd5e1;
  }

  .clause-text {
    margin: 0;
    white-space: pre-wrap;
    font-family: inherit;
  }

  .grid-two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }

  .rationale {
    border-color: rgba(239, 68, 68, 0.25);
  }

  .benchmark {
    border-color: rgba(59, 130, 246, 0.25);
  }

  .redline {
    border-color: rgba(16, 185, 129, 0.3);
  }

  .redline-hdr {
    justify-content: space-between;
  }

  .rh-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .copy-btn {
    background: #10b981;
    color: #022c22;
    border: none;
    padding: 4px 12px;
    border-radius: 4px;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s ease;
  }

  .copy-btn:hover {
    opacity: 0.9;
  }

  .redline-text {
    margin: 0;
    white-space: pre-wrap;
    font-family: inherit;
    color: #a7f3d0;
    font-weight: 500;
  }

  .empty-detail {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    color: #64748b;
    text-align: center;
    padding: 40px;
  }

  .empty-icon {
    font-size: 3rem;
    margin-bottom: 12px;
    opacity: 0.4;
  }

  .empty-detail h3 {
    color: #94a3b8;
    margin: 0 0 6px 0;
  }

  .empty-detail p {
    font-size: 0.88rem;
    margin: 0;
    max-width: 380px;
  }
</style>
