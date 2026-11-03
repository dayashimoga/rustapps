<script lang="ts">
  import type { BatchRenameResult, BatchRenameRule, RenamePreview } from '../types';

  export let selectedFiles: string[];
  export let previews: RenamePreview[];
  export let renameResult: BatchRenameResult | null;
  export let isExecuting: boolean;
  export let onGeneratePreview: (rule: BatchRenameRule) => void;
  export let onExecuteRename: (previews: RenamePreview[]) => void;
  export let onClearSelection: () => void;

  let ruleType: 'regex' | 'prefix_suffix' | 'numbering' | 'case' = 'numbering';

  // Rule parameters
  let regexPattern = '';
  let regexReplacement = '';
  let prefix = 'item_';
  let suffix = '';
  let startNumber = 1;
  let padDigits = 3;
  let caseMode: 'lower' | 'upper' = 'lower';

  function getCurrentRule(): BatchRenameRule {
    switch (ruleType) {
      case 'regex':
        return {
          type: 'RegexReplace',
          config: { pattern: regexPattern, replacement: regexReplacement },
        };
      case 'prefix_suffix':
        return {
          type: 'PrefixSuffix',
          config: { prefix, suffix },
        };
      case 'numbering':
        return {
          type: 'Numbering',
          config: { prefix, start_number: startNumber, pad_digits: padDigits },
        };
      case 'case':
        return caseMode === 'lower' ? { type: 'ToLowerCase' } : { type: 'ToUpperCase' };
    }
  }

  function handlePreview() {
    const rule = getCurrentRule();
    onGeneratePreview(rule);
  }

  function handleExecute() {
    if (previews.length > 0 && !hasConflicts) {
      onExecuteRename(previews);
    }
  }

  $: hasConflicts = previews.some(p => p.has_conflict);
</script>

