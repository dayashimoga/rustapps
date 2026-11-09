<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let piiFindings: any[] = [];
  export let pageIndex: number = 0;

  const dispatch = createEventDispatcher();

  function toggleApproval(findingId: string, currentApproved: boolean) {
    dispatch('toggleApproval', {
      findingId,
      approved: !currentApproved,
      pageIndex,
    });
  }

  function approveAll() {
    dispatch('bulkApprove', { pageIndex, approved: true });
  }

  function rejectAll() {
    dispatch('bulkApprove', { pageIndex, approved: false });
  }

  function getCategoryColor(category: string): string {
    switch (category) {
      case 'ssn':
      case 'credit_card':
        return '#ef4444'; // Red
      case 'medical_record_number':
      case 'dea_npi_number':
        return '#f59e0b'; // Amber
      case 'bank_account':
        return '#8b5cf6'; // Purple
      case 'email':
      case 'phone':
        return '#3b82f6'; // Blue
      default:
        return '#10b981'; // Green
    }
  }
</script>

<div class="pii-findings-panel">
  <div class="panel-header">
    <div class="header-left">
      <span class="shield-tag">AI PII/PHI DETECTOR</span>
      <span class="count-badge">{piiFindings.length} Entities Found</span>
    </div>

    <div class="header-actions">
      <button class="bulk-btn" on:click={approveAll}>Approve All</button>
      <button class="bulk-btn" on:click={rejectAll}>Ignore All</button>
    </div>
  </div>

  <div class="findings-list">
    {#if piiFindings.length === 0}
      <div class="empty-notice">
        <span>✓ No unredacted PII or PHI detected on Page {pageIndex + 1}.</span>
      </div>
    {:else}
      {#each piiFindings as finding}
        <div class="finding-row" class:approved={finding.approved}>
          <input
            type="checkbox"
            checked={finding.approved}
            on:change={() => toggleApproval(finding.id, finding.approved)}
            title="Include in clean-room burnout"
          />

          <span
            class="cat-pill"
            style="color: {getCategoryColor(finding.category)}; border-color: {getCategoryColor(finding.category)}44;"
          >
            {finding.category.replace('_', ' ').toUpperCase()}
          </span>

          <span class="preview-text" title={finding.matched_text}>
            {finding.matched_text}
          </span>

          <span class="arrow">→</span>

          <span class="mask-preview">
            {finding.mask_preview}
          </span>

          <span class="confidence">
            {(finding.confidence * 100).toFixed(0)}% match
          </span>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .pii-findings-panel {
    display: flex;
    flex-direction: column;
    background: #0d121a;
    border: 1px solid #1e2838;
    border-radius: 8px;
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: #131a26;
    border-bottom: 1px solid #1e2838;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .shield-tag {
    font-size: 0.72rem;
    font-family: monospace;
    font-weight: 700;
    color: #10b981;
    letter-spacing: 0.5px;
  }

  .count-badge {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid rgba(16, 185, 129, 0.3);
    color: #34d399;
    font-size: 0.68rem;
    font-family: monospace;
    padding: 1px 6px;
    border-radius: 4px;
  }

  .header-actions {
    display: flex;
    gap: 6px;
  }

  .bulk-btn {
    background: #1c2636;
    border: 1px solid #28374d;
    color: #94a3b8;
    font-size: 0.7rem;
    font-family: monospace;
    padding: 2px 8px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .bulk-btn:hover {
    color: #f1f5f9;
    border-color: #38bdf8;
  }

  .findings-list {
    max-height: 180px;
    overflow-y: auto;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .empty-notice {
    padding: 14px;
    text-align: center;
    font-size: 0.78rem;
    color: #10b981;
    font-family: monospace;
  }

  .finding-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    background: #111722;
    border: 1px solid #1b2433;
    border-radius: 4px;
    font-size: 0.75rem;
    font-family: monospace;
    transition: all 0.1s ease;
  }

  .finding-row.approved {
    border-color: rgba(16, 185, 129, 0.3);
    background: rgba(16, 185, 129, 0.05);
  }

  input[type='checkbox'] {
    accent-color: #10b981;
    cursor: pointer;
  }

  .cat-pill {
    font-size: 0.65rem;
    padding: 1px 5px;
    border-radius: 3px;
    border: 1px solid;
    font-weight: 600;
  }

  .preview-text {
    color: #f1f5f9;
    font-weight: 500;
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .arrow {
    color: #64748b;
  }

  .mask-preview {
    color: #94a3b8;
    flex: 1;
  }

  .confidence {
    color: #64748b;
    font-size: 0.68rem;
  }
</style>
