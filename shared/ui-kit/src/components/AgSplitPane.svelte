<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let direction: 'horizontal' | 'vertical' = 'horizontal';
  export let initialSplit: number = 50; // percentage
  export let minSplit: number = 15;
  export let maxSplit: number = 85;

  const dispatch = createEventDispatcher<{
    resize: { split: number };
  }>();

  let splitPercent: number = initialSplit;
  let isDragging: boolean = false;
  let containerEl: HTMLDivElement;

  function startDrag(e: MouseEvent) {
    e.preventDefault();
    isDragging = true;
    window.addEventListener('mousemove', onDrag);
    window.addEventListener('mouseup', stopDrag);
  }

  function onDrag(e: MouseEvent) {
    if (!isDragging || !containerEl) return;
    const rect = containerEl.getBoundingClientRect();
    let newPercent: number;

    if (direction === 'horizontal') {
      const offsetX = e.clientX - rect.left;
      newPercent = (offsetX / rect.width) * 100;
    } else {
      const offsetY = e.clientY - rect.top;
      newPercent = (offsetY / rect.height) * 100;
    }

    splitPercent = Math.max(minSplit, Math.min(maxSplit, newPercent));
    dispatch('resize', { split: splitPercent });
  }

  function stopDrag() {
    isDragging = false;
    window.removeEventListener('mousemove', onDrag);
    window.removeEventListener('mouseup', stopDrag);
  }

  function handleDoubleClick() {
    splitPercent = 50;
    dispatch('resize', { split: splitPercent });
  }
</script>

<div
  bind:this={containerEl}
  class="ag-split-container"
  class:horizontal={direction === 'horizontal'}
  class:vertical={direction === 'vertical'}
>
  <!-- First Pane -->
  <div
    class="ag-pane ag-pane-primary"
    style="{direction === 'horizontal' ? `width: ${splitPercent}%` : `height: ${splitPercent}%`}"
  >
    <slot name="primary" />
  </div>

  <!-- Resizer Handle -->
  <div
    class="ag-resizer"
    class:dragging={isDragging}
    on:mousedown={startDrag}
    on:dblclick={handleDoubleClick}
    role="separator"
    aria-valuenow={splitPercent}
    tabindex="0"
  >
    <div class="ag-resizer-line"></div>
  </div>

  <!-- Second Pane -->
  <div
    class="ag-pane ag-pane-secondary"
    style="{direction === 'horizontal' ? `width: ${100 - splitPercent}%` : `height: ${100 - splitPercent}%`}"
  >
    <slot name="secondary" />
  </div>
</div>

<style>
  .ag-split-container {
    display: flex;
    width: 100%;
    height: 100%;
    overflow: hidden;
    position: relative;
  }

  .ag-split-container.horizontal {
    flex-direction: row;
  }

  .ag-split-container.vertical {
    flex-direction: column;
  }

  .ag-pane {
    overflow: auto;
    position: relative;
  }

  .ag-resizer {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--ag-border);
    transition: background var(--ag-transition-fast);
    z-index: 10;
    user-select: none;
  }

  .horizontal > .ag-resizer {
    width: 5px;
    cursor: col-resize;
  }

  .vertical > .ag-resizer {
    height: 5px;
    cursor: row-resize;
  }

  .ag-resizer:hover,
  .ag-resizer.dragging {
    background: var(--ag-accent-primary);
  }

  .ag-resizer-line {
    background: oklch(1 0 0 / 0.2);
  }

  .horizontal > .ag-resizer > .ag-resizer-line {
    width: 1px;
    height: 24px;
  }

  .vertical > .ag-resizer > .ag-resizer-line {
    height: 1px;
    width: 24px;
  }
</style>

