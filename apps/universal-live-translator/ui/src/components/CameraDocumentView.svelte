<script lang="ts">
  import {
    SUPPORTED_LANGUAGES,
    tauriInvoke,
    type LanguageCode,
    type CameraTranslationResult,
    type CameraOcrBox,
  } from '../types';

  let subMode: 'camera' | 'subtitles' = 'camera';
  let targetLang: LanguageCode = 'es';

  // Camera OCR state
  let selectedFile: File | null = null;
  let imagePreviewUrl: string | null = null;
  let cameraResult: CameraTranslationResult | null = null;
  let isTranslatingCamera: boolean = false;
  let cameraViewMode: 'ar' | 'bmp' | 'split' = 'ar';
  let canvasElement: HTMLCanvasElement | null = null;
  let imageElement: HTMLImageElement | null = null;
  let hoveredBoxIndex: number | null = null;
  let naturalWidth = 640;
  let naturalHeight = 360;

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
    cameraResult = null;
    try {
      const res = await tauriInvoke<CameraTranslationResult>('translate_camera_image_cmd', {
        imageBytes: Array.from(bytes),
        targetLangStr: targetLang,
      });
      cameraResult = res;
      setTimeout(renderBoundingBoxesCanvas, 50);
    } catch (e) {
      console.error('Camera translation failed:', e);
    } finally {
      isTranslatingCamera = false;
    }
  }

  function handleImageLoad(e: Event) {
    const img = e.target as HTMLImageElement;
    naturalWidth = img.naturalWidth || 640;
    naturalHeight = img.naturalHeight || 360;
    if (canvasElement) {
      canvasElement.width = naturalWidth;
      canvasElement.height = naturalHeight;
      renderBoundingBoxesCanvas();
    }
  }

  function renderBoundingBoxesCanvas() {
    if (!canvasElement || !cameraResult || !cameraResult.boxes) return;
    const ctx = canvasElement.getContext('2d');
    if (!ctx) return;

    ctx.clearRect(0, 0, canvasElement.width, canvasElement.height);

    for (let i = 0; i < cameraResult.boxes.length; i++) {
      const b = cameraResult.boxes[i];
      const isHovered = hoveredBoxIndex === i;

      // Draw background pill masking original text
      ctx.fillStyle = isHovered ? 'rgba(15, 23, 42, 0.95)' : 'rgba(15, 23, 42, 0.85)';
      ctx.fillRect(b.x, b.y, b.width, b.height);

      // Draw glowing outline
      ctx.lineWidth = isHovered ? 2.5 : 1.5;
      ctx.strokeStyle = isHovered ? '#f59e0b' : (b.confidence > 0.8 ? '#10b981' : '#38bdf8');
      ctx.strokeRect(b.x, b.y, b.width, b.height);

      // Draw translated text
      ctx.fillStyle = isHovered ? '#fbbf24' : '#f8fafc';
      const fontSize = Math.max(10, Math.min(14, b.height - 4));
      ctx.font = `600 ${fontSize}px sans-serif`;
      ctx.textBaseline = 'middle';
      ctx.fillText(b.translatedText, b.x + 4, b.y + b.height / 2, b.width - 8);

      // Draw confidence tag when hovered
      if (isHovered) {
        ctx.fillStyle = '#10b981';
        ctx.font = 'bold 9px monospace';
        ctx.fillText(`${Math.round(b.confidence * 100)}%`, b.x + b.width - 26, b.y + 9);
      }
    }
  }

  function handleCanvasMouseMove(e: MouseEvent) {
    if (!cameraResult || !canvasElement) return;
    const rect = canvasElement.getBoundingClientRect();
    const scaleX = canvasElement.width / rect.width;
    const scaleY = canvasElement.height / rect.height;
    const mouseX = (e.clientX - rect.left) * scaleX;
    const mouseY = (e.clientY - rect.top) * scaleY;

    let found: number | null = null;
    for (let i = 0; i < cameraResult.boxes.length; i++) {
      const b = cameraResult.boxes[i];
      if (
        mouseX >= b.x &&
        mouseX <= b.x + b.width &&
        mouseY >= b.y &&
        mouseY <= b.y + b.height
      ) {
        found = i;
        break;
      }
    }

    if (hoveredBoxIndex !== found) {
      hoveredBoxIndex = found;
      renderBoundingBoxesCanvas();
    }
  }

  function handleCanvasMouseLeave() {
    if (hoveredBoxIndex !== null) {
      hoveredBoxIndex = null;
      renderBoundingBoxesCanvas();
    }
  }

  function loadSampleImage(scenario: 'airport' | 'menu') {
    const canvas = document.createElement('canvas');
    canvas.width = 640;
    canvas.height = 360;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    if (scenario === 'airport') {
      ctx.fillStyle = '#0f172a';
      ctx.fillRect(0, 0, 640, 360);

      ctx.fillStyle = '#1e293b';
      ctx.fillRect(30, 30, 580, 300);
      ctx.strokeStyle = '#38bdf8';
      ctx.lineWidth = 3;
      ctx.strokeRect(30, 30, 580, 300);

      ctx.fillStyle = '#ffffff';
      ctx.font = 'bold 22px monospace';
      ctx.fillText('TERMINAL 2 INTERNATIONAL DEPARTURES', 50, 90);

      ctx.fillStyle = '#38bdf8';
      ctx.font = 'bold 18px monospace';
      ctx.fillText('BAGGAGE CLAIM & PASSPORT CONTROL', 50, 150);

      ctx.fillStyle = '#f59e0b';
      ctx.font = 'bold 18px monospace';
      ctx.fillText('AUTHORIZED PERSONNEL ONLY BEYOND THIS POINT', 50, 210);

      ctx.fillStyle = '#10b981';
      ctx.font = 'bold 16px monospace';
      ctx.fillText('EMERGENCY EXIT KEEP CLEAR AT ALL TIMES', 50, 270);
    } else {
      ctx.fillStyle = '#1e1b4b';
      ctx.fillRect(0, 0, 640, 360);

      ctx.fillStyle = '#f8fafc';
      ctx.font = 'bold 24px monospace';
      ctx.fillText('DAILY SPECIALS & CHEF RECOMMENDATION', 50, 80);

      ctx.fillStyle = '#cbd5e1';
      ctx.font = '17px monospace';
      ctx.fillText('FRESH MEDITERRANEAN GRILLED SEAFOOD PLATTER', 50, 140);
      ctx.fillText('ORGANIC ARTISAN PASTA WITH BLACK TRUFFLE', 50, 200);
      ctx.fillText('CLASSIC TIRAMISU WITH ESPRESSO AND MASCARPONE', 50, 260);
    }

    canvas.toBlob((blob) => {
      if (blob) {
        const file = new File([blob], `${scenario}_sample.png`, { type: 'image/png' });
        processImageFile(file);
      }
    }, 'image/png');
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
        type="button"
        class="mode-btn"
        class:active={subMode === 'camera'}
        on:click={() => (subMode = 'camera')}
      >
        📷 Camera / Image OCR Overlay
      </button>
      <button
        type="button"
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
      <!-- Camera Sub-controls -->
      <div class="camera-toolbar">
        <div class="view-toggles">
          <span class="toolbar-label">Display Mode:</span>
          <button
            type="button"
            class="view-btn"
            class:active={cameraViewMode === 'ar'}
            on:click={() => (cameraViewMode = 'ar')}
          >
            AR Canvas Overlay
          </button>
          <button
            type="button"
            class="view-btn"
            class:active={cameraViewMode === 'bmp'}
            on:click={() => (cameraViewMode = 'bmp')}
          >
            Rendered BMP
          </button>
          <button
            type="button"
            class="view-btn"
            class:active={cameraViewMode === 'split'}
            on:click={() => (cameraViewMode = 'split')}
          >
            Side-by-Side
          </button>
        </div>

        <div class="sample-btn-group">
          <span class="toolbar-label">Quick Test:</span>
          <button type="button" class="sample-btn" on:click={() => loadSampleImage('airport')}>
            ✈️ Airport Sign
          </button>
          <button type="button" class="sample-btn" on:click={() => loadSampleImage('menu')}>
            🍽️ Restaurant Menu
          </button>
        </div>
      </div>

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
            <p class="drop-text">Drag and drop an image or screenshot to translate offline</p>
            <div class="drop-actions">
              <label class="browse-btn">
                Browse Image
                <input type="file" accept="image/*" on:change={handleFileInput} style="display: none;" />
              </label>
              <button type="button" class="sample-btn-main" on:click={() => loadSampleImage('airport')}>
                Load Sample Airport Sign
              </button>
            </div>
          </div>
        {:else}
          <div class="viewport-stage" class:split={cameraViewMode === 'split'}>
            <!-- Primary Image & Interactive Canvas -->
            <div class="canvas-wrap" class:hidden={cameraViewMode === 'bmp'}>
              <span class="img-badge">
                {cameraViewMode === 'split' ? 'Original' : 'AR Interactive Overlay'}
              </span>
              <img
                bind:this={imageElement}
                src={imagePreviewUrl}
                alt="Source preview"
                class="stage-img"
                on:load={handleImageLoad}
              />
              <canvas
                bind:this={canvasElement}
                width={naturalWidth}
                height={naturalHeight}
                class="ar-canvas"
                class:interactive={cameraViewMode === 'ar'}
                on:mousemove={handleCanvasMouseMove}
                on:mouseleave={handleCanvasMouseLeave}
              ></canvas>
              {#if isTranslatingCamera}
                <div class="loading-overlay">
                  <span class="spinner-sm"></span> Translating bounding boxes...
                </div>
              {/if}
            </div>

            <!-- Pre-rendered BMP or Side-by-Side Target -->
            {#if cameraViewMode === 'bmp' || cameraViewMode === 'split'}
              <div class="bmp-wrap">
                <span class="img-badge badge-translated">Rendered Output</span>
                {#if cameraResult && cameraResult.overlayImageBase64}
                  <img
                    src={`data:image/bmp;base64,${cameraResult.overlayImageBase64}`}
                    alt="Translated overlay"
                    class="stage-img"
                  />
                {:else if isTranslatingCamera}
                  <div class="loading-overlay">
                    <span class="spinner-sm"></span> Compositing image...
                  </div>
                {:else}
                  <img src={imagePreviewUrl} alt="Preview" class="stage-img dimmed" />
                {/if}
              </div>
            {/if}
          </div>
        {/if}
      </div>

      {#if cameraResult && cameraResult.boxes.length > 0}
        <div class="ocr-results-panel">
          <div class="panel-header-row">
            <h4 class="ocr-title">Detected Text Regions ({cameraResult.boxes.length})</h4>
            <span class="latency-pill">{cameraResult.latencyMs} ms processing</span>
          </div>
          <div class="ocr-grid">
            {#each cameraResult.boxes as box, i}
              <div
                class="ocr-chip"
                class:active-chip={hoveredBoxIndex === i}
                on:mouseenter={() => { hoveredBoxIndex = i; renderBoundingBoxesCanvas(); }}
                on:mouseleave={() => { hoveredBoxIndex = null; renderBoundingBoxesCanvas(); }}
                role="button"
                tabindex="0"
              >
                <span class="ocr-src">{box.originalText}</span>
                <span class="ocr-arrow">→</span>
                <span class="ocr-tgt">{box.translatedText}</span>
                <span class="ocr-conf">{Math.round(box.confidence * 100)}%</span>
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
          type="button"
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
              <button type="button" class="download-btn" on:click={downloadSubtitles}>⬇ Download</button>
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
    cursor: pointer;
    transition: all 0.15s ease;
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
    gap: 0.75rem;
  }

  .camera-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 0.45rem 0.85rem;
  }

  .view-toggles, .sample-btn-group {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .toolbar-label {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-right: 2px;
  }

  .view-btn, .sample-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .view-btn:hover, .sample-btn:hover {
    color: var(--text-main);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .view-btn.active {
    background: rgba(56, 189, 248, 0.15);
    border-color: rgba(56, 189, 248, 0.4);
    color: var(--accent-cyan);
    font-weight: 600;
  }

  .drop-zone {
    flex: 1;
    min-height: 320px;
    background: var(--bg-card);
    border: 2px dashed var(--border-subtle);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    position: relative;
    overflow: hidden;
  }

  .drop-prompt {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    color: var(--text-muted);
    text-align: center;
  }

  .drop-text {
    font-size: 0.9rem;
  }

  .drop-actions {
    display: flex;
    gap: 8px;
    margin-top: 4px;
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

  .sample-btn-main {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--accent-cyan);
    padding: 0.45rem 1rem;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
  }

  .viewport-stage {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    gap: 1rem;
  }

  .viewport-stage.split {
    display: grid;
    grid-template-columns: 1fr 1fr;
  }

  .canvas-wrap, .bmp-wrap {
    position: relative;
    background: #000;
    border-radius: var(--radius-sm);
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    max-height: 400px;
    width: 100%;
    height: 100%;
  }

  .canvas-wrap.hidden {
    display: none;
  }

  .stage-img {
    max-width: 100%;
    max-height: 400px;
    object-fit: contain;
    display: block;
  }

  .stage-img.dimmed {
    opacity: 0.4;
  }

  .ar-canvas {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
    pointer-events: none;
  }

  .ar-canvas.interactive {
    pointer-events: auto;
    cursor: crosshair;
  }

  .img-badge {
    position: absolute;
    top: 8px;
    left: 8px;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.7rem;
    background: rgba(0, 0, 0, 0.75);
    color: white;
    z-index: 10;
    border: 1px solid rgba(255, 255, 255, 0.15);
  }

  .badge-translated {
    background: var(--accent-primary);
  }

  .loading-overlay {
    position: absolute;
    inset: 0;
    background: rgba(15, 23, 42, 0.75);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--accent-cyan);
    font-size: 0.85rem;
    z-index: 20;
  }

  .ocr-results-panel {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 0.75rem 1rem;
  }

  .panel-header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .ocr-title {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin: 0;
  }

  .latency-pill {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-muted);
    background: rgba(255, 255, 255, 0.05);
    padding: 2px 6px;
    border-radius: 4px;
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
    border: 1px solid transparent;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .ocr-chip:hover, .ocr-chip.active-chip {
    border-color: rgba(56, 189, 248, 0.5);
    background: rgba(56, 189, 248, 0.1);
  }

  .ocr-src { color: var(--text-muted); }
  .ocr-arrow { color: var(--text-faint); }
  .ocr-tgt { color: var(--text-main); font-weight: 500; }
  .ocr-conf { color: var(--accent-emerald); font-size: 0.7rem; }

  /* Subtitles View */
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

  .spinner-sm {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #38bdf8;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    display: inline-block;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
