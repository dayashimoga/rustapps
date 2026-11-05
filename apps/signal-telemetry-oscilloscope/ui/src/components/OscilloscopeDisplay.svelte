<script lang="ts">
  import { onMount, afterUpdate } from 'svelte';

  export let ch1Samples: number[] = [];
  export let ch2Samples: number[] = [];
  export let triggerLevel: number = 0.0;
  export let timePerDivMs: number = 1.0;
  export let ch1VoltsPerDiv: number = 1.0;
  export let ch2VoltsPerDiv: number = 1.0;
  export let vppCh1: number = 0.0;
  export let freqCh1: number = 0.0;
  export let vppCh2: number = 0.0;

  let canvas: HTMLCanvasElement;
  let cursorX: number | null = null;
  let cursorY: number | null = null;

  function renderGrid(ctx: CanvasRenderingContext2D, width: number, height: number) {
    ctx.fillStyle = '#0a1017';
    ctx.fillRect(0, 0, width, height);

    ctx.strokeStyle = '#1a2636';
    ctx.lineWidth = 1;

    // 10 horizontal divisions, 8 vertical divisions
    const divX = width / 10;
    const divY = height / 8;

    for (let i = 1; i < 10; i++) {
      ctx.beginPath();
      ctx.moveTo(i * divX, 0);
      ctx.lineTo(i * divX, height);
      ctx.stroke();
    }

    for (let j = 1; j < 8; j++) {
      ctx.beginPath();
      ctx.moveTo(0, j * divY);
      ctx.lineTo(width, j * divY);
      ctx.stroke();
    }

    // Center axes (subdivided tick marks)
    ctx.strokeStyle = '#2d425f';
    ctx.lineWidth = 1.5;

    ctx.beginPath();
    ctx.moveTo(width / 2, 0);
    ctx.lineTo(width / 2, height);
    ctx.stroke();

    ctx.beginPath();
    ctx.moveTo(0, height / 2);
    ctx.lineTo(width, height / 2);
    ctx.stroke();
  }

  function renderTrace(
    ctx: CanvasRenderingContext2D,
    samples: number[],
    voltsPerDiv: number,
    color: string,
    width: number,
    height: number,
    glow: boolean = true
  ) {
    if (!samples || samples.length === 0) return;

    const centerY = height / 2;
    const pixelsPerVolt = (height / 8) / voltsPerDiv;
    const stepX = width / (samples.length - 1);

    ctx.save();
    ctx.strokeStyle = color;
    ctx.lineWidth = 2.0;

    if (glow) {
      ctx.shadowColor = color;
      ctx.shadowBlur = 6;
    }

    ctx.beginPath();
    for (let i = 0; i < samples.length; i++) {
      const x = i * stepX;
      const y = centerY - (samples[i] * pixelsPerVolt);
      if (i === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    }
    ctx.stroke();
    ctx.restore();
  }

  function renderTriggerLine(ctx: CanvasRenderingContext2D, width: number, height: number) {
    const centerY = height / 2;
    const pixelsPerVolt = (height / 8) / ch1VoltsPerDiv;
    const trigY = centerY - (triggerLevel * pixelsPerVolt);

    ctx.save();
    ctx.strokeStyle = '#ff9800';
    ctx.setLineDash([4, 4]);
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    ctx.moveTo(0, trigY);
    ctx.lineTo(width, trigY);
    ctx.stroke();

    // Trigger flag indicator
    ctx.fillStyle = '#ff9800';
    ctx.font = '10px monospace';
    ctx.fillText(`T: ${triggerLevel.toFixed(2)}V`, width - 65, Math.max(12, Math.min(height - 6, trigY - 4)));
    ctx.restore();
  }

  function draw() {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    renderGrid(ctx, width, height);

    // Draw Channel 2 trace first (cyan)
    renderTrace(ctx, ch2Samples, ch2VoltsPerDiv, '#00d4ff', width, height);

    // Draw Channel 1 trace on top (phosphor green)
    renderTrace(ctx, ch1Samples, ch1VoltsPerDiv, '#00ff66', width, height);

    // Render Trigger Line
    renderTriggerLine(ctx, width, height);

    // Render Cursor Crosshair
    if (cursorX !== null && cursorY !== null) {
      ctx.save();
      ctx.strokeStyle = '#ffffff66';
      ctx.setLineDash([2, 2]);
      ctx.beginPath();
      ctx.moveTo(cursorX, 0);
      ctx.lineTo(cursorX, height);
      ctx.moveTo(0, cursorY);
      ctx.lineTo(width, cursorY);
      ctx.stroke();
      ctx.restore();
    }
  }

  function handleMouseMove(e: MouseEvent) {
    const rect = canvas.getBoundingClientRect();
    cursorX = (e.clientX - rect.left) * (canvas.width / rect.width);
    cursorY = (e.clientY - rect.top) * (canvas.height / rect.height);
    draw();
  }

  function handleMouseLeave() {
    cursorX = null;
    cursorY = null;
    draw();
  }

  onMount(() => {
    draw();
  });

  afterUpdate(() => {
    draw();
  });
</script>

<div class="scope-display-container">
  <div class="scope-header">
    <div class="hud-item ch1">
      <span class="badge ch1-badge">CH1</span>
      <span class="hud-val">{ch1VoltsPerDiv} V/div</span>
      <span class="hud-sub">Vpp: {vppCh1.toFixed(2)}V</span>
      <span class="hud-sub">Freq: {freqCh1 >= 1000 ? (freqCh1 / 1000).toFixed(2) + ' kHz' : freqCh1.toFixed(1) + ' Hz'}</span>
    </div>

    <div class="hud-item ch2">
      <span class="badge ch2-badge">CH2</span>
      <span class="hud-val">{ch2VoltsPerDiv} V/div</span>
      <span class="hud-sub">Vpp: {vppCh2.toFixed(2)}V</span>
    </div>

    <div class="hud-item timebase">
      <span class="badge tb-badge">TB</span>
      <span class="hud-val">{timePerDivMs} ms/div</span>
    </div>
  </div>

  <div class="canvas-wrap">
    <canvas
      bind:this={canvas}
      width={1000}
      height={520}
      on:mousemove={handleMouseMove}
      on:mouseleave={handleMouseLeave}
    ></canvas>
  </div>
</div>

<style>
  .scope-display-container {
    display: flex;
    flex-direction: column;
    background: #0d131a;
    border: 1px solid #1e2c3d;
    border-radius: 8px;
    overflow: hidden;
    height: 100%;
  }

  .scope-header {
    display: flex;
    gap: 16px;
    padding: 8px 14px;
    background: #141c26;
    border-bottom: 1px solid #1e2c3d;
    align-items: center;
    font-size: 12px;
    font-family: var(--font-mono);
  }

  .hud-item {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .badge {
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: bold;
    font-size: 10px;
  }

  .ch1-badge {
    background: #00ff6622;
    color: #00ff66;
    border: 1px solid #00ff6666;
  }

  .ch2-badge {
    background: #00d4ff22;
    color: #00d4ff;
    border: 1px solid #00d4ff66;
  }

  .tb-badge {
    background: #ffaa0022;
    color: #ffaa00;
    border: 1px solid #ffaa0066;
  }

  .hud-val {
    font-weight: 600;
  }

  .hud-sub {
    color: #8b949e;
  }

  .canvas-wrap {
    flex: 1;
    position: relative;
    display: flex;
    justify-content: center;
    align-items: center;
    background: #0a0e14;
    padding: 6px;
  }

  canvas {
    width: 100%;
    height: 100%;
    object-fit: fill;
    border-radius: 4px;
    cursor: crosshair;
  }
</style>

