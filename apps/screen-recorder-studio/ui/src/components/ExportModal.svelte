<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invokeIpc } from '../lib/ipc';

  export let isOpen: boolean = false;
  export let sessionId: string = '';
  export let sessionTitle: string = '';

  const dispatch = createEventDispatcher<{
    close: void;
  }>();

  let format: 'InteractiveHtml' | 'Mp4' | 'WebM' | 'Gif' = 'InteractiveHtml';
  let includeAutoZoom = true;
  let trimDeadAir = true;
  let burnSubtitles = true;

  let isExporting = false;
  let exportedContent = '';
  let errorMessage = '';

  async function handleExport() {
    isExporting = true;
    errorMessage = '';
    exportedContent = '';
    try {
      const res = await invokeIpc<{ content: string }>('export_tutorial', {
        session_id: sessionId,
        config: {
          format,
          include_auto_zoom: includeAutoZoom,
          trim_dead_air: trimDeadAir,
          burn_subtitles: burnSubtitles,
          target_width: 1920,
          target_height: 1080,
        },
      });
      exportedContent = res.content;
    } catch (e: any) {
      errorMessage = e?.message || 'Tutorial export failed.';
    } finally {
      isExporting = false;
    }
  }

  function handleDownload() {
    const ext = format === 'InteractiveHtml' ? 'html' : 'srt';
    const blob = new Blob([exportedContent], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${sessionTitle.replace(/[^a-zA-Z0-9_-]/g, '_')}.${ext}`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

{#if isOpen}
  <div
    class="modal-backdrop"
    on:click={() => dispatch('close')}
    role="button"
    tabindex="0"
    on:keydown={(e) => e.key === 'Escape' && dispatch('close')}
  >
    <div
      class="modal-card"
      on:click|stopPropagation
      on:keydown|stopPropagation
      role="document"
    >
      <div class="modal-header">
        <h3>Export Tutorial & Recording</h3>
        <button class="close-btn" on:click={() => dispatch('close')}>✕</button>
      </div>

      <div class="modal-body">
        <div class="format-selection">
          <label class="form-label" for="export-format-select">Export Format</label>
          <select id="export-format-select" class="format-select" bind:value={format}>
            <option value="InteractiveHtml">Interactive HTML5 Walkthrough (.html)</option>
            <option value="Mp4">MP4 Video with Burned Subtitles (.mp4)</option>
            <option value="WebM">WebM Video (.webm)</option>
            <option value="Gif">Animated High-Res GIF (.gif)</option>
          </select>
        </div>

        <div class="toggles-grid">
          <label class="toggle-item">
            <input type="checkbox" bind:checked={includeAutoZoom} />
            <span>Apply AI Dynamic Auto-Zoom Keyframes</span>
          </label>
          <label class="toggle-item">
            <input type="checkbox" bind:checked={trimDeadAir} />
            <span>Trim Dead-Air Silences ($> 1.2s)</span>
          </label>
          <label class="toggle-item">
            <input type="checkbox" bind:checked={burnSubtitles} />
            <span>Include Subtitles & Transcript Captions</span>
          </label>
        </div>

        {#if errorMessage}
          <div class="alert error">{errorMessage}</div>
        {/if}

        {#if exportedContent}
          <div class="export-preview">
            <div class="preview-header">
              <span>Generated Output Preview</span>
              <button class="download-btn" on:click={handleDownload}>💾 Save to Disk</button>
            </div>
            <textarea readonly rows="8">{exportedContent}</textarea>
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn secondary" on:click={() => dispatch('close')}>Close</button>
        <button class="btn primary" on:click={handleExport} disabled={isExporting}>
          {isExporting ? 'Synthesizing...' : 'Export Output'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 10px;
    width: 600px;
    max-width: 90vw;
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.6);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h3 {
    font-size: 1.05rem;
    color: var(--text-main);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 1.2rem;
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .form-label {
    display: block;
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-bottom: 6px;
  }

  .format-select {
    width: 100%;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 12px;
    color: var(--text-main);
    font-size: 0.85rem;
    outline: none;
  }

  .toggles-grid {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: var(--bg-secondary);
    padding: 12px;
    border-radius: 6px;
    border: 1px solid var(--border);
  }

  .toggle-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.82rem;
    color: var(--text-muted);
    cursor: pointer;
  }

  .export-preview {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .preview-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.78rem;
    color: var(--text-sub);
  }

  .download-btn {
    background: var(--accent-green);
    border: none;
    color: #fff;
    font-size: 0.75rem;
    font-weight: 700;
    padding: 3px 10px;
    border-radius: 4px;
  }

  textarea {
    width: 100%;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px;
    color: var(--text-main);
    font-family: var(--font-mono);
    font-size: 0.78rem;
    resize: vertical;
  }

  .alert.error {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid var(--accent-red);
    color: #fca5a5;
    padding: 10px;
    border-radius: 6px;
    font-size: 0.82rem;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 20px;
    border-top: 1px solid var(--border);
  }

  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 600;
  }

  .btn.primary {
    background: var(--accent-blue);
    border: 1px solid var(--accent-blue);
    color: #fff;
  }

  .btn.secondary {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-muted);
  }
</style>
