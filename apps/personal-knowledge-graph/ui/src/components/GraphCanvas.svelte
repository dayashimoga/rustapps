<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';

  export let nodes: any[] = [];
  export let edges: any[] = [];
  export let selectedNodeId: string | null = null;

  const dispatch = createEventDispatcher();

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;

  // Viewport transforms
  let panX = 0;
  let panY = 0;
  let zoom = 1.0;
  let isDragging = false;
  let dragStartX = 0;
  let dragStartY = 0;

  // Physics node positions
  interface SimNode {
    id: string;
    title: string;
    type: string;
    x: number;
    y: number;
    vx: number;
    vy: number;
    radius: number;
  }

  let simNodes: Map<string, SimNode> = new Map();

  function getNodeColor(type: string): string {
    switch (type) {
      case 'document':
        return '#10b981'; // Emerald
      case 'concept':
        return '#f59e0b'; // Amber
      case 'citation':
        return '#8b5cf6'; // Violet
      case 'excerpt':
        return '#f43f5e'; // Rose
      default:
        return '#06b6d4'; // Cyan (Note)
    }
  }

  function initPositions() {
    if (!canvas) return;
    const width = canvas.width || 800;
    const height = canvas.height || 600;

    const newMap = new Map<string, SimNode>();
    nodes.forEach((n, idx) => {
      const existing = simNodes.get(n.id);
      if (existing) {
        newMap.set(n.id, { ...existing, title: n.title, type: n.node_type });
      } else {
        const angle = (idx / Math.max(1, nodes.length)) * Math.PI * 2;
        const radius = Math.min(width, height) * 0.35;
        newMap.set(n.id, {
          id: n.id,
          title: n.title,
          type: n.node_type,
          x: width / 2 + Math.cos(angle) * radius + (Math.random() - 0.5) * 50,
          y: height / 2 + Math.sin(angle) * radius + (Math.random() - 0.5) * 50,
          vx: 0,
          vy: 0,
          radius: n.id === selectedNodeId ? 18 : 12,
        });
      }
    });
    simNodes = newMap;
    draw();
  }

  $: if (nodes || edges || selectedNodeId) {
    initPositions();
  }

  function draw() {
    if (!ctx || !canvas) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    ctx.save();
    ctx.translate(panX, panY);
    ctx.scale(zoom, zoom);

    // Draw grid
    ctx.strokeStyle = 'rgba(75, 85, 99, 0.15)';
    ctx.lineWidth = 1;
    const gridSize = 40;
    const left = -panX / zoom;
    const top = -panY / zoom;
    const right = (canvas.width - panX) / zoom;
    const bottom = (canvas.height - panY) / zoom;

    ctx.beginPath();
    for (let x = Math.floor(left / gridSize) * gridSize; x < right; x += gridSize) {
      ctx.moveTo(x, top);
      ctx.lineTo(x, bottom);
    }
    for (let y = Math.floor(top / gridSize) * gridSize; y < bottom; y += gridSize) {
      ctx.moveTo(left, y);
      ctx.lineTo(right, y);
    }
    ctx.stroke();

    // Draw Edges
    edges.forEach((edge) => {
      const s = simNodes.get(edge.source_id);
      const t = simNodes.get(edge.target_id);
      if (s && t) {
        ctx.beginPath();
        ctx.moveTo(s.x, s.y);
        ctx.lineTo(t.x, t.y);

        const isHighlighted = edge.source_id === selectedNodeId || edge.target_id === selectedNodeId;
        ctx.strokeStyle = isHighlighted ? '#6366f1' : 'rgba(156, 163, 175, 0.35)';
        ctx.lineWidth = isHighlighted ? 2.5 : 1.2;
        ctx.stroke();

        // Edge label
        if (edge.label || edge.relation_type) {
          const midX = (s.x + t.x) / 2;
          const midY = (s.y + t.y) / 2;
          ctx.font = '10px sans-serif';
          ctx.fillStyle = isHighlighted ? '#c7d2fe' : 'rgba(156, 163, 175, 0.6)';
          ctx.textAlign = 'center';
          ctx.fillText(edge.label || edge.relation_type, midX, midY - 4);
        }
      }
    });

    // Draw Nodes
    simNodes.forEach((node) => {
      const isSelected = node.id === selectedNodeId;
      const color = getNodeColor(node.type);

      // Outer glow for selected node
      if (isSelected) {
        ctx.beginPath();
        ctx.arc(node.x, node.y, node.radius + 6, 0, Math.PI * 2);
        ctx.fillStyle = 'rgba(99, 102, 241, 0.35)';
        ctx.fill();
      }

      // Main circle
      ctx.beginPath();
      ctx.arc(node.x, node.y, node.radius, 0, Math.PI * 2);
      ctx.fillStyle = color;
      ctx.fill();
      ctx.strokeStyle = isSelected ? '#ffffff' : 'rgba(0, 0, 0, 0.4)';
      ctx.lineWidth = isSelected ? 2.5 : 1.5;
      ctx.stroke();

      // Label
      ctx.font = isSelected ? 'bold 12px sans-serif' : '11px sans-serif';
      ctx.fillStyle = isSelected ? '#ffffff' : '#e5e7eb';
      ctx.textAlign = 'center';
      ctx.fillText(node.title, node.x, node.y + node.radius + 14);
    });

    ctx.restore();
  }

  function handleMouseDown(e: MouseEvent) {
    const rect = canvas.getBoundingClientRect();
    const mouseX = (e.clientX - rect.left - panX) / zoom;
    const mouseY = (e.clientY - rect.top - panY) / zoom;

    // Check hit on nodes
    let hitNode: SimNode | null = null;
    simNodes.forEach((node) => {
      const dist = Math.hypot(node.x - mouseX, node.y - mouseY);
      if (dist <= node.radius + 4) {
        hitNode = node;
      }
    });

    if (hitNode) {
      dispatch('select', { node: hitNode });
      return;
    }

    isDragging = true;
    dragStartX = e.clientX - panX;
    dragStartY = e.clientY - panY;
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    panX = e.clientX - dragStartX;
    panY = e.clientY - dragStartY;
    draw();
  }

  function handleMouseUp() {
    isDragging = false;
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    const zoomFactor = e.deltaY < 0 ? 1.1 : 0.9;
    zoom = Math.max(0.2, Math.min(3.0, zoom * zoomFactor));
    draw();
  }

  onMount(() => {
    ctx = canvas.getContext('2d');
    const resize = () => {
      if (canvas && canvas.parentElement) {
        canvas.width = canvas.parentElement.clientWidth;
        canvas.height = canvas.parentElement.clientHeight;
        initPositions();
      }
    };
    resize();
    window.addEventListener('resize', resize);
    return () => window.removeEventListener('resize', resize);
  });
