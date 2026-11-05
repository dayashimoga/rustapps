<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { tauriInvoke, type TimelineProject } from '../types';

  export let project: TimelineProject | null = null;
  export let activePanel: 'edit' | 'color' | 'audio' = 'edit';
  export let canUndo: boolean = false;
  export let canRedo: boolean = false;
  export let tier: string = 'Free';

  const dispatch = createEventDispatcher<{
    newProject: void;
    openProject: void;
    importMedia: void;
    saveProject: void;
    undo: void;
    redo: void;
    openAiTools: void;
    openExport: void;
    switchPanel: 'edit' | 'color' | 'audio';
    refreshProject: void;
    openLicenseModal: void;
  }>();

  let showSettingsModal = false;
  let editWidth = 1920;
  let editHeight = 1080;
  let editFps = 'Fps60';

  function openSettings() {
    if (project) {
      editWidth = project.settings.width;
      editHeight = project.settings.height;
      editFps = project.settings.frame_rate;
    }
    showSettingsModal = true;
  }

  async function saveSettings() {
    try {
      const updated = await tauriInvoke<TimelineProject>('update_project_settings_cmd', {
        width: editWidth,
        height: editHeight,
        frameRate: editFps,
      });
      if (updated && project) {
        project = updated;
      }
      showSettingsModal = false;
      dispatch('refreshProject');
    } catch (e: any) {
      console.error('Failed to update project settings:', e);
    }
  }
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
        <button type="button" class="tier-badge-btn" on:click={() => dispatch('openLicenseModal')} title="Manage Offline License">
          <span class="tier-badge tier-{tier.toLowerCase()}">[{tier.toUpperCase()}]</span>
        </button>
        {#if project}
          <button
            type="button"
            class="meta-tag-btn"
            on:click={openSettings}
            title="Click to modify Project Resolution & Frame Rate"
          >
            ⚙ {project.settings.width}×{project.settings.height} · {project.settings.frame_rate.replace('Fps', '').replace('_', '.')} fps
          </button>
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

  <!-- Right Actions: Project, Import, Undo/Redo, AI Tools & Save -->
  <div class="actions">
    <div class="project-group">
      <button
        type="button"
        class="file-btn"
        on:click={() => dispatch('newProject')}
        title="Create a clean empty timeline project"
      >
        📄 New
      </button>
      <button
        type="button"
        class="file-btn"
        on:click={() => dispatch('openProject')}
        title="Open an existing .avedproj project file"
      >
        📂 Open
      </button>
      <button
        type="button"
        class="import-btn"
        on:click={() => dispatch('importMedia')}
        title="Import real Video, Audio, or Image files into the Media Pool & Timeline"
      >
        📥 Import Media
      </button>
    </div>

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

{#if showSettingsModal}
  <div
    class="modal-backdrop"
    role="presentation"
    on:keydown={(e) => e.key === 'Escape' && (showSettingsModal = false)}
    on:click|self={() => (showSettingsModal = false)}
  >
    <div class="settings-modal">
      <div class="modal-header">
        <h3>Project Sequence Settings</h3>
        <button type="button" class="close-btn" on:click={() => (showSettingsModal = false)}>✕</button>
      </div>

      <div class="modal-body">
        <div class="preset-chips">
          <button
            type="button"
            class="chip"
            on:click={() => { editWidth = 1920; editHeight = 1080; editFps = 'Fps60'; }}
          >
            1080p 60fps
          </button>
          <button
            type="button"
            class="chip"
            on:click={() => { editWidth = 3840; editHeight = 2160; editFps = 'Fps60'; }}
          >
            4K UHD 60fps
          </button>
          <button
            type="button"
            class="chip"
            on:click={() => { editWidth = 3840; editHeight = 1608; editFps = 'Fps24'; }}
          >
            2.39:1 Cinema 24fps
          </button>
          <button
            type="button"
            class="chip"
            on:click={() => { editWidth = 1080; editHeight = 1920; editFps = 'Fps60'; }}
          >
            9:16 Vertical Short
          </button>
        </div>

        <div class="form-row">
          <label>
            Width (px)
            <input type="number" min="320" max="7680" bind:value={editWidth} />
          </label>
          <label>
            Height (px)
            <input type="number" min="240" max="4320" bind:value={editHeight} />
          </label>
        </div>

        <div class="form-row">
          <label>
            Frame Rate (FPS)
            <select bind:value={editFps}>
              <option value="Fps23_976">23.976 fps (Cinematic Film)</option>
              <option value="Fps24">24.0 fps (Standard Film)</option>
              <option value="Fps25">25.0 fps (PAL / European Broadcast)</option>
              <option value="Fps29_97">29.97 fps (NTSC Broadcast)</option>
              <option value="Fps30">30.0 fps (Web Video)</option>
              <option value="Fps50">50.0 fps (High Frame Rate PAL)</option>
              <option value="Fps59_94">59.94 fps (NTSC Broadcast HFR)</option>
              <option value="Fps60">60.0 fps (Ultra Smooth 60 FPS)</option>
              <option value="Fps120">120.0 fps (High Speed / Slow Mo)</option>
            </select>
          </label>
        </div>
      </div>

      <div class="modal-footer">
        <button type="button" class="btn-cancel" on:click={() => (showSettingsModal = false)}>Cancel</button>
        <button type="button" class="btn-apply" on:click={saveSettings}>Apply Settings</button>
      </div>
    </div>
  </div>
{/if}

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

  .sla-badge {
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
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .project-group {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .file-btn {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    padding: 0.35rem 0.65rem;
    border-radius: var(--radius-sm);
    font-size: 0.76rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .file-btn:hover {
    background: rgba(255, 255, 255, 0.12);
    border-color: rgba(255, 255, 255, 0.25);
  }

  .import-btn {
    background: rgba(56, 189, 248, 0.16);
    border: 1px solid rgba(56, 189, 248, 0.45);
    color: #38bdf8;
    padding: 0.35rem 0.75rem;
    border-radius: var(--radius-sm);
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .import-btn:hover {
    background: rgba(56, 189, 248, 0.28);
    border-color: #7dd3fc;
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

  .meta-tag-btn {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.16);
    color: #94a3b8;
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 0.72rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .meta-tag-btn:hover {
    background: rgba(56, 189, 248, 0.18);
    border-color: #38bdf8;
    color: #e2e8f0;
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .settings-modal {
    background: #111827;
    border: 1px solid #374151;
    border-radius: 8px;
    width: 440px;
    max-width: 90vw;
    box-shadow: 0 16px 32px rgba(0, 0, 0, 0.6);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.85rem 1.1rem;
    border-bottom: 1px solid #1f2937;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    color: #f3f4f6;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #9ca3af;
    cursor: pointer;
    font-size: 1rem;
  }

  .close-btn:hover {
    color: #f3f4f6;
  }

  .modal-body {
    padding: 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .preset-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  .chip {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid #374151;
    color: #cbd5e1;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 0.74rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .chip:hover {
    background: #2563eb;
    border-color: #3b82f6;
    color: white;
  }

  .form-row {
    display: flex;
    gap: 0.8rem;
  }

  .form-row label {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.78rem;
    color: #94a3b8;
  }

  .form-row input,
  .form-row select {
    background: #0b0f17;
    border: 1px solid #374151;
    border-radius: 4px;
    padding: 6px 8px;
    color: #f3f4f6;
    font-size: 0.82rem;
  }

  .form-row input:focus,
  .form-row select:focus {
    outline: none;
    border-color: #3b82f6;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 0.85rem 1.1rem;
    border-top: 1px solid #1f2937;
    background: #0c1017;
  }

  .btn-cancel {
    background: transparent;
    border: 1px solid #374151;
    color: #9ca3af;
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
  }

  .btn-cancel:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .btn-apply {
    background: #2563eb;
    border: none;
    color: white;
    padding: 6px 14px;
    border-radius: 4px;
    font-weight: 600;
    cursor: pointer;
    font-size: 0.8rem;
  }

  .btn-apply:hover {
    background: #1d4ed8;
  }

  .tier-badge-btn {
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    margin-left: 4px;
  }
  .tier-badge {
    cursor: pointer;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 10px;
    text-transform: uppercase;
    transition: transform 0.15s ease, opacity 0.15s ease;
  }
  .tier-badge:hover {
    transform: scale(1.05);
    opacity: 0.9;
  }
  .tier-badge.tier-free {
    background: rgba(148, 163, 184, 0.2);
    color: #cbd5e1;
    border: 1px solid rgba(148, 163, 184, 0.3);
  }
  .tier-badge.tier-premium {
    background: rgba(59, 130, 246, 0.25);
    color: #93c5fd;
    border: 1px solid rgba(59, 130, 246, 0.45);
  }
  .tier-badge.tier-business {
    background: rgba(168, 85, 247, 0.25);
    color: #e9d5ff;
    border: 1px solid rgba(168, 85, 247, 0.45);
  }
</style>

