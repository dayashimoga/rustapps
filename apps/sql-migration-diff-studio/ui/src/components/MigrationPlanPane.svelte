<script lang="ts">
  import type { MigrationPlan } from '../lib/types';

  export let plan: MigrationPlan | null = null;
  export let onSignManifest: () => void = () => {};

  let activeTab: 'changes' | 'up_script' | 'down_script' = 'changes';
  let copyFeedback = '';

  function copyText(text: string, label: string) {
    navigator.clipboard.writeText(text);
    copyFeedback = `${label} copied!`;
    setTimeout(() => {
      copyFeedback = '';
    }, 2000);
  }
</script>

{#if plan}
  <div class="plan-pane-shell">
    <!-- Header Banner -->
    <div class="plan-header">
      <div class="ph-meta">
        <div class="plan-title-row">
          <h2 class="plan-id mono font-semibold">{plan.plan_id}</h2>
          <span class="risk-badge {plan.overall_risk}">
            {plan.overall_risk.replace(/_/g, ' ').toUpperCase()}
          </span>
        </div>
        <div class="schemas-ref">
          <span class="ref-tag base">Base: {plan.baseline_schema_id}</span>
          <span class="arrow">➔</span>
          <span class="ref-tag target">Target: {plan.target_schema_id}</span>
        </div>
      </div>

      <!-- Quick Metrics -->
      <div class="metrics-cluster">
        <div class="m-box">
          <span class="num">{plan.total_operations}</span>
          <span class="lbl">Operations</span>
        </div>
        <div class="m-box crit">
          <span class="num">{plan.breaking_changes_count}</span>
          <span class="lbl">Breaking</span>
        </div>
        <div class="m-box warn">
          <span class="num">{plan.warning_changes_count}</span>
          <span class="lbl">Warnings</span>
        </div>
        <div class="m-box safe">
          <span class="num">{plan.safe_changes_count}</span>
          <span class="lbl">Safe Online</span>
        </div>
      </div>

      <div class="ph-actions">
        <button type="button" class="btn manifest-btn" on:click={onSignManifest}>
          🛡️ Issue Integrity Manifest
        </button>
      </div>
    </div>

    <!-- Navigation Tabs -->
    <div class="nav-tabs-bar">
      <div class="tabs-left">
        <button
          type="button"
          class="tab-btn"
          class:active={activeTab === 'changes'}
          on:click={() => (activeTab = 'changes')}
        >
          Operations ({plan.changes.length})
        </button>
        <button
          type="button"
          class="tab-btn"
          class:active={activeTab === 'up_script'}
          on:click={() => (activeTab = 'up_script')}
        >
          Reversible UP Script
        </button>
        <button
          type="button"
          class="tab-btn"
          class:active={activeTab === 'down_script'}
          on:click={() => (activeTab = 'down_script')}
        >
          Rollback DOWN Script
        </button>
      </div>

      {#if copyFeedback}
        <span class="copy-alert">{copyFeedback}</span>
      {/if}
    </div>

    <!-- Tab Contents -->
    <div class="tab-body">
      {#if activeTab === 'changes'}
        <div class="changes-list">
          {#each plan.changes as change, i}
            <div class="change-card {change.risk_level}">
              <div class="card-top">
                <div class="ct-left">
                  <span class="step-num">Step {i + 1}</span>
                  <span class="change-summary">{change.summary}</span>
                </div>
                <span class="card-risk-pill {change.risk_level}">
                  {change.risk_level.replace(/_/g, ' ').toUpperCase()}
                </span>
              </div>

              <div class="guidance-box">
                <span class="g-icon">💡</span>
                <span class="g-text">{change.safety_guidance}</span>
              </div>

              <div class="sql-snippets-grid">
                <div class="sql-box up">
                  <div class="box-hdr">
                    <span>UP MIGRATION SQL</span>
                    <button
                      type="button"
                      class="copy-mini-btn"
                      on:click={() => copyText(change.up_sql, 'UP snippet')}
                    >
                      Copy
                    </button>
                  </div>
                  <pre class="sql-text mono">{change.up_sql}</pre>
                </div>

                <div class="sql-box down">
                  <div class="box-hdr">
                    <span>DOWN ROLLBACK SQL</span>
                    <button
                      type="button"
                      class="copy-mini-btn"
                      on:click={() => copyText(change.down_sql, 'DOWN snippet')}
                    >
                      Copy
                    </button>
                  </div>
                  <pre class="sql-text mono">{change.down_sql}</pre>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {:else if activeTab === 'up_script'}
        <div class="full-script-view">
          <div class="script-toolbar">
            <span class="st-label">TRANSACTIONAL REVERSIBLE MIGRATION SCRIPT (UP.SQL)</span>
            <button
              type="button"
              class="btn primary"
              on:click={() => copyText(plan.up_script, 'UP script')}
            >
              Copy Full UP Script
            </button>
          </div>
          <pre class="script-code mono">{plan.up_script}</pre>
        </div>
      {:else if activeTab === 'down_script'}
        <div class="full-script-view">
          <div class="script-toolbar">
            <span class="st-label">TRANSACTIONAL ROLLBACK SCRIPT (DOWN.SQL)</span>
            <button
              type="button"
              class="btn secondary"
              on:click={() => copyText(plan.down_script, 'DOWN script')}
            >
              Copy Rollback DOWN Script
            </button>
          </div>
          <pre class="script-code mono">{plan.down_script}</pre>
        </div>
      {/if}
    </div>
  </div>
{:else}
  <div class="empty-plan">
    <div class="empty-icon">⚖️</div>
    <h3>No Migration Plan Generated</h3>
    <p>Select a baseline and target database schema, then click "Run Differential Comparison" to synthesize reversible migration scripts.</p>
  </div>
{/if}

<style>
  .plan-pane-shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #030712;
  }

  .plan-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    background: #090e18;
    border-bottom: 1px solid #1a2336;
  }

  .ph-meta {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .plan-title-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .plan-id {
    margin: 0;
    font-size: 1.15rem;
    color: #38bdf8;
  }

  .risk-badge {
    padding: 3px 10px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 700;
  }

  .risk-badge.hazardous_breaking {
    background: rgba(239, 68, 68, 0.2);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.4);
  }

  .risk-badge.warning {
    background: rgba(245, 158, 11, 0.2);
    color: #fcd34d;
    border: 1px solid rgba(245, 158, 11, 0.4);
  }

  .risk-badge.safe_online {
    background: rgba(16, 185, 129, 0.2);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.4);
  }

  .schemas-ref {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.78rem;
  }

  .ref-tag {
    background: #060a12;
    padding: 2px 8px;
    border-radius: 4px;
    border: 1px solid #1e293b;
    font-family: monospace;
    color: #cbd5e1;
  }

  .arrow {
    color: #64748b;
  }

  .metrics-cluster {
    display: flex;
    align-items: center;
    gap: 10px;
    background: #060a12;
    padding: 6px 14px;
    border-radius: 8px;
    border: 1px solid #151f33;
  }

  .m-box {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .m-box .num {
    font-size: 0.95rem;
    font-weight: 800;
    color: #f1f5f9;
  }

  .m-box.crit .num { color: #ef4444; }
  .m-box.warn .num { color: #f59e0b; }
  .m-box.safe .num { color: #10b981; }

  .m-box .lbl {
    font-size: 0.65rem;
    color: #64748b;
    text-transform: uppercase;
  }

  .manifest-btn {
    background: rgba(109, 40, 217, 0.2);
    border: 1px solid #6d28d9;
    color: #c4b5fd;
    padding: 8px 14px;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
  }

  .manifest-btn:hover {
    background: rgba(109, 40, 217, 0.35);
  }

  .nav-tabs-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: #070b14;
    border-bottom: 1px solid #151f33;
    padding: 0 16px;
  }

  .tabs-left {
    display: flex;
  }

  .tab-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    padding: 10px 16px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    border-bottom: 2px solid transparent;
  }

  .tab-btn.active {
    color: #38bdf8;
    border-bottom-color: #38bdf8;
  }

  .copy-alert {
    color: #34d399;
    font-size: 0.78rem;
    font-weight: 600;
  }

  .tab-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  .changes-list {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .change-card {
    background: #070b14;
    border: 1px solid #1a2336;
    border-radius: 8px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .change-card.hazardous_breaking {
    border-left: 4px solid #ef4444;
  }

  .change-card.warning {
    border-left: 4px solid #f59e0b;
  }

  .change-card.safe_online {
    border-left: 4px solid #10b981;
  }

  .card-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .ct-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .step-num {
    background: #1e293b;
    color: #38bdf8;
    font-size: 0.72rem;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 4px;
  }

  .change-summary {
    font-size: 0.92rem;
    font-weight: 600;
    color: #f1f5f9;
  }

  .card-risk-pill {
    font-size: 0.68rem;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 4px;
  }

  .card-risk-pill.hazardous_breaking { background: rgba(239, 68, 68, 0.2); color: #f87171; }
  .card-risk-pill.warning { background: rgba(245, 158, 11, 0.2); color: #fcd34d; }
  .card-risk-pill.safe_online { background: rgba(16, 185, 129, 0.2); color: #34d399; }

  .guidance-box {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    background: #0a0f1d;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.8rem;
    color: #cbd5e1;
    line-height: 1.4;
  }

  .sql-snippets-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .sql-box {
    background: #04070e;
    border: 1px solid #151f33;
    border-radius: 6px;
    overflow: hidden;
  }

  .box-hdr {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 10px;
    background: #070b14;
    font-size: 0.68rem;
    font-weight: 700;
    color: #64748b;
    border-bottom: 1px solid #151f33;
    letter-spacing: 0.05em;
  }

  .copy-mini-btn {
    background: #1e293b;
    border: none;
    color: #94a3b8;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 0.65rem;
    cursor: pointer;
  }

  .copy-mini-btn:hover {
    background: #2563eb;
    color: #fff;
  }

  .sql-text {
    margin: 0;
    padding: 10px;
    font-size: 0.8rem;
    color: #e2e8f0;
    white-space: pre-wrap;
  }

  .full-script-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #04070e;
    border: 1px solid #1a2336;
    border-radius: 8px;
    overflow: hidden;
  }

  .script-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 16px;
    background: #090e18;
    border-bottom: 1px solid #1a2336;
  }

  .st-label {
    font-size: 0.74rem;
    font-weight: 700;
    color: #94a3b8;
    letter-spacing: 0.05em;
  }

  .script-code {
    flex: 1;
    margin: 0;
    padding: 16px;
    overflow-y: auto;
    font-size: 0.84rem;
    line-height: 1.5;
    color: #e2e8f0;
    background: #04070e;
  }

  .btn {
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    border: none;
  }

  .btn.primary { background: #2563eb; color: #fff; }
  .btn.secondary { background: #1e293b; color: #cbd5e1; }

  .mono { font-family: monospace; }
  .font-semibold { font-weight: 600; }

  .empty-plan {
    flex: 1;
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
    opacity: 0.4;
    margin-bottom: 12px;
  }
</style>
