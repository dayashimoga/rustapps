<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import type { LogLineRecord } from '../types';

  export let lines: LogLineRecord[] = [];
  export let totalLines: number = 0;
  export let activeLine: number = 0;
  export let searchQuery: string = '';

  const dispatch = createEventDispatcher<{
    scrollChange: { startLine: number; count: number };
    bookmarkLine: { line: number };
  }>();

  const ROW_HEIGHT = 24; // Fixed height in pixels for 60FPS O(1) math
  const BUFFER_ROWS = 15;

  let viewportElement: HTMLDivElement;
  let viewportHeight = 800;
  let scrollTop = 0;

  $: startIndex = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - BUFFER_ROWS);
  $: visibleCount = Math.ceil(viewportHeight / ROW_HEIGHT) + BUFFER_ROWS * 2;
  $: totalHeight = totalLines * ROW_HEIGHT;
  $: offsetY = startIndex * ROW_HEIGHT;

  let lastEmittedStart = -1;

  $: {
    if (startIndex !== lastEmittedStart && totalLines > 0) {
      lastEmittedStart = startIndex;
      dispatch('scrollChange', { startLine: startIndex, count: visibleCount });
    }
  }

  function handleScroll() {
    if (viewportElement) {
      scrollTop = viewportElement.scrollTop;
    }
  }

  export function scrollToLine(targetLine: number) {
    if (viewportElement && targetLine >= 0 && targetLine < totalLines) {
      viewportElement.scrollTop = targetLine * ROW_HEIGHT;
    }
  }

  export function scrollToBottom() {
    if (viewportElement && totalLines > 0) {
      viewportElement.scrollTop = Math.max(0, totalHeight - viewportHeight);
    }
  }

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        viewportHeight = entry.contentRect.height;
      }
    });
    if (viewportElement) {
      resizeObserver.observe(viewportElement);
      viewportHeight = viewportElement.clientHeight;
    }
    return () => resizeObserver.disconnect();
  });

  function getLogLevelClass(text: string): string {
    const upper = text.toUpperCase();
    if (upper.includes('ERROR') || upper.includes('FATAL') || upper.includes('PANIC') || upper.includes('FAIL')) {
      return 'log-error';
    }
    if (upper.includes('WARN') || upper.includes('WARNING')) {
      return 'log-warn';
    }
    if (upper.includes('INFO')) {
      return 'log-info';
    }
    if (upper.includes('DEBUG')) {
      return 'log-debug';
    }
    if (upper.includes('TRACE')) {
      return 'log-trace';
    }
    return 'log-default';
  }
</script>

<div
  class="viewport-container"
  bind:this={viewportElement}
  on:scroll={handleScroll}
  id="viewport-log-container"
>
  <!-- Virtual spacer creating native scrollbar proportional to total lines -->
  <div class="virtual-spacer" style="height: {totalHeight}px;">
    <!-- Active window rendered with transform offset -->
    <div class="rows-window" style="transform: translateY({offsetY}px);">
      {#each lines as line (line.line_number)}
        <div
          class="log-row {getLogLevelClass(line.text)} {line.line_number === activeLine ? 'row-active' : ''}"
          role="button"
          tabindex="0"
          on:dblclick={() => dispatch('bookmarkLine', { line: line.line_number })}
          on:keydown={(e) => e.key === 'Enter' && dispatch('bookmarkLine', { line: line.line_number })}
          title="Double click to bookmark line {line.line_number + 1}"
        >
          <!-- Line Number Gutter -->
          <span class="line-number">{line.line_number + 1}</span>

          <!-- Line Content with Syntax Coloring -->
          <span class="line-text">
            {#if searchQuery && line.text.toLowerCase().includes(searchQuery.toLowerCase())}
              <!-- Highlight Search Match -->
              {@html line.text.replace(
                new RegExp(`(${searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi'),
                '<mark class="search-highlight">$1</mark>'
              )}
            {:else}
              {line.text}
            {/if}
          </span>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .viewport-container {
    flex: 1;
    overflow-y: auto;
    overflow-x: auto;
    background: var(--bg-primary);
    position: relative;
    contain: strict;
  }

  .virtual-spacer {
    position: relative;
    width: 100%;
    min-width: max-content;
  }

  .rows-window {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    will-change: transform;
  }

  .log-row {
    display: flex;
    align-items: center;
    height: 24px;
    line-height: 24px;
    font-family: var(--font-mono);
    font-size: 12.5px;
    white-space: pre;
    padding: 0 12px;
    transition: background var(--transition-fast);
  }

  .log-row:hover {
    background: hsla(220, 20%, 25%, 0.3);
  }

  .row-active {
    background: hsla(217, 91%, 60%, 0.18) !important;
    border-left: 2px solid var(--accent-blue);
  }

  .line-number {
    display: inline-block;
    width: 65px;
    min-width: 65px;
    color: var(--log-line-num);
    text-align: right;
    padding-right: 18px;
    user-select: none;
    font-size: 11px;
    border-right: 1px solid hsla(220, 20%, 30%, 0.2);
    margin-right: 12px;
  }

  .line-text {
    flex: 1;
    color: var(--log-text-default);
  }

  /* Log Level Highlights */
  .log-error .line-text {
    color: var(--log-error);
    font-weight: 500;
  }
  .log-error {
    background: hsla(346, 87%, 65%, 0.04);
  }

  .log-warn .line-text {
    color: var(--log-warn);
  }

  .log-info .line-text {
    color: var(--log-info);
  }

  .log-debug .line-text {
    color: var(--log-debug);
  }

  .log-trace .line-text {
    color: var(--log-trace);
  }

  :global(.search-highlight) {
    background: var(--log-highlight-bg);
    color: #ffffff;
    border-radius: 2px;
    padding: 0 2px;
    border-bottom: 2px solid var(--log-highlight-border);
    font-weight: 600;
  }
</style>
