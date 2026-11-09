<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let title: string = 'No Data Loaded';
  export let description: string = 'Open a file, drag and drop, or load an instant offline sample workspace.';
  export let sampleLabel: string = 'Load Sample Workspace';
  export let sampleBadge: string = 'Instant 5s';
  export let primaryActionLabel: string = 'Open File';
  export let acceptedFileTypes: string = '*/*';

  const dispatch = createEventDispatcher<{
    loadSample: void;
    openFile: void;
    dropFiles: { files: FileList };
  }>();

  let isDragging: boolean = false;

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    isDragging = true;
  }

  function handleDragLeave() {
    isDragging = false;
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
    if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
      dispatch('dropFiles', { files: e.dataTransfer.files });
    }
  }

  function handleSampleClick() {
    dispatch('loadSample');
  }

  function handlePrimaryClick() {
    dispatch('openFile');
  }
</script>

<div
  class="ag-empty-card ag-glass-panel"
  class:dragging={isDragging}
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
  role="region"
  aria-label="Empty state onboarding card"
>
  <div class="ag-empty-icon-box">
    <slot name="icon">
      <svg class="ag-default-icon" viewBox="0 0 24 24" width="32" height="32" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M4 4v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8.342a2 2 0 0 0-.602-1.43l-4.44-4.342A2 2 0 0 0 13.56 2H6a2 2 0 0 0-2 2z"/>
        <path d="M14 2v6h6"/>
      </svg>
    </slot>
  </div>

  <h3 class="ag-empty-title">{title}</h3>
  <p class="ag-empty-desc">{description}</p>

  <div class="ag-empty-actions">
    <button type="button" class="ag-btn ag-btn-primary" on:click={handlePrimaryClick}>
      {primaryActionLabel}
    </button>
    <button type="button" class="ag-btn ag-btn-sample" on:click={handleSampleClick}>
      <span>{sampleLabel}</span>
      {#if sampleBadge}
        <span class="ag-badge ag-badge-pro">{sampleBadge}</span>
      {/if}
    </button>
  </div>

  <div class="ag-drop-hint">
    <span>or drag & drop files here ({acceptedFileTypes})</span>
  </div>
</div>

<style>
  .ag-empty-card {
    max-width: 460px;
    margin: 40px auto;
    padding: 32px 24px;
    border-radius: var(--ag-radius-lg);
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    transition: all var(--ag-transition-normal);
  }

  .ag-empty-card.dragging {
    border-color: var(--ag-accent-primary);
    box-shadow: 0 0 0 2px oklch(0.68 0.17 230 / 0.4);
    transform: scale(1.02);
  }

  .ag-empty-icon-box {
    width: 64px;
    height: 64px;
    border-radius: var(--ag-radius-xl);
    background: var(--ag-bg-panel);
    border: 1px solid var(--ag-border);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--ag-accent-primary);
    margin-bottom: 16px;
  }

  .ag-default-icon {
    opacity: 0.85;
  }

  .ag-empty-title {
    margin: 0 0 8px 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--ag-text-primary);
    letter-spacing: -0.015em;
  }

  .ag-empty-desc {
    margin: 0 0 24px 0;
    font-size: 12px;
    line-height: 1.6;
    color: var(--ag-text-muted);
    max-width: 380px;
  }

  .ag-empty-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
    flex-wrap: wrap;
    justify-content: center;
  }

  .ag-btn-sample {
    background: var(--ag-bg-panel);
    border-color: var(--ag-border-hover);
  }

  .ag-btn-sample:hover {
    background: var(--ag-bg-hover);
    border-color: var(--ag-accent-primary);
  }

  .ag-drop-hint {
    font-size: 11px;
    color: var(--ag-text-dim);
  }
</style>
