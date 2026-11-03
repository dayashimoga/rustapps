<script lang="ts">
  import { onMount } from 'svelte';
  import type { TreemapNode } from '../types';

  export let rootNode: TreemapNode | null;
  export let onDrillDown: (path: string) => void;
  export let onRefreshTreemap: () => void;
  export let onInspectDoc: (path: string) => void = () => {};

  let canvas: HTMLCanvasElement;
  let container: HTMLDivElement;
  let hoveredNode: TreemapNode | null = null;
  let tooltipX = 0;
  let tooltipY = 0;

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function getBreadcrumbs(pathStr: string): { name: string; path: string }[] {
    if (!pathStr) return [];
    const normalized = pathStr.replace(/\\/g, '/');
    const segments = normalized.split('/').filter(Boolean);
    const crumbs: { name: string; path: string }[] = [];
    let current = '';

    for (let i = 0; i < segments.length; i++) {
      const seg = segments[i];
      if (i === 0 && seg.endsWith(':')) {
        current = seg;
      } else {
        current = current ? `${current}/${seg}` : (normalized.startsWith('/') ? `/${seg}` : seg);
      }
      crumbs.push({ name: seg, path: current });
    }
    return crumbs;
  }

  function getParentPath(pathStr: string): string | null {
    if (!pathStr) return null;
    const normalized = pathStr.replace(/\\/g, '/');
    const idx = normalized.lastIndexOf('/');
    if (idx > 0) {
      return normalized.slice(0, idx);
    }
    return null;
  }

  $: breadcrumbs = rootNode?.path ? getBreadcrumbs(rootNode.path) : [];
  $: parentPath = rootNode?.path ? getParentPath(rootNode.path) : null;

  function getNodeColor(node: TreemapNode, index: number): string {
    if (node.is_dir) {
      const dirHues = [200, 215, 230, 260, 280];
      const hue = dirHues[index % dirHues.length];
      return `hsla(${hue}, 65%, 28%, 0.75)`;
    }
    const ext = node.name.split('.').pop()?.toLowerCase();
    if (['rs', 'ts', 'js', 'py', 'go'].includes(ext || '')) return 'hsla(187, 85%, 38%, 0.7)';
    if (['json', 'toml', 'yaml', 'xml'].includes(ext || '')) return 'hsla(38, 92%, 45%, 0.7)';
    if (['jpg', 'png', 'svg', 'webp', 'mp4'].includes(ext || '')) return 'hsla(271, 80%, 48%, 0.7)';
    if (['pdf', 'docx', 'txt', 'md'].includes(ext || '')) return 'hsla(158, 70%, 35%, 0.7)';
    return 'hsla(217, 33%, 30%, 0.7)';
  }

  function renderTreemap() {
    if (!canvas || !rootNode) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;
    ctx.clearRect(0, 0, width, height);

    drawNode(ctx, rootNode, 0);
  }

  function drawNode(ctx: CanvasRenderingContext2D, node: TreemapNode, index: number) {
    if (!node.children || node.children.length === 0) {
      // Draw leaf rectangle
      const b = node.bounds;
      if (b.width <= 1 || b.height <= 1) return;

      ctx.fillStyle = getNodeColor(node, index);
      ctx.fillRect(b.x, b.y, b.width, b.height);

      ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
      ctx.lineWidth = 1;
      ctx.strokeRect(b.x, b.y, b.width, b.height);

      // Label if large enough
      if (b.width > 50 && b.height > 20) {
        ctx.fillStyle = '#ffffff';
        ctx.font = '11px sans-serif';
        ctx.fillText(node.name, b.x + 6, b.y + 16, b.width - 12);

        if (b.height > 36) {
          ctx.fillStyle = 'rgba(255, 255, 255, 0.6)';
          ctx.font = '10px monospace';
          ctx.fillText(formatBytes(node.size_bytes), b.x + 6, b.y + 30, b.width - 12);
        }
      }
    } else {
      // Draw container boundary
      for (let i = 0; i < node.children.length; i++) {
        drawNode(ctx, node.children[i], i);
      }
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (!canvas || !rootNode) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    tooltipX = e.clientX + 14;
    tooltipY = e.clientY + 14;

    hoveredNode = findHitNode(rootNode, x, y);
  }

  function handleMouseLeave() {
    hoveredNode = null;
  }

  function handleClick(e: MouseEvent) {
    if (!canvas || !rootNode) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const hit = findHitNode(rootNode, x, y);
    if (hit) {
      if (hit.is_dir) {
        onDrillDown(hit.path);
      } else {
        onInspectDoc(hit.path);
      }
    }
  }

  function findHitNode(node: TreemapNode, x: number, y: number): TreemapNode | null {
    const b = node.bounds;
    if (x < b.x || x > b.x + b.width || y < b.y || y > b.y + b.height) {
      return null;
    }

    if (node.children && node.children.length > 0) {
      for (const child of node.children) {
        const hit = findHitNode(child, x, y);
        if (hit) return hit;
      }
    }

    return node;
  }

  function resizeCanvas() {
    if (!container || !canvas) return;
    canvas.width = container.clientWidth;
    canvas.height = container.clientHeight;
    renderTreemap();
  }

  $: if (rootNode && canvas) {
    renderTreemap();
  }

  onMount(() => {
    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);
    return () => {
      window.removeEventListener('resize', resizeCanvas);
    };
  });
