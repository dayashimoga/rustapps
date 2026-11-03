<script lang="ts">
  import {
    SUPPORTED_LANGUAGES,
    tauriInvoke,
    type LanguageCode,
    type CameraTranslationResult,
  } from '../types';

  let subMode: 'camera' | 'subtitles' = 'camera';
  let targetLang: LanguageCode = 'es';

  // Camera OCR state
  let selectedFile: File | null = null;
  let imagePreviewUrl: string | null = null;
  let cameraResult: CameraTranslationResult | null = null;
  let isTranslatingCamera: boolean = false;

  // Subtitle state
  let subtitleContent: string = `1\n00:00:01,000 --> 00:00:04,000\nHello and welcome to the presentation.\n\n2\n00:00:04,500 --> 00:00:08,000\nToday we demonstrate 100% offline machine translation.`;
  let subtitleFormat: 'srt' | 'vtt' = 'srt';
  let translatedSubtitles: string = '';
  let isTranslatingSubs: boolean = false;

  function handleFileDrop(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer && e.dataTransfer.files.length > 0) {
      processImageFile(e.dataTransfer.files[0]);
    }
  }

  function handleFileInput(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.files && input.files.length > 0) {
      processImageFile(input.files[0]);
    }
  }

  function processImageFile(file: File) {
    selectedFile = file;
    const reader = new FileReader();
    reader.onload = async () => {
      const buffer = reader.result as ArrayBuffer;
      const bytes = new Uint8Array(buffer);
      imagePreviewUrl = URL.createObjectURL(file);
      await translateImage(bytes);
    };
    reader.readAsArrayBuffer(file);
  }

  async function translateImage(bytes: Uint8Array) {
    isTranslatingCamera = true;
    try {
      const res = await tauriInvoke<CameraTranslationResult>('translate_camera_image_cmd', {
        imageBytes: Array.from(bytes),
        targetLangStr: targetLang,
      });
      cameraResult = res;
    } catch (e) {
      console.error('Camera translation failed:', e);
    } finally {
      isTranslatingCamera = false;
    }
  }

  async function translateSubtitles() {
    if (!subtitleContent.trim()) return;
    isTranslatingSubs = true;
    try {
      const res = await tauriInvoke<string>('translate_subtitles_cmd', {
        content: subtitleContent,
        formatStr: subtitleFormat,
        sourceLangStr: null,
        targetLangStr: targetLang,
      });
      translatedSubtitles = res;
    } catch (e) {
      console.error('Subtitle translation error:', e);
    } finally {
      isTranslatingSubs = false;
    }
  }

  function downloadSubtitles() {
    const blob = new Blob([translatedSubtitles], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `translated_${targetLang}.${subtitleFormat}`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="camera-doc-container">
  <!-- Mode selector toolbar -->
  <div class="sub-nav">
    <div class="mode-btns">
      <button
        class="mode-btn"
        class:active={subMode === 'camera'}
        on:click={() => (subMode = 'camera')}
      >
        📷 Camera / Image OCR Overlay
      </button>
      <button
        class="mode-btn"
        class:active={subMode === 'subtitles'}
        on:click={() => (subMode = 'subtitles')}
      >
        📄 Subtitle &amp; Document Translator (.srt, .vtt, .md)
      </button>
    </div>

    <div class="target-select-wrap">
      <span class="label">Translate to:</span>
      <select bind:value={targetLang} class="lang-select">
        {#each SUPPORTED_LANGUAGES as lang}
          <option value={lang.code}>{lang.displayName}</option>
        {/each}
      </select>
    </div>
  </div>

  <!-- Camera OCR View -->
  {#if subMode === 'camera'}
    <div class="camera-view">
      <div
        class="drop-zone"
        role="region"
        aria-label="Image file drop zone"
        on:dragover|preventDefault
        on:drop={handleFileDrop}
      >
        {#if !imagePreviewUrl}
          <div class="drop-prompt">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
              <circle cx="8.5" cy="8.5" r="1.5"/>
              <polyline points="21 15 16 10 5 21"/>
            </svg>
            <p class="drop-text">Drag and drop an image, or browse to translate signs and documents</p>
            <label class="browse-btn">
              Browse Image
              <input type="file" accept="image/*" on:change={handleFileInput} style="display: none;" />
            </label>
          </div>
        {:else}
          <div class="image-comparison">
            <div class="image-box">
              <span class="img-badge">Original</span>
              <img src={imagePreviewUrl} alt="Original input" class="preview-img" />
            </div>
            <div class="image-box">
              <span class="img-badge badge-translated">Translated Overlay</span>
              {#if cameraResult && cameraResult.overlayImageBase64}
                <img
                  src={`data:image/bmp;base64,${cameraResult.overlayImageBase64}`}
                  alt="Translated overlay"
                  class="preview-img"
                />
              {:else if isTranslatingCamera}
                <div class="loading-overlay">
                  <span>Translating image bounding boxes...</span>
                </div>
              {:else}
                <img src={imagePreviewUrl} alt="Preview" class="preview-img dimmed" />
              {/if}
            </div>
          </div>
        {/if}
      </div>

      {#if cameraResult && cameraResult.boxes.length > 0}
        <div class="ocr-results-panel">
          <h4 class="ocr-title">Detected Text Regions ({cameraResult.boxes.length})</h4>
          <div class="ocr-grid">
            {#each cameraResult.boxes as box}
              <div class="ocr-chip">
                <span class="ocr-src">{box.originalText}</span>
                <span class="ocr-arrow">→</span>
                <span class="ocr-tgt">{box.translatedText}</span>
                <span class="ocr-conf">({Math.round(box.confidence * 100)}%)</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Subtitle & Document View -->
  {#if subMode === 'subtitles'}
    <div class="subtitles-view">
      <div class="subs-toolbar">
        <div class="format-toggle">
          <label class="radio-label">
            <input type="radio" bind:group={subtitleFormat} value="srt" />
            <span>.SRT Subtitles</span>
          </label>
          <label class="radio-label">
            <input type="radio" bind:group={subtitleFormat} value="vtt" />
            <span>.VTT Subtitles</span>
          </label>
        </div>
        <button
          class="translate-sub-btn"
          disabled={isTranslatingSubs || !subtitleContent.trim()}
          on:click={translateSubtitles}
        >
          {isTranslatingSubs ? 'Translating...' : 'Translate Subtitles'}
        </button>
      </div>

      <div class="subs-panes">
        <div class="sub-pane">
          <span class="pane-tag">Input Subtitle Cues</span>
          <textarea
            bind:value={subtitleContent}
            placeholder="Paste .srt or .vtt cues here..."
            class="subs-area"
          ></textarea>
        </div>

        <div class="sub-pane">
          <div class="pane-tag-row">
            <span class="pane-tag">Translated Output</span>
            {#if translatedSubtitles}
              <button class="download-btn" on:click={downloadSubtitles}>⬇ Download</button>
            {/if}
          </div>
          <textarea
            bind:value={translatedSubtitles}
            readonly
            placeholder="Preserved timestamps with translated text will appear here..."
            class="subs-area output-sub-area"
          ></textarea>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .camera-doc-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 1rem 1.5rem;
    overflow-y: auto;
    gap: 1rem;
  }

  .sub-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-card);
    padding: 0.6rem 1rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
  }

  .mode-btns {
    display: flex;
    gap: 0.5rem;
  }

  .mode-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    padding: 0.4rem 0.8rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    font-weight: 500;
  }

  .mode-btn.active {
    background: var(--accent-primary);
    color: white;
    border-color: var(--accent-primary);
  }

  .target-select-wrap {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .label {
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .lang-select {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    padding: 0.35rem 0.65rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
  }

  .camera-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .drop-zone {
    flex: 1;
    min-height: 280px;
    background: var(--bg-card);
    border: 2px dashed var(--border-subtle);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
  }

  .drop-prompt {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    color: var(--text-muted);
  }

  .drop-text {
    font-size: 0.9rem;
  }

  .browse-btn {
    background: var(--accent-primary);
    color: white;
    padding: 0.45rem 1rem;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
  }

  .image-comparison {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    width: 100%;
    height: 100%;
  }

  .image-box {
    position: relative;
    background: #000;
    border-radius: var(--radius-sm);
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .preview-img {
    max-width: 100%;
    max-height: 320px;
    object-fit: contain;
  }

  .preview-img.dimmed {
    opacity: 0.5;
  }

  .img-badge {
    position: absolute;
    top: 8px;
    left: 8px;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.7rem;
    background: rgba(0, 0, 0, 0.7);
    color: white;
    z-index: 10;
  }

  .badge-translated {
    background: var(--accent-primary);
  }

  .loading-overlay {
    color: var(--accent-cyan);
    font-size: 0.9rem;
  }

  .ocr-results-panel {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 0.75rem 1rem;
  }

  .ocr-title {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin-bottom: 0.5rem;
  }

  .ocr-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .ocr-chip {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    background: var(--bg-tertiary);
    padding: 0.35rem 0.65rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
  }

  .ocr-src { color: var(--text-muted); }
  .ocr-arrow { color: var(--text-faint); }
  .ocr-tgt { color: var(--text-main); font-weight: 500; }
  .ocr-conf { color: var(--accent-emerald); font-size: 0.7rem; }

  .subtitles-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .subs-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-card);
    padding: 0.5rem 1rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
  }

  .format-toggle {
    display: flex;
    gap: 1rem;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.8rem;
    color: var(--text-muted);
    cursor: pointer;
  }

  .translate-sub-btn {
    background: var(--accent-primary);
    border: none;
    color: white;
    padding: 0.4rem 0.85rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
  }

  .translate-sub-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .subs-panes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    flex: 1;
    min-height: 320px;
  }

  .sub-pane {
    display: flex;
    flex-direction: column;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 0.75rem 1rem;
    gap: 0.5rem;
  }

  .pane-tag {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .pane-tag-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .download-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--accent-emerald);
    padding: 0.2rem 0.5rem;
    border-radius: var(--radius-sm);
    font-size: 0.7rem;
    cursor: pointer;
  }

  .subs-area {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text-main);
    font-family: var(--font-mono);
    font-size: 0.85rem;
    line-height: 1.5;
    resize: none;
    outline: none;
  }

  .output-sub-area {
    color: #e2e8f0;
  }
</style>
