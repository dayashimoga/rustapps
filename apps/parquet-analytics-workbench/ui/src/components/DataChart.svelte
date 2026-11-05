<script lang="ts">
  import { onMount, afterUpdate } from 'svelte';

  export interface ChartDataPoint {
    label: string;
    value: number;
  }

  export let points: ChartDataPoint[] = [];
  export let chartType: 'Bar' | 'Line' | 'Scatter' | 'Histogram' = 'Bar';
  export let xLabel: string = 'Dimension';
  export let yLabel: string = 'Metric';
  export let executionTimeMs: number = 0;

  let canvas: HTMLCanvasElement;

  onMount(() => {
    renderChart();
    window.addEventListener('resize', renderChart);
    return () => window.removeEventListener('resize', renderChart);
  });

  afterUpdate(() => {
    renderChart();
  });

  function renderChart() {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);

    const w = rect.width;
    const h = rect.height;

    // Clear background
    ctx.fillStyle = '#0b0f19';
    ctx.fillRect(0, 0, w, h);

    if (points.length === 0) {
      ctx.fillStyle = '#64748b';
      ctx.font = '14px Inter, sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText('No data points to visualize in current chart aggregation.', w / 2, h / 2);
      return;
    }

    const padding = { top: 40, right: 30, bottom: 60, left: 70 };
    const chartW = w - padding.left - padding.right;
    const chartH = h - padding.top - padding.bottom;

    const maxVal = Math.max(...points.map((p) => p.value), 1);
    const minVal = Math.min(...points.map((p) => p.value), 0);
    const valRange = maxVal - minVal || 1;

    // Draw horizontal grid lines
    ctx.strokeStyle = 'rgba(51, 65, 85, 0.4)';
    ctx.lineWidth = 1;
    ctx.fillStyle = '#64748b';
    ctx.font = '11px JetBrains Mono, monospace';
    ctx.textAlign = 'right';

    const gridLines = 5;
    for (let i = 0; i <= gridLines; i++) {
      const y = padding.top + (chartH / gridLines) * i;
      const v = maxVal - (valRange / gridLines) * i;
      ctx.beginPath();
      ctx.moveTo(padding.left, y);
      ctx.lineTo(w - padding.right, y);
      ctx.stroke();

      ctx.fillText(v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(1), padding.left - 10, y + 4);
    }

    // Draw axes
    ctx.strokeStyle = '#475569';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(padding.left, padding.top);
    ctx.lineTo(padding.left, h - padding.bottom);
    ctx.lineTo(w - padding.right, h - padding.bottom);
    ctx.stroke();

    const n = points.length;
    const barWidth = Math.max(12, Math.min(60, (chartW / n) * 0.7));
    const step = chartW / n;

    if (chartType === 'Bar' || chartType === 'Histogram') {
      for (let i = 0; i < n; i++) {
        const p = points[i];
        const x = padding.left + i * step + (step - barWidth) / 2;
        const norm = (p.value - minVal) / valRange;
        const barH = norm * chartH;
        const y = h - padding.bottom - barH;

        // Gradient bar
        const grad = ctx.createLinearGradient(0, y, 0, h - padding.bottom);
        grad.addColorStop(0, '#06b6d4');
        grad.addColorStop(1, '#6366f1');
        ctx.fillStyle = grad;

        ctx.fillRect(x, y, barWidth, barH);

        // Bar outline
        ctx.strokeStyle = '#38bdf8';
        ctx.lineWidth = 1;
        ctx.strokeRect(x, y, barWidth, barH);

        // Value text on top of bar
        ctx.fillStyle = '#cbd5e1';
        ctx.font = '10px JetBrains Mono, monospace';
        ctx.textAlign = 'center';
        ctx.fillText(p.value.toFixed(1), x + barWidth / 2, y - 6);

        // Label below bar
        ctx.fillStyle = '#94a3b8';
        ctx.font = '11px Inter, sans-serif';
        const labelText = p.label.length > 10 ? p.label.slice(0, 9) + '…' : p.label;
        ctx.fillText(labelText, x + barWidth / 2, h - padding.bottom + 18);
      }
    } else {
      // Line or Scatter Chart
      ctx.beginPath();
      ctx.strokeStyle = '#38bdf8';
      ctx.lineWidth = 2.5;

      for (let i = 0; i < n; i++) {
        const p = points[i];
        const x = padding.left + i * step + step / 2;
        const norm = (p.value - minVal) / valRange;
        const y = h - padding.bottom - norm * chartH;

        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();

      // Draw points
      for (let i = 0; i < n; i++) {
        const p = points[i];
        const x = padding.left + i * step + step / 2;
        const norm = (p.value - minVal) / valRange;
        const y = h - padding.bottom - norm * chartH;

        ctx.fillStyle = '#06b6d4';
        ctx.beginPath();
        ctx.arc(x, y, 5, 0, Math.PI * 2);
        ctx.fill();
        ctx.strokeStyle = '#ffffff';
        ctx.lineWidth = 1.5;
        ctx.stroke();

        ctx.fillStyle = '#94a3b8';
        ctx.font = '11px Inter, sans-serif';
        ctx.textAlign = 'center';
        const labelText = p.label.length > 10 ? p.label.slice(0, 9) + '…' : p.label;
        ctx.fillText(labelText, x, h - padding.bottom + 18);
      }
    }

    // Axis Labels
    ctx.fillStyle = '#cbd5e1';
    ctx.font = '600 12px Inter, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(`${xLabel} (Dimension)`, padding.left + chartW / 2, h - 14);

    ctx.save();
    ctx.translate(20, padding.top + chartH / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText(yLabel, 0, 0);
    ctx.restore();
  }
</script>

<div class="chart-wrapper">
  <div class="chart-header">
    <span class="chart-badge">{chartType.toUpperCase()} CHART</span>
    <span class="perf-metric">Calculated in {executionTimeMs} ms</span>
  </div>
  <div class="canvas-container">
    <canvas bind:this={canvas}></canvas>
  </div>
</div>

<style>
  .chart-wrapper {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #0b0f19;
    padding: 16px;
    gap: 8px;
  }
  .chart-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 4px;
  }
  .chart-badge {
    background: rgba(6, 182, 212, 0.15);
    color: #06b6d4;
    border: 1px solid rgba(6, 182, 212, 0.3);
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 700;
  }
  .perf-metric {
    font-size: 0.75rem;
    color: #64748b;
  }
  .canvas-container {
    flex: 1;
    position: relative;
    border: 1px solid #1e293b;
    border-radius: 8px;
    overflow: hidden;
  }
  canvas {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>