</script>

<div class="relative w-full h-full overflow-hidden bg-slate-950">
  <canvas
    bind:this={canvas}
    on:mousedown={handleMouseDown}
    on:mousemove={handleMouseMove}
    on:mouseup={handleMouseUp}
    on:mouseleave={handleMouseUp}
    on:wheel={handleWheel}
    class="w-full h-full cursor-grab active:cursor-grabbing block"
  ></canvas>

  <!-- Legend overlay -->
  <div class="absolute bottom-3 left-3 bg-slate-900/90 border border-slate-700/60 rounded-lg p-2.5 text-xs flex gap-3.5 backdrop-blur-md shadow-lg pointer-events-none">
    <div class="flex items-center gap-1.5"><span class="w-2.5 h-2.5 rounded-full bg-cyan-400"></span> Note</div>
    <div class="flex items-center gap-1.5"><span class="w-2.5 h-2.5 rounded-full bg-emerald-400"></span> Document</div>
    <div class="flex items-center gap-1.5"><span class="w-2.5 h-2.5 rounded-full bg-amber-400"></span> Concept</div>
    <div class="flex items-center gap-1.5"><span class="w-2.5 h-2.5 rounded-full bg-purple-400"></span> Citation</div>
    <div class="flex items-center gap-1.5"><span class="w-2.5 h-2.5 rounded-full bg-rose-400"></span> Excerpt</div>
  </div>
</div>

