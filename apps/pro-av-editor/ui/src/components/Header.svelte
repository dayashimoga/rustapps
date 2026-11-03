<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { TimelineProject } from '../types';

  export let project: TimelineProject | null = null;
  export let activePanel: 'edit' | 'color' | 'audio' = 'edit';
  export let canUndo: boolean = false;
  export let canRedo: boolean = false;

  const dispatch = createEventDispatcher<{
    newProject: void;
    saveProject: void;
    undo: void;
    redo: void;
    openAiTools: void;
    openExport: void;
    switchPanel: 'edit' | 'color' | 'audio';
  }>();
</script>

<header class="header">
  <div class="left-section">
    <div class="logo-badge">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polygon points="23 7 16 12 23 17 23 7"></polygon>
        <rect x="1" y="5" width="15" height="14" rx="2" ry="2"></rect>
      </svg>
    </div>
    <div class="project-info">
      <h1 class="project-title">{project ? project.name : 'Untitled Studio Project'}</h1>
      <div class="project-meta">
        <span class="offline-pill">100% OFFLINE</span>
        <span class="sla-badge">60 FPS SLA</span>
        {#if project}
          <span class="meta-tag">{project.settings.width}x{project.settings.height}</span>
          <span class="meta-tag">{project.settings.frame_rate.replace('Fps', '')} fps</span>
        {/if}
      </div>
    </div>
  </div>

  <!-- Central Workspace Mode Tabs -->
  <nav class="mode-tabs">
    <button
      class="tab-btn"
      class:active={activePanel === 'edit'}
      on:click={() => dispatch('switchPanel', 'edit')}
    >
      🎬 Edit &amp; Cut
    </button>
    <button
      class="tab-btn"
      class:active={activePanel === 'color'}
      on:click={() => dispatch('switchPanel', 'color')}
    >
      🎨 Color Grading
    </button>
    <button
      class="tab-btn"
      class:active={activePanel === 'audio'}
      on:click={() => dispatch('switchPanel', 'audio')}
    >
      🎚 Audio Mixer &amp; EQ
    </button>
  </nav>

  <!-- Right Actions: Undo/Redo, AI Tools & Save -->
  <div class="actions">
    <div class="undo-group">
      <button
        class="icon-btn"
        disabled={!canUndo}
        on:click={() => dispatch('undo')}
        title="Undo (Ctrl+Z)"
      >
        ↩
      </button>
      <button
        class="icon-btn"
        disabled={!canRedo}
        on:click={() => dispatch('redo')}
        title="Redo (Ctrl+Y)"
      >
        ↪
      </button>
    </div>

    <button type="button" class="ai-btn" on:click={() => dispatch('openAiTools')}>
      ✨ AI Studio
    </button>

    <button type="button" class="export-btn" on:click={() => dispatch('openExport')}>
      📦 Export Master / EDL
    </button>

    <button type="button" class="btn btn-primary" on:click={() => dispatch('saveProject')}>
      💾 Save Project
    </button>
  </div>
</header>

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.6rem 1.25rem;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-subtle);
  }

  .left-section {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .logo-badge {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
    background: linear-gradient(135deg, #6366f1, #3b82f6);
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
  }

  .project-title {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-main);
  }

  .project-meta {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-top: 0.15rem;
  }

  .offline-pill {
    font-size: 0.65rem;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: 4px;
    background: rgba(16, 185, 129, 0.15);
    color: var(--accent-emerald);
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .sla-badge, .meta-tag {
    font-size: 0.65rem;
    font-family: var(--font-mono);
    padding: 1px 5px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-muted);
  }

  .mode-tabs {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    background: var(--bg-primary);
    padding: 0.25rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
  }

  .tab-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    padding: 0.4rem 0.8rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    font-weight: 500;
    transition: all 0.15s;
  }

  .tab-btn:hover {
    color: var(--text-main);
  }

  .tab-btn.active {
    background: var(--accent-primary);
    color: white;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .undo-group {
    display: flex;
    gap: 0.25rem;
    background: var(--bg-tertiary);
    padding: 0.2rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
  }

  .icon-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    padding: 0.25rem 0.5rem;
    border-radius: 2px;
    font-size: 0.85rem;
  }

  .icon-btn:hover:not(:disabled) {
    color: var(--text-main);
    background: rgba(255, 255, 255, 0.08);
  }

  .icon-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .ai-btn {
    background: linear-gradient(135deg, rgba(99, 102, 241, 0.2), rgba(168, 85, 247, 0.2));
    border: 1px solid rgba(168, 85, 247, 0.4);
    color: #d8b4fe;
    padding: 0.4rem 0.85rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .ai-btn:hover {
    background: rgba(168, 85, 247, 0.3);
    border-color: #d8b4fe;
  }

  .export-btn {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid rgba(16, 185, 129, 0.4);
    color: #34d399;
    padding: 0.4rem 0.85rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .export-btn:hover {
    background: rgba(16, 185, 129, 0.25);
    border-color: #6ee7b7;
  }

  .btn {
    padding: 0.4rem 0.85rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    font-weight: 500;
    border: none;
  }

  .btn-primary {
    background: var(--accent-primary);
    color: white;
  }
</style>

