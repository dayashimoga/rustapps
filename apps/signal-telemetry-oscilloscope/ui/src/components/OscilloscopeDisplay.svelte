<script lang="ts">
  import { onMount, afterUpdate, createEventDispatcher } from 'svelte';

  export let ch1Samples: number[] = [];
  export let ch2Samples: number[] = [];
  export let mathSamples: number[] = [];
  export let triggerLevel: number = 0.0;
  export let timePerDivMs: number = 1.0;
  export let ch1VoltsPerDiv: number = 1.0;
  export let ch2VoltsPerDiv: number = 1.0;
  export let vppCh1: number = 0.0;
  export let freqCh1: number = 0.0;
  export let vppCh2: number = 0.0;
  export let mathOperation: string = 'Add';
  export let showMath: boolean = false;
  export let showCursors: boolean = true;
  export let cursors: {
    x1_time_ms: number;
    x2_time_ms: number;
    delta_t_ms: number;
    frequency_hz: number;
    y1_volts: number;
    y2_volts: number;
    delta_v_volts: number;
  } | null = null;

  const dispatch = createEventDispatcher<{
    updateCursors: { x1: number; x2: number; y1: number; y2: number };
  }>();

  let canvas: HTMLCanvasElement;
  let cursorX: number | null = null;
  let cursorY: number | null = null;

  // Active dragging state for measurement cursors
  let draggingCursor: 'x1' | 'x2' | 'y1' | 'y2' | null = null;

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

  function renderMeasurementCursors(ctx: CanvasRenderingContext2D, width: number, height: number) {
    if (!showCursors || !cursors) return;

    const totalTimeMs = timePerDivMs * 10;
    const pixelsPerMs = width / totalTimeMs;
    const centerY = height / 2;
    const pixelsPerVolt = (height / 8) / ch1VoltsPerDiv;

    const x1Px = cursors.x1_time_ms * pixelsPerMs;
    const x2Px = cursors.x2_time_ms * pixelsPerMs;
    const y1Px = centerY - (cursors.y1_volts * pixelsPerVolt);
    const y2Px = centerY - (cursors.y2_volts * pixelsPerVolt);

    ctx.save();

    // X Cursors (Vertical Cyan Dashed)
    ctx.strokeStyle = '#38bdf8';
    ctx.setLineDash([6, 3]);
    ctx.lineWidth = 1.5;

    ctx.beginPath();
    ctx.moveTo(x1Px, 0);
    ctx.lineTo(x1Px, height);
    ctx.moveTo(x2Px, 0);
    ctx.lineTo(x2Px, height);
    ctx.stroke();

    // X Cursor labels
    ctx.fillStyle = '#38bdf8';
    ctx.font = 'bold 10px monospace';
    ctx.fillText(`X1: ${cursors.x1_time_ms.toFixed(2)}ms`, Math.max(4, x1Px + 4), 16);
    ctx.fillText(`X2: ${cursors.x2_time_ms.toFixed(2)}ms`, Math.max(4, x2Px + 4), 30);

    // Y Cursors (Horizontal Yellow Dashed)
    ctx.strokeStyle = '#facc15';
    ctx.beginPath();
    ctx.moveTo(0, y1Px);
    ctx.lineTo(width, y1Px);
    ctx.moveTo(0, y2Px);
    ctx.lineTo(width, y2Px);
    ctx.stroke();

    // Y Cursor labels
    ctx.fillStyle = '#facc15';
    ctx.fillText(`Y1: ${cursors.y1_volts.toFixed(2)}V`, 10, Math.max(14, y1Px - 4));
    ctx.fillText(`Y2: ${cursors.y2_volts.toFixed(2)}V`, 10, Math.max(14, y2Px - 4));

    // Cursor Delta Readout Pill
    const dt = Math.abs(cursors.x2_time_ms - cursors.x1_time_ms);
    const dv = Math.abs(cursors.y2_volts - cursors.y1_volts);
    const freq = dt > 0.001 ? 1000.0 / dt : 0;
    const freqStr = freq >= 1000 ? `${(freq / 1000).toFixed(2)} kHz` : `${freq.toFixed(1)} Hz`;

    const hudText = `ΔT: ${dt.toFixed(2)} ms | 1/ΔT: ${freqStr} | ΔV: ${dv.toFixed(2)} V`;
    ctx.fillStyle = 'rgba(15, 23, 42, 0.85)';
    ctx.strokeStyle = '#38bdf8';
    ctx.lineWidth = 1;
    ctx.setLineDash([]);
    ctx.fillRect(width - 320, height - 36, 310, 26);
    ctx.strokeRect(width - 320, height - 36, 310, 26);

    ctx.fillStyle = '#f8fafc';
    ctx.font = 'bold 11px monospace';
    ctx.fillText(hudText, width - 310, height - 19);

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

    // Draw Math trace if active (magenta)
    if (showMath && mathSamples.length > 0) {
      renderTrace(ctx, mathSamples, ch1VoltsPerDiv, '#d946ef', width, height);
    }

    // Draw Channel 1 trace on top (phosphor green)
    renderTrace(ctx, ch1Samples, ch1VoltsPerDiv, '#00ff66', width, height);

    // Render Trigger Line
    renderTriggerLine(ctx, width, height);

    // Render Time & Voltage Cursors
    renderMeasurementCursors(ctx, width, height);

    // Render Cursor Crosshair
    if (cursorX !== null && cursorY !== null) {
      ctx.save();
      ctx.strokeStyle = '#ffffff44';
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

    if (draggingCursor && cursors) {
      const totalTimeMs = timePerDivMs * 10;
      const centerY = canvas.height / 2;
      const pixelsPerVolt = (canvas.height / 8) / ch1VoltsPerDiv;

      let { x1_time_ms, x2_time_ms, y1_volts, y2_volts } = cursors;

      if (draggingCursor === 'x1') {
        x1_time_ms = (cursorX / canvas.width) * totalTimeMs;
      } else if (draggingCursor === 'x2') {
        x2_time_ms = (cursorX / canvas.width) * totalTimeMs;
      } else if (draggingCursor === 'y1') {
        y1_volts = (centerY - cursorY) / pixelsPerVolt;
      } else if (draggingCursor === 'y2') {
        y2_volts = (centerY - cursorY) / pixelsPerVolt;
      }

      dispatch('updateCursors', { x1: x1_time_ms, x2: x2_time_ms, y1: y1_volts, y2: y2_volts });
    }

    draw();
  }

  function handleMouseDown(e: MouseEvent) {
    if (!cursors || !showCursors) return;
    const rect = canvas.getBoundingClientRect();
    const x = (e.clientX - rect.left) * (canvas.width / rect.width);
    const y = (e.clientY - rect.top) * (canvas.height / rect.height);

    const totalTimeMs = timePerDivMs * 10;
    const pixelsPerMs = canvas.width / totalTimeMs;
    const centerY = canvas.height / 2;
    const pixelsPerVolt = (canvas.height / 8) / ch1VoltsPerDiv;

    const x1Px = cursors.x1_time_ms * pixelsPerMs;
    const x2Px = cursors.x2_time_ms * pixelsPerMs;
    const y1Px = centerY - (cursors.y1_volts * pixelsPerVolt);
    const y2Px = centerY - (cursors.y2_volts * pixelsPerVolt);

    const hitDist = 12;
    if (Math.abs(x - x1Px) < hitDist) draggingCursor = 'x1';
    else if (Math.abs(x - x2Px) < hitDist) draggingCursor = 'x2';
    else if (Math.abs(y - y1Px) < hitDist) draggingCursor = 'y1';
    else if (Math.abs(y - y2Px) < hitDist) draggingCursor = 'y2';
  }

  function handleMouseUp() {
    draggingCursor = null;
  }

  function handleMouseLeave() {
    cursorX = null;
    cursorY = null;
    draggingCursor = null;
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

    {#if showMath && mathSamples.length > 0}
      <div class="hud-item math">
        <span class="badge math-badge">MATH ({mathOperation})</span>
        <span class="hud-sub">{mathSamples.length} pts</span>
      </div>
    {/if}

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
      on:mousedown={handleMouseDown}
      on:mouseup={handleMouseUp}
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

  .math-badge {
    background: rgba(217, 70, 239, 0.2);
    color: #d946ef;
    border: 1px solid #d946ef;
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
