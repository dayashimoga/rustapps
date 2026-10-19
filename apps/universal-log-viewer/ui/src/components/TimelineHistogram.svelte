<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export interface TimelineBucketUi {
    bucket_index: number;
    start_epoch_ms: number;
    end_epoch_ms: number;
    total_count: number;
    trace_count: number;
    debug_count: number;
    info_count: number;
    warn_count: number;
    error_count: number;
    fatal_count: number;
    volume_per_second: number;
  }

  export let buckets: TimelineBucketUi[] = [];
  export let minEpochMs: number = 0;
  export let maxEpochMs: number = 0;
  export let selectedRange: [number, number] | null = null;

  const dispatch = createEventDispatcher<{
    rangeSelected: { startMs: number; endMs: number };
    clearRange: void;
  }>();

  let hoveredBucket: TimelineBucketUi | null = null;
  let isDragging = false;
  let dragStartIndex: number | null = null;
  let dragEndIndex: number | null = null;

  $: maxBucketCount = Math.max(...buckets.map((b) => b.total_count), 1);

  function formatTime(epochMs: number): string {
    const d = new Date(epochMs);
    return `${d.getHours().toString().padStart(2, '0')}:${d
      .getMinutes()
      .toString()
      .padStart(2, '0')}:${d.getSeconds().toString().padStart(2, '0')}`;
  }

  function handleMouseDown(index: number) {
    isDragging = true;
    dragStartIndex = index;
    dragEndIndex = index;
  }

  function handleMouseEnter(index: number) {
    hoveredBucket = buckets[index];
    if (isDragging) {
      dragEndIndex = index;
    }
  }

  function handleMouseUp() {
    if (isDragging && dragStartIndex !== null && dragEndIndex !== null) {
      const startIdx = Math.min(dragStartIndex, dragEndIndex);
      const endIdx = Math.max(dragStartIndex, dragEndIndex);
      const startMs = buckets[startIdx].start_epoch_ms;
      const endMs = buckets[endIdx].end_epoch_ms;
      dispatch('rangeSelected', { startMs, endMs });
    }
    isDragging = false;
  }

  function clearFilter() {
    dragStartIndex = null;
    dragEndIndex = null;
    dispatch('clearRange');
  }
</script>