<div class="batch-container glass-panel">
  <!-- Top Config Header -->
  <div class="batch-header">
    <div class="header-info">
      <h2>Transactional Batch Rename</h2>
      <p class="subtitle">
        Atomic two-phase execution with dry-run collision detection & reverse-journal rollback (ADR-0024).
      </p>
    </div>

    <div class="selected-pill">
      {selectedFiles.length} files selected
      {#if selectedFiles.length > 0}
        <button class="clear-btn" on:click={onClearSelection}>&times;</button>
      {/if}
    </div>
  </div>

  {#if selectedFiles.length === 0}
    <div class="empty-state">
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
        <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path>
      </svg>
      <h3>No Files Selected</h3>
      <p>Go to the Explorer tab and check the files you wish to rename in batch.</p>
    </div>
  {:else}
    <!-- Rule Configuration Form -->
    <div class="rule-card">
      <div class="rule-type-selector">
        <button
          class="rule-pill {ruleType === 'numbering' ? 'active' : ''}"
          on:click={() => (ruleType = 'numbering')}
        >
          Sequential Numbering
        </button>
        <button
          class="rule-pill {ruleType === 'prefix_suffix' ? 'active' : ''}"
          on:click={() => (ruleType = 'prefix_suffix')}
        >
          Prefix / Suffix
        </button>
        <button
          class="rule-pill {ruleType === 'regex' ? 'active' : ''}"
          on:click={() => (ruleType = 'regex')}
        >
          Regex Replace
        </button>
        <button
          class="rule-pill {ruleType === 'case' ? 'active' : ''}"
          on:click={() => (ruleType = 'case')}
        >
          Case Conversion
        </button>
      </div>

      <!-- Rule Inputs -->
      <div class="inputs-row">
        {#if ruleType === 'numbering'}
          <div class="form-field">
            <label for="num-prefix">Prefix</label>
            <input id="num-prefix" class="input-field" bind:value={prefix} placeholder="e.g. photo_" />
          </div>
          <div class="form-field" style="width: 120px;">
            <label for="start-num">Start Number</label>
            <input id="start-num" type="number" class="input-field" bind:value={startNumber} min="0" />
          </div>
          <div class="form-field" style="width: 120px;">
            <label for="pad-digits">Padding Digits</label>
            <input id="pad-digits" type="number" class="input-field" bind:value={padDigits} min="1" max="10" />
          </div>
        {:else if ruleType === 'prefix_suffix'}
          <div class="form-field">
            <label for="prefix-input">Prefix</label>
            <input id="prefix-input" class="input-field" bind:value={prefix} placeholder="Prepended text..." />
          </div>
          <div class="form-field">
            <label for="suffix-input">Suffix</label>
            <input id="suffix-input" class="input-field" bind:value={suffix} placeholder="Appended text..." />
          </div>
        {:else if ruleType === 'regex'}
          <div class="form-field">
            <label for="regex-pattern">Find Pattern (Regex)</label>
            <input id="regex-pattern" class="input-field font-mono" bind:value={regexPattern} placeholder="e.g. draft_v(\d+)" />
          </div>
          <div class="form-field">
            <label for="regex-replacement">Replace With</label>
            <input id="regex-replacement" class="input-field font-mono" bind:value={regexReplacement} placeholder="e.g. release_v$1" />
          </div>
        {:else if ruleType === 'case'}
          <div class="form-field">
            <label for="case-mode-select">Target Casing</label>
            <select id="case-mode-select" class="input-field" bind:value={caseMode}>
              <option value="lower">all_lowercase.ext</option>
              <option value="upper">ALL_UPPERCASE.EXT</option>
            </select>
          </div>
        {/if}

        <button class="btn btn-secondary preview-btn" on:click={handlePreview}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="3"></circle>
            <path d="M2.05 12C3.81 7.21 7.92 4 12 4s8.19 3.21 9.95 8c-1.76 4.79-5.87 8-9.95 8s-8.19-3.21-9.95-8z"></path>
          </svg>
          Dry-Run Preview
        </button>
      </div>
    </div>

    <!-- Execution Status Notice -->
    {#if renameResult}
      {#if renameResult.rolled_back || renameResult.error_message}
        <div class="alert alert-danger">
          <strong>Rollback Triggered:</strong> {renameResult.error_message}
        </div>
      {:else}
        <div class="alert alert-success">
          <div class="success-row">
            <span><strong>Success:</strong> Renamed {renameResult.successful_renames} files atomically.</span>
            <button class="btn btn-secondary done-btn" on:click={onClearSelection}>
              Done (Clear Selection)
            </button>
          </div>
        </div>
      {/if}
    {/if}

    <!-- Previews Table -->
    {#if previews.length > 0}
      <div class="preview-section">
        <div class="preview-header">
          <h3>Simulation Preview ({previews.length} items)</h3>
          <button
            class="btn btn-primary"
            on:click={handleExecute}
            disabled={isExecuting || hasConflicts}
          >
            {#if isExecuting}
              Executing...
            {:else if hasConflicts}
              Resolve Conflicts to Apply
            {:else}
              Apply {previews.length} Renames
            {/if}
          </button>
        </div>

        <div class="preview-table-wrapper">
          <table class="preview-table">
            <thead>
              <tr>
                <th>Original Name</th>
                <th></th>
                <th>Proposed Name</th>
                <th style="width: 140px;">Status</th>
              </tr>
            </thead>
            <tbody>
              {#each previews as p}
                <tr class={p.has_conflict ? 'conflict-row' : ''}>
                  <td class="font-mono">{p.original_path.split(/[/\\]/).pop()}</td>
                  <td class="arrow-cell">&rarr;</td>
                  <td class="font-mono proposed-cell">{p.proposed_path.split(/[/\\]/).pop()}</td>
                  <td>
                    {#if p.has_conflict}
                      <span class="badge badge-rose" title={p.conflict_reason || 'Conflict'}>
                        Conflict
                      </span>
                    {:else}
                      <span class="badge badge-emerald">Ready</span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .batch-container {
    display: flex;
    flex-direction: column;
    flex: 1;
    margin: 0 16px 16px 16px;
    padding: 16px;
    overflow-y: auto;
    gap: 16px;
  }

  .batch-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .header-info h2 {
    font-size: 16px;
    font-weight: 700;
  }

  .subtitle {
    font-size: 12px;
    color: var(--text-faint);
  }

  .selected-pill {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(6, 182, 212, 0.15);
    border: 1px solid rgba(6, 182, 212, 0.3);
    color: var(--accent-cyan);
    padding: 4px 10px;
    border-radius: 9999px;
    font-size: 12px;
    font-weight: 600;
  }

  .clear-btn {
    background: transparent;
    border: none;
    color: var(--accent-cyan);
    font-size: 14px;
    cursor: pointer;
  }

  .rule-card {
    background: rgba(11, 17, 30, 0.5);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .rule-type-selector {
    display: flex;
    gap: 8px;
  }

  .rule-pill {
    padding: 6px 14px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
  }
  .rule-pill.active {
    background: linear-gradient(135deg, rgba(6, 182, 212, 0.25), rgba(59, 130, 246, 0.25));
    border-color: rgba(6, 182, 212, 0.4);
    color: var(--accent-cyan);
    font-weight: 600;
  }

  .inputs-row {
    display: flex;
    align-items: flex-end;
    gap: 12px;
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }
  .form-field label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-faint);
    text-transform: uppercase;
  }

  .preview-btn {
    height: 40px;
    white-space: nowrap;
  }

  .preview-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .preview-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .preview-header h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-main);
  }

  .preview-table-wrapper {
    background: rgba(11, 17, 30, 0.6);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .preview-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    text-align: left;
  }
  .preview-table th {
    background: rgba(11, 17, 30, 0.9);
    padding: 8px 14px;
    font-size: 11px;
    color: var(--text-faint);
    text-transform: uppercase;
    border-bottom: 1px solid var(--border-subtle);
  }
  .preview-table td {
    padding: 8px 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  }

  .arrow-cell {
    color: var(--text-faint);
    text-align: center;
    width: 30px;
  }

  .proposed-cell {
    color: var(--accent-cyan);
    font-weight: 500;
  }

  .conflict-row {
    background: rgba(244, 63, 94, 0.08);
  }

  .alert {
    padding: 12px 16px;
    border-radius: var(--radius-md);
    font-size: 13px;
  }
  .success-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .done-btn {
    padding: 4px 10px;
    font-size: 11px;
    height: 28px;
  }
  .alert-success {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid rgba(16, 185, 129, 0.3);
    color: var(--accent-emerald);
  }
  .alert-danger {
    background: rgba(244, 63, 94, 0.15);
    border: 1px solid rgba(244, 63, 94, 0.3);
    color: var(--accent-rose);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    text-align: center;
    color: var(--text-faint);
    gap: 12px;
  }
  .empty-state h3 {
    color: var(--text-main);
  }

  .font-mono {
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
