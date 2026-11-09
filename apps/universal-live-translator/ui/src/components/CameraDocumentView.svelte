<script lang="ts">
  import {
    SUPPORTED_LANGUAGES,
    tauriInvoke,
    type LanguageCode,
    type CameraTranslationResult,
    type CameraOcrBox,
    type BatchDocumentItem,
    type BatchTranslationResult,
  } from '../types';

  let subMode: 'camera' | 'subtitles' = 'camera';
  let targetLang: LanguageCode = 'es';
  let domainGlossary: string = '';

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

  // Subtitle & Document state
  let docSubTab: 'single' | 'batch' = 'single';
  let subtitleContent: string = `1\n00:00:01,000 --> 00:00:04,000\nHello and welcome to the presentation.\n\n2\n00:00:04,500 --> 00:00:08,000\nToday we demonstrate 100% offline machine translation.`;
  let subtitleFormat: 'srt' | 'vtt' | 'md' | 'txt' = 'srt';
  let translatedSubtitles: string = '';
  let isTranslatingSubs: boolean = false;
  let isBilingualSubtitles: boolean = false;
  let bilingualPrimaryTop: boolean = true;
  let copySubSuccess: boolean = false;
  let isDocDragOver: boolean = false;
  let parsedCuesPreview: Array<{ index: number; time: string; original: string; translated: string }> = [];

  // Batch Document Translation state
  let batchFiles: BatchDocumentItem[] = [];
  let batchResults: BatchTranslationResult[] = [];
  let isBatchTranslating: boolean = false;

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

  function loadSampleSubtitle(type: 'movie_srt' | 'web_vtt' | 'tech_md') {
    if (type === 'movie_srt') {
      subtitleFormat = 'srt';
      subtitleContent = `1\n00:00:01,200 --> 00:00:03,800\nWelcome to the international symposium.\n\n2\n00:00:04,100 --> 00:00:07,500\nToday we present private offline language translation.\n\n3\n00:00:08,000 --> 00:00:11,200\nAll neural inference occurs locally without cloud telemetry.`;
    } else if (type === 'web_vtt') {
      subtitleFormat = 'vtt';
      subtitleContent = `WEBVTT - Autonomous Navigation Subtitles\n\n00:00:01.500 --> 00:00:04.200\nObstacle detected 20 meters ahead.\n\n00:00:04.800 --> 00:00:08.500\nRerouting through secondary navigation corridor.`;
    } else if (type === 'tech_md') {
      subtitleFormat = 'md';
      subtitleContent = `# Architecture Specification\n\nThe translation engine operates in a sandboxed, zero-egress runtime environment.\n\n## Core Security Invariants\n- No telemetry or network egress\n- Offline neural weights verified by cryptographic digest\n\n\`\`\`rust\nfn verify_model_integrity(hash: &[u8]) -> bool {\n    // Guaranteed tamper-evident signature\n    true\n}\n\`\`\`\n\nPlease consult operational guidelines for additional details.`;
    }
    translatedSubtitles = '';
    parsedCuesPreview = [];
  }

  function handleDocFileDrop(e: DragEvent) {
    e.preventDefault();
    isDocDragOver = false;
    if (e.dataTransfer && e.dataTransfer.files.length > 0) {
      if (e.dataTransfer.files.length > 1 || docSubTab === 'batch') {
        docSubTab = 'batch';
        handleBatchFiles(e.dataTransfer.files);
      } else {
        loadDocFile(e.dataTransfer.files[0]);
      }
    }
  }

  function handleDocFileInput(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.files && input.files.length > 0) {
      if (input.files.length > 1 || docSubTab === 'batch') {
        docSubTab = 'batch';
        handleBatchFiles(input.files);
      } else {
        loadDocFile(input.files[0]);
      }
    }
  }

  function handleBatchFiles(files: FileList | File[]) {
    const list = Array.from(files);
    for (const file of list) {
      const name = file.name;
      const lower = name.toLowerCase();
      let format = 'txt';
      if (lower.endsWith('.srt')) format = 'srt';
      else if (lower.endsWith('.vtt')) format = 'vtt';
      else if (lower.endsWith('.md') || lower.endsWith('.markdown')) format = 'md';

      const reader = new FileReader();
      reader.onload = () => {
        const content = (reader.result as string) || '';
        batchFiles = [
          ...batchFiles.filter((b) => b.filename !== name),
          { filename: name, content, format },
        ];
      };
      reader.readAsText(file);
    }
  }

  function removeBatchFile(filename: string) {
    batchFiles = batchFiles.filter((b) => b.filename !== filename);
  }

  function clearBatchFiles() {
    batchFiles = [];
    batchResults = [];
  }

  async function runBatchTranslation() {
    if (batchFiles.length === 0) return;
    isBatchTranslating = true;
    try {
      const results = await tauriInvoke<BatchTranslationResult[]>(
        'batch_translate_documents_cmd',
        {
          documents: batchFiles,
          sourceLangStr: null,
          targetLangStr: targetLang,
          domainGlossary: domainGlossary || null,
        }
      );
      batchResults = results || [];
    } catch (e) {
      console.error('Batch translation failed:', e);
    } finally {
      isBatchTranslating = false;
    }
  }

  function loadDocFile(file: File) {
    const name = file.name.toLowerCase();
    if (name.endsWith('.srt')) {
      subtitleFormat = 'srt';
    } else if (name.endsWith('.vtt')) {
      subtitleFormat = 'vtt';
    } else if (name.endsWith('.md') || name.endsWith('.markdown')) {
      subtitleFormat = 'md';
    } else if (name.endsWith('.txt')) {
      subtitleFormat = 'txt';
    }

    const reader = new FileReader();
    reader.onload = () => {
      subtitleContent = (reader.result as string) || '';
      translatedSubtitles = '';
      parsedCuesPreview = [];
    };
    reader.readAsText(file);
  }

  async function translateSubtitles() {
    if (!subtitleContent.trim()) return;
    isTranslatingSubs = true;
    try {
      if (subtitleFormat === 'srt' || subtitleFormat === 'vtt') {
        const cmd = isBilingualSubtitles ? 'translate_subtitles_bilingual_cmd' : 'translate_subtitles_cmd';
        const res = await tauriInvoke<string>(cmd, {
          content: subtitleContent,
          formatStr: subtitleFormat,
          sourceLangStr: null,
          targetLangStr: targetLang,
          domainGlossary: domainGlossary || null,
          primaryTop: bilingualPrimaryTop,
        });
        translatedSubtitles = res;
        updateCuesPreview(subtitleContent, res);
      } else {
        const res = await tauriInvoke<string>('translate_document_cmd', {
          content: subtitleContent,
          format: subtitleFormat,
          sourceLangStr: null,
          targetLangStr: targetLang,
          domainGlossary: domainGlossary || null,
        });
        translatedSubtitles = res;
        parsedCuesPreview = [];
      }
    } catch (e) {
      console.error('Subtitle/document translation error:', e);
    } finally {
      isTranslatingSubs = false;
    }
  }

  function updateCuesPreview(src: string, tgt: string) {
    if (subtitleFormat !== 'srt' && subtitleFormat !== 'vtt') {
      parsedCuesPreview = [];
      return;
    }
    const srcBlocks = src.trim().split(/\n\s*\n/);
    const tgtBlocks = tgt.trim().split(/\n\s*\n/);
    const previews: Array<{ index: number; time: string; original: string; translated: string }> = [];

    for (let i = 0; i < srcBlocks.length; i++) {
      const sLines = srcBlocks[i].trim().split('\n');
      const tLines = (tgtBlocks[i] || '').trim().split('\n');
      if (sLines.length >= 2) {
        let time = '';
        let orig = '';
        let trans = '';
        if (sLines[1] && sLines[1].includes('-->')) {
          time = sLines[1].trim();
          orig = sLines.slice(2).join(' ');
          trans = tLines.length >= 2 && tLines[1].includes('-->') ? tLines.slice(2).join(' ') : tLines.join(' ');
        } else if (sLines[0] && sLines[0].includes('-->')) {
          time = sLines[0].trim();
          orig = sLines.slice(1).join(' ');
          trans = tLines.length >= 1 && tLines[0].includes('-->') ? tLines.slice(1).join(' ') : tLines.join(' ');
        }
        if (time) {
          previews.push({
            index: i + 1,
            time,
            original: orig,
            translated: trans,
          });
        }
      }
    }
    parsedCuesPreview = previews;
  }

  async function copySubtitles() {
    if (!translatedSubtitles) return;
    try {
      await navigator.clipboard.writeText(translatedSubtitles);
      copySubSuccess = true;
      setTimeout(() => (copySubSuccess = false), 2000);
    } catch (e) {
      console.error('Failed to copy to clipboard:', e);
    }
  }

  function downloadSubtitles() {
    const ext = subtitleFormat;
    const mime = ext === 'md' ? 'text/markdown;charset=utf-8' : 'text/plain;charset=utf-8';
    const blob = new Blob([translatedSubtitles], { type: mime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `translated_${targetLang}.${ext}`;
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
      <!-- Subtitle Mode Tabs & Glossary Selector -->
      <div class="doc-sub-nav">
        <div class="tab-pill-group">
          <button
            type="button"
            class="tab-pill"
            class:active={docSubTab === 'single'}
            on:click={() => (docSubTab = 'single')}
          >
            📄 Single Document
          </button>
          <button
            type="button"
            class="tab-pill"
            class:active={docSubTab === 'batch'}
            on:click={() => (docSubTab = 'batch')}
          >
            📚 Batch Translation {batchFiles.length > 0 ? `(${batchFiles.length})` : ''}
          </button>
        </div>

        <div class="glossary-selector-wrap">
          <span class="glossary-lbl">Terminology:</span>
          <select bind:value={domainGlossary} class="domain-select-sub">
            <option value="">General Domain</option>
            <option value="medical">Medical / Clinical</option>
            <option value="legal">Legal &amp; Compliance</option>
            <option value="aviation">Aviation &amp; Defense</option>
            <option value="software">Software Engineering</option>
          </select>
        </div>
      </div>

      {#if docSubTab === 'single'}
        <div class="subs-toolbar">
          <div class="toolbar-left">
            <span class="toolbar-label">Format:</span>
            <div class="format-toggle">
              <label class="radio-label" class:selected={subtitleFormat === 'srt'}>
                <input type="radio" bind:group={subtitleFormat} value="srt" />
                <span>.SRT</span>
              </label>
              <label class="radio-label" class:selected={subtitleFormat === 'vtt'}>
                <input type="radio" bind:group={subtitleFormat} value="vtt" />
                <span>.VTT</span>
              </label>
              <label class="radio-label" class:selected={subtitleFormat === 'md'}>
                <input type="radio" bind:group={subtitleFormat} value="md" />
                <span>.MD</span>
              </label>
              <label class="radio-label" class:selected={subtitleFormat === 'txt'}>
                <input type="radio" bind:group={subtitleFormat} value="txt" />
                <span>.TXT</span>
              </label>
            </div>
          </div>

          <div class="toolbar-center">
            <span class="toolbar-label">Templates:</span>
            <div class="sample-btn-group">
              <button type="button" class="sample-btn" on:click={() => loadSampleSubtitle('movie_srt')}>
                🎬 Movie (SRT)
              </button>
              <button type="button" class="sample-btn" on:click={() => loadSampleSubtitle('web_vtt')}>
                🌐 Web (VTT)
              </button>
              <button type="button" class="sample-btn" on:click={() => loadSampleSubtitle('tech_md')}>
                📝 Spec (MD)
              </button>
            </div>
          </div>

          {#if subtitleFormat === 'srt' || subtitleFormat === 'vtt'}
            <div class="bilingual-opt-row">
              <label class="bilingual-toggle-label" title="Render both original and translated lines simultaneously">
                <input type="checkbox" bind:checked={isBilingualSubtitles} />
                <span>🌐 Bilingual Subtitles (Dual-Language)</span>
              </label>
            </div>
          {/if}

          <button
            type="button"
            class="translate-sub-btn"
            disabled={isTranslatingSubs || !subtitleContent.trim()}
            on:click={translateSubtitles}
          >
            {#if isTranslatingSubs}
              <span class="spinner-sm"></span> Translating...
            {:else}
              ⚡ Translate {subtitleFormat.toUpperCase()}
            {/if}
          </button>
        </div>

        <div class="subs-panes">
          <!-- Input Pane -->
          <div
            class="sub-pane"
            role="region"
            aria-label="Subtitle or document source drop zone"
            class:drag-over={isDocDragOver}
            on:dragover|preventDefault={() => (isDocDragOver = true)}
            on:dragleave={() => (isDocDragOver = false)}
            on:drop={handleDocFileDrop}
          >
            <div class="pane-tag-row">
              <span class="pane-tag">
                Input {subtitleFormat.toUpperCase()} Source
                {#if subtitleContent.trim()}
                  <span class="stat-badge">
                    {subtitleContent.split('\n').filter(Boolean).length} lines • {subtitleContent.length} chars
                  </span>
                {/if}
              </span>
              <label class="file-load-btn">
                📂 Load File
                <input
                  type="file"
                  accept=".srt,.vtt,.md,.markdown,.txt"
                  on:change={handleDocFileInput}
                  style="display: none;"
                />
              </label>
            </div>
            <textarea
              bind:value={subtitleContent}
              placeholder={`Paste or drop .${subtitleFormat} content here...`}
              class="subs-area"
            ></textarea>
          </div>

          <!-- Output Pane -->
          <div class="sub-pane">
            <div class="pane-tag-row">
              <span class="pane-tag">
                Translated Output ({targetLang.toUpperCase()})
                {#if translatedSubtitles.trim()}
                  <span class="stat-badge">
                    {translatedSubtitles.split('\n').filter(Boolean).length} lines • {translatedSubtitles.length} chars
                  </span>
                {/if}
              </span>
              <div class="pane-actions">
                {#if translatedSubtitles}
                  <button type="button" class="action-btn copy-btn" on:click={copySubtitles}>
                    {copySubSuccess ? '✓ Copied' : '📋 Copy'}
                  </button>
                  <button type="button" class="action-btn download-btn" on:click={downloadSubtitles}>
                    ⬇ Download .{subtitleFormat}
                  </button>
                {/if}
              </div>
            </div>
            <textarea
              bind:value={translatedSubtitles}
              readonly
              placeholder="Preserved structural tags, timestamps, or code blocks will appear here..."
              class="subs-area output-sub-area"
            ></textarea>
          </div>
        </div>

        <!-- Cue Breakdown for Subtitles -->
        {#if parsedCuesPreview.length > 0}
          <div class="cues-preview-panel">
            <div class="panel-header-row">
              <h4 class="ocr-title">Timestamped Cues Breakdown ({parsedCuesPreview.length} Cues)</h4>
              <span class="latency-pill">Milliseconds Preserved</span>
            </div>
            <div class="cues-list">
              {#each parsedCuesPreview as cue}
                <div class="cue-card">
                  <div class="cue-meta">
                    <span class="cue-idx">#{cue.index}</span>
                    <span class="cue-time">{cue.time}</span>
                  </div>
                  <div class="cue-texts">
                    <p class="cue-orig">{cue.original}</p>
                    <p class="cue-trans">{cue.translated}</p>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      {:else}
        <!-- Batch Document Translation View -->
        <div class="batch-container">
          <div
            class="batch-drop-zone"
            role="region"
            aria-label="Batch document files drop zone"
            class:drag-over={isDocDragOver}
            on:dragover|preventDefault={() => (isDocDragOver = true)}
            on:dragleave={() => (isDocDragOver = false)}
            on:drop={handleDocFileDrop}
          >
            <div class="drop-prompt">
              <p class="drop-main">📁 Drag &amp; Drop multiple <strong>.txt, .md, .srt, .vtt</strong> files</p>
              <p class="drop-sub">Batch translate technical docs, subtitles, and specifications offline with glossaries</p>
              <label class="batch-browse-btn">
                Browse Files
                <input
                  type="file"
                  multiple
                  accept=".srt,.vtt,.md,.markdown,.txt"
                  on:change={handleDocFileInput}
                  style="display: none;"
                />
              </label>
            </div>
          </div>

          {#if batchFiles.length > 0}
            <div class="batch-actions-bar">
              <span class="batch-count">
                <strong>{batchFiles.length}</strong> file{batchFiles.length === 1 ? '' : 's'} queued
                ({batchFiles.reduce((acc, f) => acc + f.content.length, 0)} characters)
              </span>
              <div class="batch-btns">
                <button type="button" class="clear-batch-btn" on:click={clearBatchFiles}>
                  Clear All
                </button>
                <button
                  type="button"
                  class="run-batch-btn"
                  disabled={isBatchTranslating}
                  on:click={runBatchTranslation}
                >
                  {#if isBatchTranslating}
                    <span class="spinner-sm"></span> Batch Translating...
                  {:else}
                    ⚡ Translate All ({batchFiles.length} files)
                  {/if}
                </button>
              </div>
            </div>

            <div class="batch-cards-list">
              {#each batchFiles as f}
                {@const res = batchResults.find((r) => r.filename === f.filename)}
                <div class="batch-card" class:completed={res && res.success} class:error={res && !res.success}>
                  <div class="batch-card-header">
                    <span class="batch-file-badge">{f.format?.toUpperCase() || 'TXT'}</span>
                    <span class="batch-filename">{f.filename}</span>
                    <button type="button" class="remove-file-btn" on:click={() => removeBatchFile(f.filename)}>✕</button>
                  </div>
                  {#if res}
                    <div class="batch-card-result">
                      <div class="result-meta">
                        <span class="result-name">➔ {res.translated_filename}</span>
                        <span class="result-speed">{res.elapsed_ms.toFixed(1)} ms • {res.translated_chars} chars</span>
                      </div>
                      <div class="result-preview">
                        <pre>{res.translated_content.slice(0, 300)}{res.translated_content.length > 300 ? '...' : ''}</pre>
                      </div>
                    </div>
                  {:else}
                    <div class="batch-pending-hint">
                      {f.content.slice(0, 140)}{f.content.length > 140 ? '...' : ''}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
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

  /* Subtitles & Document View */
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
    gap: 1rem;
  }

  .toolbar-left, .toolbar-center {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .format-toggle {
    display: flex;
    gap: 0.35rem;
    background: var(--bg-tertiary);
    padding: 2px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px 8px;
    border-radius: 3px;
    transition: all 0.15s ease;
  }

  .radio-label input {
    display: none;
  }

  .radio-label.selected {
    background: rgba(56, 189, 248, 0.18);
    color: var(--accent-cyan);
    font-weight: 600;
  }

  .bilingual-opt-row {
    display: flex;
    align-items: center;
  }

  .bilingual-toggle-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
    color: var(--accent-cyan);
    cursor: pointer;
    background: rgba(56, 189, 248, 0.08);
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid rgba(56, 189, 248, 0.25);
  }

  .translate-sub-btn {
    background: var(--accent-primary);
    border: none;
    color: white;
    padding: 0.45rem 1rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
    transition: background 0.15s ease;
  }

  .translate-sub-btn:hover:not(:disabled) {
    background: #0284c7;
  }

  .translate-sub-btn:disabled {
    opacity: 0.45;
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
    transition: border-color 0.2s ease;
  }

  .sub-pane.drag-over {
    border-color: var(--accent-cyan);
    background: rgba(56, 189, 248, 0.05);
  }

  .pane-tag {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .stat-badge {
    font-size: 0.7rem;
    font-weight: 400;
    color: var(--text-faint);
    background: rgba(255, 255, 255, 0.04);
    padding: 1px 6px;
    border-radius: 4px;
    text-transform: none;
  }

  .pane-tag-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .pane-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .file-load-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    padding: 0.25rem 0.6rem;
    border-radius: var(--radius-sm);
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .file-load-btn:hover {
    border-color: rgba(255, 255, 255, 0.25);
  }

  .action-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    padding: 0.25rem 0.6rem;
    border-radius: var(--radius-sm);
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .copy-btn {
    color: var(--text-main);
  }

  .copy-btn:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .download-btn {
    color: var(--accent-emerald);
    border-color: rgba(16, 185, 129, 0.3);
  }

  .download-btn:hover {
    background: rgba(16, 185, 129, 0.1);
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
    min-height: 280px;
  }

  .output-sub-area {
    color: #e2e8f0;
  }

  /* Cues Preview Panel */
  .cues-preview-panel {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .cues-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 260px;
    overflow-y: auto;
  }

  .cue-card {
    display: flex;
    gap: 1rem;
    padding: 0.5rem 0.75rem;
    background: var(--bg-tertiary);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    align-items: flex-start;
  }

  .cue-meta {
    display: flex;
    flex-direction: column;
    min-width: 140px;
    gap: 2px;
  }

  .cue-idx {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--accent-cyan);
  }

  .cue-time {
    font-size: 0.7rem;
    font-family: var(--font-mono);
    color: var(--text-faint);
  }

  .cue-texts {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .cue-orig {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin: 0;
  }

  .cue-trans {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--text-main);
    margin: 0;
  }

  .doc-sub-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-card);
    padding: 0.5rem 0.8rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
  }

  .tab-pill-group {
    display: flex;
    gap: 0.5rem;
  }

  .tab-pill {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    padding: 0.35rem 0.75rem;
    border-radius: 9999px;
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tab-pill:hover {
    color: var(--text-main);
    background: rgba(255, 255, 255, 0.08);
  }

  .tab-pill.active {
    background: rgba(56, 189, 248, 0.15);
    border-color: rgba(56, 189, 248, 0.4);
    color: #38bdf8;
  }

  .glossary-selector-wrap {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .glossary-lbl {
    font-size: 0.8rem;
    color: var(--text-faint);
  }

  .domain-select-sub {
    background: var(--bg-pane);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    padding: 0.3rem 0.6rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
  }

  .batch-container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .batch-drop-zone {
    border: 2px dashed rgba(255, 255, 255, 0.15);
    border-radius: var(--radius-md);
    padding: 2.5rem 1.5rem;
    display: flex;
    justify-content: center;
    align-items: center;
    background: rgba(255, 255, 255, 0.02);
    transition: all 0.2s ease;
  }

  .batch-drop-zone.drag-over {
    border-color: #38bdf8;
    background: rgba(56, 189, 248, 0.08);
  }

  .drop-prompt {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 0.5rem;
  }

  .drop-main {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-main);
    margin: 0;
  }

  .drop-sub {
    font-size: 0.82rem;
    color: var(--text-muted);
    margin: 0;
  }

  .batch-browse-btn {
    margin-top: 0.5rem;
    background: rgba(56, 189, 248, 0.2);
    border: 1px solid rgba(56, 189, 248, 0.4);
    color: #38bdf8;
    padding: 0.4rem 1rem;
    border-radius: var(--radius-sm);
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
  }

  .batch-browse-btn:hover {
    background: rgba(56, 189, 248, 0.3);
  }

  .batch-actions-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-card);
    padding: 0.6rem 1rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
  }

  .batch-count {
    font-size: 0.85rem;
    color: var(--text-muted);
  }

  .batch-btns {
    display: flex;
    gap: 0.6rem;
  }

  .clear-batch-btn {
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    padding: 0.35rem 0.75rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    cursor: pointer;
  }

  .clear-batch-btn:hover {
    color: #f87171;
    border-color: rgba(248, 113, 113, 0.3);
  }

  .run-batch-btn {
    background: linear-gradient(135deg, #0284c7, #2563eb);
    color: #fff;
    border: none;
    padding: 0.4rem 1rem;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .run-batch-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .batch-cards-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .batch-card {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 0.8rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .batch-card.completed {
    border-color: rgba(34, 197, 94, 0.4);
  }

  .batch-card.error {
    border-color: rgba(239, 68, 68, 0.4);
  }

  .batch-card-header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .batch-file-badge {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-muted);
    font-size: 0.7rem;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    letter-spacing: 0.5px;
  }

  .batch-filename {
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-main);
    flex: 1;
  }

  .remove-file-btn {
    background: transparent;
    border: none;
    color: var(--text-faint);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .remove-file-btn:hover {
    color: #f87171;
  }

  .batch-card-result {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    padding-top: 0.5rem;
  }

  .result-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .result-name {
    font-size: 0.8rem;
    color: #4ade80;
    font-weight: 600;
  }

  .result-speed {
    font-size: 0.75rem;
    color: var(--text-faint);
  }

  .result-preview pre {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 0.78rem;
    color: var(--text-muted);
    background: rgba(0, 0, 0, 0.2);
    padding: 0.5rem;
    border-radius: 4px;
    white-space: pre-wrap;
    max-height: 80px;
    overflow-y: auto;
  }

  .batch-pending-hint {
    font-size: 0.78rem;
    color: var(--text-faint);
    font-style: italic;
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