<div class="timeline-container glass-panel" on:mouseup={handleMouseUp} role="region" aria-label="Log volume timeline">
  <div class="timeline-header">
    <div class="timeline-title">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M3 3v18h18"></path>
        <rect x="7" y="10" width="3" height="8"></rect>
        <rect x="12" y="6" width="3" height="12"></rect>
        <rect x="17" y="12" width="3" height="6"></rect>
      </svg>
      <span>CHRONOLOGICAL TIMELINE & DENSITY</span>
    </div>

    <div class="timeline-controls">
      {#if selectedRange}
        <span class="range-badge">
          Range: {formatTime(selectedRange[0])} — {formatTime(selectedRange[1])}
        </span>
        <button class="btn-clear" on:click={clearFilter} title="Reset time filter">
          Clear Filter
        </button>
      {/if}
      <span class="span-info">
        {formatTime(minEpochMs)} — {formatTime(maxEpochMs)}
      </span>
    </div>
  </div>

  <!-- Bar Histogram Canvas / SVG -->
  <div class="bars-wrapper">
    {#each buckets as bucket, i}
      {@const heightPct = (bucket.total_count / maxBucketCount) * 100}
      {@const fatalPct = (bucket.fatal_count / (bucket.total_count || 1)) * 100}
      {@const errorPct = (bucket.error_count / (bucket.total_count || 1)) * 100}
      {@const warnPct = (bucket.warn_count / (bucket.total_count || 1)) * 100}
      {@const infoPct = (bucket.info_count / (bucket.total_count || 1)) * 100}
      {@const isSelected =
        selectedRange &&
        bucket.start_epoch_ms >= selectedRange[0] &&
        bucket.end_epoch_ms <= selectedRange[1]}

      <div
        class="bar-column"
        class:selected={isSelected}
        on:mousedown={() => handleMouseDown(i)}
        on:mouseenter={() => handleMouseEnter(i)}
        on:mouseleave={() => (hoveredBucket = null)}
        role="button"
        tabindex="0"
      >
        <div class="bar-fill" style="height: {heightPct}%;">
          {#if bucket.fatal_count > 0}
            <div class="segment fatal" style="height: {fatalPct}%;"></div>
          {/if}
          {#if bucket.error_count > 0}
            <div class="segment error" style="height: {errorPct}%;"></div>
          {/if}
          {#if bucket.warn_count > 0}
            <div class="segment warn" style="height: {warnPct}%;"></div>
          {/if}
          {#if bucket.info_count > 0}
            <div class="segment info" style="height: {infoPct}%;"></div>
          {/if}
        </div>
      </div>
    {/each}
  </div>

  <!-- Tooltip on Hover -->
  {#if hoveredBucket}
    <div class="timeline-tooltip">
      <div class="tooltip-time">
        {formatTime(hoveredBucket.start_epoch_ms)} — {formatTime(hoveredBucket.end_epoch_ms)}
      </div>
      <div class="tooltip-counts">
        <span class="cnt total">Total: {hoveredBucket.total_count}</span>
        {#if hoveredBucket.error_count > 0}
          <span class="cnt err">Err: {hoveredBucket.error_count}</span>
        {/if}
        {#if hoveredBucket.warn_count > 0}
          <span class="cnt wrn">Warn: {hoveredBucket.warn_count}</span>
        {/if}
        {#if hoveredBucket.fatal_count > 0}
          <span class="cnt fat">Fatal: {hoveredBucket.fatal_count}</span>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .timeline-container {
    height: 96px;
    background: hsla(220, 20%, 9%, 0.85);
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    padding: 6px 16px 8px 16px;
    position: relative;
    user-select: none;
  }

  .timeline-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    margin-bottom: 4px;
  }

  .timeline-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .timeline-controls {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .range-badge {
    background: hsla(217, 91%, 60%, 0.15);
    color: var(--accent-blue);
    padding: 2px 8px;
    border-radius: 4px;
    border: 1px solid hsla(217, 91%, 60%, 0.3);
  }

  .btn-clear {
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    font-size: 10.5px;
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
  }

  .btn-clear:hover {
    color: var(--text-primary);
    border-color: var(--text-muted);
  }

  .span-info {
    font-size: 10.5px;
  }

  .bars-wrapper {
    flex: 1;
    display: flex;
    align-items: flex-end;
    gap: 2px;
    position: relative;
  }

  .bar-column {
    flex: 1;
    height: 100%;
    display: flex;
    align-items: flex-end;
    cursor: pointer;
    border-radius: 2px 2px 0 0;
    transition: opacity 0.1s;
  }

  .bar-column:hover {
    background: hsla(217, 91%, 60%, 0.1);
  }

  .bar-column.selected {
    background: hsla(217, 91%, 60%, 0.25);
  }

  .bar-fill {
    width: 100%;
    min-height: 2px;
    display: flex;
    flex-direction: column-reverse;
    border-radius: 2px 2px 0 0;
    overflow: hidden;
    background: hsl(215, 20%, 35%);
  }

  .segment {
    width: 100%;
  }

  .segment.fatal {
    background: hsl(340, 85%, 60%);
  }

  .segment.error {
    background: hsl(0, 85%, 60%);
  }

  .segment.warn {
    background: hsl(38, 92%, 55%);
  }

  .segment.info {
    background: hsl(205, 90%, 55%);
  }

  .timeline-tooltip {
    position: absolute;
    bottom: 8px;
    left: 50%;
    transform: translateX(-50%);
    background: hsla(220, 24%, 12%, 0.95);
    border: 1px solid var(--border-subtle);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    border-radius: 6px;
    padding: 4px 10px;
    display: flex;
    gap: 12px;
    font-size: 11px;
    font-family: var(--font-mono);
    pointer-events: none;
    z-index: 10;
  }

  .tooltip-time {
    color: var(--text-primary);
    font-weight: 500;
  }

  .tooltip-counts {
    display: flex;
    gap: 8px;
  }

  .cnt.total {
    color: var(--text-secondary);
  }
  .cnt.err {
    color: hsl(0, 85%, 65%);
    font-weight: 600;
  }
  .cnt.wrn {
    color: hsl(38, 92%, 60%);
  }
  .cnt.fat {
    color: hsl(340, 85%, 65%);
    font-weight: 700;
  }
</style>
