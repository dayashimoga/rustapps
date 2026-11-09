<script lang="ts">
  import type { ChartType, ChartSeries } from './types';

  export let type: ChartType = 'line';
  export let series: ChartSeries[] = [];
  export let width: number = 500;
  export let height: number = 240;
  export let showGrid: boolean = true;
  export let showLabels: boolean = true;

  // Compute bounding box for scales
  $: allPoints = series.flatMap((s) => s.points);
  $: xValues = allPoints.map((p) => p.x);
  $: yValues = allPoints.map((p) => p.y);

  $: minX = xValues.length > 0 ? Math.min(...xValues) : 0;
  $: maxX = xValues.length > 0 ? Math.max(...xValues) : 100;
  $: minY = yValues.length > 0 ? Math.min(0, Math.min(...yValues)) : 0;
  $: maxY = yValues.length > 0 ? Math.max(10, Math.max(...yValues) * 1.1) : 100;

  const padLeft = 45;
  const padRight = 15;
  const padTop = 15;
  const padBottom = 25;

  $: plotW = Math.max(10, width - padLeft - padRight);
  $: plotH = Math.max(10, height - padTop - padBottom);

  function scaleX(val: number): number {
    const span = maxX - minX || 1;
    return padLeft + ((val - minX) / span) * plotW;
  }

  function scaleY(val: number): number {
    const span = maxY - minY || 1;
    return padTop + plotH - ((val - minY) / span) * plotH;
  }

  // Hover state
  let hoveredPoint: { x: number; y: number; label?: string; px: number; py: number } | null = null;

  function handleMouseMove(e: MouseEvent) {
    if (type === 'sparkline') return;
    const rect = (e.currentTarget as SVGElement).getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;

    let closest: typeof hoveredPoint = null;
    let minDist = 30;

    for (const s of series) {
      for (const p of s.points) {
        const px = scaleX(p.x);
        const py = scaleY(p.y);
        const dist = Math.hypot(mx - px, my - py);
        if (dist < minDist) {
          minDist = dist;
          closest = { ...p, px, py };
        }
      }
    }
    hoveredPoint = closest;
  }

  function handleMouseLeave() {
    hoveredPoint = null;
  }
</script>

<div class="ag-chart-container" style="width: {width}px; height: {height}px;">
  <svg
    viewBox="0 0 {width} {height}"
    class="ag-chart-svg"
    on:mousemove={handleMouseMove}
    on:mouseleave={handleMouseLeave}
    role="img"
    aria-label="Interactive Data Chart"
  >
    {#if type !== 'sparkline'}
      <!-- Grid Lines -->
      {#if showGrid}
        <g class="ag-grid-lines">
          {#each [0, 0.25, 0.5, 0.75, 1.0] as t}
            {@const y = padTop + plotH * t}
            <line x1={padLeft} y1={y} x2={padLeft + plotW} y2={y} stroke="var(--ag-border)" stroke-dasharray="3,3" />
            {#if showLabels}
              {@const val = maxY - (maxY - minY) * t}
              <text x={padLeft - 8} y={y + 3} class="ag-axis-text" text-anchor="end">
                {val >= 1000 ? `${(val / 1000).toFixed(1)}k` : val.toFixed(0)}
              </text>
            {/if}
          {/each}
        </g>
      {/if}

      <!-- Bottom Baseline Axis -->
      <line x1={padLeft} y1={padTop + plotH} x2={padLeft + plotW} y2={padTop + plotH} stroke="var(--ag-border-hover)" />
    {/if}

    <!-- Series Lines / Bars / Scatters -->
    {#each series as s, sIdx}
      {@const color = s.color || `hsl(${sIdx * 65 + 200}, 80%, 65%)`}

      {#if type === 'bar'}
        {@const barW = Math.max(3, (plotW / (s.points.length || 1)) * 0.7)}
        {#each s.points as p}
          {@const bx = scaleX(p.x) - barW / 2}
          {@const by = scaleY(p.y)}
          {@const bh = Math.max(2, padTop + plotH - by)}
          <rect
            x={bx}
            y={by}
            width={barW}
            height={bh}
            fill={color}
            opacity="0.85"
            rx="2"
          />
        {/each}
      {:else if type === 'scatter'}
        {#each s.points as p}
          <circle cx={scaleX(p.x)} cy={scaleY(p.y)} r="4" fill={color} opacity="0.9" />
        {/each}
      {:else}
        <!-- Line or Sparkline -->
        {@const pathD = s.points.reduce((acc, p, i) => {
          const px = scaleX(p.x);
          const py = scaleY(p.y);
          return i === 0 ? `M ${px} ${py}` : `${acc} L ${px} ${py}`;
        }, '')}
        <path
          d={pathD}
          fill="none"
          stroke={color}
          stroke-width={type === 'sparkline' ? '1.5' : '2'}
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        {#if type !== 'sparkline'}
          {#each s.points as p}
            <circle cx={scaleX(p.x)} cy={scaleY(p.y)} r="3" fill={color} />
          {/each}
        {/if}
      {/if}
    {/each}

    <!-- Hover Indicator -->
    {#if hoveredPoint}
      <line
        x1={hoveredPoint.px}
        y1={padTop}
        x2={hoveredPoint.px}
        y2={padTop + plotH}
        stroke="var(--ag-accent-primary)"
        stroke-dasharray="2,2"
      />
      <circle cx={hoveredPoint.px} cy={hoveredPoint.py} r="5" fill="var(--ag-accent-primary)" stroke="white" stroke-width="2" />
    {/if}
  </svg>

  {#if hoveredPoint}
    <div
      class="ag-chart-tooltip ag-glass-panel"
      style="left: {hoveredPoint.px + 10}px; top: {hoveredPoint.py - 30}px;"
    >
      <div class="ag-tooltip-val ag-tabular">{hoveredPoint.y.toLocaleString()}</div>
      {#if hoveredPoint.label}
        <div class="ag-tooltip-label">{hoveredPoint.label}</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .ag-chart-container {
    position: relative;
    user-select: none;
  }

  .ag-chart-svg {
    width: 100%;
    height: 100%;
    display: block;
  }

  .ag-axis-text {
    font-size: 10px;
    font-family: var(--ag-font-mono);
    fill: var(--ag-text-muted);
  }

  .ag-chart-tooltip {
    position: absolute;
    pointer-events: none;
    padding: 4px 8px;
    border-radius: var(--ag-radius-sm);
    font-size: 11px;
    z-index: 10;
    box-shadow: var(--ag-shadow-md);
  }

  .ag-tooltip-val {
    font-weight: 600;
    color: var(--ag-text-primary);
  }

  .ag-tooltip-label {
    font-size: 10px;
    color: var(--ag-text-muted);
  }
</style>