</script>

<div class="treemap-container glass-panel">
  <!-- Treemap Toolbar -->
  <div class="treemap-toolbar">
    <div class="path-focus">
      {#if parentPath}
        <button
          class="btn btn-secondary nav-up-btn"
          title="Navigate Up ({parentPath})"
          on:click={() => onDrillDown(parentPath)}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="18 15 12 9 6 15"></polyline>
          </svg>
          Up
        </button>
      {/if}
      <span class="focus-label">Focus:</span>
      <div class="breadcrumbs-list">
        {#each breadcrumbs as crumb, i}
          <button
            class="crumb-btn {i === breadcrumbs.length - 1 ? 'active' : ''}"
            on:click={() => onDrillDown(crumb.path)}
          >
            {crumb.name}
          </button>
          {#if i < breadcrumbs.length - 1}
            <span class="crumb-separator">/</span>
          {/if}
        {/each}
      </div>
    </div>

    <div class="toolbar-actions">
      <span class="total-size-badge">
        Total: {formatBytes(rootNode?.size_bytes || 0)}
      </span>
      <button class="btn btn-secondary" on:click={onRefreshTreemap}>
        Refresh Layout
      </button>
    </div>
  </div>

  <!-- Canvas Surface -->
  <div class="canvas-wrapper" bind:this={container}>
    <canvas
      bind:this={canvas}
      on:mousemove={handleMouseMove}
      on:mouseleave={handleMouseLeave}
      on:click={handleClick}
    ></canvas>

    <!-- Floating Hover Tooltip -->
    {#if hoveredNode}
      <div class="treemap-tooltip glass-panel" style="left: {tooltipX}px; top: {tooltipY}px;">
        <div class="tooltip-header">
          <span class="tooltip-name">{hoveredNode.name}</span>
          <span class="badge {hoveredNode.is_dir ? 'badge-amber' : 'badge-cyan'}">
            {hoveredNode.is_dir ? 'Directory' : 'File'}
          </span>
        </div>
        <div class="tooltip-body">
          <span class="tooltip-size">{formatBytes(hoveredNode.size_bytes)}</span>
          {#if rootNode && rootNode.size_bytes > 0}
            <span class="tooltip-pct">
              {((hoveredNode.size_bytes / rootNode.size_bytes) * 100).toFixed(1)}% of view
            </span>
          {/if}
        </div>
        <div class="tooltip-path">{hoveredNode.path}</div>
        {#if hoveredNode.is_dir}
          <div class="tooltip-hint">Click to drill down</div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .treemap-container {
    display: flex;
    flex-direction: column;
    flex: 1;
    margin: 0 16px 16px 16px;
    overflow: hidden;
  }

  .treemap-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: rgba(11, 17, 30, 0.4);
    border-bottom: 1px solid var(--border-subtle);
  }

  .path-focus {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .nav-up-btn {
    padding: 3px 8px;
    font-size: 11px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 26px;
  }
  .breadcrumbs-list {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
  }
  .crumb-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 4px;
    transition: all 0.15s ease;
  }
  .crumb-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--accent-cyan);
  }
  .crumb-btn.active {
    color: var(--accent-cyan);
    font-weight: 700;
  }
  .crumb-separator {
    color: var(--text-faint);
    font-size: 11px;
  }
  .focus-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-faint);
    text-transform: uppercase;
  }

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .total-size-badge {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-muted);
  }

  .canvas-wrapper {
    flex: 1;
    position: relative;
    overflow: hidden;
    cursor: crosshair;
  }

  canvas {
    width: 100%;
    height: 100%;
    display: block;
  }

  .treemap-tooltip {
    position: fixed;
    pointer-events: none;
    z-index: 1000;
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 220px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.7);
  }

  .tooltip-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .tooltip-name {
    font-weight: 700;
    font-size: 13px;
    color: var(--text-main);
  }

  .tooltip-body {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .tooltip-size {
    font-family: var(--font-mono);
    font-size: 14px;
    font-weight: 700;
    color: var(--accent-cyan);
  }

  .tooltip-pct {
    font-size: 11px;
    color: var(--text-muted);
  }

  .tooltip-path {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-faint);
    word-break: break-all;
  }

  .tooltip-hint {
    font-size: 10px;
    color: var(--accent-amber);
    margin-top: 4px;
    font-style: italic;
  }
</style>
