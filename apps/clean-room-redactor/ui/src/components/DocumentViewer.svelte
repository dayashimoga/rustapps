<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let pages: any[] = [];
  export let activePageIndex: number = 0;

  const dispatch = createEventDispatcher();

  let selectedExemption: string = '(b)(4)';
  let selectedBurnStyle: string = 'solid_black';
  let manualLabel: string = 'Confidential';

  function handleSelectPage(idx: number) {
    activePageIndex = idx;
    dispatch('pageChange', { pageIndex: idx });
  }

  function handleAddMask() {
    dispatch('addMask', {
      pageIndex: activePageIndex,
      label: manualLabel,
      burnStyle: selectedBurnStyle,
      exemptionCode: selectedBurnStyle === 'exemption_code' ? selectedExemption : null,
      bbox: {
        x: 45.0,
        y: 80.0 + (pages[activePageIndex]?.redaction_masks?.length || 0) * 30.0,
        width: 180.0,
        height: 22.0,
      },
    });
  }

  function handleRemoveMask(maskId: string) {
    dispatch('removeMask', {
      pageIndex: activePageIndex,
      maskId,
    });
  }
</script>

<div class="document-viewer-container">
  <!-- Left Thumbnail Navigation Rail -->
  <aside class="thumbnail-rail">
    <div class="rail-header">
      <span>PAGES ({pages.length})</span>
    </div>

    <div class="thumb-list">
      {#each pages as page, idx}
        <button
          class="thumb-item"
          class:active={idx === activePageIndex}
          on:click={() => handleSelectPage(idx)}
        >
          <div class="thumb-preview">
            {#if page.thumbnail_svg}
              {@html page.thumbnail_svg}
            {:else}
              <div class="thumb-placeholder">Page {idx + 1}</div>
            {/if}
          </div>
          <span class="page-label">Page {idx + 1}</span>
          {#if (page.pii_findings?.length || 0) + (page.redaction_masks?.length || 0) > 0}
            <span class="mask-dot">{(page.pii_findings?.length || 0) + (page.redaction_masks?.length || 0)}</span>
          {/if}
        </button>
      {/each}
    </div>
  </aside>

  <!-- Center Active Page Display & Manual Redaction Controls -->
  <section class="main-canvas-area">
    <!-- Redaction Action Toolbar -->
    <div class="canvas-toolbar">
      <div class="tool-group">
        <label for="burn-style">Style:</label>
        <select id="burn-style" bind:value={selectedBurnStyle}>
          <option value="solid_black">Solid Blackout Mask</option>
          <option value="exemption_code">FOIA / Legal Exemption</option>
          <option value="pattern_hatch">Cross-Hatch Pattern</option>
          <option value="solid_white">Solid White Cutout</option>
        </select>
      </div>

      {#if selectedBurnStyle === 'exemption_code'}
        <div class="tool-group">
          <label for="exemption-code">Code:</label>
          <select id="exemption-code" bind:value={selectedExemption}>
            <option value="(b)(4)">(b)(4) Trade Secrets / Commercial</option>
            <option value="(b)(5)">(b)(5) Deliberative Process / Privileged</option>
            <option value="(b)(6)">(b)(6) Personal Privacy</option>
            <option value="(b)(7)(C)">(b)(7)(C) Law Enforcement Privacy</option>
            <option value="ATTORNEY-CLIENT">Attorney-Client Privilege</option>
            <option value="HIPAA-PHI">HIPAA 45 CFR § 164.514 (PHI)</option>
          </select>
        </div>
      {/if}

      <div class="tool-group">
        <input
          type="text"
          placeholder="Mask label (e.g. Recital)"
          bind:value={manualLabel}
          class="label-input"
        />
      </div>

      <button class="add-mask-btn" on:click={handleAddMask}>
        <span>+</span> Add Redaction Box
      </button>

      <span class="active-page-hud">
        Viewing Page {activePageIndex + 1} of {pages.length}
      </span>
    </div>

    <!-- Page SVG Surface -->
    <div class="page-viewport">
      {#if pages[activePageIndex]}
        <div class="page-sheet">
          {@html pages[activePageIndex].thumbnail_svg}
        </div>
      {:else}
        <div class="empty-page-state">No document loaded</div>
      {/if}
    </div>

    <!-- Active Page Manual Masks List -->
    {#if pages[activePageIndex]?.redaction_masks?.length > 0}
      <div class="active-masks-bar">
        <span class="masks-title">MANUAL MASKS ON PAGE {activePageIndex + 1}:</span>
        {#each pages[activePageIndex].redaction_masks as mask}
          <div class="mask-tag">
            <span>{mask.label} ({mask.exemption_code || mask.burn_style})</span>
            <button class="del-mask" on:click={() => handleRemoveMask(mask.id)}>✕</button>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</div>

<style>
  .document-viewer-container {
    display: flex;
    flex: 1;
    overflow: hidden;
    background: #090d14;
    border: 1px solid #1a2332;
    border-radius: 8px;
  }

  .thumbnail-rail {
    width: 140px;
    background: #0f141d;
    border-right: 1px solid #1a2332;
    display: flex;
    flex-direction: column;
  }

  .rail-header {
    padding: 8px 10px;
    background: #141b26;
    border-bottom: 1px solid #1a2332;
    font-size: 0.7rem;
    font-family: monospace;
    color: #64748b;
    font-weight: 700;
  }

  .thumb-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .thumb-item {
    background: #111722;
    border: 1px solid #1e2838;
    border-radius: 6px;
    padding: 4px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    position: relative;
    transition: all 0.15s ease;
  }

  .thumb-item:hover {
    border-color: #38bdf8;
  }

  .thumb-item.active {
    border-color: #10b981;
    box-shadow: 0 0 0 1px #10b981;
    background: #131c2a;
  }

  .thumb-preview {
    width: 100%;
    height: 100px;
    background: #ffffff;
    border-radius: 3px;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .thumb-placeholder {
    color: #94a3b8;
    font-size: 0.7rem;
    font-family: monospace;
  }

  .page-label {
    font-size: 0.7rem;
    font-family: monospace;
    color: #cbd5e1;
    margin-top: 4px;
  }

  .mask-dot {
    position: absolute;
    top: 2px;
    right: 2px;
    background: #dc2626;
    color: #ffffff;
    font-size: 0.62rem;
    font-family: monospace;
    font-weight: 700;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .main-canvas-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .canvas-toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 12px;
    background: #131a26;
    border-bottom: 1px solid #1e2838;
    font-size: 0.75rem;
    font-family: monospace;
  }

  .tool-group {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .tool-group label {
    color: #94a3b8;
  }

  select,
  .label-input {
    background: #090d14;
    border: 1px solid #28374d;
    color: #f1f5f9;
    padding: 4px 8px;
    border-radius: 4px;
    font-family: inherit;
    font-size: 0.72rem;
    outline: none;
  }

  .label-input {
    width: 140px;
  }

  .add-mask-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: #182232;
    border: 1px solid #28374d;
    color: #38bdf8;
    padding: 4px 10px;
    border-radius: 4px;
    font-weight: 600;
    cursor: pointer;
    font-family: inherit;
    font-size: 0.72rem;
  }

  .add-mask-btn:hover {
    background: #0284c7;
    color: #ffffff;
    border-color: #38bdf8;
  }

  .active-page-hud {
    margin-left: auto;
    color: #64748b;
    font-size: 0.72rem;
  }

  .page-viewport {
    flex: 1;
    overflow: auto;
    padding: 16px;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    background: #0a0e16;
  }

  .page-sheet {
    width: 612px;
    min-height: 792px;
    background: #ffffff;
    border-radius: 4px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }

  .empty-page-state {
    color: #64748b;
    font-family: monospace;
    font-size: 0.85rem;
    margin-top: 40px;
  }

  .active-masks-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background: #111722;
    border-top: 1px solid #1e2838;
    font-family: monospace;
    font-size: 0.72rem;
    overflow-x: auto;
  }

  .masks-title {
    color: #64748b;
    font-weight: 700;
  }

  .mask-tag {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #182232;
    border: 1px solid #28374d;
    color: #cbd5e1;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .del-mask {
    background: none;
    border: none;
    color: #ef4444;
    cursor: pointer;
    font-size: 0.7rem;
    padding: 0;
  }
</style>
