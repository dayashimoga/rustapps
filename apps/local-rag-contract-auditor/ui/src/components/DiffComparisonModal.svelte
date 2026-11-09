<script lang="ts">
  import { invokeIpc } from '../lib/ipc';
  import type { ContractComparisonDiff, ContractSummary } from '../lib/types';

  export let isOpen = false;
  export let contracts: ContractSummary[] = [];
  export let onClose: () => void = () => {};

  let baseId = contracts[1]?.document_id || contracts[0]?.document_id || '';
  let revisedId = contracts[0]?.document_id || '';
  let isLoading = false;
  let diffResult: ContractComparisonDiff | null = null;
  let errorMessage = '';

  $: if (contracts.length >= 2 && !baseId && !revisedId) {
    baseId = contracts[1]?.document_id || '';
    revisedId = contracts[0]?.document_id || '';
  }

  async function runComparison() {
    if (!baseId || !revisedId) {
      errorMessage = 'Please select both base and revised contract documents';
      return;
    }
    if (baseId === revisedId) {
      errorMessage = 'Please select two different contract versions to compare';
      return;
    }

    isLoading = true;
    errorMessage = '';
    diffResult = null;

    try {
      const res = await invokeIpc<ContractComparisonDiff>('compare_contracts', {
        base_id: baseId,
        revised_id: revisedId,
      });
      diffResult = res;
    } catch (e: any) {
      errorMessage = `Comparison failed: ${e.message || e}`;
    } finally {
      isLoading = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

{#if isOpen}
  <div class="modal-backdrop" on:click={onClose} on:keydown={handleKeyDown} role="presentation">
    <div class="modal-card" on:click|stopPropagation role="dialog" aria-modal="true" aria-label="Version Diff Studio" tabindex="-1">
      <div class="modal-header">
        <div class="modal-title">
          <span class="icon">⚖️</span>
          <span>Bilateral Contract Redline Diff Studio</span>
        </div>
        <button type="button" class="close-btn" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body">
        <div class="selector-grid">
          <div class="select-col">
            <label for="base-select">Base Contract (Institutional Baseline / v1.0)</label>
            <select id="base-select" bind:value={baseId}>
              {#each contracts as c}
                <option value={c.document_id}>{c.title} ({c.document_id})</option>
              {/each}
            </select>
          </div>

          <div class="select-col">
            <label for="rev-select">Revised Contract (Counterparty Markup / v2.0)</label>
            <select id="rev-select" bind:value={revisedId}>
              {#each contracts as c}
                <option value={c.document_id}>{c.title} ({c.document_id})</option>
              {/each}
            </select>
          </div>
        </div>

        <div class="action-row">
          <button type="button" class="btn primary" on:click={runComparison} disabled={isLoading}>
            {isLoading ? 'Computing Semantic Redline...' : 'Run Bilateral Comparison'}
          </button>
        </div>

        {#if errorMessage}
          <div class="error-box">{errorMessage}</div>
        {/if}

        {#if diffResult}
          <div class="diff-summary-card">
            <div class="stat-pill">
              <span class="k">Clauses Modified:</span>
              <span class="v">{diffResult.changed_clauses_count}</span>
            </div>
            <div class="stat-pill">
              <span class="k">Net Risk Score Delta:</span>
              <span
                class="v"
                class:risk-up={diffResult.overall_risk_delta > 0}
                class:risk-down={diffResult.overall_risk_delta < 0}
              >
                {diffResult.overall_risk_delta > 0 ? '+' : ''}{diffResult.overall_risk_delta.toFixed(1)} pts
              </span>
            </div>
          </div>

          <div class="diff-items-container">
            {#each diffResult.diff_items as item}
              <div class="diff-card">
                <div class="diff-card-header">
                  <div class="diff-title">
                    <span class="sec-badge">Section {item.section_number}</span>
                    <span class="cat-badge">{item.category.replace(/_/g, ' ')}</span>
                  </div>
                  <div
                    class="delta-badge"
                    class:elevated={item.risk_delta > 0}
                    class:improved={item.risk_delta < 0}
                  >
                    Risk Delta: {item.risk_delta > 0 ? '+' : ''}{item.risk_delta.toFixed(1)}
                  </div>
                </div>

                <div class="diff-summary-text">{item.summary}</div>

                <div class="text-comparison-grid">
                  <div class="text-pane base">
                    <div class="pane-lbl">Base Version:</div>
                    <pre>{item.base_text || '(Clause was absent in base contract)'}</pre>
                  </div>
                  <div class="text-pane revised">
                    <div class="pane-lbl">Revised Version:</div>
                    <pre>{item.revised_text}</pre>
                  </div>
                </div>
              </div>
            {/each}
            {#if diffResult.diff_items.length === 0}
              <div class="empty-diff">No clause text discrepancies detected between versions.</div>
            {/if}
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button type="button" class="btn secondary" on:click={onClose}>Close</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(4, 7, 14, 0.85);
    backdrop-filter: blur(8px);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
  }

  .modal-card {
    background: #0d121f;
    border: 1px solid #1f293d;
    border-radius: 12px;
    width: 960px;
    max-width: 94vw;
    max-height: 90vh;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.7);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid #1a2336;
    background: #090e18;
  }

  .modal-title {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 1.1rem;
    font-weight: 600;
    color: #f1f5f9;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #64748b;
    font-size: 1.2rem;
    cursor: pointer;
  }

  .close-btn:hover {
    color: #f1f5f9;
  }

  .modal-body {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .selector-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .select-col {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label {
    color: #94a3b8;
    font-size: 0.8rem;
    font-weight: 500;
  }

  select {
    background: #060a12;
    border: 1px solid #1e293b;
    border-radius: 6px;
    color: #e2e8f0;
    padding: 8px 10px;
    font-size: 0.86rem;
    outline: none;
  }

  .action-row {
    display: flex;
    justify-content: flex-start;
  }

  .diff-summary-card {
    display: flex;
    gap: 16px;
    background: #111827;
    border: 1px solid #1f293d;
    border-radius: 8px;
    padding: 12px 16px;
  }

  .stat-pill {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.88rem;
  }

  .stat-pill .k {
    color: #94a3b8;
  }

  .stat-pill .v {
    color: #f1f5f9;
    font-weight: 700;
  }

  .stat-pill .risk-up {
    color: #ef4444;
  }

  .stat-pill .risk-down {
    color: #10b981;
  }

  .diff-items-container {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .diff-card {
    background: #060a12;
    border: 1px solid #1e293b;
    border-radius: 8px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .diff-card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .diff-title {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .sec-badge {
    background: #1e293b;
    color: #38bdf8;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .cat-badge {
    background: #132338;
    color: #93c5fd;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    text-transform: capitalize;
  }

  .delta-badge {
    font-size: 0.78rem;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 4px;
  }

  .delta-badge.elevated {
    background: rgba(239, 68, 68, 0.2);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.4);
  }

  .delta-badge.improved {
    background: rgba(16, 185, 129, 0.2);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.4);
  }

  .diff-summary-text {
    font-size: 0.85rem;
    color: #cbd5e1;
  }

  .text-comparison-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .text-pane {
    padding: 10px;
    border-radius: 6px;
    font-size: 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .text-pane.base {
    background: rgba(239, 68, 68, 0.05);
    border: 1px solid rgba(239, 68, 68, 0.2);
  }

  .text-pane.revised {
    background: rgba(16, 185, 129, 0.05);
    border: 1px solid rgba(16, 185, 129, 0.2);
  }

  .pane-lbl {
    font-size: 0.74rem;
    font-weight: 600;
    text-transform: uppercase;
    color: #94a3b8;
  }

  pre {
    margin: 0;
    white-space: pre-wrap;
    font-family: inherit;
    line-height: 1.4;
    color: #e2e8f0;
  }

  .empty-diff {
    text-align: center;
    padding: 30px;
    color: #64748b;
  }

  .error-box {
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 0.85rem;
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: #f87171;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    padding: 14px 20px;
    border-top: 1px solid #1a2336;
    background: #090e18;
  }

  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.88rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
  }

  .btn.secondary {
    background: #1e293b;
    color: #cbd5e1;
  }

  .btn.secondary:hover {
    background: #334155;
  }

  .btn.primary {
    background: #2563eb;
    color: #fff;
  }

  .btn.primary:hover:not(:disabled) {
    background: #1d4ed8;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
