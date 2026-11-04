<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let show: boolean = false;

  const dispatch = createEventDispatcher<{
    close: void;
    create: {
      name: string;
      templateId: string;
      width: number;
      height: number;
      fps: number;
      includeDemoClips: boolean;
    };
  }>();

  interface TemplateCard {
    id: string;
    title: string;
    badge: string;
    description: string;
    width: number;
    height: number;
    fps: number;
    icon: string;
  }

  const templates: TemplateCard[] = [
    {
      id: 'tpl_standard_video',
      title: 'Standard Video 1080p60',
      badge: '16:9 Full HD',
      description: 'Standard 1920×1080 broadcast & YouTube workflow with multi-track video and audio.',
      width: 1920,
      height: 1080,
      fps: 60,
      icon: '🎬',
    },
    {
      id: 'tpl_vertical_shorts',
      title: 'Social Vertical Short (9:16)',
      badge: '9:16 Mobile',
      description: 'Optimized for TikTok, Instagram Reels, and YouTube Shorts portrait mobile displays.',
      width: 1080,
      height: 1920,
      fps: 60,
      icon: '📱',
    },
    {
      id: 'tpl_cinematic_239',
      title: 'Cinematic Scope (2.39:1)',
      badge: 'Anamorphic 4K',
      description: 'Theatrical 3840×1608 widescreen with 24 FPS cadence and adjustment grading track.',
      width: 3840,
      height: 1608,
      fps: 24,
      icon: '🎥',
    },
    {
      id: 'tpl_podcast_master',
      title: 'Multi-Host Podcast Studio',
      badge: 'Multi-Track',
      description: 'Dedicated microphone channels, remote guests, soundboard jingles and camera grid.',
      width: 1920,
      height: 1080,
      fps: 30,
      icon: '🎙️',
    },
    {
      id: 'tpl_4k_master',
      title: 'Ultra HD 4K Master',
      badge: '3840×2160',
      description: 'Maximum visual fidelity 4K UHD timeline ready for high-bitrate HDR mastering.',
      width: 3840,
      height: 2160,
      fps: 60,
      icon: '💎',
    },
  ];

  let projectName = `Studio Project ${new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
  let selectedTemplateId = 'tpl_standard_video';
  let customWidth = 1920;
  let customHeight = 1080;
  let customFps = 60;
  let includeDemoClips = true;

  function selectTemplate(tpl: TemplateCard) {
    selectedTemplateId = tpl.id;
    customWidth = tpl.width;
    customHeight = tpl.height;
    customFps = tpl.fps;
  }

  function handleCreate() {
    const finalName = projectName.trim() || 'Untitled Studio Project';
    dispatch('create', {
      name: finalName,
      templateId: selectedTemplateId,
      width: customWidth,
      height: customHeight,
      fps: customFps,
      includeDemoClips,
    });
    dispatch('close');
  }
</script>

{#if show}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
  <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Create New Studio Project" tabindex="-1" on:click|self={() => dispatch('close')} on:keydown={(e) => e.key === 'Escape' && dispatch('close')}>
    <div class="modal-dialog">
      <div class="modal-header">
        <div class="header-title">
          <span class="icon">✨</span>
          <h2>Create New Studio Project</h2>
          <span class="tag">100% OFFLINE</span>
        </div>
        <button type="button" class="close-btn" on:click={() => dispatch('close')} title="Close Modal">✕</button>
      </div>

      <div class="modal-body">
        <!-- Project Name -->
        <div class="field-section">
          <label for="project-name-input" class="section-label">Project Name</label>
          <input
            id="project-name-input"
            type="text"
            class="text-input"
            bind:value={projectName}
            placeholder="e.g. Commercial Reel 2026, Nature Doc, Vlog Ep 1"
          />
        </div>

        <!-- Template Chooser -->
        <div class="field-section">
          <span class="section-label">Select Starting Template</span>
          <div class="templates-grid">
            {#each templates as tpl}
              <button
                type="button"
                class="tpl-card"
                class:active={selectedTemplateId === tpl.id}
                on:click={() => selectTemplate(tpl)}
              >
                <div class="tpl-top">
                  <span class="tpl-icon">{tpl.icon}</span>
                  <span class="tpl-badge">{tpl.badge}</span>
                </div>
                <div class="tpl-title">{tpl.title}</div>
                <div class="tpl-desc">{tpl.description}</div>
                <div class="tpl-specs">{tpl.width}×{tpl.height} @ {tpl.fps} FPS</div>
              </button>
            {/each}
          </div>
        </div>

        <!-- Dimension & Frame Rate Overrides -->
        <div class="field-section settings-row">
          <div class="setting-item">
            <label for="p-width">Width (px)</label>
            <input id="p-width" type="number" min="320" max="7680" bind:value={customWidth} class="num-input" />
          </div>
          <div class="setting-item">
            <label for="p-height">Height (px)</label>
            <input id="p-height" type="number" min="240" max="4320" bind:value={customHeight} class="num-input" />
          </div>
          <div class="setting-item">
            <label for="p-fps">Frame Rate</label>
            <select id="p-fps" bind:value={customFps} class="select-input">
              <option value={24}>24 FPS (Cinematic)</option>
              <option value={25}>25 FPS (PAL / Europe)</option>
              <option value={30}>30 FPS (NTSC / Web)</option>
              <option value={50}>50 FPS (PAL HFR)</option>
              <option value={60}>60 FPS (Smooth Web)</option>
            </select>
          </div>
        </div>

        <!-- Starter Clips Checkbox -->
        <div class="field-section checkbox-section">
          <label class="checkbox-label">
            <input type="checkbox" bind:checked={includeDemoClips} />
            <span class="chk-text">
              <strong>Populate with Studio Demo Assets &amp; Timeline Clips</strong>
              <small>Pre-loads SMPTE color bars, studio audio bed, camera cut markers, and demo video clips so the timeline is instantly editable.</small>
            </span>
          </label>
        </div>
      </div>

      <div class="modal-footer">
        <button type="button" class="btn-secondary" on:click={() => dispatch('close')}>
          Cancel
        </button>
        <button type="button" class="btn-primary" on:click={handleCreate}>
          🚀 Create Project
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(8, 10, 18, 0.82);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    padding: 1.5rem;
  }

  .modal-dialog {
    background: #111422;
    border: 1px solid #2a3148;
    border-radius: 12px;
    width: 100%;
    max-width: 780px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.65), 0 0 0 1px rgba(255, 255, 255, 0.05);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: fadeInScale 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes fadeInScale {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1.1rem 1.5rem;
    border-bottom: 1px solid #1e2436;
    background: #15192b;
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 0.65rem;
  }

  .header-title h2 {
    font-size: 1.15rem;
    font-weight: 700;
    color: #f1f5f9;
    margin: 0;
  }

  .header-title .tag {
    font-size: 0.68rem;
    font-weight: 700;
    background: rgba(34, 197, 94, 0.15);
    color: #4ade80;
    border: 1px solid rgba(34, 197, 94, 0.3);
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    letter-spacing: 0.5px;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 1.2rem;
    cursor: pointer;
    padding: 0.3rem 0.6rem;
    border-radius: 6px;
    transition: all 0.15s ease;
  }

  .close-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }

  .modal-body {
    padding: 1.4rem 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    max-height: 70vh;
    overflow-y: auto;
  }

  .field-section {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .section-label {
    font-size: 0.82rem;
    font-weight: 600;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .text-input,
  .num-input,
  .select-input {
    background: #0b0e17;
    border: 1px solid #2d3748;
    color: #f8fafc;
    border-radius: 8px;
    padding: 0.65rem 0.9rem;
    font-size: 0.95rem;
    transition: border-color 0.15s ease;
  }

  .text-input:focus,
  .num-input:focus,
  .select-input:focus {
    outline: none;
    border-color: #6366f1;
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.2);
  }

  .templates-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.75rem;
  }

  .tpl-card {
    background: #171b2e;
    border: 1px solid #252c42;
    border-radius: 8px;
    padding: 0.9rem;
    text-align: left;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    transition: all 0.15s ease;
  }

  .tpl-card:hover {
    border-color: #4f46e5;
    background: #1c223a;
  }

  .tpl-card.active {
    border-color: #6366f1;
    background: #1e2444;
    box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.4);
  }

  .tpl-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .tpl-icon {
    font-size: 1.4rem;
  }

  .tpl-badge {
    font-size: 0.68rem;
    background: rgba(99, 102, 241, 0.15);
    color: #818cf8;
    padding: 0.15rem 0.45rem;
    border-radius: 4px;
    font-weight: 600;
  }

  .tpl-title {
    font-size: 0.9rem;
    font-weight: 700;
    color: #f1f5f9;
  }

  .tpl-desc {
    font-size: 0.74rem;
    color: #94a3b8;
    line-height: 1.35;
    flex: 1;
  }

  .tpl-specs {
    font-size: 0.72rem;
    font-family: monospace;
    color: #38bdf8;
    margin-top: 0.2rem;
  }

  .settings-row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1rem;
    background: #15192b;
    padding: 1rem;
    border-radius: 8px;
    border: 1px solid #1e2436;
  }

  .setting-item {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .setting-item label {
    font-size: 0.78rem;
    font-weight: 600;
    color: #cbd5e1;
  }

  .checkbox-section {
    background: #15192b;
    padding: 0.85rem 1rem;
    border-radius: 8px;
    border: 1px solid #1e2436;
  }

  .checkbox-label {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    cursor: pointer;
  }

  .checkbox-label input[type='checkbox'] {
    width: 1.1rem;
    height: 1.1rem;
    margin-top: 0.2rem;
    accent-color: #6366f1;
    cursor: pointer;
  }

  .chk-text {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .chk-text strong {
    font-size: 0.88rem;
    color: #f1f5f9;
  }

  .chk-text small {
    font-size: 0.74rem;
    color: #94a3b8;
    line-height: 1.3;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    border-top: 1px solid #1e2436;
    background: #15192b;
  }

  .btn-secondary {
    background: #1e2436;
    border: 1px solid #2a3148;
    color: #cbd5e1;
    padding: 0.6rem 1.2rem;
    border-radius: 7px;
    font-size: 0.88rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-secondary:hover {
    background: #252c42;
    color: #fff;
  }

  .btn-primary {
    background: #4f46e5;
    border: 1px solid #6366f1;
    color: #ffffff;
    padding: 0.6rem 1.4rem;
    border-radius: 7px;
    font-size: 0.88rem;
    font-weight: 700;
    cursor: pointer;
    box-shadow: 0 4px 12px rgba(79, 70, 229, 0.35);
    transition: all 0.15s ease;
  }

  .btn-primary:hover {
    background: #4338ca;
    box-shadow: 0 6px 16px rgba(79, 70, 229, 0.5);
  }
</style>

